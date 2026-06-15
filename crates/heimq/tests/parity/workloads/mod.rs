use crate::driver::WorkloadDriver;

mod consumer_group;
mod duplicate_sequence;
mod epoch_fence;
mod idempotent_produce;
mod out_of_order_sequence;
mod produce_fetch;
mod transactional_produce;

pub fn all() -> Vec<Box<dyn WorkloadDriver>> {
    vec![
        Box::new(produce_fetch::ProduceFetchRoundtrip),
        Box::new(consumer_group::ConsumerGroupLifecycle),
        Box::new(idempotent_produce::IdempotentProduceRoundtrip),
        Box::new(transactional_produce::TransactionalProduceRoundtrip),
        Box::new(epoch_fence::EpochFence),
        Box::new(duplicate_sequence::DuplicateSequence),
        Box::new(out_of_order_sequence::OutOfOrderSequence),
    ]
}
