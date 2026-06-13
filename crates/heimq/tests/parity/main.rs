mod broker;
mod diff;
mod driver;
mod exemptions;
mod normalize;
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
    let (_redpanda_container, _heimq_server, targets) = broker::boot().await?;
    eprintln!("containers ready");

    let workloads = workloads::all();
    let exemptions = exemptions::load()?;
    let mut any_fail = false;

    let out_dir = std::path::Path::new("target/parity");
    std::fs::create_dir_all(out_dir)?;

    for w in &workloads {
        let heimq_obs = w.run(&targets.heimq).await?;
        let redpanda_obs = w.run(&targets.redpanda).await?;

        let heimq_n = normalize::normalize(heimq_obs);
        let redpanda_n = normalize::normalize(redpanda_obs);

        let diffs = diff::diff(w.name(), &heimq_n, &redpanda_n, &exemptions);
        let unmatched = diffs.iter().filter(|d| d.exemption.is_none()).count();

        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let path = out_dir.join(format!("{}-{}.jsonl", ts, w.name()));
        let mut f = std::fs::File::create(&path)?;
        for d in &diffs {
            writeln!(f, "{}", serde_json::to_string(d)?)?;
        }

        if unmatched == 0 {
            println!("[PASS] {}: 0 diffs", w.name());
        } else {
            println!("[FAIL] {}: {} unmatched diffs", w.name(), unmatched);
            for d in diffs.iter().filter(|d| d.exemption.is_none()) {
                println!("  {}", serde_json::to_string(d)?);
            }
            any_fail = true;
        }
    }

    if workloads.is_empty() {
        println!("[PASS] scaffolding: containers up, rdkafka connected, 0 workloads, 0 diffs");
    }

    if any_fail {
        std::process::exit(1);
    }
    Ok(())
}
