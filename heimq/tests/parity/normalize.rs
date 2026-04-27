use crate::driver::Observation;

/// Apply field-level normalization transforms to remove non-determinism
/// before diffing. Rules follow HARNESS-001 §Normalization Layer.
///
/// For the scaffolding bead, observations are always empty (no workloads);
/// this function is a no-op placeholder for future NormRule expansion.
pub fn normalize(observations: Vec<Observation>) -> Vec<Observation> {
    observations.into_iter().map(normalize_one).collect()
}

fn normalize_one(obs: Observation) -> Observation {
    // Future: apply NormRule table (broker_id→0, cluster_id→"<cluster_id>",
    // leader_epoch→0, timestamps, producer_id, member_id, generation_id).
    obs
}
