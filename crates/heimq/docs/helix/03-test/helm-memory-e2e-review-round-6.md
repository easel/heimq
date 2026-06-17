# Helm Fixed-Memory E2E Plan Review: Round 6

## Verdict

BLOCK.

## Accepted Findings

- Scenario D stopped after the first storage-full error while also claiming
  later produce attempts fail.
- Scenario D acceptance referenced accepted batch bytes that were not in the
  artifact schema.
- Scenario C mixed "new append" with an assertion that latest offsets remain
  unchanged.
- Scenario B's rate limit needed to be aggregate across the scenario.
- Throughput pass/fail wording needed to be consistent between the scenario and
  napkin math.
- Current-context runs need a pullable image.
- Hardware napkin math requires runner and node context artifacts.

## Resolution

The plan was revised to continue Scenario D for 16 post-error attempts, add
batch-byte fields to the artifact contract, make Scenario C sweeper-only, state
Scenario B rate as aggregate, make throughput require both thresholds for the
local report, require `HEIMQ_E2E_IMAGE` for non-kind current-context runs, and
capture hardware/resource artifacts.
