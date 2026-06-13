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

	return nil
}

func check(name string, fn func() error) error {
	if err := fn(); err != nil {
		return fmt.Errorf("%s: %w", name, err)
	}
	fmt.Printf("  ok  %s\n", name)
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
		want_val := fmt.Sprintf("val-%d", i)
		v, ok := got[key]
		if !ok {
			return fmt.Errorf("missing key %s", key)
		}
		if v != want_val {
			return fmt.Errorf("key %s: got %q want %q", key, v, want_val)
		}
	}

	if err := cl.CommitUncommittedOffsets(ctx); err != nil {
		return fmt.Errorf("commit offsets: %w", err)
	}
	return nil
}
