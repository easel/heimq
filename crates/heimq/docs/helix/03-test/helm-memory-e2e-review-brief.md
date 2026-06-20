# Review Brief: Helm Fixed-Memory E2E Plan

You are a critic, not a validator. Find implementation rework risks,
contradictions, missing constraints, ambiguous interfaces, hidden assumptions,
and places where two competent implementers would make different choices.
Do not implement the plan or rewrite the artifact unless explicitly asked for a
separate execution step. Do not balance criticism with praise.

## Artifact Under Review

Review `crates/heimq/docs/helix/03-test/helm-memory-e2e-plan.md`.

## Review Question

Is this plan execution-ready for building a Helm-based fixed-memory e2e suite
that proves:

1. load across `1`, `10`, and `100` topics;
2. steady process memory;
3. immediate consumers receive every non-expired record;
4. `retention.ms` topics backpressure producers when unexpired data fills the
   broker cap;
5. `retention.bytes` topics drop excess records without producer backpressure;
6. the final report contains durability verdicts, captured metrics, and
   hardware napkin math.

## Constraints

- Use the existing Helm chart in `charts/heimq`.
- The test must deploy the chart, not only run the local binary.
- The test must use a fixed broker record-byte cap and a fixed Kubernetes
  memory limit.
- Any metrics needed for the report must be implemented before the report can
  claim success.
- Tracker changes must use `ddx bead`; do not edit `.ddx/beads.jsonl`
  manually.
- Review only the plan. Do not implement it.

## Output Contract

Return exactly this Markdown structure and no other top-level sections.

### Findings

| Severity | Area | Evidence | Finding | Recommendation |
|---|---|---|---|---|
| BLOCKING | <area> | <line/section/file/constraint/gap> | <specific issue> | <minimal corrective action> |
| WARNING | <area> | <line/section/file/constraint/gap> | <specific issue> | <minimal corrective action> |
| NOTE | <area> | <line/section/file/constraint/gap> | <specific issue> | <minimal corrective action> |

If there are no findings, write one row:
| NOTE | none | reviewed artifact | No evidence-backed issues found. | none |

### Verdict

APPROVE | REQUEST_CHANGES | BLOCK

### Disagreements Or Uncertainty

<Only include uncertainty that affects the verdict. Write "None." if absent.>

### Summary

<2-4 sentences>
