//! Idempotent producer session state (US-003).
//!
//! Tracks per-(producer_id, topic, partition) committed epochs and sequence
//! numbers to enforce Kafka's exactly-once invariants within a server session.
//! Producer IDs are allocated from a global counter so they remain unique
//! across Router instances (connections) within a process.

use heimq_broker::produce::{SequenceCompletion, SequenceReservation};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

static GLOBAL_NEXT_PRODUCER_ID: AtomicI64 = AtomicI64::new(0);

/// Outcome of a sequence-number validation check.
pub enum SequenceCheck<'a> {
    /// Sequence is in-order and reserved until storage completion.
    Accept(SequenceReservation<'a>),
    /// Sequence repeats a previously accepted batch (duplicate retry).
    Duplicate,
    /// Sequence is out-of-order (gap or non-zero first batch).
    OutOfOrder,
    /// Another append for this producer-partition is still in flight.
    InFlight,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct ProducerPartitionKey {
    producer_id: i64,
    topic: String,
    partition: i32,
}

struct PartitionState {
    /// Last producer epoch whose append committed successfully.
    producer_epoch: Option<i16>,
    /// Next expected base_sequence for this (producer, partition).
    next_sequence: i32,
    /// Only one append for a producer-partition may be in flight at a time.
    reservation_pending: bool,
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

    /// Validate and reserve the sequence state for a produce batch.
    ///
    /// `record_count` is the number of records in the batch. The returned
    /// reservation advances the next sequence by this amount only when
    /// committed after storage success. A definite storage error rolls it
    /// back; dropping it with an unknown storage outcome leaves the partition
    /// fenced. Concurrent validation for the same producer-partition returns
    /// [`SequenceCheck::InFlight`] without blocking the caller's executor.
    ///
    /// Returns `Accept` without touching state when `producer_id == -1`
    /// (non-idempotent producer).
    pub fn validate<'a>(
        &'a self,
        producer_id: i64,
        producer_epoch: i16,
        topic: &str,
        partition: i32,
        base_sequence: i32,
        record_count: i32,
    ) -> SequenceCheck<'a> {
        if producer_id == -1 {
            return SequenceCheck::Accept(SequenceReservation::new(|_| {}));
        }

        let key = ProducerPartitionKey {
            producer_id,
            topic: topic.to_string(),
            partition,
        };

        let mut states = self.states.lock();
        if states
            .get(&key)
            .is_some_and(|state| state.reservation_pending)
        {
            return SequenceCheck::InFlight;
        }

        let state = states.get(&key);
        let committed_epoch = state.and_then(|state| state.producer_epoch);
        if committed_epoch.is_some_and(|epoch| producer_epoch < epoch) {
            return SequenceCheck::OutOfOrder;
        }

        let epoch_advanced = committed_epoch.is_none_or(|epoch| producer_epoch > epoch);
        let next_sequence = state.map_or(0, |state| state.next_sequence);
        let expected_sequence = if epoch_advanced { 0 } else { next_sequence };
        if base_sequence == expected_sequence {
            states
                .entry(key.clone())
                .or_insert(PartitionState {
                    producer_epoch: None,
                    next_sequence: 0,
                    reservation_pending: false,
                })
                .reservation_pending = true;
            drop(states);

            return SequenceCheck::Accept(SequenceReservation::new(move |completion| {
                self.complete_reservation(&key, producer_epoch, record_count, completion);
            }));
        }
        if !epoch_advanced && base_sequence < next_sequence {
            return SequenceCheck::Duplicate;
        }
        SequenceCheck::OutOfOrder
    }

    fn complete_reservation(
        &self,
        key: &ProducerPartitionKey,
        producer_epoch: i16,
        record_count: i32,
        completion: SequenceCompletion,
    ) {
        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(key) {
            debug_assert!(state.reservation_pending);
            match completion {
                SequenceCompletion::Commit => {
                    if state.producer_epoch == Some(producer_epoch) {
                        state.next_sequence = state.next_sequence.wrapping_add(record_count);
                    } else {
                        state.producer_epoch = Some(producer_epoch);
                        state.next_sequence = record_count;
                    }
                    state.reservation_pending = false;
                }
                SequenceCompletion::Rollback => state.reservation_pending = false,
                SequenceCompletion::Abandon => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn reserve<'a>(
        manager: &'a ProducerStateManager,
        base_sequence: i32,
    ) -> SequenceReservation<'a> {
        reserve_with_epoch(manager, 0, base_sequence)
    }

    fn reserve_with_epoch(
        manager: &ProducerStateManager,
        producer_epoch: i16,
        base_sequence: i32,
    ) -> SequenceReservation<'_> {
        match manager.validate(7, producer_epoch, "events", 0, base_sequence, 1) {
            SequenceCheck::Accept(reservation) => reservation,
            SequenceCheck::Duplicate => panic!("expected reservation, got duplicate"),
            SequenceCheck::OutOfOrder => panic!("expected reservation, got out of order"),
            SequenceCheck::InFlight => panic!("expected reservation, got in flight"),
        }
    }

    #[test]
    fn rolled_back_sequence_can_be_retried() {
        let manager = ProducerStateManager::new();

        reserve(&manager, 0).rollback();
        reserve(&manager, 0).commit();

        assert!(matches!(
            manager.validate(7, 0, "events", 0, 0, 1),
            SequenceCheck::Duplicate
        ));
    }

    #[test]
    fn committed_sequence_advances_exactly_once() {
        let manager = ProducerStateManager::new();

        reserve(&manager, 0).commit();
        reserve(&manager, 1).commit();

        assert!(matches!(
            manager.validate(7, 0, "events", 0, 2, 1),
            SequenceCheck::Accept(_)
        ));
    }

    #[test]
    fn concurrent_next_sequence_returns_in_flight_then_accepts_after_commit() {
        let manager = ProducerStateManager::new();
        let first = reserve(&manager, 0);

        assert!(matches!(
            manager.validate(7, 0, "events", 0, 1, 1),
            SequenceCheck::InFlight
        ));

        first.commit();
        reserve(&manager, 1).commit();
    }

    #[test]
    fn concurrent_same_sequence_returns_in_flight_then_duplicate_after_commit() {
        let manager = ProducerStateManager::new();
        let first = reserve(&manager, 0);

        assert!(matches!(
            manager.validate(7, 0, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));

        first.commit();
        assert!(matches!(
            manager.validate(7, 0, "events", 0, 0, 1),
            SequenceCheck::Duplicate
        ));
    }

    #[test]
    fn current_thread_async_poll_does_not_block_on_pending_sequence() {
        let manager = ProducerStateManager::new();
        let mut future = Box::pin(async {
            let first = reserve(&manager, 0);
            assert!(matches!(
                manager.validate(7, 0, "events", 0, 1, 1),
                SequenceCheck::InFlight
            ));
            first.rollback();
        });
        let mut context = Context::from_waker(Waker::noop());

        assert!(matches!(
            future.as_mut().poll(&mut context),
            Poll::Ready(())
        ));
    }

    #[test]
    fn newer_epoch_resets_sequence_and_fences_older_epoch_after_commit() {
        let manager = ProducerStateManager::new();
        reserve_with_epoch(&manager, 3, 0).commit();
        reserve_with_epoch(&manager, 4, 0).commit();

        assert!(matches!(
            manager.validate(7, 3, "events", 0, 1, 1),
            SequenceCheck::OutOfOrder
        ));
        assert!(matches!(
            manager.validate(7, 4, "events", 0, 1, 1),
            SequenceCheck::Accept(_)
        ));
    }

    #[test]
    fn failed_newer_epoch_does_not_fence_committed_epoch() {
        let manager = ProducerStateManager::new();
        reserve_with_epoch(&manager, 3, 0).commit();
        reserve_with_epoch(&manager, 4, 0).rollback();

        reserve_with_epoch(&manager, 3, 1).commit();
        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::Accept(_)
        ));
    }

    #[test]
    fn abandoned_sequence_remains_fenced_as_in_flight() {
        let manager = ProducerStateManager::new();

        drop(reserve(&manager, 0));

        assert!(matches!(
            manager.validate(7, 0, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));
    }
}
