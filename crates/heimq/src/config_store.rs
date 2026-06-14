//! Per-topic dynamic configuration store.
//!
//! heimq honors a documented allow-list of topic-level config keys: AlterConfigs
//! (full replace) and IncrementalAlterConfigs (SET/DELETE) update the store, and
//! DescribeConfigs reflects the stored values. Keys outside the allow-list are
//! rejected with INVALID_CONFIG rather than silently accepted, so clients are
//! never misled into thinking an unsupported config took effect.
//!
//! Note: this records and round-trips the config values; it does not (yet)
//! enforce their runtime effect (e.g. retention/compaction). That enforcement is
//! tracked separately and is explicitly out of scope here.

use std::collections::HashMap;
use std::sync::Mutex;

/// INVALID_CONFIG (Kafka error code 40).
pub const INVALID_CONFIG: i16 = 40;

/// Topic config key for IncrementalAlterConfigs op codes.
pub const OP_SET: i8 = 0;
pub const OP_DELETE: i8 = 1;

/// Topic-level config keys heimq accepts, with their default values. Defaults
/// match what DescribeConfigs reports when no override is set.
pub const SUPPORTED_TOPIC_CONFIGS: &[(&str, &str)] = &[
    ("cleanup.policy", "delete"),
    ("retention.ms", "604800000"),
    ("segment.ms", "604800000"),
    ("compression.type", "producer"),
    ("min.insync.replicas", "1"),
    ("max.message.bytes", "1048588"),
];

pub fn is_supported(key: &str) -> bool {
    SUPPORTED_TOPIC_CONFIGS.iter().any(|(k, _)| *k == key)
}

/// One config op for IncrementalAlterConfigs: (key, op, value).
pub struct IncrementalOp<'a> {
    pub key: &'a str,
    pub op: i8,
    pub value: Option<&'a str>,
}

/// Per-topic dynamic config overrides. Only explicitly-set values are stored;
/// unset keys fall back to the defaults in [`SUPPORTED_TOPIC_CONFIGS`].
#[derive(Default)]
pub struct ConfigStore {
    // topic -> (key -> value)
    overrides: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl ConfigStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// AlterConfigs (full replace) for one topic resource. Validates every key
    /// first (returns `Err(INVALID_CONFIG)` with no partial application), then
    /// replaces the topic's override set with exactly the supplied pairs. A
    /// `None` value clears that key back to its default.
    pub fn alter_full(&self, topic: &str, configs: &[(String, Option<String>)]) -> Result<(), i16> {
        for (k, _) in configs {
            if !is_supported(k) {
                return Err(INVALID_CONFIG);
            }
        }
        let mut map = self.overrides.lock().unwrap();
        let entry = map.entry(topic.to_string()).or_default();
        entry.clear();
        for (k, v) in configs {
            if let Some(v) = v {
                entry.insert(k.clone(), v.clone());
            }
        }
        Ok(())
    }

    /// IncrementalAlterConfigs for one topic resource. Validates every op first
    /// (unsupported key or unsupported op -> `Err(INVALID_CONFIG)`, no partial
    /// application), then applies SET/DELETE. APPEND/SUBTRACT are not supported
    /// for heimq's scalar keys and are rejected.
    pub fn alter_incremental(&self, topic: &str, ops: &[IncrementalOp]) -> Result<(), i16> {
        for o in ops {
            if !is_supported(o.key) || (o.op != OP_SET && o.op != OP_DELETE) {
                return Err(INVALID_CONFIG);
            }
        }
        let mut map = self.overrides.lock().unwrap();
        let entry = map.entry(topic.to_string()).or_default();
        for o in ops {
            match o.op {
                OP_SET => match o.value {
                    Some(v) => {
                        entry.insert(o.key.to_string(), v.to_string());
                    }
                    None => {
                        entry.remove(o.key);
                    }
                },
                OP_DELETE => {
                    entry.remove(o.key);
                }
                _ => unreachable!("validated above"),
            }
        }
        Ok(())
    }

    /// Effective config for a topic: every supported key with its effective value
    /// and whether it is an explicit override. Defaults merged with overrides.
    pub fn effective(&self, topic: &str) -> Vec<(&'static str, String, bool)> {
        let map = self.overrides.lock().unwrap();
        let ov = map.get(topic);
        SUPPORTED_TOPIC_CONFIGS
            .iter()
            .map(|(k, default)| match ov.and_then(|m| m.get(*k)) {
                Some(v) => (*k, v.clone(), true),
                None => (*k, default.to_string(), false),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alter_full_round_trips_and_clears() {
        let s = ConfigStore::new();
        s.alter_full("t", &[("retention.ms".into(), Some("86400000".into()))])
            .unwrap();
        let eff = s.effective("t");
        let r = eff.iter().find(|(k, _, _)| *k == "retention.ms").unwrap();
        assert_eq!(r.1, "86400000");
        assert!(r.2, "should be marked overridden");

        // Full replace with empty set clears overrides back to default.
        s.alter_full("t", &[]).unwrap();
        let eff = s.effective("t");
        let r = eff.iter().find(|(k, _, _)| *k == "retention.ms").unwrap();
        assert_eq!(r.1, "604800000");
        assert!(!r.2);
    }

    #[test]
    fn incremental_set_then_delete_round_trips() {
        let s = ConfigStore::new();
        s.alter_incremental(
            "t",
            &[IncrementalOp { key: "retention.ms", op: OP_SET, value: Some("3600000") }],
        )
        .unwrap();
        assert_eq!(s.effective("t").iter().find(|(k, _, _)| *k == "retention.ms").unwrap().1, "3600000");

        s.alter_incremental("t", &[IncrementalOp { key: "retention.ms", op: OP_DELETE, value: None }])
            .unwrap();
        let r = s.effective("t");
        let e = r.iter().find(|(k, _, _)| *k == "retention.ms").unwrap();
        assert_eq!(e.1, "604800000");
        assert!(!e.2);
    }

    #[test]
    fn unsupported_key_rejected() {
        let s = ConfigStore::new();
        assert_eq!(
            s.alter_full("t", &[("bogus.key".into(), Some("1".into()))]),
            Err(INVALID_CONFIG)
        );
        assert_eq!(
            s.alter_incremental("t", &[IncrementalOp { key: "bogus.key", op: OP_SET, value: Some("1") }]),
            Err(INVALID_CONFIG)
        );
        // A valid key alongside an invalid one must not be partially applied.
        let _ = s.alter_full(
            "t",
            &[("retention.ms".into(), Some("5".into())), ("bogus".into(), Some("1".into()))],
        );
        assert!(!s.effective("t").iter().find(|(k, _, _)| *k == "retention.ms").unwrap().2);
    }
}
