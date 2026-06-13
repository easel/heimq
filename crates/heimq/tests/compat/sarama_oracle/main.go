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

	if err := check("produce", func() error {
		return produce(brokers, topic, cfg)
	}); err != nil {
		return err
	}

	if err := check("consume-via-group", func() error {
		return consumeViaGroup(brokers, topic, cfg)
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

func consumeViaGroup(brokers []string, topic string, cfg *sarama.Config) error {
	group := fmt.Sprintf("sarama-oracle-group-%d", time.Now().UnixNano())
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
