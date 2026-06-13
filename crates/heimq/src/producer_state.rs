//! Idempotent producer session state (US-003).
//!
//! Tracks per-(producer_id, epoch, topic, partition) sequence numbers to
//! enforce Kafka's exactly-once sequence invariants within a server session.
//! Producer IDs are allocated from a global counter so they remain unique
//! across Router instances (connections) within a process.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

static GLOBAL_NEXT_PRODUCER_ID: AtomicI64 = AtomicI64::new(0);

/// Outcome of a sequence-number validation check.
pub enum SequenceCheck {
    /// Sequence is in-order; accept the batch and advance state.
    Accept,
    /// Sequence repeats a previously accepted batch (duplicate retry).
    Duplicate,
    /// Sequence is out-of-order (gap or non-zero first batch).
    OutOfOrder,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct ProducerPartitionKey {
    producer_id: i64,
    producer_epoch: i16,
    topic: String,
    partition: i32,
}

struct PartitionState {
    /// Next expected base_sequence for this (producer, partition).
    next_sequence: i32,
}

pub struct ProducerStateManager {
    states: Mutex<HashMap<ProducerPartitionKey, PartitionState>>,
}

impl ProducerStateManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            states: Mutex::new(HashMap::new()),
        })
    }

    /// Allocate a globally unique producer ID (for InitProducerId).
    pub fn allocate_producer_id() -> i64 {
        GLOBAL_NEXT_PRODUCER_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// Validate and, on success, advance the sequence state for a produce batch.
    ///
    /// `record_count` is the number of records in the batch; the expected
    /// next sequence advances by this amount on a successful accept.
    ///
    /// Returns `Accept` without touching state when `producer_id == -1`
    /// (non-idempotent producer).
    pub fn validate(
        &self,
        producer_id: i64,
        producer_epoch: i16,
        topic: &str,
        partition: i32,
        base_sequence: i32,
        record_count: i32,
    ) -> SequenceCheck {
        if producer_id == -1 {
            return SequenceCheck::Accept;
        }

        let key = ProducerPartitionKey {
            producer_id,
            producer_epoch,
            topic: topic.to_string(),
            partition,
        };

        let mut states = self.states.lock();
        match states.get_mut(&key) {
            None => {
                if base_sequence == 0 {
                    states.insert(key, PartitionState { next_sequence: record_count });
                    SequenceCheck::Accept
                } else {
                    // First batch from this producer on this partition must start at 0.
                    SequenceCheck::OutOfOrder
                }
            }
            Some(state) => {
                if base_sequence == state.next_sequence {
                    state.next_sequence = state.next_sequence.wrapping_add(record_count);
                    SequenceCheck::Accept
                } else if base_sequence < state.next_sequence {
                    SequenceCheck::Duplicate
                } else {
                    SequenceCheck::OutOfOrder
                }
            }
        }
    }
}
