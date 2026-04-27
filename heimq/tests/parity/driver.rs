use crate::broker::BrokerTarget;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Observation {
    pub workload: &'static str,
    pub step: u32,
    pub event: ObservationEvent,
}

#[derive(Debug, Clone)]
pub enum ObservationEvent {
    RecordConsumed {
        key: Option<Bytes>,
        value: Option<Bytes>,
        headers: Vec<(String, Bytes)>,
        partition: i32,
        offset: i64,
        timestamp: i64,
    },
    ErrorCode {
        api: &'static str,
        code: i16,
    },
    GroupState {
        group_id: String,
        state: String,
        member_count: usize,
    },
    TxnOutcome {
        committed: bool,
        records_visible: bool,
    },
}

#[async_trait]
pub trait WorkloadDriver: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>>;
}
