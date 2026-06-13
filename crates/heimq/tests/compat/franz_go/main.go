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
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	ts := time.Now().UnixNano()
	topic := fmt.Sprintf("franz-compat-%d", ts)
	group := fmt.Sprintf("franz-group-%d", ts)
	hdrTopic := fmt.Sprintf("franz-hdrs-%d", ts)
	hdrGroup := fmt.Sprintf("franz-hdrs-group-%d", ts)

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

	if err := check("delete-records", func() error {
		return deleteRecords(ctx, bootstrap, topic)
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
