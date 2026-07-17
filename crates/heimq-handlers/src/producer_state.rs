//! Idempotent producer session state (US-003).
//!
//! Tracks per-(producer_id, topic, partition) committed epochs and sequence
//! numbers to enforce Kafka's exactly-once invariants within a server session.
//! Producer IDs are allocated from a global counter so they remain unique
//! across Router instances (connections) within a process.

use heimq_broker::produce::SequenceReservation;
use parking_lot::{Condvar, Mutex};
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
    state_changed: Condvar,
}

impl ProducerStateManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            states: Mutex::new(HashMap::new()),
            state_changed: Condvar::new(),
        })
    }

    /// Allocate a globally unique producer ID (for InitProducerId).
    pub fn allocate_producer_id() -> i64 {
        GLOBAL_NEXT_PRODUCER_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// Validate and reserve the sequence state for a produce batch.
    ///
    /// `record_count` is the number of records in the batch; the expected
    /// The returned reservation advances the next sequence by this amount only
    /// when committed after storage success. Dropping it rolls the reservation
    /// back. Concurrent validation for the same producer-partition waits for
    /// the in-flight reservation to resolve before making a decision.
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
        loop {
            if states
                .get(&key)
                .is_some_and(|state| state.reservation_pending)
            {
                self.state_changed.wait(&mut states);
                continue;
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

                return SequenceCheck::Accept(SequenceReservation::new(move |committed| {
                    self.complete_reservation(&key, producer_epoch, record_count, committed);
                }));
            }
            if !epoch_advanced && base_sequence < next_sequence {
                return SequenceCheck::Duplicate;
            }
            return SequenceCheck::OutOfOrder;
        }
    }

    fn complete_reservation(
        &self,
        key: &ProducerPartitionKey,
        producer_epoch: i16,
        record_count: i32,
        committed: bool,
    ) {
        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(key) {
            debug_assert!(state.reservation_pending);
            if committed {
                if state.producer_epoch == Some(producer_epoch) {
                    state.next_sequence = state.next_sequence.wrapping_add(record_count);
                } else {
                    state.producer_epoch = Some(producer_epoch);
                    state.next_sequence = record_count;
                }
            }
            state.reservation_pending = false;
        }
        self.state_changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

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
        }
    }

    #[test]
    fn rolled_back_sequence_can_be_retried() {
        let manager = ProducerStateManager::new();

        drop(reserve(&manager, 0));
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
    fn concurrent_next_sequence_waits_for_commit() {
        let manager = ProducerStateManager::new();
        let first = reserve(&manager, 0);
        let (started_tx, started_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let other_manager = manager.clone();

        let waiter = thread::spawn(move || {
            started_tx.send(()).expect("signal waiter start");
            let accepted = match other_manager.validate(7, 0, "events", 0, 1, 1) {
                SequenceCheck::Accept(reservation) => {
                    reservation.commit();
                    true
                }
                SequenceCheck::Duplicate | SequenceCheck::OutOfOrder => false,
            };
            result_tx.send(accepted).expect("send validation result");
        });

        started_rx.recv().expect("waiter started");
        assert!(matches!(
            result_rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));

        first.commit();
        assert!(result_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("waiter completed"));
        waiter.join().expect("waiter did not panic");
    }

    #[test]
    fn concurrent_same_sequence_becomes_duplicate_after_commit() {
        let manager = ProducerStateManager::new();
        let first = reserve(&manager, 0);
        let (started_tx, started_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let other_manager = manager.clone();

        let waiter = thread::spawn(move || {
            started_tx.send(()).expect("signal waiter start");
            let duplicate = matches!(
                other_manager.validate(7, 0, "events", 0, 0, 1),
                SequenceCheck::Duplicate
            );
            result_tx.send(duplicate).expect("send validation result");
        });

        started_rx.recv().expect("waiter started");
        assert!(matches!(
            result_rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));

        first.commit();
        assert!(result_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("waiter completed"));
        waiter.join().expect("waiter did not panic");
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
        drop(reserve_with_epoch(&manager, 4, 0));

        reserve_with_epoch(&manager, 3, 1).commit();
        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::Accept(_)
        ));
    }
}
