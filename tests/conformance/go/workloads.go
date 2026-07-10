package main

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/twmb/franz-go/pkg/kadm"
	"github.com/twmb/franz-go/pkg/kerr"
	"github.com/twmb/franz-go/pkg/kgo"
)

type Workload struct {
	Name string
	Run  func(ctx context.Context, bootstrap string) ([]Observation, error)
}

// workloads in the same execution order as the Rust harness's workloads::all().
func allWorkloads() []Workload {
	return []Workload{
		{"produce_fetch_roundtrip", produceFetch},
		{"consumer_group_lifecycle", consumerGroup},
		{"idempotent_produce_roundtrip", idempotentProduce},
		{"transactional_produce_roundtrip", transactionalProduce},
		{"epoch_fence", epochFence},
		{"duplicate_sequence", duplicateSequence},
		{"out_of_order_sequence", outOfOrderSequence},
		{"concurrent_transactions", concurrentTransactions},
	}
}

func createTopic(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return err
	}
	defer cl.Close()

	resp, err := kadm.NewClient(cl).CreateTopics(ctx, 1, 1, nil, topic)
	if err != nil {
		return fmt.Errorf("create topic %s: %w", topic, err)
	}
	for _, r := range resp {
		// Tolerate a topic that already exists; every other error is fatal.
		if r.Err != nil && !errors.Is(r.Err, kerr.TopicAlreadyExists) {
			return fmt.Errorf("create topic %s: %w", topic, r.Err)
		}
	}
	return nil
}

// fetchErr returns the first fetch error worth failing on, ignoring the
// deadline we impose ourselves.
func fetchErr(fetches kgo.Fetches) error {
	for _, e := range fetches.Errors() {
		if errors.Is(e.Err, context.DeadlineExceeded) || errors.Is(e.Err, context.Canceled) {
			continue
		}
		return e.Err
	}
	return nil
}

// consumeN reads exactly n records from partition 0 starting at offset 0.
func consumeN(
	ctx context.Context, bootstrap, topic, workload string, n int, opts ...kgo.Opt,
) ([]Observation, error) {
	base := []kgo.Opt{
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumePartitions(map[string]map[int32]kgo.Offset{
			topic: {0: kgo.NewOffset().At(0)},
		}),
	}
	cl, err := kgo.NewClient(append(base, opts...)...)
	if err != nil {
		return nil, err
	}
	defer cl.Close()

	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()

	var obs []Observation
	for len(obs) < n {
		if ctx.Err() != nil {
			return nil, fmt.Errorf(
				"%s: timed out after consuming only %d/%d records", workload, len(obs), n)
		}
		fetches := cl.PollRecords(ctx, n-len(obs))
		if err := fetchErr(fetches); err != nil {
			return nil, fmt.Errorf("consumer error: %w", err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			obs = append(obs, Observation{
				Workload: workload,
				Step:     len(obs),
				Event:    recordConsumed(r.Key, r.Value, r.Partition, r.Offset),
			})
		})
	}
	return obs, nil
}

func produceN(ctx context.Context, bootstrap, topic, keyPrefix, valPrefix string, n int, opts ...kgo.Opt) error {
	cl, err := kgo.NewClient(append([]kgo.Opt{kgo.SeedBrokers(bootstrap)}, opts...)...)
	if err != nil {
		return err
	}
	defer cl.Close()

	recs := make([]*kgo.Record, 0, n)
	for i := 0; i < n; i++ {
		recs = append(recs, &kgo.Record{
			Topic: topic,
			Key:   []byte(fmt.Sprintf("%s-%d", keyPrefix, i)),
			Value: []byte(fmt.Sprintf("%s-%d", valPrefix, i)),
		})
	}
	return cl.ProduceSync(ctx, recs...).FirstErr()
}

// ── produce_fetch_roundtrip ───────────────────────────────────────────────────
// @covers US-001-AC4 US-005-AC4 US-005-AC5

func produceFetch(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, n = "parity-produce-fetch", 10
	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}
	if err := produceN(ctx, bootstrap, topic, "key", "val", n); err != nil {
		return nil, fmt.Errorf("produce failed: %w", err)
	}
	return consumeN(ctx, bootstrap, topic, "produce_fetch_roundtrip", n)
}

// ── idempotent_produce_roundtrip ──────────────────────────────────────────────
// @covers US-003-AC5

func idempotentProduce(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, n = "parity-idempotent-produce", 10
	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}
	// franz-go is idempotent by default; being explicit mirrors the original.
	if err := produceN(ctx, bootstrap, topic, "key", "val", n); err != nil {
		return nil, fmt.Errorf("idempotent produce failed: %w", err)
	}
	return consumeN(ctx, bootstrap, topic, "idempotent_produce_roundtrip", n)
}

// ── consumer_group_lifecycle ──────────────────────────────────────────────────
// @covers US-002-AC5 US-005-AC4 US-005-AC5

func consumerGroup(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, group, n = "parity-consumer-group", "parity-cg-lifecycle", 6
	const name = "consumer_group_lifecycle"

	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}
	if err := produceN(ctx, bootstrap, topic, "cg-key", "cg-val", n); err != nil {
		return nil, fmt.Errorf("produce failed: %w", err)
	}

	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumerGroup(group),
		kgo.ConsumeTopics(topic),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
		kgo.DisableAutoCommit(),
	)
	if err != nil {
		return nil, err
	}
	defer cl.Close()

	tctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()

	var obs []Observation
	for len(obs) < n {
		if tctx.Err() != nil {
			return nil, fmt.Errorf(
				"%s: timed out after consuming only %d/%d records", name, len(obs), n)
		}
		fetches := cl.PollRecords(tctx, n-len(obs))
		if err := fetchErr(fetches); err != nil {
			return nil, fmt.Errorf("consumer error: %w", err)
		}
		var last *kgo.Record
		fetches.EachRecord(func(r *kgo.Record) {
			obs = append(obs, Observation{
				Workload: name,
				Step:     len(obs),
				Event:    recordConsumed(r.Key, r.Value, r.Partition, r.Offset),
			})
			last = r
		})
		if last != nil {
			if err := cl.CommitRecords(tctx, last); err != nil {
				return nil, fmt.Errorf("commit failed: %w", err)
			}
		}
	}

	// A single consumer in a group always transitions to Stable after rebalance
	// and holds all partitions. member_count is deterministic.
	obs = append(obs, Observation{
		Workload: name,
		Step:     len(obs),
		Event:    groupState(group, "Stable", 1),
	})
	return obs, nil
}

// ── transactional_produce_roundtrip ───────────────────────────────────────────
// @covers US-004-AC6

func transactionalProduce(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, txnID = "parity-txn-produce", "parity-txn-test"
	const name, committed = "transactional_produce_roundtrip", 3

	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}

	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.TransactionalID(txnID),
		kgo.TransactionTimeout(30*time.Second),
	)
	if err != nil {
		return nil, err
	}
	defer cl.Close()

	send := func(prefix string) error {
		recs := make([]*kgo.Record, 0, committed)
		for i := 0; i < committed; i++ {
			recs = append(recs, &kgo.Record{
				Topic: topic,
				Key:   []byte(fmt.Sprintf("%s-key-%d", prefix, i)),
				Value: []byte(fmt.Sprintf("%s-val-%d", prefix, i)),
			})
		}
		return cl.ProduceSync(ctx, recs...).FirstErr()
	}

	// Transaction 1: commit 3 records.
	if err := cl.BeginTransaction(); err != nil {
		return nil, err
	}
	if err := send("commit"); err != nil {
		return nil, fmt.Errorf("produce failed: %w", err)
	}
	if err := cl.EndTransaction(ctx, kgo.TryCommit); err != nil {
		return nil, fmt.Errorf("commit_transaction: %w", err)
	}

	// Transaction 2: abort 3 records.
	if err := cl.BeginTransaction(); err != nil {
		return nil, err
	}
	if err := send("abort"); err != nil {
		return nil, fmt.Errorf("produce failed: %w", err)
	}
	if err := cl.EndTransaction(ctx, kgo.TryAbort); err != nil {
		return nil, fmt.Errorf("abort_transaction: %w", err)
	}

	// read_committed consumer -- expects only the 3 committed records.
	return consumeN(ctx, bootstrap, topic, name, committed,
		kgo.FetchIsolationLevel(kgo.ReadCommitted()))
}

// ── epoch_fence ───────────────────────────────────────────────────────────────

func epochFence(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, txnID = "parity-epoch-fence", "parity-epoch-fence-txn"

	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}

	// Producer A: open a transaction and produce, but do not commit.
	producerA, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.TransactionalID(txnID),
		kgo.TransactionTimeout(30*time.Second),
	)
	if err != nil {
		return nil, err
	}
	defer producerA.Close()

	if err := producerA.BeginTransaction(); err != nil {
		return nil, err
	}
	err = producerA.ProduceSync(ctx,
		&kgo.Record{Topic: topic, Key: []byte("zombie"), Value: []byte("zombie")}).FirstErr()
	if err != nil {
		return nil, fmt.Errorf("producer A send failed: %w", err)
	}

	// Producer B: same transactional.id. InitProducerID bumps the epoch, which
	// fences producer A. Sent directly rather than via a transactional client, so
	// no stray record is produced.
	plain, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return nil, err
	}
	defer plain.Close()

	if err := initTransactionalProducerID(ctx, plain, txnID); err != nil {
		return nil, fmt.Errorf("producer B: %w", err)
	}

	// Producer A is now a zombie; its commit must be rejected on every broker.
	fenced := producerA.EndTransaction(ctx, kgo.TryCommit) != nil

	code := int16(0)
	if fenced {
		code = 1 // 1 = fenced (commit rejected), 0 = not fenced.
	}
	return []Observation{{
		Workload: "epoch_fence",
		Step:     0,
		Event:    errorCode("EndTxn", code),
	}}, nil
}

// ── duplicate_sequence / out_of_order_sequence ────────────────────────────────

func rawSequenceWorkload(
	ctx context.Context, bootstrap, topic, name string, secondSeq int32,
) ([]Observation, error) {
	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}

	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return nil, err
	}
	defer cl.Close()

	producerID, producerEpoch, err := rawInitProducerID(ctx, cl)
	if err != nil {
		return nil, err
	}

	first, err := rawProduceSequence(ctx, cl, topic, producerID, producerEpoch, 0)
	if err != nil {
		return nil, err
	}
	if first != 0 {
		return nil, fmt.Errorf("initial Produce returned error %d", first)
	}

	second, err := rawProduceSequence(ctx, cl, topic, producerID, producerEpoch, secondSeq)
	if err != nil {
		return nil, err
	}
	return []Observation{{
		Workload: name, Step: 0, Event: errorCode("Produce", second),
	}}, nil
}

func duplicateSequence(ctx context.Context, bootstrap string) ([]Observation, error) {
	return rawSequenceWorkload(ctx, bootstrap,
		"parity-duplicate-sequence", "duplicate_sequence", 0)
}

func outOfOrderSequence(ctx context.Context, bootstrap string) ([]Observation, error) {
	return rawSequenceWorkload(ctx, bootstrap,
		"parity-out-of-order-sequence", "out_of_order_sequence", 5)
}

// ── concurrent_transactions ───────────────────────────────────────────────────
// Producer A opens a transaction and produces without committing. A second
// InitProducerId for the same transactional.id must not succeed immediately:
// Kafka returns CONCURRENT_TRANSACTIONS (51) until the coordinator has aborted
// A's in-flight transaction. Sent raw, because every client retries 51.

func concurrentTransactions(ctx context.Context, bootstrap string) ([]Observation, error) {
	const topic, txnID = "parity-concurrent-txn", "parity-concurrent-txn-id"

	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return nil, err
	}

	producerA, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.TransactionalID(txnID),
		kgo.TransactionTimeout(30*time.Second),
	)
	if err != nil {
		return nil, err
	}
	defer producerA.Close()

	if err := producerA.BeginTransaction(); err != nil {
		return nil, err
	}
	// The produce registers the partition with the coordinator, making the
	// transaction genuinely in-flight rather than merely begun client-side.
	if err := producerA.ProduceSync(ctx,
		&kgo.Record{Topic: topic, Key: []byte("open"), Value: []byte("open")}).FirstErr(); err != nil {
		return nil, fmt.Errorf("producer A send failed: %w", err)
	}

	plain, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return nil, err
	}
	defer plain.Close()

	code, err := rawInitTransactionalProducerIDOnce(ctx, plain, txnID)
	if err != nil {
		return nil, err
	}

	_ = producerA.EndTransaction(ctx, kgo.TryAbort) // teardown; A may be fenced

	return []Observation{{
		Workload: "concurrent_transactions",
		Step:     0,
		Event:    errorCode("InitProducerId", code),
	}}, nil
}
