//! Transactional producer state (US-004).
//!
//! Tracks per-transactional_id state for Kafka EOS transactions.

use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::producer_state::ProducerStateManager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxnStatus {
    Empty,
    Ongoing,
    PrepareAbort,
    PrepareCommit,
    Dead,
}

pub struct TxnEntry {
    pub producer_id: i64,
    pub epoch: i16,
    /// topic+partition → first_offset at which this txn started producing
    pub partitions: HashMap<(String, i32), i64>,
    pub groups: HashSet<String>,
    /// topic+partition+group → (offset, metadata)
    pub pending_offsets: HashMap<(String, i32, String), (i64, Option<String>)>,
    pub status: TxnStatus,
}

pub struct AbortedRange {
    pub producer_id: i64,
    pub first_offset: i64,
}

pub struct TransactionManager {
    state: Mutex<TxnManagerState>,
}

struct TxnManagerState {
    transactions: HashMap<String, TxnEntry>,
    /// per-partition aborted transactions: (topic, partition) → Vec<AbortedRange>
    aborted: HashMap<(String, i32), Vec<AbortedRange>>,
    /// per-partition open txn min-offsets: (topic, partition) → BTreeMap<first_offset, producer_id>
    open_txns: HashMap<(String, i32), BTreeMap<i64, i64>>,
}

impl TransactionManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(TxnManagerState {
                transactions: HashMap::new(),
                aborted: HashMap::new(),
                open_txns: HashMap::new(),
            }),
        })
    }

    /// InitProducerId with transactional_id.
    /// Returns (producer_id, epoch).
    pub fn init_transactional_producer(&self, txn_id: &str) -> (i64, i16) {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => {
                let pid = ProducerStateManager::allocate_producer_id();
                state.transactions.insert(
                    txn_id.to_string(),
                    TxnEntry {
                        producer_id: pid,
                        epoch: 0,
                        partitions: HashMap::new(),
                        groups: HashSet::new(),
                        pending_offsets: HashMap::new(),
                        status: TxnStatus::Empty,
                    },
                );
                (pid, 0)
            }
            Some(entry) => {
                // Bump epoch on re-init (fencing previous instance)
                let new_epoch = entry.epoch.wrapping_add(1);
                entry.epoch = new_epoch;
                entry.partitions.clear();
                entry.groups.clear();
                entry.pending_offsets.clear();
                entry.status = TxnStatus::Empty;
                (entry.producer_id, new_epoch)
            }
        }
    }

    /// Validate epoch for an existing transaction.
    pub fn validate_epoch(&self, txn_id: &str, producer_id: i64, epoch: i16) -> bool {
        let state = self.state.lock();
        match state.transactions.get(txn_id) {
            Some(entry) => entry.producer_id == producer_id && entry.epoch == epoch,
            None => false,
        }
    }

    /// AddPartitionsToTxn: add a partition to the transaction.
    /// Returns error_code (0 = success, 47 = INVALID_PRODUCER_EPOCH).
    pub fn add_partitions(
        &self,
        txn_id: &str,
        producer_id: i64,
        epoch: i16,
        topic: &str,
        partition: i32,
        first_offset: i64,
    ) -> i16 {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => 47, // INVALID_PRODUCER_EPOCH
            Some(entry) => {
                if entry.producer_id != producer_id || entry.epoch != epoch {
                    return 47; // INVALID_PRODUCER_EPOCH
                }
                if entry.status == TxnStatus::Dead {
                    return 49; // INVALID_TXN_STATE
                }
                entry.status = TxnStatus::Ongoing;
                entry
                    .partitions
                    .entry((topic.to_string(), partition))
                    .or_insert(first_offset);
                0
            }
        }
    }

    /// AddOffsetsToTxn: add consumer group to transaction.
    /// Returns error_code.
    pub fn add_offsets(&self, txn_id: &str, producer_id: i64, epoch: i16, group_id: &str) -> i16 {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => 47,
            Some(entry) => {
                if entry.producer_id != producer_id || entry.epoch != epoch {
                    return 47;
                }
                if entry.status == TxnStatus::Dead {
                    return 49;
                }
                entry.status = TxnStatus::Ongoing;
                entry.groups.insert(group_id.to_string());
                0
            }
        }
    }

    /// Record that a transactional produce batch started at this offset.
    /// Only call when producer_id != -1 and batch is transactional.
    /// `txn_id` is used to update the transaction entry's first_offset,
    /// correcting the placeholder stored by AddPartitionsToTxn.
    pub fn record_produce(
        &self,
        txn_id: Option<&str>,
        topic: &str,
        partition: i32,
        first_offset: i64,
        producer_id: i64,
    ) {
        let mut state = self.state.lock();
        state
            .open_txns
            .entry((topic.to_string(), partition))
            .or_default()
            .entry(first_offset)
            .or_insert(producer_id);

        // Update the transaction entry's first_offset for this partition with the
        // actual storage offset, keeping the minimum across multiple batches.
        if let Some(txn_id) = txn_id {
            if let Some(entry) = state.transactions.get_mut(txn_id) {
                let key = (topic.to_string(), partition);
                entry
                    .partitions
                    .entry(key)
                    .and_modify(|existing| {
                        if first_offset < *existing {
                            *existing = first_offset;
                        }
                    })
                    .or_insert(first_offset);
            }
        }
    }

    /// TxnOffsetCommit: store pending transactional offsets.
    /// Returns error_code.
    pub fn commit_offset(
        &self,
        txn_id: &str,
        producer_id: i64,
        epoch: i16,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        metadata: Option<String>,
    ) -> i16 {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => 47,
            Some(entry) => {
                if entry.producer_id != producer_id || entry.epoch != epoch {
                    return 47;
                }
                if entry.status != TxnStatus::Ongoing {
                    return 49; // INVALID_TXN_STATE
                }
                entry.pending_offsets.insert(
                    (topic.to_string(), partition, group_id.to_string()),
                    (offset, metadata),
                );
                0
            }
        }
    }

    /// EndTxn: finalize the transaction.
    /// Returns (error_code, affected_partitions_with_first_offsets).
    /// The vec contains (topic, partition, first_offset, producer_id).
    pub fn end_txn(
        &self,
        txn_id: &str,
        producer_id: i64,
        epoch: i16,
        committed: bool,
    ) -> (i16, Vec<(String, i32, i64, i64)>) {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => (47, vec![]),
            Some(entry) => {
                if entry.producer_id != producer_id || entry.epoch != epoch {
                    return (47, vec![]);
                }
                if entry.status == TxnStatus::Dead {
                    return (49, vec![]);
                }

                let pid = entry.producer_id;

                // Collect affected partitions, skipping sentinels where no records were produced.
                let affected: Vec<(String, i32, i64, i64)> = entry
                    .partitions
                    .iter()
                    .filter(|(_, &fo)| fo != i64::MAX)
                    .map(|((t, p), fo)| (t.clone(), *p, *fo, pid))
                    .collect();

                if !committed {
                    // Abort: record aborted ranges in per-partition aborted list
                    for (topic, partition, first_offset, _) in &affected {
                        let key = (topic.clone(), *partition);
                        state
                            .aborted
                            .entry(key.clone())
                            .or_default()
                            .push(AbortedRange {
                                producer_id: pid,
                                first_offset: *first_offset,
                            });
                        // Remove from open_txns
                        if let Some(map) = state.open_txns.get_mut(&key) {
                            map.remove(first_offset);
                        }
                    }
                } else {
                    // Commit: remove from open_txns
                    for (topic, partition, first_offset, _) in &affected {
                        let key = (topic.clone(), *partition);
                        if let Some(map) = state.open_txns.get_mut(&key) {
                            map.remove(first_offset);
                        }
                    }
                }

                // Reset transaction state
                let entry = state.transactions.get_mut(txn_id).unwrap();
                let _pending = std::mem::take(&mut entry.pending_offsets);
                entry.partitions.clear();
                entry.groups.clear();
                entry.status = TxnStatus::Empty;

                (0, affected)
            }
        }
    }

    /// End txn and return pending offsets along with affected partitions.
    /// Returns (error_code, affected_partitions, pending_offsets).
    pub fn end_txn_with_offsets(
        &self,
        txn_id: &str,
        producer_id: i64,
        epoch: i16,
        committed: bool,
    ) -> (
        i16,
        Vec<(String, i32, i64, i64)>,
        HashMap<(String, i32, String), (i64, Option<String>)>,
    ) {
        let mut state = self.state.lock();
        match state.transactions.get_mut(txn_id) {
            None => (47, vec![], HashMap::new()),
            Some(entry) => {
                if entry.producer_id != producer_id || entry.epoch != epoch {
                    return (47, vec![], HashMap::new());
                }
                if entry.status == TxnStatus::Dead {
                    return (49, vec![], HashMap::new());
                }

                let pid = entry.producer_id;

                // Skip partitions where record_produce was never called (sentinel still in place).
                let affected: Vec<(String, i32, i64, i64)> = entry
                    .partitions
                    .iter()
                    .filter(|(_, &fo)| fo != i64::MAX)
                    .map(|((t, p), fo)| (t.clone(), *p, *fo, pid))
                    .collect();

                let pending = std::mem::take(&mut entry.pending_offsets);

                if !committed {
                    for (topic, partition, first_offset, _) in &affected {
                        let key = (topic.clone(), *partition);
                        state
                            .aborted
                            .entry(key.clone())
                            .or_default()
                            .push(AbortedRange {
                                producer_id: pid,
                                first_offset: *first_offset,
                            });
                        if let Some(map) = state.open_txns.get_mut(&key) {
                            map.remove(first_offset);
                        }
                    }
                } else {
                    for (topic, partition, first_offset, _) in &affected {
                        let key = (topic.clone(), *partition);
                        if let Some(map) = state.open_txns.get_mut(&key) {
                            map.remove(first_offset);
                        }
                    }
                }

                let entry = state.transactions.get_mut(txn_id).unwrap();
                entry.partitions.clear();
                entry.groups.clear();
                entry.status = TxnStatus::Empty;

                (0, affected, pending)
            }
        }
    }

    /// WriteTxnMarkers: apply markers.
    /// Returns list of (topic, partition, first_offset) for writing markers.
    pub fn apply_markers(
        &self,
        producer_id: i64,
        _epoch: i16,
        commit: bool,
    ) -> Vec<(String, i32, i64)> {
        let mut state = self.state.lock();
        let mut result = Vec::new();

        // Collect what to remove first, then mutate
        let mut to_remove: Vec<((String, i32), i64)> = Vec::new();
        for (key, map) in state.open_txns.iter() {
            for (&fo, &pid) in map.iter() {
                if pid == producer_id {
                    to_remove.push((key.clone(), fo));
                    result.push((key.0.clone(), key.1, fo));
                }
            }
        }

        // Now apply the removals and aborted recordings
        for (key, fo) in to_remove {
            if let Some(map) = state.open_txns.get_mut(&key) {
                map.remove(&fo);
            }
            if !commit {
                state.aborted.entry(key).or_default().push(AbortedRange {
                    producer_id,
                    first_offset: fo,
                });
            }
        }

        result
    }

    /// Get the LSO (Last Stable Offset) for a partition.
    /// LSO = min open transaction first_offset, or high_watermark if none open.
    pub fn get_lso(&self, topic: &str, partition: i32, high_watermark: i64) -> i64 {
        let state = self.state.lock();
        let key = (topic.to_string(), partition);
        match state.open_txns.get(&key) {
            Some(map) if !map.is_empty() => *map.keys().next().unwrap(),
            _ => high_watermark,
        }
    }

    /// Get aborted transactions overlapping [start_offset, ∞).
    /// Returns Vec<(producer_id, first_offset)>.
    pub fn get_aborted_transactions(
        &self,
        topic: &str,
        partition: i32,
        start_offset: i64,
    ) -> Vec<(i64, i64)> {
        let state = self.state.lock();
        let key = (topic.to_string(), partition);
        match state.aborted.get(&key) {
            None => vec![],
            Some(ranges) => ranges
                .iter()
                .filter(|r| r.first_offset >= start_offset)
                .map(|r| (r.producer_id, r.first_offset))
                .collect(),
        }
    }
}
