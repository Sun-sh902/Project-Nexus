# Project Nexus security specification

Status: canonical Phase 0 threat model, 2026-10-07. These are design and future release obligations, not claims of implemented protection or certification. Read with [ARCHITECTURE](ARCHITECTURE.md) and [AGENT_MANIFEST](AGENT_MANIFEST.md).

## 1. Assets, actors, and trust boundaries

Protect developer files, toolchains, Agent configuration, API/OAuth credentials, project identity/paths, trusted executable identity, backup integrity, and informed user control. Adversaries include a compromised renderer, malicious local project/configuration, poisoned PATH candidate, malicious Extension publisher, intercepted or compromised metadata/download infrastructure, a compromised upstream package, and prompt-injected AI output.

| Boundary | Untrusted side | Enforcement side |
| --- | --- | --- |
| Webview → native IPC | Renderer state, user-supplied strings, forged/replayed messages | Rust schemas, origin/window policy, Action Registry, native authorization |
| Native → filesystem/processes | Files can change; executable names and environment can be poisoned | Validated handles/identities, locks, fixed recipes and argument schemas |
| Network → metadata/cache | Provider responses, manifests, release listings, redirects | Limits, schema/integrity/signatures, origin policies, version/freshness checks |
| Secret store → consuming action | Caller requests and endpoint selections | Opaque references, exact destination binding, minimum lifetime |
| Nexus → managed Agent | Agent config can start hooks, MCP, helpers, and plugins | Supported-field allowlist, execution-aware preview, trusted extension recipes |
| Evidence → UI/export/AI | Logs may contain secrets, control sequences, instructions | Sanitized structured data, redaction, export review, no execution |
| Release publisher → installed Nexus | Update metadata/binaries may be substituted | Independent signing keys, signed application artifacts, rollback/freshness policy |

MVP runs as the current user without a persistent privileged service. A fully compromised OS/admin account is outside the isolation guarantee; secure storage and Rust cannot protect against every same-user memory attack. Nexus manages readiness, not the security sandbox of every launched Agent. These limits must not justify unsafe default behavior.

## 2. Mandatory invariants

| ID | Invariant |
| --- | --- |
| S01 | No unrestricted shell/process execution from frontend UI, remote data, Manifest, or AI output |
| S02 | Every native side effect maps to a typed allowlisted action with native validation and resource scope |
| S03 | Manifests contain no arbitrary command/script/argument fields and cannot load native code or expand shipped policy |
| S04 | Nexus-managed secrets are held in OS secure storage; SQLite contains references only; no plaintext fallback |
| S05 | No secrets in logs, analytics, diagnostic payloads, URLs, argv, ordinary backups, or exported evidence |
| S06 | Existing configuration/toolchains are never blindly overwritten, replaced, or deleted |
| S07 | Changes require an exact impact preview and explicit confirmation; no silent escalation or auto-repair |
| S08 | Verification uses fresh postconditions; partial/inconclusive/rollback-failed results cannot be reported as successful repair |
| S09 | Untrusted paths, downloads, metadata, config, and evidence remain untrusted even if apparently official or signed |
| S10 | Essential confirmation, protected backups, recovery, and security updates are available without a paid plan |
| S11 | Remote services and AI cannot grant execution permission; no cloud dependency for deterministic local Health |
| S12 | Offline/stale/denied/unknown states never become fabricated healthy or compatible states |

An action is trusted because reviewed native code restricts its semantics and parameters, not merely because its name appears in a registry. A generic `run_program`, `write_file`, `fetch_url`, or `install_package` with arbitrary arguments violates these invariants.

## 3. Threat analysis and required evidence

| Threat | Required controls | Future verification |
| --- | --- | --- |
| Frontend compromise / IPC misuse | Local bundled UI only; restrictive CSP; no remote HTML/script or permissive navigation; minimal Tauri permissions; origin/window checks; Rust validates every request; no secret-read command | Forged, oversized, cross-window, replayed, unknown-action and stale-plan requests rejected |
| Forged user confirmation | Rust prepares immutable plan; native-owned trusted confirmation for configuration/execution-sensitive and elevated actions; user-gesture receipt bound to plan digest, target and expiry | Compromised renderer cannot mint receipt, alter plan after approval, or reuse approval |
| Command injection / shell interpolation | Direct reviewed executable + typed argv; fixed environment/cwd; no shell string concatenation; reject flags in identifiers; platform-specific serializers only where unavoidable | Metacharacters, newlines, leading dashes, spaces, Unicode and Windows quoting cases cannot alter action |
| Poisoned PATH / version probe | Resolve/validate candidate before executing; inspect metadata first; no automatic shell rc loading, npm/npx acquisition, project hooks, or arbitrary executable probing | Hostile current directory, PATH shim and malformed binary fixtures remain unexecuted |
| Manifest or Manifest-server compromise | Signed envelope, pinned trust roots, anti-rollback/freshness, schema limits, shipped recipe identity/host constraints; unknown adapter/action rejected | Valid signature on an unauthorized package/path still fails; expired/replayed manifests cannot authorize mutation |
| Supply-chain package/script compromise | Vetted exact package/source/version, integrity and publisher evidence, staged download, known transitive behavior; no floating install at execution time | Mismatched hash/publisher, lifecycle-script policy violation and changed dependency resolution fail safely |
| Malicious downloads / redirects | HTTPS, exact source/origin allowlists for installers, no downgrade; validate each redirect; size/time limits; staged artifacts; verify before execution | Redirect to another host/private service, malformed archive, path escape and digest mismatch rejected |
| Unsafe config overwrite / parser loss | Preserve unknown keys/comments where supported; targeted allowlisted edits; backup; conflict fingerprint; re-read validation | Ordinary edits leave invalid inputs untouched; dedicated confirmed restore validates backup and protects replaced bytes |
| Crash / disk full / multi-file failure | Durable per-step journal, protected backup, secure sibling temp file, platform replace; no claim of cross-file atomicity | Kill/fault injection at each boundary yields original, intended, or explicitly recoverable partial state |
| Concurrent external edit / updater | Identity/fingerprint recheck immediately before write; resource locks; observe external changes; no blind rollback | Concurrent modification makes plan stale and requires new preview; restore never erases later edits |
| Symlink / traversal / reparse / hard-link attack | Secure handle-based operations where available; approved roots; reject unexpected links, path aliases, junctions, alternate streams and hard links on mutable targets | Race tests and archive traversal fixtures cannot escape target root; unsupported safe primitive disables mutation |
| Privilege escalation | Prefer per-user installs; explain affected resources/need first; native OS prompt only for narrowly scoped action; no saved admin credentials/helper wildcard | Denial leaves existing state intact or precisely reports completed unprivileged steps |
| Secret leakage / hostile error output | Redact before IPC/storage/export, schema allowlists, bounded output, no raw debug dump, disabled automatic crash-payload upload | Synthetic key canaries in headers/JSON/log fragments/ANSI output never reach saved or displayed diagnostic payloads |
| Credential misrouting / malicious Provider URL | Bind reference to Provider origin/protocol; baseURL change invalidates credential routing approval; validate redirect and resolved destination | No credential sent to newly selected origin without approval; no redirect forwarding of Authorization |
| Unsafe Nexus updater | Signed artifacts and metadata, protected release keys, anti-downgrade, platform signing, recovery checks; separate Manifest trust root | Invalid signature/channel/architecture/version rejected; interrupted upgrade recoverable |
| AI prompt injection / generated command | Evidence is quoted data; AI has no executor/secret interface; action IDs resolved and revalidated by deterministic engine | Instructions embedded in logs/Skills/model replies cannot mutate state or bypass confirmation |
| MCP/Skills as indirect execution | No generic command import; reviewed stdio recipe IDs only for Nexus-authored bindings; source/permissions preview; never run during discovery | Malicious config hook/command field cannot be introduced through a model or Extension edit |
| Partial install / unsafe uninstall | Installation ownership ledger and source-specific recovery; do not remove shared tools/config; verify executable identity | Cancelled/interrupted install reconciles safely; adopted ambiguous install cannot be recursively deleted |

Tauri capability scoping and updater signatures are useful platform primitives, not the complete threat model. Official references: [capabilities](https://v2.tauri.app/security/capabilities/) and [updater](https://v2.tauri.app/plugin/updater/), checked 2026-10-07.

## 4. Execution and confirmation policy

Read-only requests still receive schema, resource, output, and rate limits. Low-risk local preference changes such as theme/language are explicit user actions and need no repair dialog. Every installation, update, uninstall, configuration/Extension mutation, credential rerouting, or repair requires an exact plan preview and confirmation. Launch requires a direct user action and a validated target; it never weakens Agent permission settings.

The frontend may render a richer preview, but cannot grant authority by sending `confirmed: true`. Rust creates a short-lived, single-use authorization bound to the digest after a native-owned confirmation UI presents target, source, impact, privilege and rollback summary. Privileged operations additionally use the OS authorization prompt. Cancellation invalidates authorization. Native confirmation is a required implementation spike before Phase 4; a renderer-only fallback does not satisfy this contract.

The registry permits finite action semantics, e.g. applying a validated model patch or restoring an eligible backup. Package names, recipe paths, interpreter choices, flags, and allowed download origins are shipped policy, not arbitrary frontend fields. Even direct argv can cause injection via a tool's options; validate each argument semantically and use option terminators where supported.

On Windows, `.cmd` wrappers and process command-line reconstruction need a dedicated reviewed strategy. Do not pass untrusted values to `cmd /c`, PowerShell `-Command`, or POSIX `sh -c`. If an installer requires a script interpreter, it must run a verified immutable script file under the narrowly scoped policy below, never a remotely supplied command string.

## 5. Installer and update supply chain

Prefer verifiable official binaries or vetted package-manager identities where appropriate. A GitHub release location alone is not an authenticity proof. Pin repository/publisher identity, platform, artifact identity, expected digest from a trusted channel, and signature evidence when available. A checksum fetched from the same compromised unsigned server does not establish authenticity.

OfficialScriptInstaller is a reserved mechanism, disabled by default. Enabling it requires a reviewed immutable script revision, independently trusted digest/signature, fixed interpreter, bounded structured inputs, documented side effects and child-download verification. If transitive behavior cannot be bounded, offer official manual installation and subsequent adoption. Never pipe live network bytes into an interpreter.

Package managers may execute lifecycle scripts and resolve dependencies. A recipe must document whether scripts run, which trusted identity/version is installed, how resolution is frozen, and how the installed result is verified. If the manager cannot honor the confirmed artifact/version, stop and replan. Do not claim Nexus can universally roll back package-manager side effects.

**Recorded exception (2026-10-27):** the owner approved publishing `v0.2.0-beta.1` as a GitHub *pre-release* with an ad-hoc-signed, non-notarized Apple Silicon DMG, disclosed as such in the release notes and README. This exception covers that pre-release only; it does not satisfy the signing/notarization gate for a public Beta or any later release, and there is no auto-updater.

Application updater and Manifest publisher use separate keys. Establish release signing, key custody, revocation/rotation, incident response, anti-downgrade, platform notarization/signing validation, and interrupted-update recovery before public distribution. A signed Manifest cannot update Nexus or rotate its own trust root. Compromised-root recovery requires a separately trusted application release or explicit out-of-band recovery procedure.

## 6. Configuration integrity and backups

Before mutation, identify the exact current-user scope and file identity; validate ownership/access and adapter version. Never follow an unexpected link to a new target. System-managed policy is read-only in MVP. Preserve unknown fields and comments; a parser incapable of preserving them cannot perform a broad rewrite. Ordinary Model edits cannot add hooks, plugin packages, credential-helper commands, or permission changes.

Changing a file may activate pre-existing hooks or live reload in a running Agent. An adapter must account for those side effects during preflight. If it cannot establish a safe write window, request that the user close the relevant sessions and defer the write; never silently terminate sessions, disable hooks, or claim that preserving a hook makes triggering it harmless. Restore previews also evaluate execution-sensitive fields in the backup against current policy. A historical file is not automatically safe to reintroduce.

Use Read → Parse → Validate → Backup → Minimal merge → Atomic write → Re-read → Validate → Rollback where practical. The backup is verified before the first write. Validate the freshly read file against both syntax and intended semantic postconditions. A parse success alone is insufficient.

Backups may contain pre-existing Agent secrets even when the Nexus patch does not. Keep exact-byte backups encrypted with an OS-secured key, protected by current-user permissions, outside repositories/cloud-sync defaults. Plaintext copies, SQLite blobs, and raw diff snapshots are forbidden. Do not rely on redaction for an exact restorative backup. If secure backup is unavailable, block the mutation; retain read-only/manual guidance. Temporary files inherit safe permissions and must not expose secret-bearing originals.

Recovery records contain hashes/opaque references and safe metadata, not original secret bytes. Automatic compensation must compare current state with the operation's known output and refuse if external changes intervene. A user-requested restore instead needs a new plan bound to the current fingerprint, a validated known-good backup, explicit replacement-impact disclosure, a protected copy of current bytes, and fresh confirmation. This is the only supported automated replacement of malformed configuration; ordinary edits remain blocked. Multi-file recovery shows each restored/unrestored resource. Retain necessary backup/journal state until verification and the disclosed recovery window finish; never prune unresolved operations automatically. Default proposed local retention is 7 days for redacted diagnostic evidence and 30 days for verified-operation recovery records/backups, with user deletion and disk limits. Unresolved operations are exempt and require explicit review. Phase 4 must validate storage limits and secure deletion claims; do not promise physical erasure on SSDs.

## 7. Secrets, Provider networking, and privacy

Nexus-managed API keys, tokens, passwords, sensitive custom headers, and backup-encryption keys use SecretStore. SQLite stores opaque references. Keychain/Credential Manager denial or lock is a normal recoverable condition. Resolve secrets only for a specific authorized action; keep lifetime short, avoid copies, clear where practical, and never claim memory zeroization as a universal guarantee.

Agent-owned login remains Agent-owned. Adoption does not read/copy/normalize its credentials. Prefer official login or credential-store integration; use reviewed stdin or narrowly scoped child environment where supported, without echo/argv/shell-history persistence. Such child environment handoff is visible to the child and potentially same-user inspection, so disclose its scope. If an Agent only accepts persistent plaintext credentials, Nexus does not automate that handoff. Existing user-managed configuration may remain adopted with an explicit limitation; do not silently delete it.

Provider URL policy differs from installer URL policy. Custom Providers may intentionally be local or private. Default HTTPS with normal TLS verification; no “disable certificate validation” convenience option. Allow loopback HTTP only by explicit local-endpoint selection and consent; no public plaintext credential transport. Bind approved origin and resolved address scope; reject cloud metadata/link-local and unexpected private redirects, constrain proxies and redirects, and validate destinations on connection to resist DNS rebinding. Installer/Manifest fetches do not inherit these custom Provider exceptions.

Provider tests show what will be sent and to which host. Do not send prompts/project data for connectivity checks or make paid completion calls automatically. Discovery results, model names, metadata, and errors are untrusted strings with size limits. Credential reset/replacement does not silently reroute other Agents. Account subscription rights must never be assumed equivalent to API access.

Local-first means no telemetry, diagnostic upload, or AI evidence upload by default. Secrets never enter analytics even with opt-in. Export requires a preview, redaction, target choice, and explicit user action; home/project path identifiers are minimized. Raw evidence containing uncertain secrets is withheld rather than declared sanitized. Debug mode cannot disable these rules. Strip terminal control sequences, render evidence as text, and disallow clickable unsafe URI schemes/HTML.

## 8. Platform differences and residual risk

| macOS | Windows |
| --- | --- |
| Keychain user presence/access failures; signing/notarization/quarantine | Credential Manager failures; Authenticode and UAC boundaries |
| POSIX ownership/permissions and symlink/hard-link races | ACL inheritance, junctions/reparse points, drive/UNC aliases and alternate data streams |
| GUI PATH differs from interactive shell; do not source rc files | Inherited environment differs from PowerShell profile; `.cmd` wrappers are not ordinary executables |
| Atomic replacement/durability and package-manager prefixes require tests | Locked files/antivirus and replace semantics require independent tests |

Native Windows and WSL are different execution/credential/filesystem boundaries. Do not tunnel operations between them implicitly. Minimum Nexus OS support is a packaging decision based on evidence; an unsupported Agent does not disable Nexus itself.

Residual risks include malicious behavior inside a legitimately installed Agent/Extension, compromised upstream packages signed by their rightful publisher, and same-user/admin compromise. Show relevant execution/permission boundaries; do not market “verified” as a guarantee of harmless behavior.

## 9. Security release gates and incident handling

Before Phase 2 probes: validate untrusted candidate handling and redaction. Before Phase 4 mutations: prove native authorization, per-resource validation, protected backups, operation journals and recovery on both OSes. Before Phase 5: prove credential-store failure handling, origin binding and adapter handoff. Before Phase 6: prove execution-bearing Extension/config fields cannot bypass policy. Before Phase 7 Beta: run the threat matrix, supply-chain/update verification, failure injection, and confirmation review.

On a malicious Manifest/artifact report: stop relevant mutation, retain safe evidence, revoke affected metadata with signed policy where supported, preserve existing installs, and provide manual recovery guidance. Revocation never uninstalls software or performs a repair without confirmation. A root-key compromise needs a trusted application update/out-of-band path. Specific key custody, signing algorithm/library, vulnerability-reporting channel, support process, and OS implementation primitives must be chosen before their delivery phases; these deferrals do not relax S01–S12.
