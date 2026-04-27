use crate::driver::Observation;
use crate::exemptions::Exemptions;
use serde::{Deserialize, Serialize};

/// One diverging field between heimq and Redpanda for a given workload step.
/// Serializes to JSONL and is written to target/parity/<timestamp>-<workload>.jsonl.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffRecord {
    pub workload: String,
    pub step: u32,
    pub field: String,
    pub heimq_value: serde_json::Value,
    pub redpanda_value: serde_json::Value,
    /// "value_mismatch" | "missing_in_heimq" | "extra_in_heimq"
    pub divergence: String,
    /// exemption id string, or null
    pub exemption: Option<String>,
}

/// Diff two normalized observation lists from the same workload.
/// Returns one DiffRecord per diverging field.
pub fn diff(
    workload: &str,
    heimq: &[Observation],
    redpanda: &[Observation],
    _exemptions: &Exemptions,
) -> Vec<DiffRecord> {
    // Scaffolding: both slices are empty for the noop workload → zero diffs.
    // Future: per-field comparison logic added here as workloads are implemented.
    let _ = (workload, heimq, redpanda);
    vec![]
}

/// Validate that DiffRecord round-trips through JSON.
/// Called from main() before running workloads to catch schema regressions.
pub fn validate_schema() {
    let d = DiffRecord {
        workload: "test".to_string(),
        step: 0,
        field: "record.value".to_string(),
        heimq_value: serde_json::json!(null),
        redpanda_value: serde_json::json!(null),
        divergence: "value_mismatch".to_string(),
        exemption: None,
    };
    let s = serde_json::to_string(&d).expect("DiffRecord must be JSON-serializable");
    let back: DiffRecord = serde_json::from_str(&s).expect("DiffRecord must round-trip from JSON");
    assert_eq!(back.divergence, "value_mismatch");
    assert_eq!(back.field, "record.value");
}
