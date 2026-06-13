#!/usr/bin/env bash
# Integration target 2: Go confluent-kafka-go (librdkafka CGO binding).
# Produces 20 messages and consumes them back via heimq.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

TOPIC="eco-go-${RUN_ID}"

echo "==> [2/8] librdkafka Go (confluent-kafka-go)"

docker run --rm --network host \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$TOPIC" \
    golang:1.22-bookworm \
    bash -c '
set -e
apt-get update -q && apt-get install -y -q librdkafka-dev 2>/dev/null

mkdir -p /tmp/kafkatest && cd /tmp/kafkatest
go mod init kafkatest

cat > main.go << '"'"'GOEOF'"'"'
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/confluentinc/confluent-kafka-go/v2/kafka"
)

func main() {
	bootstrap := os.Getenv("BOOTSTRAP")
	topic := os.Getenv("TOPIC")
	n := 20

	// Admin: create topic
	admin, err := kafka.NewAdminClient(&kafka.ConfigMap{"bootstrap.servers": bootstrap})
	if err != nil {
		panic(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	results, err := admin.CreateTopics(ctx, []kafka.TopicSpecification{{
		Topic: topic, NumPartitions: 1, ReplicationFactor: 1,
	}})
	if err != nil {
		fmt.Fprintf(os.Stderr, "CreateTopics error: %v\n", err)
	} else {
		for _, r := range results {
			if r.Error.Code() != kafka.ErrNoError && r.Error.Code() != kafka.ErrTopicAlreadyExists {
				fmt.Fprintf(os.Stderr, "topic %s: %v\n", r.Topic, r.Error)
			}
		}
	}
	admin.Close()

	// Produce
	p, err := kafka.NewProducer(&kafka.ConfigMap{"bootstrap.servers": bootstrap})
	if err != nil {
		panic(err)
	}
	for i := 0; i < n; i++ {
		err = p.Produce(&kafka.Message{
			TopicPartition: kafka.TopicPartition{Topic: &topic, Partition: kafka.PartitionAny},
			Value:          []byte(fmt.Sprintf("msg-%d", i)),
		}, nil)
		if err != nil {
			panic(err)
		}
	}
	p.Flush(10000)
	p.Close()

	// Consume
	c, err := kafka.NewConsumer(&kafka.ConfigMap{
		"bootstrap.servers":  bootstrap,
		"group.id":           "eco-go-test",
		"auto.offset.reset":  "earliest",
		"enable.auto.commit": "false",
	})
	if err != nil {
		panic(err)
	}
	if err = c.SubscribeTopics([]string{topic}, nil); err != nil {
		panic(err)
	}

	received := 0
	deadline := time.Now().Add(15 * time.Second)
	for received < n && time.Now().Before(deadline) {
		msg, err := c.ReadMessage(time.Second)
		if err != nil {
			continue
		}
		_ = msg
		received++
	}
	c.Close()

	if received < n {
		fmt.Fprintf(os.Stderr, "FAIL: expected %d messages, received %d\n", n, received)
		os.Exit(1)
	}
	fmt.Printf("produced %d, consumed %d\n", n, received)
}
GOEOF

go get github.com/confluentinc/confluent-kafka-go/v2/kafka
go run main.go
'

eco_pass "librdkafka Go: produce+consume via confluent-kafka-go"
eco_summary
