# Backend implementation progress (Phases 2–7 core backend)

Status date: 2026-10-27. Branch `claude/nexus-backend-mvp`. This is the resumable record for the
Core Backend Beta Candidate program. It records what exists and what was verified; it does not
claim anything that was not executed. Product authority remains [PRD](PRD.md); sequencing remains
[ROADMAP](ROADMAP.md).

## Current gate

| Gate | State |
| --- | --- |
| A — Repository/architecture preflight | Done (see §Repository recovery) |
| B — Agent + System discovery | Implemented, unit + fixture tested, real macOS read-only scan run |
| C — Health engine | Implemented, pure/deterministic, tested |
| D — Lifecycle | Implemented (npm / Homebrew / Claude self-update recipes), fake-npm tested; **no real install executed** |
| E — Models/Providers | Implemented (records, compatibility, assignment apply/verify, Keychain secrets) |
| F — MCP + Skills | MCP read for all four; toggle/add-remote/remove-managed for Codex, Gemini, OpenCode; Skills read-only |
| G — Diagnostics + Safe Repair | Implemented: findings, restore backup, reapply assignment, recover interrupted install, disable broken MCP |
| H — Frontend integration | Done: every page reads Rust data in the desktop app; fixtures only in explicit browser demo mode |
| I — Validation / review | Automated suites green; real desktop app launched with live data; independent review — see §Independent review |

## Repository recovery (Gate A)

`Project-Nexus-Dev` was a linked worktree whose parent repository (`~/Documents/Project-Nexus`)
had been deleted; history was unrecoverable (no Trash/Time Machine/remote copy). With user
approval: full tarball backup at `~/Documents/Project-Nexus-Dev-backup-20261027.tar.gz`, dead
pointer kept as `.git.broken-worktree-pointer`, fresh `git init`, baseline commit `a0ca45a` on
`main` (exact tree as found), work on `claude/nexus-backend-mvp`. Nothing pushed.

## Architecture as built (`src-tauri/src`)

| Module | Responsibility |
| --- | --- |
| `core.rs` | UTC instants, ids, `NexusError` (stable code + redacted params), redaction |
| `platform/` | `HostContext` (home/env/PATH abstraction for tests), bounded reads, safe atomic replace, the only process runner (no shell, cleared env, timeout, output cap, process-group kill) |
| `domain.rs` | Snapshot/observation types with explicit `ObsStatus` (observed/notFound/accessDenied/timeout/parseFailed/notChecked/unsafe) |
| `discovery/` | Candidate search (PATH + well-known dirs), source classification, probe trust policy, version from metadata first, reviewed `--version` probe second; shell-profile static scan; skills; system tools |
| `adapters/` | Compiled per-Agent adapters: base detector trait + optional `ModelTarget` and `McpTarget` traits; recipes per Agent |
| `config/` | Span-preserving JSON/JSONC editor, `toml_edit` for TOML, `ConfigEdit` plans with diff, transaction (precheck → encrypted backup → atomic write → re-read) and compensation |
| `secrets.rs` / `backup.rs` | OS credential store (keyring: Keychain / Credential Manager); AES-256-GCM encrypted exact-byte backups, key in OS store, no plaintext fallback |
| `store.rs` | SQLite (bundled), versioned migrations, refuses newer schema; Providers, Models, Profile revisions, Assignments, operation journal + steps, backup index, Health history, managed-extension and installation ledgers. No secret columns |
| `health.rs` | Pure rules `nexus-readiness-1`, normative aggregation, staleness, explicit coverage/not-checked |
| `diagnostics.rs` | Deterministic findings with cause status, evidence, eligible repairs or manual guidance |
| `models.rs` | Provider/Model/Profile/Assignment types, built-in seeds with provenance, compatibility per Agent |
| `lifecycle.rs` | Recipes, toolchain resolution, version pinning at plan time, fixed argv construction |
| `actions.rs` | Action Registry: typed `ActionRequest`, immutable digest-bound plans (10 min TTL, single use), native confirmation, journaled execution, independent verification, compensation |
| `app.rs` | Use cases: overview, reports, models view, Model-domain record mutations |
| `native_confirm.rs` | Rust-owned OS confirmation dialog built from the plan (tauri-plugin-dialog), text from `src/locales/en-US/native.json` |
| `ipc.rs` | 15 typed commands, `deny_unknown_fields`, main-window only; each allowlisted in `capabilities/main.json` |

## Agent capability matrix (macOS verified on this machine; Windows implemented-but-unverified)

| Capability | Codex | Claude Code | Gemini CLI | OpenCode |
| --- | --- | --- | --- | --- |
| Discovery (install, version, source, duplicates) | Yes | Yes | Yes | Yes |
| Version evidence | npm manifest / brew layout / reviewed probe | native layout / npm / brew | npm manifest | npm manifest / reviewed probe |
| Config read + precedence | user + `/etc/codex` | env > shell(uncertain) > managed > env block > user > default env | env > system > user > system-defaults | content env > custom > global json/jsonc |
| Model write | `model` (TOML) | `model` (strict JSON) | `model.name` | `model` (blocked when both opencode.json and .jsonc exist) |
| Provider routing (no secret handoff) | OpenAI | Anthropic | Google | Anthropic / OpenAI / Google |
| MCP read | Yes | Yes (`~/.claude.json`, read-only) | Yes | Yes |
| MCP enable/disable | Yes | Unsupported (Agent-owned state) | Yes (`mcp.excluded`; blocked under `mcp.allowed`) | Yes |
| Add remote MCP / remove Nexus-added | Yes | Unsupported | Yes (`httpUrl`) | Yes (`type: remote`) |
| Skills | Read | Read | Read | Read |
| Skills install/enable | Unsupported (deferred) | Unsupported | Unsupported | Unsupported |
| Install | npm `@openai/codex`, brew cask `codex` | brew cask `claude-code`, npm | npm, brew `gemini-cli` | npm `opencode-ai` |
| Update | npm/brew installs; standalone script install unsupported | npm/brew; native via `claude update` | npm/brew | npm only |
| Uninstall | npm/brew | npm/brew; native = manual guidance | npm/brew | npm only |
| Launch | macOS Terminal | macOS Terminal | macOS Terminal | macOS Terminal |
| Online credential/provider validation | Not implemented | Not implemented | Not implemented | Not implemented |

Script installers (`curl … | sh`) are intentionally never executed (Security §5).

## Evidence (official sources fetched 2026-10-27)

- Claude Code settings/precedence/env: https://code.claude.com/docs/en/settings ; install/update/uninstall: https://code.claude.com/docs/en/setup
- Gemini CLI configuration (layers, `model.name`, `mcpServers`, `mcp.excluded`): https://geminicli.com/docs/reference/configuration/ ; install + Node 20: https://geminicli.com/docs/get-started/installation/
- OpenCode config (merge order, JSONC), MCP schema, install: https://docs.opencode.ai/docs/config/ , /mcp-servers/ , /docs/ ; V2 reads V1-shaped config: https://v2.opencode.ai/docs/migrate-v1
- Codex config basics and MCP: https://learn.chatgpt.com/docs/config-file/config-basic , https://learn.chatgpt.com/docs/extend/mcp?surface=cli ; npm/brew identities: https://github.com/openai/codex
- Local layouts observed read-only on this Mac (macOS 27.2, arm64): Claude native `~/.local/bin/claude → ~/.local/share/claude/versions/2.1.311`; Gemini npm 0.64.0; Codex standalone `/opt/homebrew/bin/codex` 0.164.0 (probe); OpenCode script install `~/.opencode/bin/opencode` 2.1.4 (probe).

## Dependencies added (justification)

| Crate | Why | Notes |
| --- | --- | --- |
| rusqlite 0.37 (bundled) | Versioned local persistence for intent/journal | MIT; bundled SQLite avoids system lib drift |
| keyring 3 (apple-native, windows-native) | Keychain / Credential Manager | MIT/Apache |
| toml_edit 0.23 | Comment/format-preserving Codex TOML edits | MIT/Apache; already transitive (older versions) |
| sha2, aes-gcm 0.10, getrandom 0.3 | Fingerprints, encrypted backups, ids | RustCrypto, MIT/Apache |
| libc | uid/ownership checks, process-group kill | already transitive |
| tauri-plugin-dialog 2 | Native-owned confirmation dialog | Official Tauri plugin; no renderer permission granted |
| tempfile (dev) | Isolated test homes | dev-only |

No frontend dependencies added.

## Tests executed

`cargo test --locked --manifest-path src-tauri/Cargo.toml` → **120 passed, 0 failed, 1 ignored** (2026-10-27).
The ignored test `real_machine_discovery_smoke` was run explicitly (`-- --ignored`): read-only scan
of this Mac in ~0.12 s; Codex/Claude/Gemini Ready, OpenCode Attention (`configAmbiguous`: both
`opencode.json` and `opencode.jsonc` exist). No secrets in serialized output.

Scenario coverage (`src/scenario_tests.rs`, isolated fake homes + fake npm): no agents; installed
agents with versions/sources; malformed config; missing/old Node; env + shell-profile model
override; MCP conflict repaired by toggle with preservation; denied/replayed/forged confirmation;
external edit between preview and confirm; installer failure with redacted output; apply → verify
→ drift → reapply → corruption → restore; install/update/uninstall with verification; crash-
interrupted install reconciled then recovered; remote MCP add/remove preserving user entries;
strict IPC typing; hostile names/URLs/identifiers; secrets only in credential store; OpenCode
ambiguity blocking mutation.

## Security review notes (self-review, not independent)

See [SECURITY](SECURITY.md) S01–S12. Implemented controls: no shell anywhere; one process primitive
with absolute validated program, fixed argv, cleared env, timeouts, bounded redacted output;
recipes compiled in; versions validated against a strict grammar; native confirmation bound to
plan digest, single-use, expiring; fingerprints re-checked immediately before rename; symlink /
hard-link / ownership / permission / traversal refusal on mutable targets; encrypted backups with
id-bound AEAD; compensation refuses when an external edit intervened; secrets write-only to the OS
store; redaction before IPC/persistence; MCP URLs restricted (HTTPS or loopback HTTP, no
userinfo/link-local/private ranges); no stdio MCP creation.

## Platform status

- macOS (arm64, 27.2): unit, scenario and read-only real discovery executed.
- Windows: code paths exist (paths, `.exe`/`.cmd` handling, Credential Manager feature) but **no
  Windows build or runtime test has been executed**. Windows ACL/ownership checks for mutation are
  not implemented; Windows mutation capabilities report `unverified`.

## Known limitations / remaining work

- Provider online validation and model discovery from Provider APIs are not implemented (no HTTP
  client added; requires the Security §7 origin/redirect/DNS-rebinding controls first).
- Nexus-managed secrets are stored but not handed to any Agent (no secure handoff path exists for
  the four Agents without plaintext config or env persistence) — custom providers are therefore
  `unsupported` for assignment, by design.
- Skills install/enable/remove not implemented; Claude MCP mutation not implemented.
- Real lifecycle execution (install/update/uninstall against the developer's machine) was not
  performed; only fake-npm scenario tests.
- Windows runtime validation outstanding.

## Frontend integration (Gate H)

| Area | Live behaviour (desktop app) |
| --- | --- |
| Data boundary | `src/app/backend.ts` (typed IPC mirror) → `src/app/nexus-data.tsx` provider → `src/data/live.ts` view mapping. `src/data/demo.ts` maps the unchanged Phase 1 fixtures only when not running inside Tauri; the shell labels the mode (“This computer” vs “Demo data”). A failed scan shows an error state, never fixtures |
| Home / Agents | Real readiness register, readiness lines from Health checks, findings as exhibits, real tools, operation journal |
| Agent Detail | Overview (top finding + trusted repair), Model (precedence layers, assignment editor, apply plan), Extensions (MCP toggle / add remote / remove Nexus-added), Configuration (installations with update/uninstall/launch, config sources, auth evidence, capability matrix, encrypted backups with restore), Diagnostics (structured findings, repair buttons, all checks, evidence) |
| Models | Assignment chain, catalog (built-in aliases, observed-in-config, manual), profile revisions, Providers incl. custom OpenAI-compatible endpoints and write-only Keychain secret entry |
| Extensions / System / Settings | Real MCP + Skills inventory; real OS/tools/PATH/data dir; data mode in About |
| Action flow | `src/components/ActionFlow.tsx`: exact plan (diff or argv, version, backup, rollback, privileges, warnings, preserved items) → “Confirm…” asks Rust to show the native dialog → verified result with per-check outcomes |

UI files touched: `src/App.tsx`, `src/main.tsx`, all `src/pages/*`, `src/components/{AgentRows,AgentConfiguration,HealthEvidence,ExtensionRows,ReadinessLine,ui}.tsx`, new `src/components/ActionFlow.tsx`, `src/styles.css` (appended block of token-only rules for plan sheets, small buttons, forms; no existing rule changed), locale files (additive keys; 5 new namespaces). `src/app/readiness.ts` replaced by view-model nodes. The approved visual system (tokens, typography, layout, components, themes) is unchanged; all 40 pre-existing Playwright UI tests (bilingual, themes, narrow window, keyboard, axe) pass unmodified except for two import path updates.

## Test results (2026-10-27, macOS 27.2 arm64)

| Command | Result |
| --- | --- |
| `cargo test --locked` (src-tauri) | 126 passed, 0 failed, 3 ignored (opt-in: real-machine scan, fixture export, real Keychain self-test) |
| `cargo test keychain_selftest -- --ignored` | Real macOS Keychain: backup key stored, reused across instances, deleted; no leftover item |
| `cargo clippy --locked --all-targets` | 0 warnings |
| `cargo test real_machine -- --ignored` | Read-only scan of this Mac, ~0.12 s, no secrets in output |
| `pnpm typecheck` | pass |
| `npx eslint src tests` | 0 problems (the repository-wide `eslint .` still reports 258 pre-existing errors confined to `design-lab/` prototypes, unchanged from baseline) |
| `pnpm test` (vitest) | 16 passed |
| `pnpm test:ui` (Playwright, Chrome) | 50 passed: 40 existing demo-mode UI tests + 10 live-mode tests driven by backend-generated fixtures in `tests/fixtures/live/` |
| `pnpm tauri build --debug --bundles app` | built; app launched; Home showed live data (3 of 4 Agents ready, OpenCode attention: two global config files) |

## Independent review (Gate I)

A separate reviewer agent with no implementation context reviewed commits a7481d1 + 0b75a64 read-only. Findings and dispositions:

| Sev | Finding | Disposition |
| --- | --- | --- |
| P1 | npm update/uninstall used the first npm on PATH, so it could act on a different copy (nvm vs Homebrew Node); pnpm/bun/yarn globals were treated as npm | **Fixed**: operations on existing installs pass `--prefix` derived from the selected installation; uninstall refuses without a prefix; pnpm/bun/yarn classified Unknown (no recipe). Tests added |
| P1 | Homebrew/native-Claude updates change the resolved-path id, so verification always failed | **Fixed**: verification follows the stable launcher path; regression test added |
| P2 | Model written into a config routed to another provider (Codex `model_provider`, Bedrock/Vertex) | **Fixed**: `model.routing_mismatch` refusal; test added |
| P2 | Codex `profile` ignored when reporting the effective model | **Fixed**: active profile reported as an uncertain higher layer (drift → possiblyOverridden, verification → inconclusive) |
| P2 | Restore plan sent whole-file diff (possible secrets) to the renderer | **Fixed**: restores disclose fingerprints and warnings only; test asserts empty diff |
| P2 | Creating a new file could replace one created meanwhile | **Fixed**: creation publishes with hard-link (no-replace) semantics; test added |
| P2 | Written-but-unverified re-read reported as unchanged | **Fixed**: treated as verification failure and compensated; external-edit refusal reported as `partial` |
| P2 | `node` used by npm without trust checks | **Fixed**: node/npm files and their parent dirs must be user/root-owned and not world-writable |
| P3 | Native dialog showed only a warning count | **Fixed**: warning texts listed (from `native.json`) |
| P3 | MCP names redacted, preventing toggles | **Fixed**: names are control-stripped, never masked |
| P3 | URL policy missed IPv6 ULA/link-local, IPv4-mapped, CGNAT | **Fixed**; tests extended |
| P3 | Brew recovery rewrote npm-style `pkg@v` | **Fixed**: pin rewrite only for npm; brew uses the plan-time resolved version |
| P3 | Launch skipped trust checks | **Fixed**: launch target and resolved file must pass owner/permission checks. Launch remains a direct user action without a dialog (Security §4) |
| P3 | JSONC: comment in empty object lost; CRLF files got LF | **Fixed** with tests. **Open**: removing an entry can drop an adjacent comment line; Gemini `mcp.excluded` array is re-rendered |
| P3 | TOML edits lacked unrelated-change verification | **Fixed**: same `verify_only_changed` check as JSON |
| P3 | Restore "reintroduces executable settings" check is substring-based | **Open** (conservative warning; structural comparison not yet implemented) |
| P3 | Secret presence check reads the secret (may prompt) | **Open** |
| P3 | Journal step writes inside the transaction callback are best-effort | **Open** |
| P3 | Windows `npm.cmd` fails the executable check, so Windows npm recipes are inert | **Open**, consistent with Windows `unverified` status |

## Release preparation (2026-10-27)

- Branch merged into `main` (fast-forward). Before publication, history was rewritten (with user approval, never pushed before) to exclude `design-lab/`, `.impeccable/` (third-party reference images, local paths) and the dead worktree pointer; absolute home paths were scrubbed from Phase 1 records. Those folders remain on disk and are git-ignored. Pre-cleanup `.git` backup: `~/Documents/Project-Nexus-Dev-git-before-cleanup-20261027.tar.gz`.
- Version 0.2.0 (tag `v0.2.0-beta.1`, pre-release). `cargo fmt` applied.
- Release artifact: `Project Nexus_0.2.0_aarch64.dmg` — ad-hoc signed, **not notarized** (Security §5 signing gate still open; disclosed in release notes).

## English-only change (2026-10-27)

Owner decision: the product ships in English only. Removed the zh-CN catalog, the language preference, the toolbar language selector, the Settings language section, the `nexus_set_locale` IPC command and its capability, Chinese native menu/dialog text, `:lang(zh-CN)` CSS and the Source Han Sans/Serif fonts (~18 MB). Chinese-language Phase 1 records were rewritten in English. I18N, PRD, AGENTS, ROADMAP and DESIGN_SYSTEM updated. The i18n structure remains with a single `en-US` catalog. Saved preferences from earlier builds keep their theme; the old language field is ignored.

## Continuation point

All gates implemented. Next steps toward a public Beta: Windows build + runtime validation (paths,
`.cmd` shims, Credential Manager, ACL checks for mutation), online Provider validation with the
Security §7 network controls, Skills install/enable, reviewed stdio MCP recipes, signed release
and updater, usability sessions.
