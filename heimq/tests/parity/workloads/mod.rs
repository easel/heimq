use crate::driver::WorkloadDriver;

pub fn all() -> Vec<Box<dyn WorkloadDriver>> {
    // Scaffolding bead: no workloads yet.
    // Subsequent beads add produce_fetch and consumer_group workloads.
    vec![]
}
