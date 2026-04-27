# US-010 — Debezium emits CDC into heimq

**Feature**: FEAT-005
**Priority**: P0

## Story

As a Debezium user,
I want a Debezium connector to emit CDC events into a heimq topic,
so that CDC pipelines work against heimq for tests / local
development.

## Acceptance Criteria

- A Debezium connector (e.g., the PostgreSQL connector or Debezium
  Server) is configured against heimq and produces CDC envelopes for
  a sample database change set.
- An rdkafka consumer reads the expected CDC envelope from the
  configured heimq topic.
- A single script brings up the connector, runs the change set, and
  tears down; exits 0.
