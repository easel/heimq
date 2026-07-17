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
    /// Only one append for a producer-partition may be reserved at a time.
    reservation: Option<ReservationState>,
    /// Highest valid sequence-zero epoch observed while another reservation
    /// was active. Older epochs remain fenced after that reservation resolves.
    fence_epoch: Option<i16>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReservationState {
    /// The append is still being driven to a terminal storage result.
    Active { producer_epoch: i16 },
    /// The append future was dropped with an unknown storage result.
    Abandoned { producer_epoch: i16 },
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
    /// fenced until a strictly newer epoch restarts at sequence zero.
    /// Concurrent validation for the same producer-partition returns
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
            .and_then(|state| state.fence_epoch)
            .is_some_and(|epoch| producer_epoch < epoch)
        {
            return SequenceCheck::OutOfOrder;
        }

        if let Some(reservation) = states.get(&key).and_then(|state| state.reservation) {
            match reservation {
                ReservationState::Active {
                    producer_epoch: active_epoch,
                } => {
                    if producer_epoch > active_epoch {
                        if base_sequence != 0 {
                            return SequenceCheck::OutOfOrder;
                        }
                        let state = states
                            .get_mut(&key)
                            .expect("reservation state must have a partition state");
                        state.fence_epoch = Some(
                            state
                                .fence_epoch
                                .map_or(producer_epoch, |epoch| epoch.max(producer_epoch)),
                        );
                    }
                    return SequenceCheck::InFlight;
                }
                ReservationState::Abandoned {
                    producer_epoch: abandoned_epoch,
                } if producer_epoch <= abandoned_epoch => return SequenceCheck::InFlight,
                ReservationState::Abandoned { .. } => {
                    if base_sequence != 0 {
                        return SequenceCheck::OutOfOrder;
                    }
                    let state = states
                        .get_mut(&key)
                        .expect("reservation state must have a partition state");
                    state.reservation = Some(ReservationState::Active { producer_epoch });
                    state.fence_epoch = Some(
                        state
                            .fence_epoch
                            .map_or(producer_epoch, |epoch| epoch.max(producer_epoch)),
                    );
                    drop(states);

                    return SequenceCheck::Accept(SequenceReservation::new(move |completion| {
                        self.complete_reservation(&key, producer_epoch, record_count, completion);
                    }));
                }
            }
        }

        let state = states.get(&key);
        let committed_epoch =
            state.and_then(|state| match (state.producer_epoch, state.fence_epoch) {
                (Some(committed), Some(fence)) => Some(committed.max(fence)),
                (Some(committed), None) => Some(committed),
                (None, Some(fence)) => Some(fence),
                (None, None) => None,
            });
        if committed_epoch.is_some_and(|epoch| producer_epoch < epoch) {
            return SequenceCheck::OutOfOrder;
        }

        let epoch_advanced = state
            .and_then(|state| state.producer_epoch)
            .is_none_or(|epoch| producer_epoch > epoch);
        let next_sequence = state.map_or(0, |state| state.next_sequence);
        let expected_sequence = if epoch_advanced { 0 } else { next_sequence };
        if base_sequence == expected_sequence {
            let state = states.entry(key.clone()).or_insert(PartitionState {
                producer_epoch: None,
                next_sequence: 0,
                reservation: None,
                fence_epoch: None,
            });
            state.reservation = Some(ReservationState::Active { producer_epoch });
            if epoch_advanced {
                state.fence_epoch = Some(
                    state
                        .fence_epoch
                        .map_or(producer_epoch, |epoch| epoch.max(producer_epoch)),
                );
            }
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
            if state.reservation != Some(ReservationState::Active { producer_epoch }) {
                debug_assert!(false, "completion must match the active reservation epoch");
                return;
            }
            match completion {
                SequenceCompletion::Commit => {
                    if state.producer_epoch == Some(producer_epoch) {
                        state.next_sequence = state.next_sequence.wrapping_add(record_count);
                    } else {
                        state.producer_epoch = Some(producer_epoch);
                        state.next_sequence = record_count;
                    }
                    if state
                        .fence_epoch
                        .is_some_and(|epoch| epoch <= producer_epoch)
                    {
                        state.fence_epoch = None;
                    }
                    state.reservation = None;
                }
                SequenceCompletion::Rollback => state.reservation = None,
                SequenceCompletion::Abandon => {
                    state.reservation = Some(ReservationState::Abandoned { producer_epoch });
                }
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
    fn rolled_back_newer_epoch_still_fences_committed_epoch() {
        let manager = ProducerStateManager::new();
        reserve_with_epoch(&manager, 3, 0).commit();
        reserve_with_epoch(&manager, 4, 0).rollback();

        assert!(matches!(
            manager.validate(7, 3, "events", 0, 1, 1),
            SequenceCheck::OutOfOrder
        ));
        reserve_with_epoch(&manager, 4, 0).commit();
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

    #[test]
    fn newer_epoch_recovers_abandoned_sequence_and_fences_old_epoch() {
        let manager = ProducerStateManager::new();
        reserve_with_epoch(&manager, 3, 0).commit();
        drop(reserve_with_epoch(&manager, 3, 1));

        assert!(matches!(
            manager.validate(7, 3, "events", 0, 1, 1),
            SequenceCheck::InFlight
        ));
        assert!(matches!(
            manager.validate(7, 2, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));
        reserve_with_epoch(&manager, 4, 0).commit();

        assert!(matches!(
            manager.validate(7, 3, "events", 0, 1, 1),
            SequenceCheck::OutOfOrder
        ));
        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::Duplicate
        ));
    }

    #[test]
    fn newer_epoch_must_restart_at_zero_to_recover_abandoned_sequence() {
        let manager = ProducerStateManager::new();
        drop(reserve_with_epoch(&manager, 3, 0));

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 1, 1),
            SequenceCheck::OutOfOrder
        ));
        reserve_with_epoch(&manager, 4, 0).commit();
    }

    #[test]
    fn active_reservation_cannot_be_preempted_by_newer_epoch() {
        let manager = ProducerStateManager::new();
        let active = reserve_with_epoch(&manager, 3, 0);

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));

        active.rollback();
        reserve_with_epoch(&manager, 4, 0).commit();
    }

    #[test]
    fn observed_newer_epoch_fence_survives_active_rollback() {
        let manager = ProducerStateManager::new();
        let active = reserve_with_epoch(&manager, 3, 0);

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));
        active.rollback();

        assert!(matches!(
            manager.validate(7, 3, "events", 0, 0, 1),
            SequenceCheck::OutOfOrder
        ));
        reserve_with_epoch(&manager, 4, 0).commit();
    }

    #[test]
    fn highest_observed_sequence_zero_epoch_wins_fence_intent() {
        let manager = ProducerStateManager::new();
        let active = reserve_with_epoch(&manager, 3, 0);

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));
        assert!(matches!(
            manager.validate(7, 5, "events", 0, 0, 1),
            SequenceCheck::InFlight
        ));
        active.rollback();

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 0, 1),
            SequenceCheck::OutOfOrder
        ));
        reserve_with_epoch(&manager, 5, 0).commit();
    }

    #[test]
    fn higher_nonzero_sequence_does_not_install_fence_intent() {
        let manager = ProducerStateManager::new();
        let active = reserve_with_epoch(&manager, 3, 0);

        assert!(matches!(
            manager.validate(7, 4, "events", 0, 1, 1),
            SequenceCheck::OutOfOrder
        ));
        active.rollback();

        reserve_with_epoch(&manager, 3, 0).commit();
    }

    #[test]
    fn lower_epoch_during_active_reservation_is_fenced() {
        let manager = ProducerStateManager::new();
        let active = reserve_with_epoch(&manager, 3, 0);

        assert!(matches!(
            manager.validate(7, 2, "events", 0, 0, 1),
            SequenceCheck::OutOfOrder
        ));
        active.rollback();

        reserve_with_epoch(&manager, 3, 0).commit();
    }
}
