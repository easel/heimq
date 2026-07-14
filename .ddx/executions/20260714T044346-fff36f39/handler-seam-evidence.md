# Handler Seam Evidence

Bead: `heimq-0e59fef5`

Evidence:

- Extraction commit: `35e8a15` (`refactor(handlers): extract request-level handlers into heimq-handlers`).
- Published tag: `v0.1.4` at `48e6e34`.
- Current release-candidate baseline: `d000a24` (`git describe`: `v0.1.4-29-gd000a24`).
- `crates/heimq-handlers/src/codec.rs` exposes `decode_request`, `decode_request_bytes`, `encode_response`, and `encode_response_body`.
- `crates/heimq-handlers/src/produce.rs` exposes `handle_async`, `handle_async_with_config_store`, and `handle_async_with_context_and_config_store`; async dispatch calls `append_records_async` with `LogBackend` and `RequestContext`.
- Downstream Niflheim work remains open in `niflheim-ba3e609c`: bump to the published Heimq tag and retire the in-tree request path. This bead does not claim that downstream bead complete.

Verification command:

```sh
cargo test -p heimq-handlers
```

Result: passed on 2026-07-14 in this execution worktree; 27 unit tests passed
and doc-tests passed with 0 tests.
