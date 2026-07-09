use anyhow::{Context, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use heimq_protocol::messages::api_versions_request::ApiVersionsRequest;
use heimq_protocol::messages::api_versions_response::ApiVersionsResponse;
use heimq_protocol::messages::init_producer_id_request::InitProducerIdRequest;
use heimq_protocol::messages::init_producer_id_response::InitProducerIdResponse;
use heimq_protocol::messages::produce_request::{
    PartitionProduceData, ProduceRequest, TopicProduceData,
};
use heimq_protocol::messages::produce_response::ProduceResponse;
use heimq_protocol::messages::{ProducerId, TopicName};
use heimq_protocol::protocol::{Decodable, Encodable, StrBytes};
use heimq_protocol::records::{
    Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const CLIENT_ID: &str = "heimq-parity-raw";
const SOCKET_TIMEOUT: Duration = Duration::from_secs(15);

pub struct RawKafkaClient {
    stream: TcpStream,
    correlation_id: i32,
}

impl RawKafkaClient {
    pub fn connect(bootstrap: &str) -> Result<Self> {
        let stream = TcpStream::connect(bootstrap)
            .with_context(|| format!("connect raw protocol client to {bootstrap}"))?;
        stream.set_read_timeout(Some(SOCKET_TIMEOUT))?;
        stream.set_write_timeout(Some(SOCKET_TIMEOUT))?;
        Ok(Self {
            stream,
            correlation_id: 1,
        })
    }

    pub fn api_versions(&mut self) -> Result<ApiVersionsResponse> {
        self.send(18, 0, &ApiVersionsRequest::default())
            .context("ApiVersions v0")
    }

    pub fn init_producer_id(&mut self) -> Result<(i64, i16)> {
        let mut request = InitProducerIdRequest::default();
        request.transactional_id = None;
        request.transaction_timeout_ms = 60_000;
        request.producer_id = ProducerId(-1);
        request.producer_epoch = -1;

        let response: InitProducerIdResponse =
            self.send(22, 0, &request).context("InitProducerId v0")?;
        if response.error_code != 0 {
            anyhow::bail!("InitProducerId returned error {}", response.error_code);
        }
        Ok((response.producer_id.0, response.producer_epoch))
    }

    pub fn produce_sequence(
        &mut self,
        topic: &str,
        producer_id: i64,
        producer_epoch: i16,
        base_sequence: i32,
    ) -> Result<i16> {
        let mut partition = PartitionProduceData::default();
        partition.index = 0;
        partition.records = Some(encode_idempotent_batch(
            producer_id,
            producer_epoch,
            base_sequence,
        )?);

        let mut topic_data = TopicProduceData::default();
        topic_data.name = TopicName(StrBytes::from_string(topic.to_string()));
        topic_data.partition_data = vec![partition];

        let mut request = ProduceRequest::default();
        request.transactional_id = None;
        request.acks = 1;
        request.timeout_ms = 15_000;
        request.topic_data = vec![topic_data];

        let response: ProduceResponse = self.send(0, 3, &request).context("Produce v3")?;
        let code = response
            .responses
            .first()
            .and_then(|topic| topic.partition_responses.first())
            .map(|partition| partition.error_code)
            .context("Produce response missing topic/partition response")?;
        Ok(code)
    }

    fn send<R, S>(&mut self, api_key: i16, api_version: i16, request: &R) -> Result<S>
    where
        R: Encodable,
        S: Decodable,
    {
        let correlation_id = self.correlation_id;
        self.correlation_id += 1;

        let payload = encode_request(api_key, api_version, correlation_id, request)?;
        self.stream.write_all(&payload)?;

        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = i32::from_be_bytes(len_buf);
        if len < 4 {
            anyhow::bail!("invalid response frame length {len}");
        }
        let mut response_buf = vec![0u8; len as usize];
        self.stream.read_exact(&mut response_buf)?;

        let mut cursor = std::io::Cursor::new(response_buf);
        let response_correlation_id = cursor.get_i32();
        if response_correlation_id != correlation_id {
            anyhow::bail!(
                "correlation id mismatch: request {correlation_id}, response {response_correlation_id}"
            );
        }
        S::decode(&mut cursor, api_version).context("decode response")
    }
}

fn encode_request<R: Encodable>(
    api_key: i16,
    api_version: i16,
    correlation_id: i32,
    request: &R,
) -> Result<Vec<u8>> {
    let mut body = BytesMut::new();
    body.put_i16(api_key);
    body.put_i16(api_version);
    body.put_i32(correlation_id);
    body.put_i16(CLIENT_ID.len() as i16);
    body.put_slice(CLIENT_ID.as_bytes());
    request.encode(&mut body, api_version)?;

    let mut framed = BytesMut::new();
    framed.put_i32(body.len() as i32);
    framed.extend_from_slice(&body);
    Ok(framed.to_vec())
}

fn encode_idempotent_batch(
    producer_id: i64,
    producer_epoch: i16,
    base_sequence: i32,
) -> Result<Bytes> {
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id,
        producer_epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: i64::from(base_sequence),
        sequence: base_sequence,
        offset: 0,
        key: Some(format!("key-{base_sequence}").into()),
        value: Some(format!("value-{base_sequence}").into()),
        headers: Default::default(),
    };

    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        &[record],
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )?;
    Ok(encoded.freeze())
}
