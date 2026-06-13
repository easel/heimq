use crate::driver::WorkloadDriver;

mod consumer_group;
mod produce_fetch;

pub fn all() -> Vec<Box<dyn WorkloadDriver>> {
    vec![
        Box::new(produce_fetch::ProduceFetchRoundtrip),
        Box::new(consumer_group::ConsumerGroupLifecycle),
    ]
}
