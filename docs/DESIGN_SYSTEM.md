# Project Nexus design system

Status: canonical visual system for the **Phase 1.2 — Readiness Atelier** production redesign, 2026-10-27 (Asia/Shanghai). It supersedes the Phase 1.1B-1R "Editorial Instrument" visuals, which the product owner rejected. The tokens, type roles and layouts below describe the current React implementation in [src/styles.css](../src/styles.css). Final aesthetic approval remains with the product owner. [PRD](PRD.md) owns product hierarchy, [I18N](I18N.md) owns terminology, [ARCHITECTURE](ARCHITECTURE.md) owns readiness semantics and [ROADMAP](ROADMAP.md) remains the sole phase sequence. Native evidence for this revision is in [.impeccable/review/phase1-2-atelier](../.impeccable/review/phase1-2-atelier/README.md).

The implemented application remains a **fixture-only Phase 1 shell**. Health evidence, installation states, Models, configuration records, System information and activity are demonstrations, labelled in the sidebar register on every page. Descriptions of installation, authentication, configuration application and repair later in this document are future phase-gated principles, not implemented operations or authorization to begin Phase 2.

## 1. Visual identity and hierarchy

Nexus is drawn as a **readiness atelier**: calm paper (or a composed night ledger), ruled ledgers instead of card stacks, and one product-specific drawing — the **readiness line**. Three type roles never mix: **conclusions are serif, the interface is sans, evidence is mono.**

- **Mark.** A high-contrast serif N (hairline stems, one heavy diagonal) overprinted on a cobalt signal disc, like two plates in a letterpress or riso print. The N follows ink; the disc is the brand cobalt. The icon sets this on a warm paper tile with grain and a hairline baseline, inside the macOS 824px body grid. The sidebar mark is that same tile drawn inline (fixed colours in both themes), so the in-app mark and the app icon are one image. The wordmark "Nexus" is set in the display serif. App icons are generated from [public/nexus-mark.svg](../public/nexus-mark.svg).
- **Readiness line.** Every Agent row draws one path — Install → Runtime → Config → Model → Online — built only by [src/app/readiness.ts](../src/app/readiness.ts) from fixture checks and configuration records. Node shapes: pass solid square; fails diamond; pending review half disc; assigned neutral ink square; *not checked* hatched; *not reached* dotted. Nothing after a failure is presumed to pass, and online service is always drawn as not checked. Agent Detail expands the same path into a five-step procession.
- **Seal.** A ruled, tinted box in mono caps (Chinese without case transform) is reserved for an Agent's overall readiness. Check results never use the seal.
- **Hatch means no conclusion.** Diagonal hatching marks what this example never assessed (online service, the "Not checked in this example" sheet). It is never used for a result.
- **Configuration sheet.** Desired → applied → effective are three separately labelled columns joined by drawn operators: *matches* (=), *differs* (≠, filled attention), *no record* (dashed) and *separate source* (dotted). An effective model owned by the Agent is never drawn as matching a Nexus Profile.
- **Assignment chain.** Models draws Provider → Model → Profile → Agent as a rail; a dotted link is a step the Agent does not use.

Home hierarchy: two-tone verdict headline (lead in ink, qualification in ink-3) with a readiness register of serif counts → My Agents ledger with readiness lines and seals → Needs attention exhibits (numbered I, II; blocked before attention) beside compact System and Activity sheets → a five-state reading guide. Counts are fixture results, explicitly not a health score.

Premium quality means coherent density, preserved context, predictable keyboard behaviour, native window integration and equal Chinese/English polish. Do not copy another product's layout, assets, branding or wording; components from a UI library are primitives, not a page composition.

## 2. Desktop layout

All dimensions are logical pixels. Proposed minimum practical window: **960 × 640**. Initial comfortable size: **1200 × 800**, bounded by available work area. This is a desktop application, not a marketing website. Do not enforce minimum dimensions that push essential controls off a smaller work area; use the narrow layout and scroll fallback. Accessibility zoom/reflow must also retain complete functionality.

| Element | Default | Narrow desktop / large screen behavior |
| --- | --- | --- |
| Window chrome | Platform-native title bar/control placement; safe drag regions | Do not place buttons underneath traffic lights, caption controls, or resize handles |
| Sidebar | 220 wide; numbered navigation (01–06), 34 row height; demo register and version at the foot | 196 at 720–1099; user-collapsed 60 icon rail; below 720 the 60 rail keeps accessible navigation names |
| Toolbar / main content | 48 sticky toolbar inside the main pane: location + context, fixture clock, theme and language menus; content max 1180 with 44 inset | 36 inset below 1280, 28 below 1100 (context and clock hidden), 16 below 720 |
| Reading/forms | 640–720 max content width | Single column; labels above controls when translated labels would squeeze inputs |
| Home | Hero (verdict + 320 register), full-width Agent ledger, exhibits beside System/Activity | Register becomes a four-column strip and rows regroup (identity / model / verdict over the line) below 1100; single column below 900 |
| Agent Detail | Hero (seal, serif name, two-tone deck) + 340 fact register; five-step procession; tabs; configuration sheet + 330 evidence column; next-step rule | Fact register two columns and procession 2×3 below 1100; evidence sheets reflow under the main column |
| Settings | § contents list (196) plus sections with live theme specimens | Contents become a horizontal row below 1100; specimens stack below 720 |
| Dense table | Content-driven widths and stable header | Keep Agent, readiness, issue and action; move secondary version/source metadata into row details; horizontal scrolling only for genuinely technical comparison/evidence tables |
| Dialog | 440 standard, 600–720 plan/diff | Width ≤ available pane minus 32; height ≤ viewport minus 48; scroll body while title/actions stay available |

Below the practical minimum or at high zoom, switch lists to stacked rows, wrap tabs or use a labeled overflow selector, collapse secondary columns, and retain keyboard access to every action. Do not force two-dimensional scrolling for forms or ordinary reading content. Keep technical code/diff horizontal scroll inside its own region.

Every page opens with a kicker, a serif title and a serif deck; Home and Agent Detail use the verdict exceptions in §3. Agents default to a ruled ledger, not a gallery. Agent rows carry identity, the readiness line, effective model and source, and a verdict (seal, finding, detail link) as separate keyboard-reachable targets; never nest interactive controls inside a clickable row. Reduce repeated surface boxes before reducing essential reading text.

Conceptual Home composition:

```text
Sidebar (numbered nav, demo register)   Toolbar: location · context      fixture clock · theme · language
  Kicker · fixture time
  Verdict: "2 of 4 Agents are ready." + ink-3 qualification         Readiness register (serif counts)
  My Agents ledger: identity │ Install─Runtime─Config─Model─Online │ effective model │ seal · finding · View
  Line legend · "every mark is demonstration data"
  Needs attention: I Gemini CLI (blocked) / II Claude Code (attention)   │  System sheet · Activity sheet
  How to read readiness (five states)
```

Use actual observation counts in a future live view. The current packaged Phase 1 application uses clearly labelled fixture counts; it never makes live health claims.

## 3. Typography and spacing

All fonts are SIL OFL 1.1 and bundled offline; provenance, revisions and SHA-256 hashes live in [public/fonts/sources.json](../public/fonts/sources.json) with licence texts beside each file. Font choices were made from rendered zh/en specimens at UI sizes, not by reputation.

| Role | Face | Why |
| --- | --- | --- |
| Conclusions — verdicts, page and section titles, numerals | **Source Serif 4** (`Nexus Serif`, variable, opsz) + **Source Han Serif CN** (`Nexus Han Serif`) | Designed Latin/CJK siblings: matching proportions and colour let a bilingual two-tone headline read as one line |
| Interface | **IBM Plex Sans** (`Nexus Sans`, variable) + **Source Han Sans CN** (`Nexus Han`) | Plex's Latin matches Han's colour in mixed lines and shares its skeleton with the mono face |
| Evidence — IDs, versions, times, kickers | **IBM Plex Mono** (`Nexus Mono`) | Exact values read as measured fact |

| Role | Size / line height | Weight |
| --- | --- | --- |
| Home verdict / page title | 40 / 46 (zh 34 / 46) | 400 (zh 460, dark 420) |
| Agent Detail name | 50 / 56 | 400 |
| Deck (two-tone continuation) | 21 / 30 (zh 18.5 / 30) | 400 |
| Section title | 27 / 32 (zh 23) | 400 (zh 460) |
| Sheet title | 20 / 26 (zh 17.5) | 400 (zh 460) |
| Register numerals | 31 / 36, lining tabular | 400 |
| Body / rows | 14 / 22 (zh 23); Agent name 15.5 / 22 | 400 / 500 (dark 380 / 480) |
| Meta | 12.5 / 18 | 400 |
| Kicker / ledger head | Mono 10.5 / 16, caps, +0.09em; zh: Sans 11.5, +0.06em, no case transform | 500 |
| Technical values | Mono 12.5–16, tabular, slashed zero | 400 |

Chinese headings use positive tracking (+0.01em) rather than Latin's negative tracking. Dark mode trims weights because light text blooms optically. No ultra-light or synthetic styles (`font-synthesis: none`).

Spacing is 4-based with editorial rhythm: 44 page inset, 48 between sections, 22 sheet inset, 16–18 row padding. Rules: 1.5px ink rule for ledger tops/bottoms and registers, 1px border between rows. Radius: 2–3 tags and seals, 4 controls, 6 sheets. Sheets carry one soft drop shadow (plus a 1px top highlight in Night); there is no other elevation.

## 4. Semantic tokens and themes

Semantic tokens are defined on `:root, [data-theme]` so any element can re-scope the palette (Settings renders live specimens this way). Themes are Light (Paper), Dark (Night) and Follow System; explicit choice persists and Follow System reacts live. Night is composed, not inverted: sheets sit lighter than the canvas, ink is bone rather than white, and weights are trimmed.

| Token | Paper | Night | Role |
| --- | --- | --- | --- |
| canvas / sidebar | #F2F0EA / #EDEAE2 | #141413 / #111110 | Working canvas and navigation plane |
| sheet / raised / sunken | #FAF9F5 / #FFFEFA / #E9E6DC | #1C1C1A / #232321 / #0F0F0E | Ledgers; controls; wells |
| border / border-strong / rule | #DAD5C9 / #B6AF9F / #191A1C | #32312D / #4B4943 / #CFCAC0 | Row dividers; controls; ledger rules |
| ink / ink-2 / ink-3 | #191A1C / #47453F / #66635A | #ECE9E2 / #BCB8AE / #959187 | Text hierarchy (all ≥ 4.5:1 on every surface) |
| ink-4 | #A29D91 | #63605A | Decoration and lines only — never text |
| brand / focus | #2E44B5 | #A9B4FF | Links, focus, Profile node, mark signal; never a readiness state |
| ready (ink) | #2B6A47 (#23593B) | #5AAE7E (#85C99F) | Ready / pass |
| attention (ink) | #94620A (#7E5307) | #D49B33 (#E5B862) | Needs attention / pending review |
| blocked (ink) | #B13B22 (#9C301A) | #E36B4F (#F09681) | Blocked / fails |
| unknown (ink) | #57544C (#4A4740) | #8C887E (#B9B4A8) | Unknown / unsupported, with hatch |

Status colours are marks and seal inks; readiness is always also encoded by shape and label. The primary button is ink on canvas (not the brand colour) so the accent stays reserved for interaction. [tests/unit/contrast.test.ts](../tests/unit/contrast.test.ts) verifies every text token on every surface in both themes; axe WCAG 2.1 AA runs on every route in Playwright. These are test results, not a conformance claim.

## 5. Agent readiness, Health, and Diagnostics

| Domain state | English / Chinese | Visual and action |
| --- | --- | --- |
| ready | Ready / 已就绪 | Solid disc glyph, ready seal; Launch when available (future) |
| attention | Needs attention / 需关注 | Half-disc glyph, attention seal, concise non-blocking issue, Review |
| blocked | Blocked / 无法就绪 | Diamond glyph, blocked seal, blocking impact, Diagnose |
| unknown | Unknown / 待检查 | Dashed-ring glyph with hatch, explain missing/stale evidence, Check again |
| unsupported | Unsupported / 不支持 | Slashed-ring glyph, specific capability reason, View requirements |

Installation (`not installed`, installed, ambiguous) and operation overlays (`checking`, `repairing`) remain separate. A progress animation never changes a blocked Agent to green. Update available is secondary unless a rule establishes a blocker. Never use “100% healthy,” invented scores, or blank green badges for unperformed checks.

Agent Detail identity, readiness, installation/version and scope precede the five established tabs: Overview, Model, Extensions, Configuration and Diagnostics. Detail order is **seal, name and issue → fact register → readiness procession → tabs**; Overview order is **configuration sheet → important Extensions → recommended next step**, with evidence alongside. Health evidence and the hatched “Not checked in this example” sheet occupy the right column at wide sizes and reflow beneath the main column below 1100; provenance/coverage remains easy to find. The current evidence is fixed demonstration data. A future live Overview must state observation time/coverage, group checks by affected capability and make uncertain checks explicit.

Diagnostics detail order: summary → user impact → observed versus expected → established/possible cause → eligible next action → expandable evidence. Evidence displays source/time and redaction notice, supports safe copy, and opens no links automatically. The log pane is optional and bounded, never the main error screen. Group shared System findings and list affected Agents.

## 6. Models and configuration experience

Models view lists display name, exact model identifier, Provider, known capability labels, availability freshness, and compatible Agents. Unknown metadata reads “Unknown / 未知” with explanation. Do not decorate custom models with fabricated capabilities.

Provider management is a subordinate view inside Models. The connection form depends on provider type/protocol. Show destination host, authentication method, secure-storage status, and explicit Test connection. Do not collapse account login and API key into one field. Secret fields are write-only after save, with Replace/Remove rather than reading the stored value back. Provider connection success is not Model or Agent success.

Agent Detail → Model is the assignment flow: current effective source → select direct Model/Profile/native mode → supported options → compatibility and override disclosure → exact diff/impact → confirmation → application and verification. Hide meaningless controls; explain unsupported requested options instead of silently dropping them.

Model Profiles display reusable name, Provider/Model, revision, and affected Agents. Editing opens an impact preview with per-Agent compatibility and pending/applied revision. Offer clear “Apply to selected Agents,” not a silent save that mutates every setup. Bulk operations display per-Agent outcomes and partial success.

Configuration view separates requested, last applied, and observed effective settings. In the fixture UI, the configuration sheet shows desired, applied and effective as three columns joined by drawn operators (§1); the effective Model carries its own source. A revision record is not a configuration snapshot or field diff; missing records are stated explicitly. Future native configuration operations must preserve unknown keys; show “Managed by your organization” when relevant, without presenting a bypass. Diff views redact secret values and retain important paths/scope labels. A conflict explains that the file changed and asks the user to refresh the plan.

Restoring a backup is labeled **Restore configuration / 恢复配置**, with backup time, target, current changes that will be replaced, protected-copy status, and any reintroduced execution-sensitive settings. Distinguish it from **Undo this operation / 撤销本次操作**, which is available only when automatic compensation preconditions still hold. Never hide these consequences behind a generic Repair button.

## 7. Component behavior

| Component | Contract |
| --- | --- |
| Buttons | One primary per action group; verb plus object where useful; secondary neutral, destructive explicitly labeled; busy state retains label and layout |
| Forms | Persistent labels; help before failure; inline field error plus summary for multi-field failure; validation never relies on toast alone |
| Lists/tables | Stable row identity/sorting; align comparable values; preserve selection/focus on refresh; announced sort direction; row action names include Agent |
| Status indicators | Icon + text + optional timestamp; semantic status never color-only |
| Tabs | Agent Detail areas, keyboard arrow behavior and clear focus/selection; do not use tabs for unrelated primary navigation |
| Segmented controls | Small mutually exclusive choices such as theme; accessible group label, no unexplained icon-only choices |
| Dialogs | Focus entry/trap/return; concise title, consequences, cancel and exact action; cancellation default for irreversible action; Escape cancels unless a noninterruptible operation explains why |
| Popovers | Lightweight context or selection; remain within viewport; dismiss predictably and return focus |
| Tooltips | Supplemental text on hover and focus; never sole location of error, compatibility reason, or required instruction |
| Toasts | Brief noncritical outcomes; screen-reader polite announcement; no auto-disappearing sole record of failure; durable activity link for completed operations |
| Progress | Named actual stage and measured fraction only when known; indeterminate otherwise; cancellability and partial outcomes visible |
| Empty states | Explain absence and one relevant next action: discover, adopt, add connection; no fake demo data in a live view |
| Loading | Reserve stable geometry, modest skeleton for initial reads; keep last-known data with checking state during refresh |
| Errors | Explain affected Agent, cause certainty and next step; preserve unsaved form input; retry only if safe |

Installation UX: show official source/installer/channel, prerequisites, destination/ownership, privilege needs, credential handling, and rollback limits before confirmation. Progress uses real stages such as downloading/verifying/installing/checking. Cancellation reports completed effects. Uninstall clearly states what is retained; configuration deletion is a separate explicitly destructive action, outside default removal.

Repair UX: structured plan with before/after, affected Agents, protected backup state, verification and recovery limits. Confirmation must satisfy the native-owned security boundary. Keep domain language prominent, expose technical evidence on demand, and retain a useful result screen if verification/rollback fails. Do not label a green process exit “Fixed.”

## 8. Motion, icons, and accessibility

Use one coherent stroke icon family, normally 16px in rows/controls and 20px for prominent state. Icons align optically with Chinese and Latin baselines; primary icons are never emoji. Agent/vendor assets require verified usage rights; use plain names until approved, not copied branding assets.

Motion communicates user action: 100–160ms hover/focus, 160–220ms disclosure/dialog. Avoid page entrance cascades, pulsing Health, animated background gradients, or decorative counters. Reduced motion removes translation/scale and looping decorative animation; progress remains readable as text and static stage changes.

Use semantic buttons, forms, lists and tables. Provide landmarks and a skip-to-content path. Tab order follows visual order. Focus is always visible, restored after dialogs and preserved after async refresh. Expose busy/progress/status updates sparingly with live regions; never reannounce every poll. All icon actions have localized accessible names, including the target Agent when ambiguous.

Test VoiceOver on macOS and NVDA or Narrator on Windows, keyboard-only flows, OS high-contrast behavior, and 200% zoom; assess 400% reflow for applicable reading/forms. Do not suppress OS focus/high-contrast outlines. Do not rely on dragging, hover, color, or sound alone. Resizable regions have keyboard alternatives.

## 9. Bilingual acceptance and anti-patterns

zh-CN and en-US share information hierarchy and interaction behavior. Allow at least 40% Latin label expansion in test fixtures, realistic long Chinese explanations, and mixed Latin paths. Buttons use content-driven widths; sidebar names do not truncate; critical confirmation text wraps. Technical identifiers may truncate only with a keyboard-accessible full-value view/copy. Toasts, tables, dialogs, empty states, tooltips and accessible labels all use the same locale.

Future sessions must not add generic admin dashboards, giant card grids, crypto styling, purple AI gradients, excessive glass/shadows/rounding, neon, decorative analytics, forced full-width stretching, emoji primary icons, untranslated native errors, meaningless Provider/Model conflation, future billing navigation, or animated health scores. Test designs against the actual Agent recovery journey, not a component-library showcase.

Phase 1 acceptance requires rendered Light/Dark screenshots at 960×640 and larger desktop sizes in both locales, keyboard/assistive-tech review, composed contrast verification, long-content/error fixtures, reduced-motion behavior, and user-language/theme persistence. Phase 1.2 additionally required native Tauri screenshot iteration and refinement after inspecting those captures. Executed checks, native evidence and remaining platform/assistive-tech limits belong to [the current delivery record](../.impeccable/review/phase1-2-atelier/README.md); [PHASE1_UI_REDESIGN](PHASE1_UI_REDESIGN.md) is retained as the Phase 1.1B-1R history. Automated results never constitute aesthetic approval.

Earlier Phase 0 arithmetic checks concerned the superseded palette and are not evidence for these colors. Validate current composed text/status/control/focus/button colors from the changed state. The user retains final aesthetic authority; Windows runtime review, spoken screen-reader output and long-term reading comfort require separate evidence.
