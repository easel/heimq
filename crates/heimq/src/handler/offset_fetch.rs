//! OffsetFetch request handler (API Key 9)

use crate::error::Result;
use crate::storage::{OffsetStore, RequestContext};
use bytes::Bytes;
use kafka_protocol::messages::offset_fetch_request::OffsetFetchRequest;
use kafka_protocol::messages::offset_fetch_response::{
    OffsetFetchResponseGroup, OffsetFetchResponsePartition, OffsetFetchResponsePartitions,
    OffsetFetchResponseTopic, OffsetFetchResponseTopics,
};
use kafka_protocol::messages::{GroupId, OffsetFetchResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
) -> Result<OffsetFetchResponse> {
    handle_with_context(api_version, body, offset_store, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
) -> Result<OffsetFetchResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match OffsetFetchRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(OffsetFetchResponse::default()),
    };

    let mut response = OffsetFetchResponse::default();

    if api_version >= 8 {
        // v8+: response uses `groups` (OffsetFetchResponseGroup).
        if let Some(group) = request.groups.into_iter().next() {
            let group_id = group.group_id.0.to_string();
            let mut resp_group = OffsetFetchResponseGroup::default();
            resp_group.group_id = GroupId(StrBytes::from_string(group_id.clone()));
            build_group_topics_v8(
                &group_id,
                group.topics.as_deref(),
                offset_store,
                ctx,
                &mut resp_group,
            );
            response.groups.push(resp_group);
        }
    } else {
        // v0-7: response uses top-level `topics` (OffsetFetchResponseTopic).
        let group_id = request.group_id.0.to_string();
        let topics_slice: Option<Vec<(String, &[i32])>> = request.topics.as_ref().map(|ts| {
            ts.iter()
                .map(|t| (t.name.0.to_string(), t.partition_indexes.as_slice()))
                .collect()
        });
        build_group_topics_v0_7(
            &group_id,
            topics_slice.as_deref(),
            offset_store,
            ctx,
            &mut response,
        );
        response.error_code = 0;
    }

    Ok(response)
}

fn build_group_topics_v8(
    group_id: &str,
    topics: Option<&[kafka_protocol::messages::offset_fetch_request::OffsetFetchRequestTopics]>,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
    resp_group: &mut OffsetFetchResponseGroup,
) {
    match topics {
        None => fetch_all_v8(group_id, offset_store, ctx, resp_group),
        Some(ts) => {
            for topic in ts {
                let topic_name = topic.name.0.to_string();
                let mut topic_resp = OffsetFetchResponseTopics::default();
                topic_resp.name = TopicName(StrBytes::from_string(topic_name.clone()));
                for &pi in &topic.partition_indexes {
                    topic_resp.partitions.push(build_partition_v8(
                        group_id,
                        &topic_name,
                        pi,
                        offset_store,
                        ctx,
                    ));
                }
                resp_group.topics.push(topic_resp);
            }
        }
    }
}

fn fetch_all_v8(
    group_id: &str,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
    resp_group: &mut OffsetFetchResponseGroup,
) {
    let all_offsets = offset_store.fetch_all_for_group_with_context(ctx, group_id);
    let mut topic_map: std::collections::HashMap<String, Vec<(i32, i64, Option<String>)>> =
        std::collections::HashMap::new();
    for ((topic, partition), committed) in all_offsets {
        topic_map
            .entry(topic)
            .or_default()
            .push((partition, committed.offset, committed.metadata));
    }
    for (topic_name, partitions) in topic_map {
        let mut topic_resp = OffsetFetchResponseTopics::default();
        topic_resp.name = TopicName(StrBytes::from_string(topic_name.clone()));
        for (partition, offset, metadata) in partitions {
            let mut p = OffsetFetchResponsePartitions::default();
            p.partition_index = partition;
            p.committed_offset = offset;
            p.metadata = metadata.map(StrBytes::from_string);
            p.error_code = 0;
            topic_resp.partitions.push(p);
        }
        resp_group.topics.push(topic_resp);
    }
}

fn build_partition_v8(
    group_id: &str,
    topic_name: &str,
    partition_index: i32,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
) -> OffsetFetchResponsePartitions {
    let mut p = OffsetFetchResponsePartitions::default();
    p.partition_index = partition_index;
    if let Some(committed) =
        offset_store.fetch_with_context(ctx, group_id, topic_name, partition_index)
    {
        p.committed_offset = committed.offset;
        p.metadata = committed.metadata.map(StrBytes::from_string);
        p.error_code = 0;
    } else {
        p.committed_offset = -1;
        p.error_code = 0;
    }
    p
}

fn build_group_topics_v0_7(
    group_id: &str,
    topics: Option<&[(String, &[i32])]>,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
    response: &mut OffsetFetchResponse,
) {
    match topics {
        None => fetch_all_v0_7(group_id, offset_store, ctx, response),
        Some(ts) => {
            for (topic_name, partition_indexes) in ts {
                let mut topic_response = OffsetFetchResponseTopic::default();
                topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));
                for &pi in *partition_indexes {
                    topic_response.partitions.push(build_partition_v0_7(
                        group_id,
                        topic_name,
                        pi,
                        offset_store,
                        ctx,
                    ));
                }
                response.topics.push(topic_response);
            }
        }
    }
}

fn fetch_all_v0_7(
    group_id: &str,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
    response: &mut OffsetFetchResponse,
) {
    let all_offsets = offset_store.fetch_all_for_group_with_context(ctx, group_id);
    let mut topic_map: std::collections::HashMap<String, Vec<(i32, i64, Option<String>)>> =
        std::collections::HashMap::new();
    for ((topic, partition), committed) in all_offsets {
        topic_map
            .entry(topic)
            .or_default()
            .push((partition, committed.offset, committed.metadata));
    }
    for (topic_name, partitions) in topic_map {
        let mut topic_response = OffsetFetchResponseTopic::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name));
        for (partition, offset, metadata) in partitions {
            let mut p = OffsetFetchResponsePartition::default();
            p.partition_index = partition;
            p.committed_offset = offset;
            p.metadata = metadata.map(StrBytes::from_string);
            p.error_code = 0;
            topic_response.partitions.push(p);
        }
        response.topics.push(topic_response);
    }
}

fn build_partition_v0_7(
    group_id: &str,
    topic_name: &str,
    partition_index: i32,
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
) -> OffsetFetchResponsePartition {
    let mut p = OffsetFetchResponsePartition::default();
    p.partition_index = partition_index;
    if let Some(committed) =
        offset_store.fetch_with_context(ctx, group_id, topic_name, partition_index)
    {
        p.committed_offset = committed.offset;
        p.metadata = committed.metadata.map(StrBytes::from_string);
        p.error_code = 0;
    } else {
        p.committed_offset = -1;
        p.error_code = 0;
    }
    p
}
