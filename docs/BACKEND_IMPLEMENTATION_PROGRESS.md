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
| H — Frontend integration | In progress (see §Continuation) |
| I — Validation / review | Pending |

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
| `native_confirm.rs` | Rust-owned OS confirmation dialog built from the plan (tauri-plugin-dialog), text from `src/locales/*/native.json` |
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

## Continuation point

Gate H: wire React to IPC (`src/app/backend.ts`, data provider), keep `src/data/fixtures.ts` only
for explicit demo mode (browser/non-Tauri), add plan preview / confirmation UI using existing
components, bilingual strings for checks/findings/errors. Then Gate I: frontend typecheck/lint/
unit/Playwright, Tauri build and native launch check, independent review.
