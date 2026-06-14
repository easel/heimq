// Franz-go compatibility oracle for heimq.
//
// Uses franz-go, a pure-Go Kafka client that implements the protocol
// independently of librdkafka, to verify that heimq handles the Kafka
// wire protocol correctly from a second, unrelated implementation.
//
// Usage: go run . <bootstrap-servers>
// Exit 0 on success; non-zero with a FAIL message on any deviation.
package main

import (
	"context"
	"fmt"
	"os"
	"sync"
	"time"

	"github.com/twmb/franz-go/pkg/kadm"
	"github.com/twmb/franz-go/pkg/kgo"
	"github.com/twmb/franz-go/pkg/kmsg"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: main <bootstrap-servers>")
		os.Exit(1)
	}
	if err := run(os.Args[1]); err != nil {
		fmt.Fprintf(os.Stderr, "FAIL: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("PASS")
}

func run(bootstrap string) error {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()

	ts := time.Now().UnixNano()
	topic := fmt.Sprintf("franz-compat-%d", ts)
	group := fmt.Sprintf("franz-group-%d", ts)
	hdrTopic := fmt.Sprintf("franz-hdrs-%d", ts)
	hdrGroup := fmt.Sprintf("franz-hdrs-group-%d", ts)
	compTopic := fmt.Sprintf("franz-comp-%d", ts)
	compGroup := fmt.Sprintf("franz-comp-group-%d", ts)
	lz4Topic := fmt.Sprintf("franz-lz4-%d", ts)
	lz4Group := fmt.Sprintf("franz-lz4-group-%d", ts)
	zstdTopic := fmt.Sprintf("franz-zstd-%d", ts)
	zstdGroup := fmt.Sprintf("franz-zstd-group-%d", ts)
	resumeTopic := fmt.Sprintf("franz-resume-%d", ts)
	resumeGroup := fmt.Sprintf("franz-resume-group-%d", ts)
	txnTopic := fmt.Sprintf("franz-txn-%d", ts)
	txnGroup := fmt.Sprintf("franz-txn-group-%d", ts)
	mmgTopic := fmt.Sprintf("franz-mmg-%d", ts)

	if err := check("create-topic", func() error {
		return createTopic(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("produce", func() error {
		return produce(ctx, bootstrap, topic, 5)
	}); err != nil {
		return err
	}

	if err := check("consume-via-group", func() error {
		return consumeViaGroup(ctx, bootstrap, topic, group, 5)
	}); err != nil {
		return err
	}

	if err := check("offset-delete", func() error {
		return offsetDelete(ctx, bootstrap, topic, group)
	}); err != nil {
		return err
	}

	if err := check("list-groups", func() error {
		return listGroups(ctx, bootstrap, group)
	}); err != nil {
		return err
	}

	if err := check("describe-groups", func() error {
		return describeGroups(ctx, bootstrap, group)
	}); err != nil {
		return err
	}

	if err := check("delete-groups", func() error {
		return deleteGroups(ctx, bootstrap, group)
	}); err != nil {
		return err
	}

	if err := check("describe-configs", func() error {
		return describeConfigs(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("describe-log-dirs", func() error {
		return describeLogDirs(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("describe-cluster", func() error {
		return describeCluster(ctx, bootstrap)
	}); err != nil {
		return err
	}

	if err := check("alter-configs", func() error {
		return alterConfigs(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("incremental-alter-configs", func() error {
		return incrementalAlterConfigs(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("list-offsets", func() error {
		return listOffsets(ctx, bootstrap, topic, 5)
	}); err != nil {
		return err
	}

	if err := check("create-partitions", func() error {
		return createPartitions(ctx, bootstrap, topic, 3)
	}); err != nil {
		return err
	}

	if err := check("offset-for-leader-epoch", func() error {
		return offsetForLeaderEpoch(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("elect-leaders", func() error {
		return electLeaders(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("list-transactions", func() error {
		return listTransactions(ctx, bootstrap)
	}); err != nil {
		return err
	}

	if err := check("delete-records", func() error {
		return deleteRecords(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("describe-topic-partitions", func() error {
		return describeTopicPartitions(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("delete-topic", func() error {
		return deleteTopic(ctx, bootstrap, topic)
	}); err != nil {
		return err
	}

	if err := check("produce-with-headers", func() error {
		return produceWithHeaders(ctx, bootstrap, hdrTopic)
	}); err != nil {
		return err
	}

	if err := check("consume-headers-roundtrip", func() error {
		return consumeHeadersRoundtrip(ctx, bootstrap, hdrTopic, hdrGroup)
	}); err != nil {
		return err
	}

	if err := check("produce-compressed", func() error {
		return produceCompressed(ctx, bootstrap, compTopic)
	}); err != nil {
		return err
	}

	if err := check("consume-compressed-roundtrip", func() error {
		return consumeCompressedRoundtrip(ctx, bootstrap, compTopic, compGroup)
	}); err != nil {
		return err
	}

	if err := check("produce-lz4-compressed", func() error {
		return produceWithCodec(ctx, bootstrap, lz4Topic, kgo.Lz4Compression())
	}); err != nil {
		return err
	}

	if err := check("consume-lz4-roundtrip", func() error {
		return consumeCompressedRoundtrip(ctx, bootstrap, lz4Topic, lz4Group)
	}); err != nil {
		return err
	}

	if err := check("produce-zstd-compressed", func() error {
		return produceWithCodec(ctx, bootstrap, zstdTopic, kgo.ZstdCompression())
	}); err != nil {
		return err
	}

	if err := check("consume-zstd-roundtrip", func() error {
		return consumeCompressedRoundtrip(ctx, bootstrap, zstdTopic, zstdGroup)
	}); err != nil {
		return err
	}

	if err := check("offset-resume", func() error {
		return offsetResume(ctx, bootstrap, resumeTopic, resumeGroup)
	}); err != nil {
		return err
	}

	if err := check("transactional-produce", func() error {
		return transactionalProduce(ctx, bootstrap, txnTopic)
	}); err != nil {
		return err
	}

	if err := check("transactional-consume-committed", func() error {
		return transactionalConsumeCommitted(ctx, bootstrap, txnTopic, txnGroup)
	}); err != nil {
		return err
	}

	txnUncommitGroup := fmt.Sprintf("franz-txn-uncommit-group-%d", ts)
	if err := check("transactional-consume-uncommitted", func() error {
		return transactionalConsumeUncommitted(ctx, bootstrap, txnTopic, txnUncommitGroup)
	}); err != nil {
		return err
	}

	if err := check("multi-member-group", func() error {
		return multiMemberGroup(ctx, bootstrap, mmgTopic)
	}); err != nil {
		return err
	}

	return nil
}

func check(name string, fn func() error) error {
	if err := fn(); err != nil {
		return fmt.Errorf("%s: %w", name, err)
	}
	fmt.Printf("  ok  %s\n", name)
	return nil
}

func createPartitions(ctx context.Context, bootstrap, topic string, totalCount int) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	resp, err := adm.UpdatePartitions(ctx, totalCount, topic)
	if err != nil {
		return fmt.Errorf("create partitions rpc: %w", err)
	}
	if _, err := resp.On(topic, func(r *kadm.CreatePartitionsResponse) error { return r.Err }); err != nil {
		return fmt.Errorf("create partitions response for %q: %w", topic, err)
	}
	return nil
}

func createTopic(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	resp, err := adm.CreateTopics(ctx, 1, 1, nil, topic)
	if err != nil {
		return fmt.Errorf("create topics rpc: %w", err)
	}
	if r, ok := resp[topic]; ok && r.Err != nil {
		return fmt.Errorf("topic %s: %w", topic, r.Err)
	}
	return nil
}

func produce(ctx context.Context, bootstrap, topic string, n int) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.DefaultProduceTopic(topic),
	)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer cl.Close()

	for i := 0; i < n; i++ {
		res := cl.ProduceSync(ctx, &kgo.Record{
			Key:   []byte(fmt.Sprintf("key-%d", i)),
			Value: []byte(fmt.Sprintf("val-%d", i)),
		})
		if err := res.FirstErr(); err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

func consumeViaGroup(ctx context.Context, bootstrap, topic, group string, want int) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumeTopics(topic),
		kgo.ConsumerGroup(group),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
	)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer cl.Close()

	got := make(map[string]string, want)
	for len(got) < want {
		if ctx.Err() != nil {
			return fmt.Errorf("timeout: consumed %d/%d records", len(got), want)
		}
		fetches := cl.PollFetches(ctx)
		if errs := fetches.Errors(); len(errs) > 0 {
			return fmt.Errorf("fetch: %v", errs[0].Err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			got[string(r.Key)] = string(r.Value)
		})
	}

	for i := 0; i < want; i++ {
		key := fmt.Sprintf("key-%d", i)
		wantVal := fmt.Sprintf("val-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit offsets: %w", err)
	}
	return nil
}

// listOffsets checks that the high-watermark equals wantHWM and LSO equals 0.
func listOffsets(ctx context.Context, bootstrap, topic string, wantHWM int64) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)

	endOffsets, err := adm.ListEndOffsets(ctx, topic)
	if err != nil {
		return fmt.Errorf("list end offsets rpc: %w", err)
	}
	foundEnd := false
	endOffsets.Each(func(lo kadm.ListedOffset) {
		if lo.Topic != topic {
			return
		}
		if lo.Err != nil {
			err = fmt.Errorf("end offset partition %d: %w", lo.Partition, lo.Err)
			return
		}
		if lo.Offset != wantHWM {
			err = fmt.Errorf("end offset partition %d: got %d want %d", lo.Partition, lo.Offset, wantHWM)
			return
		}
		foundEnd = true
	})
	if err != nil {
		return err
	}
	if !foundEnd {
		return fmt.Errorf("no end offset result for topic %s", topic)
	}

	startOffsets, err := adm.ListStartOffsets(ctx, topic)
	if err != nil {
		return fmt.Errorf("list start offsets rpc: %w", err)
	}
	foundStart := false
	startOffsets.Each(func(lo kadm.ListedOffset) {
		if lo.Topic != topic {
			return
		}
		if lo.Err != nil {
			err = fmt.Errorf("start offset partition %d: %w", lo.Partition, lo.Err)
			return
		}
		if lo.Offset != 0 {
			err = fmt.Errorf("start offset partition %d: got %d want 0", lo.Partition, lo.Offset)
			return
		}
		foundStart = true
	})
	if err != nil {
		return err
	}
	if !foundStart {
		return fmt.Errorf("no start offset result for topic %s", topic)
	}

	return nil
}

// deleteTopic creates then deletes a topic and verifies no error from the broker.
func deleteTopic(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	resp, err := adm.DeleteTopics(ctx, topic)
	if err != nil {
		return fmt.Errorf("delete topics rpc: %w", err)
	}
	if r, ok := resp[topic]; ok && r.Err != nil {
		return fmt.Errorf("delete topic %s: %w", topic, r.Err)
	}
	return nil
}

// produceWithHeaders sends 3 records each carrying x-trace and x-seq headers.
func produceWithHeaders(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.DefaultProduceTopic(topic),
	)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer cl.Close()

	for i := 0; i < 3; i++ {
		rec := &kgo.Record{
			Key:   []byte(fmt.Sprintf("hdr-key-%d", i)),
			Value: []byte(fmt.Sprintf("hdr-val-%d", i)),
			Headers: []kgo.RecordHeader{
				{Key: "x-trace", Value: []byte(fmt.Sprintf("trace-%d", i))},
				{Key: "x-seq", Value: []byte(fmt.Sprintf("%d", i))},
			},
		}
		res := cl.ProduceSync(ctx, rec)
		if err := res.FirstErr(); err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// consumeHeadersRoundtrip consumes 3 records and verifies x-trace and x-seq headers.
func consumeHeadersRoundtrip(ctx context.Context, bootstrap, topic, group string) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumeTopics(topic),
		kgo.ConsumerGroup(group),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
	)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer cl.Close()

	type hdrRecord struct {
		val, trace, seq string
	}
	got := make(map[string]hdrRecord)
	want := 3

	for len(got) < want {
		if ctx.Err() != nil {
			return fmt.Errorf("timeout: consumed %d/%d header records", len(got), want)
		}
		fetches := cl.PollFetches(ctx)
		if errs := fetches.Errors(); len(errs) > 0 {
			return fmt.Errorf("fetch: %v", errs[0].Err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			hr := hdrRecord{val: string(r.Value)}
			for _, h := range r.Headers {
				switch h.Key {
				case "x-trace":
					hr.trace = string(h.Value)
				case "x-seq":
					hr.seq = string(h.Value)
				}
			}
			got[string(r.Key)] = hr
		})
	}

	for i := 0; i < want; i++ {
		key := fmt.Sprintf("hdr-key-%d", i)
		hr, ok := got[key]
		if !ok {
			return fmt.Errorf("missing record %s", key)
		}
		wantVal := fmt.Sprintf("hdr-val-%d", i)
		wantTrace := fmt.Sprintf("trace-%d", i)
		wantSeq := fmt.Sprintf("%d", i)
		if hr.val != wantVal {
			return fmt.Errorf("key %s value: got %q want %q", key, hr.val, wantVal)
		}
		if hr.trace != wantTrace {
			return fmt.Errorf("key %s x-trace: got %q want %q", key, hr.trace, wantTrace)
		}
		if hr.seq != wantSeq {
			return fmt.Errorf("key %s x-seq: got %q want %q", key, hr.seq, wantSeq)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit offsets: %w", err)
	}
	return nil
}

// listGroups verifies that ListGroups (API 16) returns the consumer group that
// was created by consumeViaGroup.
func listGroups(ctx context.Context, bootstrap, wantGroup string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	listed, err := adm.ListGroups(ctx)
	if err != nil {
		return fmt.Errorf("list groups rpc: %w", err)
	}
	for _, g := range listed {
		if g.Group == wantGroup {
			return nil
		}
	}
	return fmt.Errorf("group %q not found in list; got %d groups", wantGroup, len(listed))
}

// describeGroups verifies that DescribeGroups (API 15) returns valid state for
// a known group.
func describeGroups(ctx context.Context, bootstrap, wantGroup string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	described, err := adm.DescribeGroups(ctx, wantGroup)
	if err != nil {
		return fmt.Errorf("describe groups rpc: %w", err)
	}
	dg, ok := described[wantGroup]
	if !ok {
		return fmt.Errorf("group %q missing from describe response", wantGroup)
	}
	if dg.Err != nil {
		return fmt.Errorf("group %q describe error: %w", wantGroup, dg.Err)
	}
	return nil
}

// describeConfigs verifies that DescribeConfigs (API 32) returns a non-error
// response for a topic resource.
// describeLogDirs exercises DescribeLogDirs (API 35).
func describeLogDirs(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	var ts kadm.TopicsSet
	ts.Add(topic, 0)
	allDirs, err := adm.DescribeAllLogDirs(ctx, ts)
	if err != nil {
		return fmt.Errorf("describe log dirs rpc: %w", err)
	}
	for _, dirs := range allDirs {
		if err := dirs.Error(); err != nil {
			return fmt.Errorf("describe log dirs: %w", err)
		}
		if len(dirs) == 0 {
			return fmt.Errorf("describe log dirs: no directories returned")
		}
	}
	return nil
}

func describeConfigs(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	resp, err := adm.DescribeTopicConfigs(ctx, topic)
	if err != nil {
		return fmt.Errorf("describe topic configs rpc: %w", err)
	}
	for _, r := range resp {
		if r.Err != nil {
			return fmt.Errorf("describe configs %q: %w", topic, r.Err)
		}
	}
	return nil
}

// deleteGroups verifies that DeleteGroups (API 42) succeeds for a known group.
func offsetDelete(ctx context.Context, bootstrap, topic, group string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	var ts kadm.TopicsSet
	ts.Add(topic, 0)
	resp, err := adm.DeleteOffsets(ctx, group, ts)
	if err != nil {
		return fmt.Errorf("delete offsets rpc: %w", err)
	}
	if err := resp.Error(); err != nil {
		return fmt.Errorf("delete offsets response: %w", err)
	}
	return nil
}

// deleteRecords exercises DeleteRecords (API 21) by truncating at offset 2.
func deleteRecords(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	offsets := make(kadm.Offsets)
	offsets.Add(kadm.Offset{Topic: topic, Partition: 0, At: 2})
	resp, err := adm.DeleteRecords(ctx, offsets)
	if err != nil {
		return fmt.Errorf("delete records rpc: %w", err)
	}
	if err := resp.Error(); err != nil {
		return fmt.Errorf("delete records response: %w", err)
	}
	return nil
}

func deleteGroups(ctx context.Context, bootstrap, group string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	resp, err := adm.DeleteGroups(ctx, group)
	if err != nil {
		return fmt.Errorf("delete groups rpc: %w", err)
	}
	if r, ok := resp[group]; ok && r.Err != nil {
		return fmt.Errorf("delete group %q: %w", group, r.Err)
	}
	return nil
}

// alterConfigs exercises AlterConfigs (API 33) by setting a no-op config.
func alterConfigs(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	v := "604800000"
	resp, err := adm.AlterTopicConfigsState(ctx, []kadm.AlterConfig{{Name: "retention.ms", Value: &v}}, topic)
	if err != nil {
		return fmt.Errorf("alter configs rpc: %w", err)
	}
	if _, err := resp.On(topic, func(r *kadm.AlterConfigsResponse) error { return r.Err }); err != nil {
		return fmt.Errorf("alter configs response for %q: %w", topic, err)
	}
	return nil
}

// describeCluster exercises DescribeCluster (API 60) via raw kmsg.
func describeCluster(ctx context.Context, bootstrap string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	resp, err := cl.Request(ctx, &kmsg.DescribeClusterRequest{})
	if err != nil {
		return fmt.Errorf("describe cluster rpc: %w", err)
	}
	dcr, ok := resp.(*kmsg.DescribeClusterResponse)
	if !ok {
		return fmt.Errorf("unexpected response type %T", resp)
	}
	if dcr.ErrorCode != 0 {
		return fmt.Errorf("describe cluster error_code=%d", dcr.ErrorCode)
	}
	if len(dcr.Brokers) == 0 {
		return fmt.Errorf("describe cluster returned no brokers")
	}
	return nil
}

// electLeaders exercises ElectLeaders (API 43).
// Elects the preferred replica for partition 0 of the topic.
func electLeaders(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	var ts kadm.TopicsSet
	ts.Add(topic, 0)
	results, err := adm.ElectLeaders(ctx, kadm.ElectPreferredReplica, ts)
	if err != nil {
		return fmt.Errorf("elect leaders rpc: %w", err)
	}
	for _, partRes := range results {
		for pid, r := range partRes {
			if r.Err != nil {
				return fmt.Errorf("elect leaders partition %d: %w", pid, r.Err)
			}
		}
	}
	return nil
}

// listTransactions exercises ListTransactions (API 66).
// With no in-flight transactions the response must be an empty list.
func listTransactions(ctx context.Context, bootstrap string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	_, err = adm.ListTransactions(ctx, nil, nil)
	if err != nil {
		return fmt.Errorf("list transactions: %w", err)
	}
	return nil
}

// offsetForLeaderEpoch exercises OffsetForLeaderEpoch (API 23).
// Sends a raw request for partition 0 at leader epoch 0 and verifies the
// response contains a non-negative end offset.
func offsetForLeaderEpoch(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	req := &kmsg.OffsetForLeaderEpochRequest{
		Topics: []kmsg.OffsetForLeaderEpochRequestTopic{
			{
				Topic: topic,
				Partitions: []kmsg.OffsetForLeaderEpochRequestTopicPartition{
					{Partition: 0, LeaderEpoch: 0, CurrentLeaderEpoch: -1},
				},
			},
		},
	}
	resp, err := cl.Request(ctx, req)
	if err != nil {
		return fmt.Errorf("rpc: %w", err)
	}
	epochResp, ok := resp.(*kmsg.OffsetForLeaderEpochResponse)
	if !ok {
		return fmt.Errorf("unexpected response type %T", resp)
	}
	for _, t := range epochResp.Topics {
		for _, p := range t.Partitions {
			if p.ErrorCode != 0 {
				return fmt.Errorf("partition %d error_code=%d", p.Partition, p.ErrorCode)
			}
			if p.EndOffset < 0 {
				return fmt.Errorf("partition %d: negative end offset %d", p.Partition, p.EndOffset)
			}
		}
	}
	return nil
}

// describeTopicPartitions exercises DescribeTopicPartitions (API 75, KIP-848).
func describeTopicPartitions(ctx context.Context, bootstrap, topic string) error {
	// kgo defaults to kversion.Stable() which does not yet include API 75.
	// Pass MaxVersions(nil) to skip the client-side version cap and let the
	// server's own ApiVersions response govern negotiation.
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap), kgo.MaxVersions(nil))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	req := &kmsg.DescribeTopicPartitionsRequest{
		Topics: []kmsg.DescribeTopicPartitionsRequestTopic{
			{Topic: topic},
		},
		ResponsePartitionLimit: 200,
	}
	resp, err := req.RequestWith(ctx, cl)
	if err != nil {
		return fmt.Errorf("rpc: %w", err)
	}
	if len(resp.Topics) == 0 {
		return fmt.Errorf("expected at least one topic in response, got none")
	}
	t := resp.Topics[0]
	if t.ErrorCode != 0 {
		return fmt.Errorf("topic %q error_code=%d", topic, t.ErrorCode)
	}
	if len(t.Partitions) == 0 {
		return fmt.Errorf("topic %q: expected partitions, got none", topic)
	}
	for _, p := range t.Partitions {
		if p.ErrorCode != 0 {
			return fmt.Errorf("topic %q partition %d error_code=%d", topic, p.Partition, p.ErrorCode)
		}
		if p.LeaderID < 0 {
			return fmt.Errorf("topic %q partition %d: invalid leader_id=%d", topic, p.Partition, p.LeaderID)
		}
	}
	return nil
}

// incrementalAlterConfigs exercises IncrementalAlterConfigs (API 44).
func incrementalAlterConfigs(ctx context.Context, bootstrap, topic string) error {
	cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer cl.Close()

	adm := kadm.NewClient(cl)
	v := "604800000"
	resp, err := adm.AlterTopicConfigs(ctx, []kadm.AlterConfig{{Name: "retention.ms", Value: &v}}, topic)
	if err != nil {
		return fmt.Errorf("incremental alter configs rpc: %w", err)
	}
	if _, err := resp.On(topic, func(r *kadm.AlterConfigsResponse) error { return r.Err }); err != nil {
		return fmt.Errorf("incremental alter configs response for %q: %w", topic, err)
	}
	return nil
}

// produceCompressed creates compTopic and produces 4 records using gzip compression.
// The topic is created inline so no separate create-topic check is needed.
func produceCompressed(ctx context.Context, bootstrap, topic string) error {
	// First create the topic.
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
		if err != nil {
			return fmt.Errorf("new client: %w", err)
		}
		adm := kadm.NewClient(cl)
		resp, err := adm.CreateTopics(ctx, 1, 1, nil, topic)
		cl.Close()
		if err != nil {
			return fmt.Errorf("create topic: %w", err)
		}
		if r, ok := resp[topic]; ok && r.Err != nil {
			return fmt.Errorf("create topic %q: %w", topic, r.Err)
		}
	}

	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.DefaultProduceTopic(topic),
		kgo.ProducerBatchCompression(kgo.GzipCompression()),
	)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer cl.Close()

	for i := 0; i < 4; i++ {
		res := cl.ProduceSync(ctx, &kgo.Record{
			Key:   []byte(fmt.Sprintf("comp-key-%d", i)),
			Value: []byte(fmt.Sprintf("comp-val-%d", i)),
		})
		if err := res.FirstErr(); err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// consumeCompressedRoundtrip consumes the 4 gzip-compressed records and verifies
// that the server correctly decompresses them during fetch.
func consumeCompressedRoundtrip(ctx context.Context, bootstrap, topic, group string) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumeTopics(topic),
		kgo.ConsumerGroup(group),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
	)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer cl.Close()

	got := make(map[string]string)
	want := 4
	for len(got) < want {
		if ctx.Err() != nil {
			return fmt.Errorf("timeout: consumed %d/%d compressed records", len(got), want)
		}
		fetches := cl.PollFetches(ctx)
		if errs := fetches.Errors(); len(errs) > 0 {
			return fmt.Errorf("fetch: %v", errs[0].Err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			got[string(r.Key)] = string(r.Value)
		})
	}

	for i := 0; i < want; i++ {
		key := fmt.Sprintf("comp-key-%d", i)
		wantVal := fmt.Sprintf("comp-val-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit offsets: %w", err)
	}
	return nil
}

// produceWithCodec creates the given topic and produces 4 records with the specified
// compression codec. Reuses consumeCompressedRoundtrip for the consume side.
func produceWithCodec(ctx context.Context, bootstrap, topic string, codec kgo.CompressionCodec) error {
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
		if err != nil {
			return fmt.Errorf("new client: %w", err)
		}
		adm := kadm.NewClient(cl)
		resp, err := adm.CreateTopics(ctx, 1, 1, nil, topic)
		cl.Close()
		if err != nil {
			return fmt.Errorf("create topic: %w", err)
		}
		if r, ok := resp[topic]; ok && r.Err != nil {
			return fmt.Errorf("create topic %q: %w", topic, r.Err)
		}
	}

	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.DefaultProduceTopic(topic),
		kgo.ProducerBatchCompression(codec),
	)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer cl.Close()

	for i := 0; i < 4; i++ {
		res := cl.ProduceSync(ctx, &kgo.Record{
			Key:   []byte(fmt.Sprintf("comp-key-%d", i)),
			Value: []byte(fmt.Sprintf("comp-val-%d", i)),
		})
		if err := res.FirstErr(); err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// offsetResume verifies that committed consumer group offsets are persisted and
// respected when a new consumer session joins the same group.
//
// 1. Creates resumeTopic, produces 8 records.
// 2. Admin client explicitly commits offset 4 for resumeGroup on partition 0.
//    (This simulates a prior consumer session that processed records 0-3.)
// 3. Session B (resumeGroup) subscribes and must start from offset 4,
//    receiving only records 4-7.
func offsetResume(ctx context.Context, bootstrap, topic, group string) error {
	// Create topic.
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
		if err != nil {
			return fmt.Errorf("new client: %w", err)
		}
		adm := kadm.NewClient(cl)
		resp, err := adm.CreateTopics(ctx, 1, 1, nil, topic)
		cl.Close()
		if err != nil {
			return fmt.Errorf("create resume topic: %w", err)
		}
		if r, ok := resp[topic]; ok && r.Err != nil {
			return fmt.Errorf("create resume topic %q: %w", topic, r.Err)
		}
	}

	// Produce 8 records.
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap), kgo.DefaultProduceTopic(topic))
		if err != nil {
			return fmt.Errorf("new producer: %w", err)
		}
		for i := 0; i < 8; i++ {
			res := cl.ProduceSync(ctx, &kgo.Record{
				Key:   []byte(fmt.Sprintf("rkey-%d", i)),
				Value: []byte(fmt.Sprintf("rval-%d", i)),
			})
			if err := res.FirstErr(); err != nil {
				cl.Close()
				return fmt.Errorf("produce record %d: %w", i, err)
			}
		}
		cl.Close()
	}

	// Admin-commit offset 4 for the group. This bypasses consumer-session lifecycle
	// and directly tests the OffsetCommit → OffsetFetch persisting path.
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
		if err != nil {
			return fmt.Errorf("admin client: %w", err)
		}
		adm := kadm.NewClient(cl)
		offsets := make(kadm.Offsets)
		offsets.Add(kadm.Offset{Topic: topic, Partition: 0, At: 4})
		_, err = adm.CommitOffsets(ctx, group, offsets)
		if err != nil {
			cl.Close()
			return fmt.Errorf("commit offsets: %w", err)
		}
		cl.Close()
	}

	// Session B: subscribe with the same group; must resume from offset 4, not 0.
	{
		cl, err := kgo.NewClient(
			kgo.SeedBrokers(bootstrap),
			kgo.ConsumeTopics(topic),
			kgo.ConsumerGroup(group),
			// AtEnd is the no-committed-offset fallback; committed offset (4) takes priority.
			kgo.ConsumeResetOffset(kgo.NewOffset().AtEnd()),
		)
		if err != nil {
			return fmt.Errorf("session B client: %w", err)
		}
		defer cl.Close()

		gotB := make(map[string]string)
		for len(gotB) < 4 {
			if ctx.Err() != nil {
				return fmt.Errorf("session B timeout: consumed %d/4 records after resume", len(gotB))
			}
			fetches := cl.PollFetches(ctx)
			if errs := fetches.Errors(); len(errs) > 0 {
				return fmt.Errorf("session B fetch error: %v", errs[0].Err)
			}
			fetches.EachRecord(func(r *kgo.Record) {
				gotB[string(r.Key)] = string(r.Value)
			})
		}

		// Verify records 4-7 arrived (not 0-3).
		for i := 4; i < 8; i++ {
			key := fmt.Sprintf("rkey-%d", i)
			wantVal := fmt.Sprintf("rval-%d", i)
			v, ok := gotB[key]
			if !ok {
				return fmt.Errorf("session B missing key %s; got keys: %v", key, gotB)
			}
			if v != wantVal {
				return fmt.Errorf("session B key %s: got %q want %q", key, v, wantVal)
			}
		}
		for i := 0; i < 4; i++ {
			if _, ok := gotB[fmt.Sprintf("rkey-%d", i)]; ok {
				return fmt.Errorf("session B re-read committed record rkey-%d (started before offset 4)", i)
			}
		}
		if err := cl.CommitUncommittedOffsets(ctx); err != nil {
			return fmt.Errorf("session B commit: %w", err)
		}
	}

	return nil
}

// transactionalProduce creates txnTopic, commits 3 records in a transaction,
// then aborts a second transaction containing 2 records.
func transactionalProduce(ctx context.Context, bootstrap, topic string) error {
	if err := createTopic(ctx, bootstrap, topic); err != nil {
		return fmt.Errorf("create topic: %w", err)
	}

	txnID := fmt.Sprintf("franz-txn-%d", time.Now().UnixNano())
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.TransactionalID(txnID),
		kgo.DefaultProduceTopic(topic),
	)
	if err != nil {
		return fmt.Errorf("new txn client: %w", err)
	}
	defer cl.Close()

	// Committed transaction.
	if err := cl.BeginTransaction(); err != nil {
		return fmt.Errorf("begin txn: %w", err)
	}
	for i := 0; i < 3; i++ {
		r := &kgo.Record{Key: []byte(fmt.Sprintf("tkey-%d", i)), Value: []byte(fmt.Sprintf("tval-%d", i))}
		if err := cl.ProduceSync(ctx, r).FirstErr(); err != nil {
			return fmt.Errorf("txn produce record %d: %w", i, err)
		}
	}
	if err := cl.EndTransaction(ctx, kgo.TryCommit); err != nil {
		return fmt.Errorf("commit txn: %w", err)
	}

	// Aborted transaction — these records must not be visible to read_committed.
	if err := cl.BeginTransaction(); err != nil {
		return fmt.Errorf("begin abort txn: %w", err)
	}
	for i := 0; i < 2; i++ {
		r := &kgo.Record{Key: []byte(fmt.Sprintf("akey-%d", i)), Value: []byte(fmt.Sprintf("aval-%d", i))}
		if err := cl.ProduceSync(ctx, r).FirstErr(); err != nil {
			return fmt.Errorf("abort txn produce record %d: %w", i, err)
		}
	}
	if err := cl.EndTransaction(ctx, kgo.TryAbort); err != nil {
		return fmt.Errorf("abort txn: %w", err)
	}

	return nil
}

// transactionalConsumeCommitted reads from txnTopic using read_committed isolation
// and verifies that only the 3 committed records (tkey-0..tkey-2) arrive, not
// the 2 aborted records (akey-0, akey-1).
func transactionalConsumeCommitted(ctx context.Context, bootstrap, topic, group string) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumerGroup(group),
		kgo.ConsumeTopics(topic),
		kgo.FetchIsolationLevel(kgo.ReadCommitted()),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
	)
	if err != nil {
		return fmt.Errorf("new consumer client: %w", err)
	}
	defer cl.Close()

	got := make(map[string]string)
	deadline := time.Now().Add(30 * time.Second)
	for len(got) < 3 && time.Now().Before(deadline) {
		fetches := cl.PollFetches(ctx)
		if errs := fetches.Errors(); len(errs) > 0 {
			return fmt.Errorf("txn fetch error: %v", errs[0].Err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			got[string(r.Key)] = string(r.Value)
		})
	}

	if len(got) < 3 {
		return fmt.Errorf("timeout: consumed %d/3 committed records; got keys: %v", len(got), got)
	}

	for i := 0; i < 3; i++ {
		key := fmt.Sprintf("tkey-%d", i)
		wantVal := fmt.Sprintf("tval-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing committed record %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}
	// Aborted records must not appear.
	for i := 0; i < 2; i++ {
		if _, ok := got[fmt.Sprintf("akey-%d", i)]; ok {
			return fmt.Errorf("aborted record akey-%d leaked into read_committed consumer", i)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit offsets: %w", err)
	}
	return nil
}

// transactionalConsumeUncommitted reads txnTopic with read_uncommitted isolation
// and verifies that BOTH committed (tkey-*) and aborted (akey-*) records are visible.
func transactionalConsumeUncommitted(ctx context.Context, bootstrap, topic, group string) error {
	cl, err := kgo.NewClient(
		kgo.SeedBrokers(bootstrap),
		kgo.ConsumerGroup(group),
		kgo.ConsumeTopics(topic),
		kgo.FetchIsolationLevel(kgo.ReadUncommitted()),
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
	)
	if err != nil {
		return fmt.Errorf("new consumer client: %w", err)
	}
	defer cl.Close()

	got := make(map[string]string)
	deadline := time.Now().Add(30 * time.Second)
	// Expect 5 records: 3 committed (tkey-0..2) + 2 aborted (akey-0..1).
	for len(got) < 5 && time.Now().Before(deadline) {
		fetches := cl.PollFetches(ctx)
		if errs := fetches.Errors(); len(errs) > 0 {
			return fmt.Errorf("txn uncommit fetch error: %v", errs[0].Err)
		}
		fetches.EachRecord(func(r *kgo.Record) {
			got[string(r.Key)] = string(r.Value)
		})
	}

	if len(got) < 5 {
		return fmt.Errorf("timeout: read_uncommitted consumed %d/5 records; keys: %v", len(got), got)
	}
	// Committed records present.
	for i := 0; i < 3; i++ {
		if _, ok := got[fmt.Sprintf("tkey-%d", i)]; !ok {
			return fmt.Errorf("read_uncommitted missing committed record tkey-%d", i)
		}
	}
	// Aborted records also present under read_uncommitted.
	for i := 0; i < 2; i++ {
		if _, ok := got[fmt.Sprintf("akey-%d", i)]; !ok {
			return fmt.Errorf("read_uncommitted missing aborted record akey-%d", i)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit uncommitted: %w", err)
	}
	return nil
}

// multiMemberGroup creates a 3-partition topic, produces N keyed records, then
// starts TWO concurrent consumers in the same consumer group and verifies that
// every produced record is consumed exactly once (no gaps, map-deduplicated by
// (partition, offset) so rebalance re-deliveries are tolerated but not counted
// as new records).  This exercises partition assignment and group rebalancing
// with the single-node heimq broker.
func multiMemberGroup(ctx context.Context, bootstrap, topic string) error {
	const N = 60
	group := fmt.Sprintf("mmg-grp-%d", time.Now().UnixNano())

	// Create 3-partition topic.
	{
		cl, err := kgo.NewClient(kgo.SeedBrokers(bootstrap))
		if err != nil {
			return fmt.Errorf("admin client: %w", err)
		}
		adm := kadm.NewClient(cl)
		resp, err := adm.CreateTopics(ctx, 3, 1, nil, topic)
		cl.Close()
		if err != nil {
			return fmt.Errorf("create topics: %w", err)
		}
		if r, ok := resp[topic]; ok && r.Err != nil {
			return fmt.Errorf("topic create: %w", r.Err)
		}
	}

	// Produce N records; collect produced (partition:offset) set.
	produced := make(map[string]bool, N)
	{
		cl, err := kgo.NewClient(
			kgo.SeedBrokers(bootstrap),
			kgo.DefaultProduceTopic(topic),
		)
		if err != nil {
			return fmt.Errorf("producer: %w", err)
		}
		for i := 0; i < N; i++ {
			res := cl.ProduceSync(ctx, &kgo.Record{
				Key:   []byte(fmt.Sprintf("mmg-key-%d", i)),
				Value: []byte(fmt.Sprintf("mmg-val-%d", i)),
			})
			if err := res.FirstErr(); err != nil {
				cl.Close()
				return fmt.Errorf("produce %d: %w", i, err)
			}
			for _, pr := range res {
				produced[fmt.Sprintf("%d:%d", pr.Record.Partition, pr.Record.Offset)] = true
			}
		}
		cl.Close()
	}
	if len(produced) != N {
		return fmt.Errorf("produced %d unique (partition,offset) pairs, expected %d", len(produced), N)
	}

	// Two concurrent consumers in the same group.
	cctx, ccancel := context.WithTimeout(ctx, 30*time.Second)
	defer ccancel()

	// Each consumer sends its consumed (partition:offset) strings to ch.
	// Buffer is large enough that goroutines never block on send.
	ch := make(chan string, N*3)
	var wg sync.WaitGroup

	startConsumer := func() {
		wg.Add(1)
		go func() {
			defer wg.Done()
			cl, err := kgo.NewClient(
				kgo.SeedBrokers(bootstrap),
				kgo.ConsumerGroup(group),
				kgo.ConsumeTopics(topic),
				kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()),
			)
			if err != nil {
				return // main goroutine will time out
			}
			defer cl.Close()
			for cctx.Err() == nil {
				fetches := cl.PollFetches(cctx)
				fetches.EachRecord(func(r *kgo.Record) {
					select {
					case ch <- fmt.Sprintf("%d:%d", r.Partition, r.Offset):
					case <-cctx.Done():
					}
				})
			}
		}()
	}

	startConsumer()
	startConsumer()

	// Collect until we have N unique records or timeout.
	consumed := make(map[string]bool, N)
	for len(consumed) < N {
		select {
		case k := <-ch:
			consumed[k] = true
		case <-cctx.Done():
			ccancel()
			wg.Wait()
			return fmt.Errorf("timeout: consumed %d/%d unique records", len(consumed), N)
		}
	}
	ccancel()
	wg.Wait()

	// Every produced record must appear in the consumed set.
	for k := range produced {
		if !consumed[k] {
			return fmt.Errorf("produced record %s not consumed by any group member", k)
		}
	}
	return nil
}
