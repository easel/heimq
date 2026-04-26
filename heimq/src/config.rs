//! Configuration for heimq server

use clap::Parser;
use std::path::PathBuf;

/// A fast, lightweight, single-node Kafka-compatible API server
#[derive(Parser, Debug, Clone)]
#[command(name = "heimq")]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0", env = "HEIMQ_HOST")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 9092, env = "HEIMQ_PORT")]
    pub port: u16,

    /// Data directory for persistence
    #[arg(long, default_value = "./data", env = "HEIMQ_DATA_DIR")]
    pub data_dir: PathBuf,

    /// Run in memory-only mode (no persistence, fastest)
    #[arg(long, default_value_t = false, env = "HEIMQ_MEMORY_ONLY")]
    pub memory_only: bool,

    /// Maximum segment size in bytes
    #[arg(long, default_value_t = 1024 * 1024 * 1024, env = "HEIMQ_SEGMENT_SIZE")]
    pub segment_size: u64,

    /// Message retention in milliseconds (default: 7 days)
    #[arg(long, default_value_t = 7 * 24 * 60 * 60 * 1000, env = "HEIMQ_RETENTION_MS")]
    pub retention_ms: u64,

    /// Number of partitions for auto-created topics
    #[arg(long, default_value_t = 1, env = "HEIMQ_DEFAULT_PARTITIONS")]
    pub default_partitions: i32,

    /// Enable auto-creation of topics
    #[arg(long, default_value_t = true, env = "HEIMQ_AUTO_CREATE_TOPICS")]
    pub auto_create_topics: bool,

    /// Broker ID (for Kafka protocol compatibility)
    #[arg(long, default_value_t = 0, env = "HEIMQ_BROKER_ID")]
    pub broker_id: i32,

    /// Cluster ID
    #[arg(long, default_value = "heimq-cluster", env = "HEIMQ_CLUSTER_ID")]
    pub cluster_id: String,

    /// Enable Prometheus metrics endpoint
    #[arg(long, default_value_t = false, env = "HEIMQ_METRICS")]
    pub metrics: bool,

    /// Metrics port
    #[arg(long, default_value_t = 9093, env = "HEIMQ_METRICS_PORT")]
    pub metrics_port: u16,

    /// Pre-create topics at startup. Format: "name:partitions", repeatable.
    /// Env var: comma-separated list.
    #[arg(long = "create-topic", env = "HEIMQ_CREATE_TOPICS", value_delimiter = ',')]
    pub create_topics: Vec<String>,

    /// URL for the log-storage backend. Currently only `memory://` is supported.
    #[arg(long = "storage-log", default_value = "memory://", env = "HEIMQ_STORAGE_LOG")]
    pub storage_log: String,

    /// URL for the consumer-group offset store. Currently only `memory://` is supported.
    #[arg(long = "storage-offsets", default_value = "memory://", env = "HEIMQ_STORAGE_OFFSETS")]
    pub storage_offsets: String,

    /// URL for the consumer-group coordinator backend. Currently only `memory://` is supported.
    #[arg(long = "storage-groups", default_value = "memory://", env = "HEIMQ_STORAGE_GROUPS")]
    pub storage_groups: String,
}

/// Storage backend URLs grouped together for ergonomic access.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub log: String,
    pub offsets: String,
    pub groups: String,
}

impl Config {
    /// Get the socket address to bind to
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Storage backend URLs as a single struct.
    pub fn storage(&self) -> StorageConfig {
        StorageConfig {
            log: self.storage_log.clone(),
            offsets: self.storage_offsets.clone(),
            groups: self.storage_groups.clone(),
        }
    }

    /// Get advertised listener for Kafka protocol
    #[allow(dead_code)]
    pub fn advertised_listener(&self) -> String {
        if self.host == "0.0.0.0" {
            format!("127.0.0.1:{}", self.port)  // Use IPv4 to avoid IPv6 resolution issues
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::parse_from::<[&str; 0], &str>([]);
        assert_eq!(config.port, 9092);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.memory_only);
    }

    #[test]
    fn test_memory_only_flag() {
        let config = Config::parse_from(["heimq", "--memory-only"]);
        assert!(config.memory_only);
    }

    #[test]
    fn test_advertised_listener() {
        let mut config = Config::parse_from(["heimq"]);
        config.port = 9094;
        config.host = "0.0.0.0".to_string();
        assert_eq!(config.advertised_listener(), "127.0.0.1:9094");

        config.host = "127.0.0.1".to_string();
        assert_eq!(config.advertised_listener(), "127.0.0.1:9094");
    }

    #[test]
    fn test_storage_defaults_to_memory_urls() {
        let config = Config::parse_from(["heimq"]);
        assert_eq!(config.storage_log, "memory://");
        assert_eq!(config.storage_offsets, "memory://");
        assert_eq!(config.storage_groups, "memory://");
        let storage = config.storage();
        assert_eq!(storage.log, "memory://");
        assert_eq!(storage.offsets, "memory://");
        assert_eq!(storage.groups, "memory://");
    }

    #[test]
    fn test_storage_overrides_via_flags() {
        let config = Config::parse_from([
            "heimq",
            "--storage-log",
            "memory://log",
            "--storage-offsets",
            "postgres://o",
            "--storage-groups",
            "weird://g",
        ]);
        let storage = config.storage();
        assert_eq!(storage.log, "memory://log");
        assert_eq!(storage.offsets, "postgres://o");
        assert_eq!(storage.groups, "weird://g");
    }

    #[test]
    fn test_bind_addr() {
        let mut config = Config::parse_from(["heimq"]);
        config.host = "127.0.0.1".to_string();
        config.port = 5555;
        assert_eq!(config.bind_addr(), "127.0.0.1:5555");
    }
}
