use crate::driver::Observation;
use crate::exemptions::Exemptions;
pub use heimq::test_support::DiffRecord;

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
