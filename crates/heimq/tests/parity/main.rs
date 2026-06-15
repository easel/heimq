mod bench;
mod broker;
mod diff;
mod driver;
mod exemptions;
mod normalize;
mod raw_protocol;
mod workloads;

use anyhow::Result;
use std::io::Write as _;

fn main() -> Result<()> {
    if std::env::var("PARITY_TESTS").is_err() {
        eprintln!("parity tests skipped (set PARITY_TESTS=1 to run; requires Docker)");
        return Ok(());
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(run())
}

async fn run() -> Result<()> {
    eprintln!("booting containers...");
    let (_redpanda_container, _kafka_container, _heimq_server, targets) = broker::boot().await?;
    eprintln!("containers ready");

    // Comparative throughput benchmark instead of the diff suite.
    if std::env::var("BENCH_COMPARE").is_ok() {
        let h = targets.heimq.bootstrap_servers.clone();
        let k = targets.kafka.bootstrap_servers.clone();
        let r = targets.redpanda.bootstrap_servers.clone();
        tokio::task::spawn_blocking(move || bench::run_compare(&h, &k, &r)).await??;
        return Ok(());
    }

    let workloads = workloads::all();
    let exemptions = exemptions::load()?;
    let mut any_fail = false;

    // heimq is diffed against each reference broker independently. Apache Kafka is the
    // canonical oracle; Redpanda is retained as a second, independent implementation.
    let oracles: [(&str, &broker::BrokerTarget); 2] =
        [("kafka", &targets.kafka), ("redpanda", &targets.redpanda)];

    let out_dir = std::path::Path::new("target/parity");
    std::fs::create_dir_all(out_dir)?;

    for w in &workloads {
        let heimq_obs = w.run(&targets.heimq).await?;
        let heimq_n = normalize::normalize(heimq_obs);

        for (oracle_name, oracle_target) in oracles {
            let oracle_obs = w.run(oracle_target).await?;
            let oracle_n = normalize::normalize(oracle_obs);

            let diffs = diff::diff(w.name(), oracle_name, &heimq_n, &oracle_n, &exemptions);
            let unmatched = diffs.iter().filter(|d| d.exemption.is_none()).count();

            let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
            let path = out_dir.join(format!("{}-{}-{}.jsonl", ts, oracle_name, w.name()));
            let mut f = std::fs::File::create(&path)?;
            for d in &diffs {
                writeln!(f, "{}", serde_json::to_string(d)?)?;
            }

            if unmatched == 0 {
                println!("[PASS] {} vs {}: 0 diffs", w.name(), oracle_name);
            } else {
                println!(
                    "[FAIL] {} vs {}: {} unmatched diffs",
                    w.name(),
                    oracle_name,
                    unmatched
                );
                for d in diffs.iter().filter(|d| d.exemption.is_none()) {
                    println!("  {}", serde_json::to_string(d)?);
                }
                any_fail = true;
            }
        }
    }

    if workloads.is_empty() {
        println!("[PASS] scaffolding: containers up, rdkafka connected, 0 workloads, 0 diffs");
    }

    if any_fail {
        anyhow::bail!("one or more parity workloads had unmatched diffs");
    }
    Ok(())
}
