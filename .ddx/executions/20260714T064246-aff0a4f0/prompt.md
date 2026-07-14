<bead-review>
  <bead id="heimq-168f24e2" iter=1>
    <title>Stabilize and document the heimq-handlers embedder contract and SASL ownership</title>
    <description>
Document the supported public embedder contract required by Niflheim: codec request/response envelope entrypoints; the async Produce handler with context and config store; Metadata, ApiVersions, and InitProducerId handlers; and RecordBatchView::decode_all. Record the architectural decision that TLS/SASL authentication remains embedder-owned and is intentionally outside heimq-handlers dispatch. Add API-level tests or compile-time coverage sufficient to detect accidental surface removal.
    </description>
    <acceptance>
1. Public rustdoc identifies codec::{decode_request, decode_request_bytes, encode_response, encode_response_body}, produce::handle_async_with_context_and_config_store, the Metadata/ApiVersions/InitProducerId handlers, and RecordBatchView::decode_all as supported embedder APIs. 2. A governing HELIX artifact explicitly states that TLS/SASL is embedder-owned and Heimq handlers do not own authentication. 3. Tests exercise or compile against every named entrypoint. 4. cargo test --workspace --all-features and cargo doc --workspace --no-deps complete successfully.
    </acceptance>
    <labels>helix, cross-repo, niflheim, embedder, kafka, documentation</labels>
  </bead>

  <changed-files>
    <file>crates/heimq-handlers/src/lib.rs</file>
    <file>crates/heimq-handlers/tests/embedder_api.rs</file>
    <file>crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="2b12962ea24bca3347c5696101d30f9f3e52cfe0">
<untrusted-data>
diff --git a/crates/heimq-handlers/src/lib.rs b/crates/heimq-handlers/src/lib.rs
index 668c1cf..4142af0 100644
--- a/crates/heimq-handlers/src/lib.rs
+++ b/crates/heimq-handlers/src/lib.rs
@@ -7,6 +7,26 @@
 //! directly on its public surface. Embedders adopt the same `heimq_protocol`
 //! fork, so exposing those types here does not resolve a second, conflicting
 //! copy of `kafka-protocol`.
+//!
+//! ## Supported embedder API
+//!
+//! The public embedder contract intentionally includes:
+//!
+//! - [`codec::decode_request`], [`codec::decode_request_bytes`],
+//!   [`codec::encode_response`], and [`codec::encode_response_body`] for
+//!   request-envelope decode and response-envelope encode.
+//! - [`produce::handle_async_with_context_and_config_store`] for async Produce
+//!   dispatch with [`storage::RequestContext`] and [`config_store::ConfigStore`]
+//!   supplied by the embedding broker.
+//! - [`metadata::handle`], [`api_versions::handle`], and
+//!   [`init_producer_id::handle`] for the Metadata, ApiVersions, and
+//!   InitProducerId request handlers.
+//! - [`heimq_broker::storage::RecordBatchView::decode_all`] for decoding every
+//!   record set carried by a produce partition before backend append.
+//!
+//! TLS/SASL authentication is not part of this crate's dispatch contract.
+//! Embedders authenticate connections before calling `heimq-handlers` and pass
+//! any resulting identity through [`storage::RequestContext`].
 
 // The handler and codec modules were extracted from the heimq binary, which
 // allows this lint crate-wide; keep the same policy so the moved code's
diff --git a/crates/heimq-handlers/tests/embedder_api.rs b/crates/heimq-handlers/tests/embedder_api.rs
new file mode 100644
index 0000000..8ce5c30
--- /dev/null
+++ b/crates/heimq-handlers/tests/embedder_api.rs
@@ -0,0 +1,83 @@
+use bytes::Bytes;
+use heimq_broker::storage::{ClusterView, LogBackend, RecordBatchView};
+use heimq_broker::RequestContext;
+use heimq_handlers::config_store::ConfigStore;
+use heimq_handlers::producer_state::ProducerStateManager;
+use heimq_handlers::transaction_state::TransactionManager;
+use heimq_handlers::{api_versions, codec, init_producer_id, metadata, produce};
+use heimq_protocol::messages::{ApiVersionsResponse, InitProducerIdResponse, MetadataResponse};
+use std::sync::Arc;
+
+type MetadataHandler = fn(
+    i16,
+    &[u8],
+    &Arc<dyn LogBackend>,
+    &dyn ClusterView,
+) -> heimq_handlers::error::Result<MetadataResponse>;
+type ApiVersionsHandler = fn(i16, &[(i16, i16, i16)]) -> ApiVersionsResponse;
+type InitProducerIdHandler = fn(
+    i16,
+    &[u8],
+    &Arc<TransactionManager>,
+) -> heimq_handlers::error::Result<InitProducerIdResponse>;
+
+#[test]
+fn codec_embedder_entrypoints_decode_and_encode() {
+    let request = [
+        0x00, 0x12, // api_key = ApiVersions
+        0x00, 0x00, // api_version = 0
+        0x00, 0x00, 0x00, 0x2A, // correlation_id = 42
+        0xFF, 0xFF, // client_id = null
+    ];
+
+    let (header, body) = codec::decode_request(&request).expect("decode borrowed request");
+    assert_eq!(header.api_key, 18);
+    assert_eq!(header.correlation_id, 42);
+    assert!(body.is_empty());
+
+    let (owned_header, owned_body) = codec::decode_request_bytes(Bytes::copy_from_slice(&request))
+        .expect("decode owned request");
+    assert_eq!(owned_header.api_version, 0);
+    assert!(owned_body.is_empty());
+
+    let response = ApiVersionsResponse::default();
+    let framed = codec::encode_response(42, 18, 0, &response).expect("encode framed response");
+    assert!(framed.len() >= 8);
+
+    let body = codec::encode_response_body(42, 0, &response).expect("encode response body");
+    assert!(body.len() >= 4);
+}
+
+#[test]
+fn typed_handler_entrypoints_keep_their_embedder_signatures() {
+    let _: MetadataHandler = metadata::handle;
+    let _: ApiVersionsHandler = api_versions::handle;
+    let _: InitProducerIdHandler = init_producer_id::handle;
+}
+
+#[test]
+fn record_batch_decode_all_embedder_entrypoint_accepts_empty_input() {
+    let decoded = RecordBatchView::decode_all(&[]).expect("empty record set decodes");
+    assert!(decoded.is_empty());
+}
+
+#[allow(dead_code)]
+fn compile_against_async_produce_embedder_signature(storage: Option<Arc<dyn LogBackend>>) {
+    if let Some(storage) = storage {
+        let producer_state = ProducerStateManager::new();
+        let transaction_manager = TransactionManager::new();
+        let ctx = RequestContext::ANONYMOUS;
+        let config_store = Arc::new(ConfigStore::new());
+
+        let _future = produce::handle_async_with_context_and_config_store(
+            11,
+            &[],
+            &storage,
+            &producer_state,
+            &transaction_manager,
+            &ctx,
+            &config_store,
+            604_800_000,
+        );
+    }
+}
diff --git a/crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md b/crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md
index 90bb09e..9303876 100644
--- a/crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md
+++ b/crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md
@@ -41,6 +41,14 @@ tracked by `niflheim-ba3e609c`.
 | `heimq-handlers` | Request-level decode/dispatch/encode layer above `heimq-broker`; exposes embedder-facing codec helpers and typed handlers for Produce, ApiVersions, Metadata, and InitProducerId |
 | `heimq-testkit` | Per-trait conformance suites, contract-test pattern, differential parity harness |
 
+**Authentication ownership:** TLS/SASL authentication is embedder-owned for the
+engine consumption model. `heimq-handlers` does not own authentication, does not
+run SASL handshakes, and does not dispatch SaslHandshake/SaslAuthenticate as
+handler APIs. An embedding gateway authenticates the connection before handler
+dispatch and threads any principal, tenant, or client identity through
+`RequestContext`. The standalone `heimq` distribution and future wire gateways
+may choose their own TLS/SASL policy outside the `heimq-handlers` crate.
+
 **Excluded vendored dependency:**
 
 | Crate | Status |
</untrusted-data>
  </diff>

  <strictness-mode mode="strict">strict — each AC must be anchored to a named Test* function or a diff-touched symbol; file-only evidence is insufficient.</strictness-mode>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

## AC-Check Ratification

When an &lt;ac-check&gt; section is present, ratify the mechanical results rather
than re-verifying them independently from the diff:

- result="pass": confirm the evidence is credible. Override to fail only if
  the evidence is fabricated — include judgment_override_reason and a diff
  citation (file:line) in the per_ac evidence string.
- result="fail": mechanically verified failure. Grade as fail and BLOCK unless
  the commit message contains an explicit AC-Waive trailer for this AC.
- result="needs_judgment": adjudicate from the diff. If you cannot determine
  pass/fail without additional bead context from the operator, use
  REQUEST_CLARIFICATION for that AC item.
- result="error": treat as needs_judgment.

Overriding a mechanical grade (pass→fail or fail→pass) requires an explicit
judgment_override_reason note and a concrete diff citation in the evidence.

## Strictness Mode

The &lt;strictness-mode&gt; tag specifies per-bead evidence requirements:

- strict (kind:fix, kind:feat): each AC must be anchored to a named Test*
  function or a diff-touched symbol; file-only evidence is insufficient.
- behavior-light (kind:refactor, kind:chore): build green plus file/symbol
  evidence suffices; test-name match required only when an AC explicitly
  names a Test* function.
- mechanical (kind:doc, kind:mechanical): file presence, renames, or symbol
  evidence only; no test-name or runtime evidence required.

## Verdicts

For each acceptance-criteria (AC) item, decide whether it is implemented
correctly, then assign one overall verdict:

- APPROVE — every AC item is fully and correctly implemented.
- REQUEST_CHANGES — some AC items are partial or have fixable minor issues.
- BLOCK — at least one AC item is not implemented or incorrectly implemented;
  or the diff is insufficient to evaluate.
- REQUEST_CLARIFICATION — you cannot adjudicate one or more needs_judgment AC
  items without operator clarification. Use this ONLY when the item is
  ambiguous even given the full diff. This verdict does NOT block the queue;
  it routes to the operator lane for input.

## Required output format (schema_version: 1)

Respond with EXACTLY one JSON object as your final response, fenced as a single ```json … ``` code block. Do not include any prose outside the fenced block. The JSON must match this schema:

```json
{
  "schema_version": 1,
  "verdict": "APPROVE",
  "summary": "≤300 char human-readable verdict justification",
  "per_ac": [
    { "number": 1, "item": "acceptance criterion text", "grade": "pass", "evidence": "file:line or test evidence" }
  ],
  "findings": [
    { "severity": "info", "summary": "what is wrong or notable", "location": "path/to/file.go:42" }
  ]
}
```

Rules:
- "verdict" must be exactly one of "APPROVE", "REQUEST_CHANGES", "BLOCK", "REQUEST_CLARIFICATION".
- "severity" must be exactly one of "info", "warn", "block".
- Output the JSON object inside ONE fenced ```json … ``` block. No additional prose, no extra fences, no markdown headings.
- Do not echo this template back. Do not write the verdict value anywhere except as the JSON value of the verdict field.
  </instructions>
</bead-review>
