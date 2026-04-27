# US-008 — Kafka Connect runs against heimq

**Feature**: FEAT-005
**Priority**: P0

## Story

As a developer using Kafka Connect,
I want a representative source connector and a representative sink
connector to run against heimq,
so that Connect-based pipelines work in heimq-backed test environments.

## Acceptance Criteria

- A representative source connector (e.g., `FileStreamSource`) runs
  against heimq and writes records to a topic.
- A representative sink connector (e.g., `FileStreamSink`) runs
  against heimq and consumes records from a topic.
- Each integration has a single script that brings up Connect, runs
  the test, and tears down.
- Tests exit 0 with no error lines.
