//! Group coordinator implementation

use crate::config::Config;
use std::sync::Arc;

/// The coordinator handles consumer group operations
pub struct Coordinator {
    config: Arc<Config>,
}

impl Coordinator {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Get the broker ID for this coordinator
    pub fn broker_id(&self) -> i32 {
        self.config.broker_id
    }

    /// Get the host for this coordinator
    pub fn host(&self) -> &str {
        if self.config.host == "0.0.0.0" {
            "localhost"
        } else {
            &self.config.host
        }
    }

    /// Get the port for this coordinator
    pub fn port(&self) -> i32 {
        self.config.port as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_coordinator_fields() {
        let mut config = Config::parse_from(["heimq"]);
        config.host = "0.0.0.0".to_string();
        config.port = 9099;
        config.broker_id = 7;
        let coordinator = Coordinator::new(Arc::new(config));

        assert_eq!(coordinator.broker_id(), 7);
        assert_eq!(coordinator.host(), "localhost");
        assert_eq!(coordinator.port(), 9099);

        let mut config = Config::parse_from(["heimq"]);
        config.host = "127.0.0.1".to_string();
        config.port = 9092;
        let coordinator = Coordinator::new(Arc::new(config));
        assert_eq!(coordinator.host(), "127.0.0.1");
    }
}
