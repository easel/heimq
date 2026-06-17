# Helm Fixed-Memory E2E Plan Review: Round 5

## Verdict

BLOCK.

## Accepted Findings

- The reclaimed-bytes metric needs unambiguous attribution; the current
  `reclaim_topic` shape returns only aggregate freed bytes.
- Scenario D needs byte-level evidence that backpressure occurred at the fixed
  broker cap, not only a broad accepted-record count.
- Scenario C needs an immediate-consume deadline shorter than its retention
  window.
- The throughput threshold needs an environment contract.
- The GitHub workflow must be run at least once and archive artifacts, even if
  it remains non-blocking.

## Resolution

The plan was revised to require separate expired-byte and size-trim byte
accounting before metrics are emitted, assert Scenario D retained bytes are
within one batch of the cap at first storage-full, add a Scenario C consume
deadline, scope throughput enforcement to the local report unless explicitly
enabled in CI, and require one on-demand workflow run with archived artifacts.
