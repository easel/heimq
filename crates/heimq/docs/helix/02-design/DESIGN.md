---
ddx:
  id: helix.design-system.microsite
  depends_on:
    - helix.product-vision
    - helix.prd
  review:
    self_hash: 307e061e00f8bdb809b6af005acf7156d7d5647f9f6b8ec20d67aca623f0b523
    deps:
      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
      helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
    reviewed_at: "2026-06-22T21:30:26Z"
---

# DESIGN.md - heimq Microsite

This is the interface system for the checked-in `site/` microsite. It records
the concrete design and voice decisions used by the public documentation site.
It does not define runtime architecture, release policy, or Kafka protocol
behavior.

## Navigation and Active State

The microsite uses a compact top navigation shared by every page. The model is
flat: Overview, Quick start, Deployment, Compatibility / testing,
Architecture, and Resources. The nav must stay small enough to scan before a
reader runs a command.

**Active-state convention (required):** the current page's nav link uses a
tinted surface, visible border, stronger text color, and
`aria-current="page"`. The visible cue is derived from the semantic state with
`.nav a[aria-current="page"]`; it is not set with page-specific classes.

| Surface | Component | Active cue (visible) | Semantic |
|---|---|---|---|
| Primary nav | Header links | Tinted surface + border + stronger text | `aria-current="page"` |
| Footer links | Plain text links | No active cue; footer is contextual only | N/A |

## Visual Hierarchy

- **Layout**: A constrained `--content: 1120px` shell, top navigation, broad
  hero, then scannable sections. The overview page leads with the product
  promise; secondary pages lead with the reader task.
- **Type scale**: Display H1 `4.4rem / 1.05` on desktop and `2.7rem / 1.05`
  on mobile. Section headings use `1.32rem / 1.05`; body copy uses
  `1rem / 1.6`; labels use `0.72rem` to `0.76rem` uppercase.
- **Weight and emphasis**: Product claims use bold headings and short body
  copy. Evidence, commands, paths, and artifact names stay in `code` or in
  compact cards. Limit lists to concrete facts.
- **Spacing rhythm**: 8px between related labels and values, 16px to 22px
  inside cards and panels, 30px section padding, 42px page-end spacing.
- **Information order**: State what heimq is, who it is for, what it is not,
  how to run it, and how to verify claims. Do not foreground the fact that the
  site is static unless the page is specifically about deployment.

## Interaction States

States are used where the static site has an applicable element.

| State | Applies to | Convention |
|---|---|---|
| Hover + `:focus-visible` | Header links, footer links, inline links | Accent text on hover; visible 2px focus outline with offset |
| Active/current | Primary nav links | `.nav a[aria-current="page"]` tinted surface + border |
| Disabled | N/A | The static site has no disabled controls |
| Loading | N/A | The static site has no async actions |
| Empty | N/A | The static site has no dynamic empty states |
| Error | N/A | GitHub Pages 404 remains platform-owned; no custom error page yet |

## Tokens

### Color

- Page background: `#f6f8f4`
- Alternate background: `#e7f1ec`
- Panel: `rgba(255, 255, 255, 0.86)`
- Strong panel: `rgba(255, 255, 255, 0.96)`
- Text: `#17211f`
- Muted text: `#52605b`
- Primary accent: `#0e7c66`
- Warm proof accent: `#f2c84b`
- Risk / boundary accent: `#be3455`
- Border: `rgba(23, 33, 31, 0.14)`

### Spacing

4 / 6 / 8 / 10 / 12 / 14 / 16 / 18 / 20 / 22 / 24 / 30 / 36 / 42 / 48px.
The implementation uses direct CSS values rather than named variables for each
step because the site is small and static.

### Type

- Family: `Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont,
  "Segoe UI", sans-serif`
- Monospace: `ui-monospace, "SFMono-Regular", Consolas, "Liberation Mono",
  monospace`
- Scale: display H1 `4.4rem`, mobile H1 `2.7rem`, section heading `1.32rem`,
  card heading `1.05rem`, body `1rem`, lede `1.08rem`, labels `0.72rem` to
  `0.76rem`

## Voice

The microsite uses the HELIX `public-site` profile adapted to heimq:
protocol-precise, evidence-first, and operator-friendly.

- Lead with the user's job: run a Kafka-compatible endpoint, embed a wire
  surface, verify parity, or find release artifacts.
- Name the boundary before it can be misunderstood: single-node, memory-first,
  not a Kafka cluster replacement.
- Support product claims with repository evidence: workflows, scripts, tests,
  contracts, benchmarks, and HELIX artifacts.
- Avoid marketing adjectives that cannot be tested. Prefer "standard Kafka
  clients connect unchanged" over generic claims like "seamless" or
  "production-grade."

## Non-Goals

This document is the microsite interface system only. It does not cover:

- **Runtime architecture** - the broker and engine structure belongs in PRD,
  contracts, ADRs, solution designs, and implementation docs.
- **Data flow** - Kafka request, log, offset, transaction, and backend flows
  belong in contracts and solution designs.
- **Component implementation internals** - HTML file layout and CSS selectors
  are implementation details owned by `site/`.
- **Architecture-significant decisions** - release, storage, protocol, and
  engine decisions belong in ADRs or release plans.

## Review Checklist

- [x] Navigation section names the active-state convention and requires
      `aria-current="page"` on the active nav item.
- [x] Active visual cue is derived from `.nav a[aria-current="page"]`.
- [x] Interaction states are scoped where applicable.
- [x] Visual hierarchy is concrete enough to build against.
- [x] Tokens name real values.
- [x] Non-goals keep architecture, data flow, component internals, and ADR
      material out of this document.
