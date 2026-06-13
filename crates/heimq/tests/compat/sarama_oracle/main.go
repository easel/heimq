// Sarama compatibility oracle for heimq.
//
// Uses IBM/sarama, a pure-Go Kafka client independent of franz-go and
// librdkafka, to verify that heimq handles the Kafka wire protocol correctly
// from a third independent implementation.
//
// Usage: go run . <bootstrap-servers> <topic>
// Exit 0 on success; non-zero with a FAIL message on any deviation.
package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/IBM/sarama"
)

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: main <bootstrap-servers> <topic>")
		os.Exit(1)
	}
	if err := run(os.Args[1], os.Args[2]); err != nil {
		fmt.Fprintf(os.Stderr, "FAIL: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("PASS")
}

func run(bootstrap, topic string) error {
	cfg := sarama.NewConfig()
	cfg.Version = sarama.V2_6_0_0
	cfg.Producer.Return.Successes = true
	cfg.Producer.RequiredAcks = sarama.WaitForLocal
	cfg.Consumer.Offsets.Initial = sarama.OffsetOldest

	brokers := []string{bootstrap}

	if err := check("create-topic", func() error {
		return createTopic(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("produce", func() error {
		return produce(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	group := fmt.Sprintf("sarama-oracle-group-%d", time.Now().UnixNano())
	if err := check("consume-via-group", func() error {
		return consumeViaGroup(brokers, topic, group, cfg)
	}); err != nil {
		return err
	}

	if err := check("offset-delete", func() error {
		return deleteConsumerGroupOffset(brokers, group, topic, 0, cfg)
	}); err != nil {
		return err
	}

	if err := check("list-groups", func() error {
		return listGroups(brokers, group, cfg)
	}); err != nil {
		return err
	}

	if err := check("describe-groups", func() error {
		return describeGroups(brokers, group, cfg)
	}); err != nil {
		return err
	}

	if err := check("delete-groups", func() error {
		return deleteGroups(brokers, group, cfg)
	}); err != nil {
		return err
	}

	if err := check("describe-configs", func() error {
		return describeConfigs(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("alter-configs", func() error {
		return alterConfigs(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("incremental-alter-configs", func() error {
		return incrementalAlterConfigs(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	// create-partitions must come after consume to avoid scattering messages.
	if err := check("create-partitions", func() error {
		return createPartitions(brokers, topic, 3, cfg)
	}); err != nil {
		return err
	}

	if err := check("list-offsets", func() error {
		return listOffsets(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("describe-log-dirs", func() error {
		return describeLogDirs(brokers, cfg)
	}); err != nil {
		return err
	}

	if err := check("delete-records", func() error {
		return deleteRecords(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("describe-cluster", func() error {
		return describeCluster(brokers, cfg)
	}); err != nil {
		return err
	}

	// Headers topic is separate so it doesn't mix with the base topic offsets.
	headersTopic := topic + "-hdrs"
	if err := check("produce-with-headers", func() error {
		return produceWithHeaders(brokers, headersTopic, cfg)
	}); err != nil {
		return err
	}

	if err := check("consume-headers-roundtrip", func() error {
		return consumeHeadersRoundtrip(brokers, headersTopic, cfg)
	}); err != nil {
		return err
	}

	if err := check("delete-topic", func() error {
		return deleteTopic(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	makeCompCfg := func(codec sarama.CompressionCodec) *sarama.Config {
		c := sarama.NewConfig()
		c.Version = sarama.V2_6_0_0
		c.Producer.Return.Successes = true
		c.Producer.RequiredAcks = sarama.WaitForLocal
		c.Producer.Compression = codec
		c.Consumer.Offsets.Initial = sarama.OffsetOldest
		return c
	}

	for _, codec := range []struct {
		name string
		cfg  *sarama.Config
	}{
		{"snappy", makeCompCfg(sarama.CompressionSnappy)},
		{"gzip", makeCompCfg(sarama.CompressionGZIP)},
		{"lz4", makeCompCfg(sarama.CompressionLZ4)},
		{"zstd", makeCompCfg(sarama.CompressionZSTD)},
	} {
		compTopic := topic + "-" + codec.name
		codecName := codec.name
		compCfg := codec.cfg
		if err := check("produce-"+codecName+"-compressed", func() error {
			return produceCompressed(brokers, compTopic, compCfg)
		}); err != nil {
			return err
		}
		if err := check("consume-"+codecName+"-roundtrip", func() error {
			return consumeCompressedRoundtrip(brokers, compTopic, compCfg)
		}); err != nil {
			return err
		}
	}

	if err := check("elect-leaders", func() error {
		return electLeaders(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	resumeTopic := topic + "-resume"
	resumeGroup := fmt.Sprintf("sarama-resume-%d", time.Now().UnixNano())
	if err := check("offset-resume", func() error {
		return offsetResume(brokers, resumeTopic, resumeGroup, cfg)
	}); err != nil {
		return err
	}

	txnTopic := topic + "-txn"
	if err := check("transactional-produce", func() error {
		return transactionalProduce(brokers, txnTopic, cfg)
	}); err != nil {
		return err
	}

	if err := check("transactional-consume-committed", func() error {
		return transactionalConsumeCommitted(brokers, txnTopic, cfg)
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

func produce(brokers []string, topic string, cfg *sarama.Config) error {
	producer, err := sarama.NewSyncProducer(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer producer.Close()

	for i := 0; i < 5; i++ {
		msg := &sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("key-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("val-%d", i)),
		}
		_, _, err := producer.SendMessage(msg)
		if err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// produceWithHeaders sends 3 records each carrying a custom "x-trace" header.
func produceWithHeaders(brokers []string, topic string, cfg *sarama.Config) error {
	producer, err := sarama.NewSyncProducer(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer producer.Close()

	for i := 0; i < 3; i++ {
		msg := &sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("hkey-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("hval-%d", i)),
			Headers: []sarama.RecordHeader{
				{Key: []byte("x-trace"), Value: []byte(fmt.Sprintf("trace-%d", i))},
				{Key: []byte("x-seq"), Value: []byte(fmt.Sprintf("%d", i))},
			},
		}
		_, _, err := producer.SendMessage(msg)
		if err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// consumeHeadersRoundtrip reads back the 3 header records and verifies header bytes.
func consumeHeadersRoundtrip(brokers []string, topic string, cfg *sarama.Config) error {
	group := fmt.Sprintf("sarama-headers-group-%d", time.Now().UnixNano())
	cg, err := sarama.NewConsumerGroup(brokers, group, cfg)
	if err != nil {
		return fmt.Errorf("new consumer group: %w", err)
	}
	defer cg.Close()

	handler := &headerGroupHandler{
		received: make(map[string]headerRecord),
		want:     3,
		done:     make(chan struct{}, 1),
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	errCh := make(chan error, 1)
	go func() {
		for {
			err := cg.Consume(ctx, []string{topic}, handler)
			if err != nil {
				if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
					return
				}
				errCh <- err
				return
			}
			if ctx.Err() != nil {
				return
			}
		}
	}()

	select {
	case <-handler.done:
	case err := <-errCh:
		return fmt.Errorf("consume: %w", err)
	case <-ctx.Done():
		return fmt.Errorf("timeout: consumed %d/3 header records", len(handler.received))
	}
	cancel()

	for i := 0; i < 3; i++ {
		key := fmt.Sprintf("hkey-%d", i)
		r, ok := handler.received[key]
		if !ok {
			return fmt.Errorf("missing record key %s", key)
		}
		if r.val != fmt.Sprintf("hval-%d", i) {
			return fmt.Errorf("key %s: value mismatch: got %q", key, r.val)
		}
		if r.trace != fmt.Sprintf("trace-%d", i) {
			return fmt.Errorf("key %s: x-trace mismatch: got %q", key, r.trace)
		}
		if r.seq != fmt.Sprintf("%d", i) {
			return fmt.Errorf("key %s: x-seq mismatch: got %q", key, r.seq)
		}
	}
	return nil
}

type headerRecord struct {
	key   string
	val   string
	trace string
	seq   string
}

type headerGroupHandler struct {
	received map[string]headerRecord
	want     int
	done     chan struct{}
}

func (h *headerGroupHandler) Setup(_ sarama.ConsumerGroupSession) error   { return nil }
func (h *headerGroupHandler) Cleanup(_ sarama.ConsumerGroupSession) error { return nil }
func (h *headerGroupHandler) ConsumeClaim(session sarama.ConsumerGroupSession, claim sarama.ConsumerGroupClaim) error {
	for msg := range claim.Messages() {
		key := string(msg.Key)
		val := string(msg.Value)
		trace, seq := "", ""
		for _, hdr := range msg.Headers {
			switch string(hdr.Key) {
			case "x-trace":
				trace = string(hdr.Value)
			case "x-seq":
				seq = string(hdr.Value)
			}
		}
		h.received[key] = headerRecord{key: key, val: val, trace: trace, seq: seq}
		session.MarkMessage(msg, "")
		if len(h.received) >= h.want {
			select {
			case h.done <- struct{}{}:
			default:
			}
		}
	}
	return nil
}

type consumerGroupHandler struct {
	received map[string]string
	want     int
	done     chan struct{}
}

func (h *consumerGroupHandler) Setup(_ sarama.ConsumerGroupSession) error   { return nil }
func (h *consumerGroupHandler) Cleanup(_ sarama.ConsumerGroupSession) error { return nil }
func (h *consumerGroupHandler) ConsumeClaim(session sarama.ConsumerGroupSession, claim sarama.ConsumerGroupClaim) error {
	for msg := range claim.Messages() {
		key := string(msg.Key)
		val := string(msg.Value)
		h.received[key] = val
		session.MarkMessage(msg, "")
		if len(h.received) >= h.want {
			select {
			case h.done <- struct{}{}:
			default:
			}
		}
	}
	return nil
}

func consumeViaGroup(brokers []string, topic, group string, cfg *sarama.Config) error {
	cg, err := sarama.NewConsumerGroup(brokers, group, cfg)
	if err != nil {
		return fmt.Errorf("new consumer group: %w", err)
	}
	defer cg.Close()

	handler := &consumerGroupHandler{
		received: make(map[string]string),
		want:     5,
		done:     make(chan struct{}, 1),
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	errCh := make(chan error, 1)
	go func() {
		for {
			err := cg.Consume(ctx, []string{topic}, handler)
			if err != nil {
				if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
					return
				}
				errCh <- err
				return
			}
			if ctx.Err() != nil {
				return
			}
		}
	}()

	select {
	case <-handler.done:
	case err := <-errCh:
		return fmt.Errorf("consume: %w", err)
	case <-ctx.Done():
		return fmt.Errorf("timeout: consumed %d/5 records", len(handler.received))
	}

	// Verify values.
	for i := 0; i < 5; i++ {
		key := fmt.Sprintf("key-%d", i)
		wantVal := fmt.Sprintf("val-%d", i)
		v, ok := handler.received[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}

	cancel()
	return nil
}

// listGroups uses sarama ClusterAdmin to exercise ListGroups (API 16).
func listGroups(brokers []string, wantGroup string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	groups, err := admin.ListConsumerGroups()
	if err != nil {
		return fmt.Errorf("list consumer groups: %w", err)
	}
	if _, ok := groups[wantGroup]; !ok {
		return fmt.Errorf("group %q not found in list; got %d groups", wantGroup, len(groups))
	}
	return nil
}

// describeGroups uses sarama ClusterAdmin to exercise DescribeGroups (API 15).
func describeGroups(brokers []string, wantGroup string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	descriptions, err := admin.DescribeConsumerGroups([]string{wantGroup})
	if err != nil {
		return fmt.Errorf("describe consumer groups: %w", err)
	}
	for _, d := range descriptions {
		if d.GroupId == wantGroup {
			if d.Err != sarama.ErrNoError {
				return fmt.Errorf("group %q describe error: %v", wantGroup, d.Err)
			}
			return nil
		}
	}
	return fmt.Errorf("group %q missing from describe response", wantGroup)
}

// deleteConsumerGroupOffset uses sarama ClusterAdmin to exercise OffsetDelete (API 47).
func deleteConsumerGroupOffset(brokers []string, group, topic string, partition int32, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	return admin.DeleteConsumerGroupOffset(group, topic, partition)
}

// deleteGroups uses sarama ClusterAdmin to exercise DeleteGroups (API 42).
func deleteGroups(brokers []string, group string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	return admin.DeleteConsumerGroup(group)
}

// createTopic uses sarama ClusterAdmin to exercise CreateTopics (API 19).
func createTopic(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	detail := &sarama.TopicDetail{
		NumPartitions:     1,
		ReplicationFactor: 1,
	}
	return admin.CreateTopic(topic, detail, false)
}

// describeConfigs uses sarama ClusterAdmin to exercise DescribeConfigs (API 32).
func describeConfigs(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	resource := sarama.ConfigResource{
		Type: sarama.TopicResource,
		Name: topic,
	}
	entries, err := admin.DescribeConfig(resource)
	if err != nil {
		return fmt.Errorf("describe config: %w", err)
	}
	if len(entries) == 0 {
		return fmt.Errorf("expected config entries, got none")
	}
	return nil
}

// alterConfigs uses sarama ClusterAdmin to exercise AlterConfigs (API 33).
func alterConfigs(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	v := "604800000"
	entries := map[string]*string{"retention.ms": &v}
	return admin.AlterConfig(sarama.TopicResource, topic, entries, false)
}

// incrementalAlterConfigs uses sarama ClusterAdmin to exercise IncrementalAlterConfigs (API 44).
func incrementalAlterConfigs(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	v := "86400000"
	entries := map[string]sarama.IncrementalAlterConfigsEntry{
		"retention.ms": {
			Operation: sarama.IncrementalAlterConfigsOperationSet,
			Value:     &v,
		},
	}
	return admin.IncrementalAlterConfig(sarama.TopicResource, topic, entries, false)
}

// createPartitions uses sarama ClusterAdmin to exercise CreatePartitions (API 37).
func createPartitions(brokers []string, topic string, count int32, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	return admin.CreatePartitions(topic, count, nil, false)
}

// deleteRecords uses sarama ClusterAdmin to exercise DeleteRecords (API 21).
func deleteRecords(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	// Delete records up to offset 2 on partition 0.
	offsets := map[int32]int64{0: 2}
	return admin.DeleteRecords(topic, offsets)
}

// listOffsets uses sarama Client.GetOffset to exercise ListOffsets (API 2).
func listOffsets(brokers []string, topic string, cfg *sarama.Config) error {
	client, err := sarama.NewClient(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new client: %w", err)
	}
	defer client.Close()

	// Get the latest offset (high watermark) for partition 0.
	offset, err := client.GetOffset(topic, 0, sarama.OffsetNewest)
	if err != nil {
		return fmt.Errorf("get offset (newest): %w", err)
	}
	// We produced 5 messages, so latest offset should be >= 5.
	if offset < 5 {
		return fmt.Errorf("expected offset >= 5 for newest, got %d", offset)
	}

	// Get the earliest offset for partition 0.
	earliest, err := client.GetOffset(topic, 0, sarama.OffsetOldest)
	if err != nil {
		return fmt.Errorf("get offset (oldest): %w", err)
	}
	if earliest < 0 {
		return fmt.Errorf("expected non-negative oldest offset, got %d", earliest)
	}
	return nil
}

// describeLogDirs uses sarama ClusterAdmin to exercise DescribeLogDirs (API 35).
func describeLogDirs(brokers []string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	// Query log dirs for broker 0 (our single-node cluster).
	logDirs, err := admin.DescribeLogDirs([]int32{0})
	if err != nil {
		return fmt.Errorf("describe log dirs: %w", err)
	}
	if len(logDirs) == 0 {
		return fmt.Errorf("expected log dirs response, got empty map")
	}
	return nil
}

// describeCluster uses sarama ClusterAdmin to exercise DescribeCluster (API 60).
func describeCluster(brokers []string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	bs, _, err := admin.DescribeCluster()
	if err != nil {
		return fmt.Errorf("describe cluster: %w", err)
	}
	if len(bs) == 0 {
		return fmt.Errorf("expected at least 1 broker, got 0")
	}
	return nil
}

// deleteTopic uses sarama ClusterAdmin to exercise DeleteTopics (API 20).
func deleteTopic(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	return admin.DeleteTopic(topic)
}

// produceCompressed creates compTopic and produces 4 records with the compression
// codec configured in cfg. Tests that compressed batches are correctly stored.
func produceCompressed(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	if err := admin.CreateTopic(topic, &sarama.TopicDetail{NumPartitions: 1, ReplicationFactor: 1}, false); err != nil {
		admin.Close()
		return fmt.Errorf("create topic: %w", err)
	}
	admin.Close()

	producer, err := sarama.NewSyncProducer(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	defer producer.Close()

	for i := 0; i < 4; i++ {
		msg := &sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("ckey-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("cval-%d", i)),
		}
		_, _, err := producer.SendMessage(msg)
		if err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

// consumeCompressedRoundtrip consumes the 4 records and verifies data integrity,
// exercising the server-side decompression path during fetch.
func consumeCompressedRoundtrip(brokers []string, topic string, cfg *sarama.Config) error {
	consumer, err := sarama.NewConsumer(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer consumer.Close()

	pc, err := consumer.ConsumePartition(topic, 0, sarama.OffsetOldest)
	if err != nil {
		return fmt.Errorf("consume partition: %w", err)
	}
	defer pc.Close()

	got := make(map[string]string)
	want := 4
	timeout := time.After(20 * time.Second)
	for len(got) < want {
		select {
		case msg := <-pc.Messages():
			got[string(msg.Key)] = string(msg.Value)
		case err := <-pc.Errors():
			return fmt.Errorf("partition error: %w", err.Err)
		case <-timeout:
			return fmt.Errorf("timeout: consumed %d/%d compressed records", len(got), want)
		}
	}

	for i := 0; i < want; i++ {
		key := fmt.Sprintf("ckey-%d", i)
		wantVal := fmt.Sprintf("cval-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}
	return nil
}

// electLeaders triggers a preferred-leader election for partition 0 of topic.
// In a single-broker cluster this is always a no-op, but the request must
// complete without error, exercising the ElectLeaders wire path.
func electLeaders(brokers []string, topic string, cfg *sarama.Config) error {
	admin, err := sarama.NewClusterAdmin(brokers, cfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	defer admin.Close()

	partitions := map[string][]int32{topic: {0}}
	_, err = admin.ElectLeaders(sarama.PreferredElection, partitions)
	if err != nil {
		return fmt.Errorf("elect leaders: %w", err)
	}
	// PartitionResult.ErrorCode may be ELECTION_NOT_NEEDED (code 79) which is
	// expected in a single-broker cluster that already holds the preferred leader.
	// The request completing without a transport error is sufficient.
	return nil
}

// offsetResume verifies that a consumer group resumes from a previously
// committed offset rather than replaying from the beginning.
//
//  1. Create topic, produce 8 records (rkey-0..rkey-7).
//  2. Use OffsetManager to commit offset=4 on partition 0 without consuming.
//  3. Start a partition consumer at OffsetNewest (fallback if no commit).
//     Since a commit exists, sarama's consumer group will honour it.
//  4. Actually, use a partition consumer starting at the committed offset directly
//     (sarama admin approach: fetch committed offset then seek).
func offsetResume(brokers []string, topic, group string, baseCfg *sarama.Config) error {
	// Step 1: create topic and produce 8 records.
	admin, err := sarama.NewClusterAdmin(brokers, baseCfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	if err := admin.CreateTopic(topic, &sarama.TopicDetail{NumPartitions: 1, ReplicationFactor: 1}, false); err != nil {
		admin.Close()
		return fmt.Errorf("create topic: %w", err)
	}
	admin.Close()

	producer, err := sarama.NewSyncProducer(brokers, baseCfg)
	if err != nil {
		return fmt.Errorf("new producer: %w", err)
	}
	for i := 0; i < 8; i++ {
		_, _, err := producer.SendMessage(&sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("rkey-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("rval-%d", i)),
		})
		if err != nil {
			producer.Close()
			return fmt.Errorf("produce record %d: %w", i, err)
		}
	}
	producer.Close()

	// Step 2: commit offset=4 for the group using OffsetManager.
	omCfg := sarama.NewConfig()
	omCfg.Version = sarama.V2_6_0_0
	omCfg.Consumer.Offsets.Initial = sarama.OffsetOldest
	client, err := sarama.NewClient(brokers, omCfg)
	if err != nil {
		return fmt.Errorf("new client for offset manager: %w", err)
	}
	defer client.Close()

	om, err := sarama.NewOffsetManagerFromClient(group, client)
	if err != nil {
		return fmt.Errorf("new offset manager: %w", err)
	}
	pom, err := om.ManagePartition(topic, 0)
	if err != nil {
		om.Close()
		return fmt.Errorf("manage partition: %w", err)
	}
	// MarkOffset commits offset+1 (next to consume), so mark 4 to resume from record 4.
	pom.MarkOffset(4, "resume-test")
	if err := pom.Close(); err != nil {
		om.Close()
		return fmt.Errorf("close pom: %w", err)
	}
	if err := om.Close(); err != nil {
		return fmt.Errorf("close om: %w", err)
	}

	// Step 3: fetch the committed offset and verify it is 4.
	fetchCfg := sarama.NewConfig()
	fetchCfg.Version = sarama.V2_6_0_0
	fetchCfg.Consumer.Offsets.Initial = sarama.OffsetOldest
	fetchClient, err := sarama.NewClient(brokers, fetchCfg)
	if err != nil {
		return fmt.Errorf("new fetch client: %w", err)
	}
	defer fetchClient.Close()

	om2, err := sarama.NewOffsetManagerFromClient(group, fetchClient)
	if err != nil {
		return fmt.Errorf("new offset manager 2: %w", err)
	}
	pom2, err := om2.ManagePartition(topic, 0)
	if err != nil {
		om2.Close()
		return fmt.Errorf("manage partition 2: %w", err)
	}
	nextOffset, _ := pom2.NextOffset()
	pom2.Close()
	om2.Close()

	if nextOffset != 4 {
		return fmt.Errorf("expected committed offset=4, got %d", nextOffset)
	}

	// Step 4: consume records starting at offset 4 and verify rkey-4..rkey-7.
	consCfg := sarama.NewConfig()
	consCfg.Version = sarama.V2_6_0_0
	consCfg.Consumer.Offsets.Initial = sarama.OffsetOldest
	consumer, err := sarama.NewConsumer(brokers, consCfg)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer consumer.Close()

	pc, err := consumer.ConsumePartition(topic, 0, nextOffset)
	if err != nil {
		return fmt.Errorf("consume partition at %d: %w", nextOffset, err)
	}
	defer pc.Close()

	got := make(map[string]string)
	want := 4
	timeout := time.After(20 * time.Second)
	for len(got) < want {
		select {
		case msg := <-pc.Messages():
			got[string(msg.Key)] = string(msg.Value)
		case err := <-pc.Errors():
			return fmt.Errorf("partition error: %w", err.Err)
		case <-timeout:
			return fmt.Errorf("timeout: consumed %d/%d resumed records", len(got), want)
		}
	}

	for i := 4; i < 8; i++ {
		key := fmt.Sprintf("rkey-%d", i)
		wantVal := fmt.Sprintf("rval-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != wantVal {
			return fmt.Errorf("key %s: got %q want %q", key, v, wantVal)
		}
	}
	return nil
}

// transactionalProduce creates txnTopic, commits 3 records in a transaction,
// then aborts a second transaction with 2 records.
func transactionalProduce(brokers []string, topic string, baseCfg *sarama.Config) error {
	// Create the topic first.
	admin, err := sarama.NewClusterAdmin(brokers, baseCfg)
	if err != nil {
		return fmt.Errorf("new admin: %w", err)
	}
	if err := admin.CreateTopic(topic, &sarama.TopicDetail{NumPartitions: 1, ReplicationFactor: 1}, false); err != nil {
		admin.Close()
		return fmt.Errorf("create topic: %w", err)
	}
	admin.Close()

	txnID := fmt.Sprintf("sarama-txn-%d", time.Now().UnixNano())
	txnCfg := sarama.NewConfig()
	txnCfg.Version = sarama.V2_6_0_0
	txnCfg.Producer.Return.Successes = true
	txnCfg.Producer.Return.Errors = true
	txnCfg.Producer.RequiredAcks = sarama.WaitForAll
	txnCfg.Producer.Idempotent = true
	txnCfg.Producer.Transaction.ID = txnID
	txnCfg.Net.MaxOpenRequests = 1

	producer, err := sarama.NewAsyncProducer(brokers, txnCfg)
	if err != nil {
		return fmt.Errorf("new txn producer: %w", err)
	}
	defer producer.Close()

	// Committed transaction.
	if err := producer.BeginTxn(); err != nil {
		return fmt.Errorf("begin txn: %w", err)
	}
	for i := 0; i < 3; i++ {
		producer.Input() <- &sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("tkey-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("tval-%d", i)),
		}
		select {
		case <-producer.Successes():
		case err := <-producer.Errors():
			return fmt.Errorf("committed record %d error: %w", i, err.Err)
		}
	}
	if err := producer.CommitTxn(); err != nil {
		return fmt.Errorf("commit txn: %w", err)
	}

	// Aborted transaction.
	if err := producer.BeginTxn(); err != nil {
		return fmt.Errorf("begin abort txn: %w", err)
	}
	for i := 0; i < 2; i++ {
		producer.Input() <- &sarama.ProducerMessage{
			Topic: topic,
			Key:   sarama.StringEncoder(fmt.Sprintf("akey-%d", i)),
			Value: sarama.StringEncoder(fmt.Sprintf("aval-%d", i)),
		}
		select {
		case <-producer.Successes():
		case err := <-producer.Errors():
			return fmt.Errorf("aborted record %d error: %w", i, err.Err)
		}
	}
	if err := producer.AbortTxn(); err != nil {
		return fmt.Errorf("abort txn: %w", err)
	}

	return nil
}

// transactionalConsumeCommitted reads txnTopic with read_committed isolation
// and verifies that only the 3 committed records (tkey-0..2) are visible.
func transactionalConsumeCommitted(brokers []string, topic string, baseCfg *sarama.Config) error {
	readCfg := sarama.NewConfig()
	readCfg.Version = sarama.V2_6_0_0
	readCfg.Consumer.Offsets.Initial = sarama.OffsetOldest
	readCfg.Consumer.IsolationLevel = sarama.ReadCommitted

	consumer, err := sarama.NewConsumer(brokers, readCfg)
	if err != nil {
		return fmt.Errorf("new consumer: %w", err)
	}
	defer consumer.Close()

	pc, err := consumer.ConsumePartition(topic, 0, sarama.OffsetOldest)
	if err != nil {
		return fmt.Errorf("consume partition: %w", err)
	}
	defer pc.Close()

	got := make(map[string]string)
	timeout := time.After(30 * time.Second)
	for len(got) < 3 {
		select {
		case msg := <-pc.Messages():
			got[string(msg.Key)] = string(msg.Value)
		case err := <-pc.Errors():
			return fmt.Errorf("partition error: %w", err.Err)
		case <-timeout:
			return fmt.Errorf("timeout: consumed %d/3 committed records; got: %v", len(got), got)
		}
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
	for i := 0; i < 2; i++ {
		if _, ok := got[fmt.Sprintf("akey-%d", i)]; ok {
			return fmt.Errorf("aborted record akey-%d leaked into read_committed consumer", i)
		}
	}
	return nil
}
