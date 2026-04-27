# US-009 — Flink Kafka source/sink against heimq

**Feature**: FEAT-005
**Priority**: P0

## Story

As a Flink user,
I want a Flink job using the Kafka source and Kafka sink connectors to
read from one heimq topic and write to another,
so that Flink pipelines (including EOS sinks where applicable) work
against heimq.

## Acceptance Criteria

- A Flink job reads from a heimq topic and writes to another heimq
  topic.
- An EOS-configured Flink sink completes a checkpoint cycle without
  errors (depends on FEAT-002 transactional support).
- A single script brings up Flink, runs the job to a checkpoint, and
  tears down; exits 0.
