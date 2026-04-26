//! URL-scheme dispatch for pluggable storage backends.
//!
//! Each `dispatch_*` function parses the URL scheme of its input and returns
//! an `Arc<dyn ...>` implementing the relevant trait. Only the `memory://`
//! scheme is supported at this step; unknown schemes return a configuration
//! error so the server fails fast at startup rather than at first request.

use crate::config::Config;
use crate::consumer_group::{ConsumerGroupManager, GroupCoordinatorBackend, MemoryOffsetStore};
use crate::error::{HeimqError, Result};
use crate::storage::{LogBackend, MemoryLog, OffsetStore};
use std::sync::Arc;

/// Parse the scheme prefix of `url` (everything before `://`).
fn parse_scheme(url: &str) -> Result<&str> {
    url.split_once("://")
        .map(|(scheme, _)| scheme)
        .ok_or_else(|| {
            HeimqError::Config(format!(
                "storage URL missing scheme (expected `<scheme>://...`): {}",
                url
            ))
        })
}

/// Dispatch a log-backend URL to a concrete implementation.
pub fn dispatch_log_backend(url: &str, config: Arc<Config>) -> Result<Arc<dyn LogBackend>> {
    let scheme = parse_scheme(url)?;
    match scheme {
        "memory" => Ok(Arc::new(MemoryLog::new(config))),
        other => Err(HeimqError::Config(format!(
            "unsupported log-backend scheme `{}://` in URL `{}` (supported: memory://)",
            other, url
        ))),
    }
}

/// Dispatch an offset-store URL to a concrete implementation.
pub fn dispatch_offset_store(url: &str) -> Result<Arc<dyn OffsetStore>> {
    let scheme = parse_scheme(url)?;
    match scheme {
        "memory" => Ok(Arc::new(MemoryOffsetStore::new())),
        other => Err(HeimqError::Config(format!(
            "unsupported offset-store scheme `{}://` in URL `{}` (supported: memory://)",
            other, url
        ))),
    }
}

/// Dispatch a group-coordinator URL to a concrete implementation.
///
/// The memory backend is the existing `ConsumerGroupManager`; callers pass it
/// in (already wired to the chosen offset store) so the server keeps a single
/// instance for the rest of the wiring.
pub fn dispatch_group_coordinator(
    url: &str,
    memory_manager: Arc<ConsumerGroupManager>,
) -> Result<Arc<dyn GroupCoordinatorBackend>> {
    let scheme = parse_scheme(url)?;
    match scheme {
        "memory" => Ok(memory_manager),
        other => Err(HeimqError::Config(format!(
            "unsupported group-coordinator scheme `{}://` in URL `{}` (supported: memory://)",
            other, url
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn cfg() -> Arc<Config> {
        Arc::new(Config::parse_from(["heimq"]))
    }

    #[test]
    fn dispatches_memory_log() {
        let backend = dispatch_log_backend("memory://", cfg()).unwrap();
        assert_eq!(backend.capabilities().name, "in-memory");
    }

    #[test]
    fn dispatches_memory_offsets() {
        let store = dispatch_offset_store("memory://").unwrap();
        store.commit("g", "t", 0, 7, 0, None);
        assert_eq!(store.fetch("g", "t", 0).unwrap().offset, 7);
    }

    #[test]
    fn dispatches_memory_groups() {
        let mgr = Arc::new(ConsumerGroupManager::new(cfg()));
        let coord = dispatch_group_coordinator("memory://", mgr).unwrap();
        assert_eq!(coord.capabilities().name, "in-memory");
    }

    fn err_msg<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected error"),
            Err(e) => format!("{}", e),
        }
    }

    #[test]
    fn unknown_log_scheme_errors() {
        let msg = err_msg(dispatch_log_backend("weird://x", cfg()));
        assert!(msg.contains("weird"), "msg = {}", msg);
        assert!(msg.contains("memory://"), "msg = {}", msg);
    }

    #[test]
    fn unknown_offset_scheme_errors() {
        let msg = err_msg(dispatch_offset_store("postgres://x"));
        assert!(msg.contains("postgres"));
    }

    #[test]
    fn unknown_group_scheme_errors() {
        let mgr = Arc::new(ConsumerGroupManager::new(cfg()));
        let msg = err_msg(dispatch_group_coordinator("weird://", mgr));
        assert!(msg.contains("weird"));
    }

    #[test]
    fn missing_scheme_errors() {
        let msg = err_msg(dispatch_log_backend("memory", cfg()));
        assert!(msg.contains("missing scheme"));
    }
}
