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
