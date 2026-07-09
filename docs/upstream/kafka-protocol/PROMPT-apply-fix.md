# Prompt: upstream the kafka-protocol OOM decode fix

Use this prompt to (re)apply the fix on a fresh checkout of upstream
`kafka-protocol-rs` and open a PR. It is self-contained — everything needed is
below.

---

## Task

You are fixing a **remotely triggerable OOM denial-of-service** in the
`kafka-protocol` Rust crate (`kafka-protocol-rs/kafka-protocol-rs`). The fix must
land upstream so downstream brokers (heimq, and thus fjord/niflheim) can drop
their pinned fork.

### Background

`Array<E>::decode` and `CompactArray<E>::decode` in `src/protocol/types.rs`
pre-allocate a `Vec` using the wire-supplied element count *before reading any
element*. That count is attacker-controlled (an `i32` up to `i32::MAX` for
`Array`, an unsigned varint for `CompactArray`). A few-byte request can therefore
make a broker call `Vec::with_capacity` for hundreds of millions of elements and
reserve tens of gigabytes, getting OOM-killed. `kafka-protocol` decodes untrusted
client requests broker-side (`RequestKind::decode`, `MetadataRequest::decode`,
`ProduceRequest::decode`), so this is remotely exploitable.

The same unbounded code exists identically in 0.15, 0.16, and 0.17.

### Steps

1. **Clone upstream and branch:**
   ```
   git clone https://github.com/kafka-protocol-rs/kafka-protocol-rs
   cd kafka-protocol-rs
   git checkout -b fix/bound-array-decode-capacity
   ```

2. **Patch both decode sites in `src/protocol/types.rs`.**

   In `impl<T, E: Decoder<T>> Decoder<Option<Vec<T>>> for Array<E>`:
   ```rust
   // before
   let mut result = Vec::with_capacity(n as usize);
   // after
   let mut result = Vec::with_capacity((n as usize).min(buf.remaining()));
   ```

   In `impl<T, E: Decoder<T>> Decoder<Option<Vec<T>>> for CompactArray<E>`:
   ```rust
   // before
   let mut result = Vec::with_capacity((n - 1) as usize);
   // after
   let mut result = Vec::with_capacity(((n - 1) as usize).min(buf.remaining()));
   ```

   Add a short comment at each site explaining that a valid array of `n`
   elements needs at least `n` bytes remaining, so the pre-allocation is bounded
   by `buf.remaining()`; the `Vec` still grows as elements are pushed.

3. **Add the regression test** `tests/decode_allocation_bound.rs`. It must assert
   on *allocation size*, not on the returned `Result` — both patched and
   unpatched code return `Err`, so a `Result`-based test proves nothing. Use a
   tracking `#[global_allocator]` that records the peak single allocation, decode
   a 4-byte hostile `MetadataRequest` v0 body (`[0x32,0x32,0x32,0x32]`, declaring
   842,150,450 topics), and assert the peak stays under ~1 MiB. The exact file is
   in this directory's PR write-up / the `easel/kafka-protocol-rs` fork branch.

4. **Verify the test actually catches the bug:**
   - With the fix: `cargo test --test decode_allocation_bound` passes.
   - Revert only `src/protocol/types.rs`, keep the test, rerun: it must FAIL,
     reporting a multi-gigabyte reservation (~60 GB for this input). Restore the
     fix afterward.

5. **Confirm no regressions:** `cargo test --lib` and the integration tests stay
   green. `cargo fmt`/`cargo clippy` clean on the files you changed. (Upstream
   `main` has some pre-existing `rustfmt` skew in `src/records.rs` and one
   feature-gated doctest failure — both unrelated; do not "fix" them in this PR.)

6. **Open the PR** against `kafka-protocol-rs/kafka-protocol-rs:main` using the
   description in `PR-bound-array-decode-capacity.md` (same directory). Title:
   `fix: bound Array/CompactArray decode pre-allocation to remaining buffer (OOM DoS)`.

### Definition of done

- Both decode sites bounded by `buf.remaining()`.
- Allocation-observing regression test present, passing with the fix and failing
  without it.
- Existing suite green; changed files fmt/clippy clean.
- PR opened upstream with the provided description.
- Once merged and released, remove the `[patch.crates-io]` entry and the
  `RUSTSEC-2024-0436` ignore from heimq's `deny.toml`, and bump `kafka-protocol`
  to the released version.
