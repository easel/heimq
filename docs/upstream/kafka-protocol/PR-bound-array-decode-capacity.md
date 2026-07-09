# Bound `Array` / `CompactArray` decode pre-allocation to remaining bytes

**Upstream:** `kafka-protocol-rs/kafka-protocol-rs`
**Fork branch to open the PR from:** `easel/kafka-protocol-rs:fix/bound-array-decode-capacity`
**Affects:** all published versions (0.15, 0.16, 0.17) — the code is identical across them.

---

## Title

```
fix: bound Array/CompactArray decode pre-allocation to remaining buffer (OOM DoS)
```

## Summary

`Array::decode` and `CompactArray::decode` call `Vec::with_capacity(n)` with the
wire-supplied element count *before reading a single element*. The count is an
attacker-controlled `i32` (up to `i32::MAX`) / unsigned varint, so a tiny request
can make a broker reserve tens of gigabytes and get OOM-killed. Because
`kafka-protocol` is used broker-side to decode untrusted client requests
(`RequestKind::decode`, `MetadataRequest::decode`, `ProduceRequest::decode`, …),
this is a **remotely triggerable denial of service**.

A valid array of `n` elements needs at least `n` bytes still in the buffer, so
this PR caps the initial capacity at `buf.remaining()`. The `Vec` still grows as
elements are pushed, so well-formed messages are unaffected; only the unbounded
*pre*-allocation is removed.

## The bug

`src/protocol/types.rs`:

```rust
// Array::decode
n if n >= 0 => {
    let mut result = Vec::with_capacity(n as usize);   // <-- n up to i32::MAX
    for _ in 0..n {
        result.push(self.0.decode(buf)?);
    }
    ...
}

// CompactArray::decode
n => {
    let mut result = Vec::with_capacity((n - 1) as usize);  // <-- n-1 up to u32::MAX-1
    for _ in 1..n {
        result.push(self.0.decode(buf)?);
    }
    ...
}
```

## The fix

```rust
let mut result = Vec::with_capacity((n as usize).min(buf.remaining()));
```

```rust
let mut result = Vec::with_capacity(((n - 1) as usize).min(buf.remaining()));
```

Two lines, plus explanatory comments.

## Reproduction / regression test

`tests/decode_allocation_bound.rs` decodes a **4-byte** `MetadataRequest` v0 body
whose topic-array length field is `0x32323232` (842,150,450). A tracking global
allocator records the peak single allocation during decode:

- **Before the fix:** decode reserves **~60.6 GB** for the 4-byte request, then
  errors on the missing element. The test fails its `peak < 1 MiB` assertion.
- **After the fix:** peak allocation stays far under 1 MiB; decode still returns
  the same `Err`. Test passes.

The test asserts on *allocation*, not on the returned `Result`, because both the
patched and unpatched code return `Err` — the only observable difference is how
much memory is reserved on the way there.

## Compatibility

No API change. No behavioural change for well-formed input (the `Vec` grows to
the true length exactly as before). Backport-safe to the 0.15 and 0.16 lines —
the affected code is byte-for-byte identical there.

## Checklist

- [x] Both decode sites bounded (`Array` and `CompactArray`)
- [x] Regression test that fails before the fix and passes after
- [x] Existing test suite green (`cargo test --lib`, integration tests)
- [x] `cargo fmt` / `cargo clippy` clean on changed files
