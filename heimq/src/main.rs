//! heimq: A fast, lightweight, single-node Kafka-compatible API server
//!
//! Focused on transport, simplicity, and speed at the expense of durability.

use heimq::config;
use heimq::server;

use clap::Parser;
use config::Config;
use server::Server;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "heimq=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

fn max_connections_from_env() -> Option<usize> {
    std::env::var("HEIMQ_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}

async fn run_with_config(config: Config, max_connections: Option<usize>) -> anyhow::Result<()> {
    init_tracing();

    info!(
        "Starting heimq v{} on {}:{}",
        env!("CARGO_PKG_VERSION"),
        config.host,
        config.port
    );

    if config.memory_only {
        info!("Running in memory-only mode (no persistence)");
    } else {
        info!("Data directory: {}", config.data_dir.display());
    }

    let server = Server::new(config).expect("valid server configuration");
    server.run_with_max_connections(max_connections).await?;

    Ok(())
}

fn config_from_env() -> Config {
    #[cfg(any(test, coverage))]
    {
        if let Ok(args) = std::env::var("HEIMQ_TEST_ARGS") {
            let mut argv = vec!["heimq".to_string()];
            argv.extend(args.split_whitespace().map(|arg| arg.to_string()));
            return Config::parse_from(argv);
        }
        return Config::parse_from(["heimq"]);
    }
    #[cfg(not(any(test, coverage)))]
    Config::parse()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let max_connections = max_connections_from_env();
    run_with_config(config_from_env(), max_connections).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::net::TcpStream;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    async fn run_once(memory_only: bool) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let port = listener.local_addr().expect("listener addr").port();
        drop(listener);

        let config = Config {
            host: "127.0.0.1".to_string(),
            port,
            data_dir: std::path::PathBuf::from("/tmp/heimq-test"),
            memory_only,
            segment_size: 1024 * 1024,
            retention_ms: 60_000,
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
        };

        let task = tokio::spawn(async move { run_with_config(config, Some(1)).await });

        for _ in 0..10 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        task.await.unwrap()
    }

    #[tokio::test]
    async fn test_run_with_config_memory_modes() {
        run_once(true).await.unwrap();
        run_once(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_run_with_config_bind_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let config = Config {
            host: "127.0.0.1".to_string(),
            port,
            data_dir: std::path::PathBuf::from("/tmp/heimq-test"),
            memory_only: true,
            segment_size: 1024 * 1024,
            retention_ms: 60_000,
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
        };

        let result = run_with_config(config, Some(1)).await;
        assert!(result.is_err());

        drop(listener);
    }

    #[test]
    fn test_max_connections_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("HEIMQ_MAX_CONNECTIONS", "2");
        assert_eq!(max_connections_from_env(), Some(2));
        std::env::remove_var("HEIMQ_MAX_CONNECTIONS");
        assert_eq!(max_connections_from_env(), None);
    }

    #[test]
    fn test_main_uses_env_args() {
        let _guard = ENV_LOCK.lock().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        std::env::set_var(
            "HEIMQ_TEST_ARGS",
            format!("--host 127.0.0.1 --port {} --memory-only", port),
        );
        std::env::set_var("HEIMQ_MAX_CONNECTIONS", "1");

        let config = config_from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, port);
        assert!(config.memory_only);

        let connector = std::thread::spawn(move || {
            for _ in 0..20 {
                if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        });

        let result = super::main();
        assert!(result.is_ok());
        connector.join().unwrap();

        std::env::remove_var("HEIMQ_TEST_ARGS");
        std::env::remove_var("HEIMQ_MAX_CONNECTIONS");
    }

    #[test]
    fn test_config_from_env_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("HEIMQ_TEST_ARGS");
        let config = config_from_env();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9092);
    }
}
