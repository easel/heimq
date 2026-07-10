package main

import (
	"context"
	"encoding/binary"
	"fmt"
	"hash/crc32"
	"time"

	"github.com/twmb/franz-go/pkg/kgo"
	"github.com/twmb/franz-go/pkg/kmsg"
)

// Two workloads send frames no client library will send: a duplicate idempotent
// sequence number, and one that skips ahead. That means building the Produce
// request and its record batch by hand.
//
// The Rust original built these with heimq_protocol -- the server's own protocol
// library -- so a bug in that encoder was invisible. Encoding independently here
// is the point.

var castagnoli = crc32.MakeTable(crc32.Castagnoli)

func varint(v int64) []byte {
	uv := uint64(v << 1)
	if v < 0 {
		uv = uint64(^(v << 1))
	}
	buf := make([]byte, 0, 10)
	for {
		if uv&^0x7f == 0 {
			return append(buf, byte(uv))
		}
		buf = append(buf, byte(uv&0x7f)|0x80)
		uv >>= 7
	}
}

// encodeRecordBatch builds one record in a v2 batch, no compression.
func encodeRecordBatch(producerID int64, producerEpoch int16, baseSeq int32) []byte {
	key := []byte(fmt.Sprintf("key-%d", baseSeq))
	value := []byte(fmt.Sprintf("value-%d", baseSeq))
	timestamp := int64(baseSeq)

	var rec []byte
	rec = append(rec, 0) // attributes
	rec = append(rec, varint(0)...)
	rec = append(rec, varint(0)...)
	rec = append(rec, varint(int64(len(key)))...)
	rec = append(rec, key...)
	rec = append(rec, varint(int64(len(value)))...)
	rec = append(rec, value...)
	rec = append(rec, varint(0)...) // header count

	recWithLen := append(varint(int64(len(rec))), rec...)

	// Everything the CRC covers: attributes through the record set.
	body := make([]byte, 0, 64)
	body = binary.BigEndian.AppendUint16(body, 0)                  // attributes
	body = binary.BigEndian.AppendUint32(body, 0)                  // last offset delta
	body = binary.BigEndian.AppendUint64(body, uint64(timestamp))  // base timestamp
	body = binary.BigEndian.AppendUint64(body, uint64(timestamp))  // max timestamp
	body = binary.BigEndian.AppendUint64(body, uint64(producerID)) //
	body = binary.BigEndian.AppendUint16(body, uint16(producerEpoch))
	body = binary.BigEndian.AppendUint32(body, uint32(baseSeq))
	body = binary.BigEndian.AppendUint32(body, 1) // record count
	body = append(body, recWithLen...)

	crc := crc32.Checksum(body, castagnoli)

	afterLen := make([]byte, 0, len(body)+9)
	afterLen = binary.BigEndian.AppendUint32(afterLen, 0) // partition leader epoch
	afterLen = append(afterLen, 2)                        // magic
	afterLen = binary.BigEndian.AppendUint32(afterLen, crc)
	afterLen = append(afterLen, body...)

	batch := make([]byte, 0, len(afterLen)+12)
	batch = binary.BigEndian.AppendUint64(batch, 0) // base offset
	batch = binary.BigEndian.AppendUint32(batch, uint32(len(afterLen)))
	batch = append(batch, afterLen...)
	return batch
}

func rawInitProducerID(ctx context.Context, cl *kgo.Client) (int64, int16, error) {
	req := kmsg.NewPtrInitProducerIDRequest()
	req.Version = 0
	req.TransactionalID = nil
	req.TransactionTimeoutMillis = 60_000

	resp, err := req.RequestWith(ctx, cl)
	if err != nil {
		return 0, 0, fmt.Errorf("InitProducerId v0: %w", err)
	}
	if resp.ErrorCode != 0 {
		return 0, 0, fmt.Errorf("InitProducerId returned error %d", resp.ErrorCode)
	}
	return resp.ProducerID, resp.ProducerEpoch, nil
}

func rawProduceSequence(
	ctx context.Context, cl *kgo.Client, topic string,
	producerID int64, producerEpoch int16, baseSeq int32,
) (int16, error) {
	partition := kmsg.NewProduceRequestTopicPartition()
	partition.Partition = 0
	partition.Records = encodeRecordBatch(producerID, producerEpoch, baseSeq)

	topicData := kmsg.NewProduceRequestTopic()
	topicData.Topic = topic
	topicData.Partitions = []kmsg.ProduceRequestTopicPartition{partition}

	req := kmsg.NewPtrProduceRequest()
	req.Version = 3
	req.TransactionID = nil
	req.Acks = 1
	req.TimeoutMillis = 15_000
	req.Topics = []kmsg.ProduceRequestTopic{topicData}

	resp, err := req.RequestWith(ctx, cl)
	if err != nil {
		return 0, fmt.Errorf("Produce v3: %w", err)
	}
	if len(resp.Topics) == 0 || len(resp.Topics[0].Partitions) == 0 {
		return 0, fmt.Errorf("Produce response missing topic/partition response")
	}
	return resp.Topics[0].Partitions[0].ErrorCode, nil
}

// Error codes the transaction coordinator returns while it settles. Kafka makes
// InitProducerId retriable this way; librdkafka's init_transactions() loops
// internally, so the Rust harness never had to handle them.
func retriableInitErr(code int16) bool {
	switch code {
	case 14, // COORDINATOR_LOAD_IN_PROGRESS
		15, // COORDINATOR_NOT_AVAILABLE
		16, // NOT_COORDINATOR
		51: // CONCURRENT_TRANSACTIONS -- producer A's txn is still open
		return true
	}
	return false
}

// initTransactionalProducerID is what init_transactions() does: bump the epoch
// for this transactional id, fencing any earlier producer using it.
func initTransactionalProducerID(ctx context.Context, cl *kgo.Client, txnID string) error {
	deadline := time.Now().Add(30 * time.Second)
	var lastCode int16
	for time.Now().Before(deadline) {
		req := kmsg.NewPtrInitProducerIDRequest()
		req.Version = 0
		id := txnID
		req.TransactionalID = &id
		req.TransactionTimeoutMillis = 30_000

		resp, err := req.RequestWith(ctx, cl)
		if err != nil {
			return fmt.Errorf("InitProducerId: %w", err)
		}
		if resp.ErrorCode == 0 {
			return nil
		}
		lastCode = resp.ErrorCode
		if !retriableInitErr(resp.ErrorCode) {
			return fmt.Errorf("InitProducerId error %d", resp.ErrorCode)
		}
		time.Sleep(200 * time.Millisecond)
	}
	return fmt.Errorf("InitProducerId still returning %d after 30s", lastCode)
}
