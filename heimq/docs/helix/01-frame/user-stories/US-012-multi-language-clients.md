# US-012 — Multi-language librdkafka clients against heimq

**Feature**: FEAT-005
**Priority**: P0

## Story

As a developer using a non-Rust Kafka client,
I want canonical librdkafka-based clients in multiple languages to run
against heimq,
so that polyglot test environments and services can use heimq.

## Acceptance Criteria

- An integration test in Go using `confluent-kafka-go` produces and
  consumes from heimq.
- An integration test in Python using `confluent-kafka` produces and
  consumes from heimq.
- An integration test in Node.js using `node-rdkafka` produces and
  consumes from heimq.
- Each test is reproducible via a single script per language; each
  exits 0.
