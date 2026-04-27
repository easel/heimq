# US-011 — Schema Registry round-trip against heimq

**Feature**: FEAT-005
**Priority**: P0

## Story

As a developer using a Schema Registry,
I want a producer with an Avro/Protobuf serializer to publish records
to heimq and a consumer with the matching deserializer to consume them,
so that schema-aware pipelines work against heimq for tests / local
development.

## Acceptance Criteria

- A schema is registered against a Confluent Schema Registry
  instance.
- A producer with the registry serializer writes records to a heimq
  topic.
- A consumer with the registry deserializer reads and decodes those
  records correctly.
- A single script brings up the registry, runs the round-trip, and
  tears down; exits 0.
