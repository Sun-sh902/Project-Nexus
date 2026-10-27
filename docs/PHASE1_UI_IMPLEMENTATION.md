# Phase 1.1B-1 — production desktop UI (historical record, visual baseline rejected)

Recorded 2026-10-11. English rewrite 2026-10-27 (the project is English-only; the original record was written in Chinese and is condensed here). The design evidence this record linked to (`design-lab/`, `.impeccable/`) is kept in the local development environment and is not part of the published repository.

**Status: historical.** The production visual baseline delivered by this batch was rejected by the product owner and superseded by Phase 1.1B-1R ([PHASE1_UI_REDESIGN](PHASE1_UI_REDESIGN.md)), which was in turn superseded by Phase 1.2 “Readiness Atelier”. The current visual specification is [DESIGN_SYSTEM](DESIGN_SYSTEM.md). Nothing below describes the current application.

## Scope

| Item | Record |
| --- | --- |
| Branch / base | `codex/phase1-ui-implementation` at `6821805a` (original repository, history not preserved) |
| Visual basis | Owner-selected “Joined Measure” direction (provisional) |
| Interaction basis | Owner-selected “Continuity” direction: selected row → context |
| Batch scope | Global shell, shared tokens / fonts / components, Home and Agent Detail |

Business data came entirely from fixtures. No Phase 2 work, dependencies, persistence migrations, Rust/Tauri capability changes or real Agent operations were added.

## Changes

- `src/styles.css`: local fonts, separate Light / Dark tokens, type scale, shell, Home working surface, Detail, focus and responsive rules.
- `src/App.tsx`: brand, six-item navigation, Agent count, version, collapsible navigation, compact location bar, global theme and language controls.
- `src/components/*`: Agent rows with selection state and complete accessible row descriptions; five readiness states with icon plus text; new `NexusIdentity`, `AgentConfiguration` (desired / applied / effective records) and `HealthEvidence` (fixture checks, online service explicitly not checked).
- `src/pages/Home.tsx`, `src/pages/AgentDetail.tsx`: single readiness summary, four-Agent index, selected-Agent context, five Detail areas with real tab navigation and focus transfer.
- Fonts with SIL OFL 1.1 licenses, pinned upstream revisions, byte counts and SHA-256 in `public/fonts/sources.json`.
- Tests: contrast for rail / elevated surfaces, local font integrity, production UI semantics and preferences.

## Accessibility corrections

An independent review found two defects, both fixed before the batch closed:

- **R1** — row metadata after a named selection button was not accessible. Each row kept a concise selection name with `aria-pressed` / `aria-controls` and an `aria-describedby` link to a full description (Agent, readiness, vendor / installation, effective model, assignment source).
- **R2** — call-to-action visible text and accessible name differed. Names are now built from the visible label plus the Agent name.

Native accessibility-tree captures confirmed the structure; spoken screen-reader output was not verified.

## Verification at the time

| Check | Result |
| --- | --- |
| `pnpm typecheck`, production-scope ESLint | Pass |
| `pnpm lint` (whole repository) | Fail: 258 pre-existing errors in archived design-lab prototypes |
| `pnpm test` | 15 unit tests pass |
| `pnpm test:ui` | 28 Playwright tests pass (themes × 960×640 / 1440×1000, axe, keyboard, reflow, reduced motion) |
| `pnpm build`, `pnpm desktop:build --bundles app` | Pass |
| `cargo check` / fmt / clippy `-D warnings` / 3 Rust tests | Pass |

These checks are not a WCAG conformance claim or aesthetic approval.

## Known weaknesses (as recorded)

- Home left noticeable empty space below the four Agent rows; System and activity sat below the fold.
- On narrow Detail views the health evidence appeared after Important Extensions.
- Final native 960×640 evidence, Windows rendering, real screen-reader output and OS zoom were not verified; signing and notarization were out of scope.
