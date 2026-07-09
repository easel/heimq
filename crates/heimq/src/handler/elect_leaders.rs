//! ElectLeaders request handler (API Key 43)
//!
//! No-op stub: in a single-node setup the current broker is always the leader
//! for every partition, so every election succeeds immediately.

use crate::error::Result;
use bytes::Bytes;
use heimq_protocol::messages::elect_leaders_request::ElectLeadersRequest;
use heimq_protocol::messages::elect_leaders_response::{
    ElectLeadersResponse, PartitionResult, ReplicaElectionResult,
};
use heimq_protocol::messages::TopicName;
use heimq_protocol::protocol::{Decodable, StrBytes};

pub fn handle(api_version: i16, body: &[u8]) -> Result<ElectLeadersResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match ElectLeadersRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(ElectLeadersResponse::default()),
    };

    let mut response = ElectLeadersResponse::default();
    response.error_code = 0;

    if let Some(topic_partitions) = &request.topic_partitions {
        for tp in topic_partitions {
            let mut results: Vec<PartitionResult> = Vec::new();
            for &pid in &tp.partitions {
                let mut pr = PartitionResult::default();
                pr.partition_id = pid;
                pr.error_code = 0;
                results.push(pr);
            }
            let mut rel = ReplicaElectionResult::default();
            rel.topic = TopicName(StrBytes::from_string(tp.topic.0.as_str().to_string()));
            rel.partition_result = results;
            response.replica_election_results.push(rel);
        }
    }

    Ok(response)
}
