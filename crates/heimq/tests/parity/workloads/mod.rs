use crate::driver::WorkloadDriver;

mod consumer_group;
mod idempotent_produce;
mod produce_fetch;
mod transactional_produce;

pub fn all() -> Vec<Box<dyn WorkloadDriver>> {
    vec![
        Box::new(produce_fetch::ProduceFetchRoundtrip),
        Box::new(consumer_group::ConsumerGroupLifecycle),
        Box::new(idempotent_produce::IdempotentProduceRoundtrip),
        Box::new(transactional_produce::TransactionalProduceRoundtrip),
    ]
}
