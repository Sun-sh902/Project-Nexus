# Phase 1.1B-1R — Editorial Instrument (historical record)

Recorded 2026-10-11 on macOS arm64. English rewrite 2026-10-27 (the project is English-only; the original record was written in Chinese and is condensed here). Linked design evidence (`design-lab/`, `.impeccable/`) is kept locally and is not part of the published repository.

**Status: historical.** This redesign replaced the rejected [Phase 1.1B-1](PHASE1_UI_IMPLEMENTATION.md) baseline and was later superseded by Phase 1.2 “Readiness Atelier”, specified in [DESIGN_SYSTEM](DESIGN_SYSTEM.md).

## Scope

The real React / Tauri application was restyled and packaged as a runnable `.app` awaiting the owner's visual approval. No Rust permissions or behaviour, typed IPC, fixtures, routes, preference logic, dependencies or font licenses changed. No discovery, installation, authentication, credentials, real Health, configuration writes or repair existed.

## Research and decisions

Rendered pages from Apple, Pentagram / Isomorphic Labs, Pi, Resend, Linear and Things were studied (17 screenshots). No code, navigation, trademarks, graphics or product flows were copied. An art-direction memo was written before editing.

| Change | Purpose |
| --- | --- |
| 168px navigation (152px compact) with a headline-style Nexus wordmark | More working area; replaced the earlier pink “N” |
| Distinct reading roles for Latin display, interface and exact identifiers (Plex Mono) | Hierarchy through reading role; supporting text mostly 13–14px |
| Home as an open Agent list with a neutral explanation column | The four Agents become the main subject; state, cause and next step are clear |
| Primary action directly after the issue, then effective model and revision records | Less hunting for the next step without mixing desired, applied and effective state |
| Agent Detail without the giant outer frame or nested model cards | Identity, readiness, action, configuration and evidence form one workspace |
| Cool grey Light canvas, blue-grey Dark canvas; blue only for interaction | Clear material relationship; readiness still encoded by colour, icon and text |

## Native iteration

Native screenshots were taken from the packaged app at a 1200×800 logical window (2400×1600 images) for Home, Claude Code and System in both themes. Two rounds of review removed a duplicated vendor line, reordered information and actions, and fixed a CSS precedence issue in assignment-source spacing. Exact native captures at 1440×900, 1280×800 and 960×640 could not be produced with the available window control; 36 browser screenshots covered those sizes instead and do not replace native verification.

## Verification at the time

| Check | Result |
| --- | --- |
| Typecheck, production-scope ESLint, `pnpm build` | Pass |
| `pnpm test` | 15 unit tests pass |
| `pnpm test:ui` | `production-ui.spec.ts` 18 pass, `shell.spec.ts` 22 pass |
| `cargo check` / fmt / clippy `-D warnings` / 3 Rust tests | Pass |
| `pnpm desktop:build --bundles app` | Pass; app launched and captured |
| `pnpm desktop:build` (full) | `.app` built; DMG packaging failed |
| `pnpm lint` (whole repository) | Fail: 258 pre-existing design-lab errors |

## Remaining questions (as recorded)

- Whether the restrained wordmark is distinctive enough was left to the owner.
- At 1200×800 the Home health evidence sat near the lower edge; System and activity required scrolling.
- Exact native sizes, Windows rendering, VoiceOver / NVDA / Narrator output and long-session reading comfort were not verified.
- The deliverable was a local `.app`, not a signed, notarized or cross-platform release.
