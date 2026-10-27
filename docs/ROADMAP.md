# Project Nexus canonical roadmap

Status: sole active Phase 0–13 roadmap, 2026-10-07. [PRD](PRD.md) owns scope. This is a dependency sequence with acceptance gates, not a schedule or claim of delivered functionality. Phase 0 is documentation only; implementation starts only after separate authorization.

**Phase 7 is the differentiated Nexus MVP / public Beta boundary.** Installing Agents, switching Providers, or managing MCP alone is insufficient. Health, structured Diagnostics, and useful Safe Repair must already provide clear user value.

Every delivery phase follows [SECURITY](SECURITY.md), [ARCHITECTURE](ARCHITECTURE.md), [DESIGN_SYSTEM](DESIGN_SYSTEM.md), [I18N](I18N.md), and [AGENT_MANIFEST](AGENT_MANIFEST.md). Acceptance applies to both macOS and Windows and both locales wherever the phase has user-visible behavior. Unsupported upstream capabilities are explicit; they do not excuse missing product support without an approved scope decision.

## Phase 0 — Product / architecture / security / design foundation

- **Goal:** Establish one coherent commercial product foundation before code.
- **Scope:** Engineering constitution; product definition/lifecycle/MVP; capability architecture; Models/Providers; Health/Diagnostics/Repair; threat model; Manifest contract; premium desktop design; bilingual terminology; canonical roadmap; dated official research.
- **Non-goals:** Tauri initialization, dependencies, React/Rust source, SQLite migrations, implementation of any domain, Phase 1.
- **Dependencies:** User's product brief and current official evidence; no previous Nexus documents to preserve.
- **Security considerations:** Close arbitrary-execution and credential/configuration-loss paths at design time; separate signed metadata from executable authority.
- **Acceptance criteria:** Eight requested documents exist, cross-reference consistently, distinguish fact from design/deferred decisions, include all phase gates, and pass four-perspective review with corrections in the actual text. Repository changes remain documentation-only. Final consistency audit finds one product definition and one active roadmap. No runtime claims are made.

## Phase 1 — Application shell + premium bilingual UI

- **Goal:** Deliver a polished desktop foundation with honest nonfunctional states.
- **Scope:** Separately authorized Tauri 2/Rust/React/TypeScript/Vite shell; tokens and Light/Dark/Follow System; Home/Agents/Models/Extensions/System/Settings; navigation and basic accessible primitives; i18n and persisted language/theme. Decide evidence-based Nexus OS/architecture distribution targets and typed IPC conventions. Use clearly labeled development fixtures.
- **Non-goals:** Native Agent/System probing, real Health evaluation, installs, credentials, SQLite domain schema, Providers/Models implementation, Extensions, AI Doctor, Marketplace, Sync, Team, Billing or Upgrade navigation.
- **Dependencies:** Phase 0 accepted; current Tauri/runtime requirements researched; minimal build/dependency/license choices recorded.
- **Security considerations:** Local bundled assets/CSP, least-privilege Tauri permissions, no shell/filesystem/network shortcut exposed to renderer; native preference operations remain bounded.
- **Acceptance criteria:** Runs on the selected macOS/Windows matrix; both locales and themes render at 960×640 and larger size; narrow/zoom layouts remain usable; language switches without restart; keyboard/focus/screen-reader and contrast checks pass. UI does not invent installed/healthy Agents. No future upsell areas appear.

## Phase 2 — Agent Discovery + System Discovery

- **Goal:** Know what exists without changing developer environments.
- **Scope:** Read-only native detection for the four initial Agents and relevant OS/shell/PATH/runtime/tool metadata; immutable snapshots, evidence provenance and failures; bundled Manifest validation; installation-source and duplicate-candidate reporting. Display unsupported platform/capability reasons.
- **Non-goals:** Installation, update, uninstall, shell profile mutation, credential migration, paid/network health inference, automatic WSL management, remote Manifest infrastructure.
- **Dependencies:** Phase 1; official per-Agent detection research with exact version fixtures; trust-aware probe policy.
- **Security considerations:** Do not execute unknown PATH candidates or startup scripts; bound scan/process resources; redact environment/config observations; no full home scan.
- **Acceptance criteria:** Clean, installed, duplicate, unavailable, denied-access, malformed and timeout fixtures produce distinct accurate observations on both platforms. Discovery performs no intended environment mutation, and reviewed probes have documented side effects. UI shows freshness and source without raw secrets. Both OSes have real native detection tests.

## Phase 3 — Agent Health Engine

- **Goal:** Turn observations into explainable readiness.
- **Scope:** Pure deterministic Health rules/aggregation, check coverage, freshness, issue IDs, installation/runtime/prerequisite/config-read checks, Agent Overview and Home readiness. Reserve richer Model/Extension inputs without building them prematurely.
- **Non-goals:** AI, repair execution, installation, comprehensive Diagnostics workflow, Provider/Model configuration, Extension management, Beta claims.
- **Dependencies:** Phase 2 snapshots and adapter evidence contracts. Does not require Phases 5/6; deferred checks remain explicitly not checked.
- **Security considerations:** Health never mutates or calls untrusted binaries directly; stale/unknown evidence cannot become Ready; remote test opt-in and cost boundaries stay intact.
- **Acceptance criteria:** Identical inputs/rules/time produce identical output. Tests cover all five aggregate states, required/advisory checks, stale evidence, partial snapshots and unsupported versions. Each issue has traceable evidence and Agent impact. UI describes checked coverage rather than claiming universal health.

## Phase 4 — Agent Lifecycle

- **Goal:** Safely install, adopt, update, uninstall and launch supported Agent installations.
- **Scope:** Source-specific trusted installer recipes; capability matrix; action registry; immutable plans and native-owned confirmations; bounded execution/progress/cancellation; ownership ledger; protected backup and operation journal as needed; postconditions and interrupted-operation recovery. Introduce only necessary persistence/schema migrations now. Resolve updater/signing requirements before distributing signed builds.
- **Non-goals:** Generic package manager, automatic toolchain replacement, arbitrary scripts, silent updates/privilege escalation, broad config editing, public repair catalog, full Providers/Models.
- **Dependencies:** Phases 2/3; current official installation/update/uninstall/platform evidence; tested native authorization, file/secret backup primitives and recipe integrity policy. Shell and typed IPC from Phase 1.
- **Security considerations:** Artifact/source verification, package scripts, Windows argument handling, external updater races, partial install recovery, precise privilege disclosure. Remote Manifests remain optional; enabling them requires the complete signature/anti-rollback gate.
- **Acceptance criteria:** Each supported lifecycle route verifies resulting identity/version/state; adoption retains config/auth; uninstall preserves user data/shared tools by default. Denied elevation, cancellation, changed artifact, crash, disk-full and interrupted install are covered. Unknown ownership/version stops unsafe mutation. Native confirmation cannot be forged by a renderer. All four Agents have a researched platform/capability matrix with evidence-backed limitations.

## Phase 5 — Models + Providers + safe Agent configuration

- **Goal:** Make supported model setups reusable and effective without losing native configuration.
- **Scope:** ProviderProfile, ModelDefinition/Reference, revisioned ModelProfile, AgentModelAssignment; native/direct/profile modes; built-in/discovered/manual metadata; secure secret references; compatible Provider forms; assignment preview/minimal mutation; desired/applied/effective state; Model-related Health. Ship a documented validated Provider subset rather than every named future category.
- **Non-goals:** Universal model catalog, invented capabilities, every provider option, persistent plaintext key handoff, account subscription conversion, inference routing/proxy service, AI Doctor, pricing dashboards.
- **Dependencies:** Phase 4 action/backup/journal primitives; Phase 3 Health; version-qualified ModelTarget and ConfigAdapter contracts; exact Provider/auth documentation and secure handoff tested on both OSes.
- **Security considerations:** Keychain/Credential Manager unavailable/locked behavior; origin-bound credential use; custom endpoint validation; managed policy/environment overrides; concurrent config edits and protected backups.
- **Acceptance criteria:** Provider ≠ Model in data/UI. Manual unknown models retain unknown metadata. Profile changes show every affected Agent and require application confirmation; partial results preserve applied revisions. Unknown keys/comments survive supported edits. Invalid config, external edits, locked storage, deleted references and authentication failures produce clear outcomes. No secrets reach SQLite/logs/diagnostic exports; native login adoption remains usable.

## Phase 6 — Extensions: MCP + Skills

- **Goal:** Manage basic compatible Agent Extensions while preserving their execution boundaries.
- **Scope:** Internal MCP/Skill representation; per-Agent inventory, compatibility, scope, enable/disable/install/remove where safely supported; reviewed local MCP recipes and remote MCP connections; source/permission/verification disclosure; extension observations contribute to Health.
- **Non-goals:** Marketplace/registry service, arbitrary pasted executable commands, full vendor plugin feature parity, auto-running discovered content, bypassing Agent runtime permissions, Workspace orchestration.
- **Dependencies:** Phase 5 safe mutation and credential handling; Phase 4 actions; verified MCP/Skills documentation and adapter fixtures for each Agent/version.
- **Security considerations:** Prompt injection, executable Skill resources, MCP child processes/network access, archive traversal, config conflicts, credential scopes, malicious publisher claims.
- **Acceptance criteria:** The four-Agent matrix explicitly reports each supported read/write operation. Compatible bindings round-trip safely; unsupported differences are explained. Discovery never executes content. New stdio bindings cannot introduce arbitrary commands. Tests include malicious archives, hidden scripts, conflicting scopes, unknown permission metadata and revocation; unknown publisher verification is never displayed as verified.

## Phase 7 — Diagnostics + Safe Repair

- **Goal:** Complete the differentiated Nexus MVP / public Beta.
- **Scope:** Structured DiagnosticFinding and evidence UI; deterministic cause/impact explanation; trusted RepairPlan/Action/Result/VerificationResult; useful initial repair catalog; explicit preview/confirmation, protected backup, verification and safe compensation; end-to-end Agent recovery journeys and minimal local recovery history.
- **Non-goals:** AI-generated execution, automatic background repairs, AI Doctor, Workspaces, accounts, Pro paywalls, marketplace, usage dashboards.
- **Dependencies:** Phases 1–6; all destructive primitives already validated in Phase 4; Model/Extension evidence available; official research refreshed for the Beta release matrix.
- **Security considerations:** Plan/finding freshness, authorization replay, rollback conflicts, irreversible actions, evidence redaction, signing/update incidents, and unverified-result honesty. Essential safety/recovery remains free.
- **Acceptance criteria:** All [PRD success criteria](PRD.md) pass. Ship at least the three specified useful repair classes across the compatibility matrix and a seeded recovery journey for each initial Agent. Verify missing credentials, config drift, incompatible Model, environment override and Extension faults yield accurate findings, even when repair is manual. Test denied confirmation, crash, stale plan, external edit, verification failure and rollback failure. Both platforms/locales pass usability/accessibility and security review. No P0/P1 safety/data-loss blocker remains; unresolved limitations are documented. Only then may a release be called public Beta.

## Phase 8 — Project Workspaces

- **Goal:** Let users return to a project with its intended Agent setup.
- **Scope:** Project path identity; preferred/fallback Agent; Provider/Model/Profile and Extension associations; instructions/permissions; context-aware readiness and explicit launch configuration. Reuse existing IDs and adapters.
- **Non-goals:** Cloud sync, team policy engine, AI diagnosis, universal session migration, automatic trust of project content.
- **Dependencies:** Phase 7 stable lifecycle and safe mutation; project trust/path and per-context precedence research.
- **Security considerations:** Untrusted repositories, symlinks, path relocation, inherited instructions, project-scoped credentials and fallback changing data destinations.
- **Acceptance criteria:** Opening a Workspace explains effective Agent/Model/Extensions; missing paths and incompatible fallback are explicit; fallback requiring changed credential/destination needs consent. Project association never silently edits global settings. Local workflows remain functional offline.

## Phase 9 — AI Doctor

- **Goal:** Help users understand complex failures beyond deterministic explanations.
- **Scope:** Opt-in redacted evidence interpretation, probable-cause ranking, explanations, and recommendations for existing registered actions, with uncertainty and provider/cost disclosure.
- **Non-goals:** Arbitrary generated commands, direct execution, bypassing confirmation, replacing deterministic Health, automatic upload of secrets/logs/projects.
- **Dependencies:** Phase 7 findings/actions and Phase 8 context where relevant; a separate AI privacy/provider review and evaluation dataset.
- **Security considerations:** Prompt-injected evidence, fabricated diagnoses, unsafe action parameters, credential/evidence exfiltration, model availability and spend.
- **Acceptance criteria:** AI has no executor/secret interface. Recommendations are revalidated by native policy and current snapshots; unsupported actions fail. Adversarial evidence cannot authorize changes. Disabling AI leaves deterministic Health/Diagnostics/Repair usable. Measured evaluation distinguishes grounded advice from speculation.

## Phase 10 — Session Continuity + contextual usage/cost insights

- **Goal:** Resume useful project work with context and explain available resource usage.
- **Scope:** Version-qualified Agent session references/resume capabilities, Workspace continuity, contextual Project/Agent/Provider/Model/Session usage and cost when evidence exists.
- **Non-goals:** Guaranteed cross-Agent conversation portability, scraping secrets, invented token/cost precision, generic analytics dashboard, billing provider replacement.
- **Dependencies:** Phase 8 Workspaces; validated session/usage APIs or formats per Agent; Phase 9 only where AI summaries are explicitly used.
- **Security considerations:** Session content sensitivity, retention, unsupported formats, permission changes on resume, estimates mistaken for bills.
- **Acceptance criteria:** Supported sessions resume in the right project/context with permission review; unsupported resume is explicit. Usage carries provenance/time/unit; estimated or unknown cost is labeled. No project/session upload by default; user can remove retained references.

## Phase 11 — Accounts + cloud sync + Pro

- **Goal:** Add optional paid convenience and cross-device continuity without weakening the local core.
- **Scope:** Optional identity, entitlements, Pro workflows, explicitly selected Workspace/preferences/history sync, conflict resolution and recovery. Evaluate cloud infrastructure only now; no provider is preselected solely by the stack list.
- **Non-goals:** Mandatory login for local readiness, paywalled essential safety, plaintext credential sync, full team control plane, marketplace.
- **Dependencies:** Stable local entities/schema evolution from preceding phases; commercial validation; explicit sync/encryption/privacy/deletion design and entitlement policy.
- **Security considerations:** Account takeover, tenant isolation, key recovery, sync conflicts, subscription outage behavior, regional privacy obligations. Credential sync is excluded unless separately designed and approved.
- **Acceptance criteria:** Offline/free discovery/Health/essential recovery remain available. Account deletion, data export/removal, sync opt-out, revocation and conflict handling work. Pro expiration never blocks recovery of an in-flight action. Security review covers cloud isolation and encryption before launch.

## Phase 12 — Verified Extension Registry / Marketplace

- **Goal:** Make trustworthy Extension discovery and distribution understandable.
- **Scope:** Publisher identity, compatibility/version metadata, signed distribution, review/verification levels, provenance, revocation and reporting; marketplace commerce only if justified separately.
- **Non-goals:** Treating verification as harmlessness, remotely downloaded native adapters, automatic execution or policy bypass, copied catalog assets/content.
- **Dependencies:** Phase 6 Extension safety model, Phase 11 service/identity operations where needed, review and incident-response capability.
- **Security considerations:** Publisher compromise, malicious updates, typosquatting, permissions expansion, signing/root rotation, revocation and stale caches.
- **Acceptance criteria:** Users distinguish publisher identity from code review/integrity. Permission increases require approval; revoked/stale artifacts cannot silently install. Registry outage leaves existing local management usable. Threat-tested publisher/update/revocation flows precede public listing.

## Phase 13 — Team / organization control plane

- **Goal:** Give organizations transparent governance of Agent readiness and approved configurations.
- **Scope:** Approved Agents/Providers/Models/Extensions, version/configuration compliance, scoped Health visibility, Workspace templates, central administration, audit and policy precedence.
- **Non-goals:** Hidden developer surveillance, unbounded remote shell, secret harvesting, invisible policy changes, universal fleet management.
- **Dependencies:** Phase 11 identity/tenant isolation, Phase 12 provenance where registry policy applies, mature native policy enforcement and an explicit organization trust model.
- **Security considerations:** Administrator authority, least privilege, policy signing, tenant isolation, local/remote conflicts, sensitive evidence sharing, revocation/offline operation.
- **Acceptance criteria:** Role boundaries and cross-tenant denial tested; users can see policy source/effect and shared data. Policy never grants arbitrary remote execution. Offline enforcement and emergency recovery behavior are documented/tested. Health visibility minimizes personal/project content and secrets.

## Deferred decision register

| Decision | Owner perspective | Resolve before | Required evidence |
| --- | --- | --- | --- |
| Exact Nexus minimum OS and shipped CPU targets | Engineering | Phase 1 distribution | Official Tauri/runtime guidance plus real platform builds/tests |
| Final per-Agent version/installer support matrix | Engineering + Security | Phase 2 probes / Phase 4 lifecycle | Current official docs, version fixtures, integrity and recovery tests |
| Native trusted confirmation primitive | Security + Design | Phase 4 mutation | Renderer-forgery denial and bilingual accessibility tests |
| Signature algorithm/encoding/root rotation and key custody | Security | Remote Manifest or public updater activation | Standard implementation, test vectors, incident/recovery drill |
| Exact config parsers and atomic-write primitives | Engineering + Security | First config mutation | Preservation fixtures, symlink/race/crash tests on both OSes |
| Beta Provider coverage and safe credential handoff | Product + Engineering + Security | Phase 5 release | Official per-Agent compatibility/auth evidence and no-plaintext tests |
| Probe freshness/timeouts and storage budgets | Engineering + Product | Relevant native phase; finalized by Phase 7 | Real performance/failure/privacy tests; Security defaults remain binding until revised |
| Typography rendering and token adjustments | Design | Phase 1 sign-off | Both OSes/locales/themes, measured contrast, screenshots and accessibility review |
| Pricing, cloud vendor, sync encryption and marketplace business model | Product + Security | Respective post-MVP phase | User demand, cost/privacy/security evidence; no early dependency |

## Implementation status (2026-10-27)

Phases 2–7 core backend were authorized together and implemented on `claude/nexus-backend-mvp` as a Core Backend Beta Candidate, validated on macOS arm64 only. The phase definitions above are unchanged. Resolved for this candidate (details and evidence in [BACKEND_IMPLEMENTATION_PROGRESS](BACKEND_IMPLEMENTATION_PROGRESS.md)):

| Deferred decision | Resolution for the candidate |
| --- | --- |
| Native trusted confirmation primitive | Rust-owned OS message dialog (tauri-plugin-dialog, no renderer permission) built from the immutable plan; plans are digest-bound, single-use, 10-minute expiry |
| Exact config parsers and atomic-write primitives | Span-preserving JSON/JSONC editor and `toml_edit`; sibling temp file + fsync + rename with fingerprint recheck; symlink/hard-link/owner/permission refusal; encrypted exact-byte backups (AES-256-GCM, key in Keychain) |
| Beta Provider coverage and safe credential handoff | Agent-owned login routes only (Codex→OpenAI, Claude→Anthropic, Gemini→Google, OpenCode→Anthropic/OpenAI/Google). Nexus-stored secrets are kept in Keychain but not handed to Agents; custom endpoints are not assignable |
| Probe freshness/timeouts | `--version` probe 5 s / 4 KiB; snapshot validity 600 s; lifecycle process timeout 15 min |
| Final per-Agent installer matrix | npm and Homebrew recipes plus Claude `claude update`; vendor script installers are never executed |

Still open before a public Beta: Windows runtime validation and ACL checks, online Provider validation, Skills installation, usability study, signing/updater.

## Phase 0 review and audit record

Complete-set self-review performed on 2026-10-07 from the following four perspectives. This is a design review, not an independent audit or validation of an application. Findings were corrected in their owning sections; this record does not waive a contradiction.

| Perspective | Focus | Disposition |
| --- | --- | --- |
| Staff Engineer | Boundary size, phased dependencies, version/identity, migration and failure recovery | Corrected undefined required-check/warning combinations; separated automatic compensation from newly confirmed restore; pinned applied Provider revisions. Logical layers remain a modular monolith. See Architecture §§6, 8, 9, 12. |
| Security Engineer | RCE, indirect execution, secrets, writes, signatures, privilege and rollback | Added live-reload/hook preflight and backup-content revalidation; clarified malformed-config restore controls; tightened Manifest capability/declaration consistency and informational-field exceptions. See Security §6 and Manifest §§3, 5. |
| Product Designer | Agent-first hierarchy, Models, desktop ergonomics and bilingual accessibility | Distinguished Restore configuration from Undo this operation in both locales; confirmed narrow desktop and expanded-label behavior; measured base semantic-token contrast without claiming rendered UI verification. See Design §§2, 6, 9 and I18N §3. |
| Product Manager | Differentiation, early scope, Beta boundary, evidence and commercial restraint | Added explicit full-domain Beta readiness policy so limited Phase 3 checks cannot satisfy the release promise. Phase 7 remains the only Beta boundary; essential recovery remains free. See PRD §8 and Architecture §6. |

Final semantic audit: identical product definition in PRD/README/AGENTS; this sole Phase 0–13 sequence; Agent-first hierarchy; separate Models/Providers; first-class deterministic Health/Diagnostics/Repair; AI above those systems; Phase 7 Beta; supporting System; no arbitrary remote command execution; consistent bilingual design; both platforms first-class; independent product identity. All are consistent after the corrections above.

Requirements coverage: brief §§1–20 and 34–38 map to PRD/Architecture; §§21–27 and 42–43 to Architecture/Security; §§28–33 and 44–45 to Design/I18N; §§23–25 and 46 to Architecture/Manifest; §§35 and 47 to this roadmap; §§40 and 48 to the eight-file set/AGENTS; §§39 and 49–51 to the official evidence register, this review, final checks and completion report. This is a coverage index, not a second roadmap.

Mechanical verification covers local Markdown links, balanced code fences, both JSON examples, all fourteen phase blocks with six required fields, exact canonical-definition consistency, terminology/state alignment, base token contrast, whitespace and the documentation-only file inventory. No application build, runtime security test, or rendered UI test is claimed in Phase 0.
