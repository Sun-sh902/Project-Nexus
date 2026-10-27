# Project Nexus product requirements

Status: canonical Phase 0 specification. Research baseline: 2026-10-07 (Asia/Shanghai). Requirements describe intended behavior, not implemented features.

## 1. Product definition and promise

**Project Nexus is an AI Agent Lifecycle Manager that keeps local AI agents installed, configured, healthy, and ready to work.**

Chinese product concept: **AI Agent 管家**. Core promise: **Keep every AI Agent ready to work.** / **让你的所有 AI Agent 始终可用。**

Nexus is a local-first desktop product for macOS and Windows. It manages OpenAI Codex, Claude Code, Gemini CLI, and OpenCode, initially through their local CLI integrations. Their desktop apps, IDE integrations, cloud execution, and every upstream feature are not automatically in scope. Nexus does not replace Agents or become a new coding chat client.

The mental model is “My Agents are managed by Nexus.” Agent readiness is the product outcome. Installation, System, Providers, Models, MCP, and Skills support that outcome.

This file is the sole product-scope authority. [ROADMAP](ROADMAP.md) is the sole active phase sequence; [ARCHITECTURE](ARCHITECTURE.md), [SECURITY](SECURITY.md), [DESIGN_SYSTEM](DESIGN_SYSTEM.md), [I18N](I18N.md), and [AGENT_MANIFEST](AGENT_MANIFEST.md) own their respective contracts. Repeated definition text elsewhere must be identical, not a competing positioning statement.

## 2. Users, problems, and jobs

| Primary user | Recurring problem | Job to be done |
| --- | --- | --- |
| Individual developer using one or more Agents | Installation exists but the Agent cannot work; errors span runtime, authentication, Models, and configuration | Tell me what is ready and safely restore what is broken |
| Developer trying another Agent | Existing toolchains and credentials must be preserved | Adopt what I already have; install only what is missing |
| Developer switching model services | Provider connection and model compatibility are confused | Assign a usable Model without destroying Agent-specific settings |
| Chinese/English bilingual developer | Mixed terminology, truncated UI, and raw native errors make recovery difficult | Explain issues and consequences clearly in my chosen language |

Organization administrators are a future audience. Enterprise control, fleet management, and team dashboards must not distort individual MVP workflows.

## 3. Canonical lifecycle and differentiation

Discover Agent → Install / Adopt → Configure Model → Configure Extensions → Evaluate Agent Health → Diagnose Issues → Safe Repair → Verify → Ready.

This is a conceptual loop, not a mandatory onboarding wizard. Adoption may preserve a working native login/model and skip configuration. Extensions are optional. New evidence or external changes can invalidate Ready and restart evaluation.

The three differentiators are Agent Health, structured Diagnostics, and Safe Repair. Health answers whether an Agent is ready for the checked context; Diagnostics explains why; Repair offers bounded, trusted changes and proves their result. Raw logs, successful installation, or a Provider switch alone do not deliver the promise.

Readiness is evidence with scope and freshness, not a universal guarantee. `ready`, `attention`, `blocked`, `unknown`, and `unsupported` are defined in Architecture. A stale or unperformed required check cannot produce Ready. Offline Nexus remains usable; remote availability may remain unknown.

## 4. Information architecture

The MVP primary navigation, in order, is **Home, Agents, Models, Extensions, System, Settings**. Chinese labels follow I18N. No AI Doctor, Marketplace, Sync, Team, Billing, or Upgrade navigation appears in the early shell.

**Home** answers which Agents exist, whether they are ready, what needs attention, and what to do next. Its order is readiness summary, My Agents, Needs Attention, contextual Quick Actions, compact System health, and recent meaningful activity. Prefer an Agent list and actionable findings over card grids. System issues name affected Agents. Activity records meaningful state changes, not every poll. No decorative analytics or token charts.

**Agents** is the primary product area. Each Agent row can show installed state, readiness, version, update availability, assigned Model, important issue, and one contextual primary action. Discovery distinguishes multiple installations instead of silently picking one. Adoption records the selected installation and preserves its existing configuration.

**Agent Detail** is first-class, with Overview, Model, Extensions, Configuration, and Diagnostics areas. Overview leads with Health and next action. Configuration initially provides effective-state/provenance and safe supported edits, not an unrestricted text editor. Areas appear as their phase ships; Phase 1 must not pretend native capabilities work.

**Models** contains Models, Providers, and Model Profiles as subordinate views. The Agent Detail Model area remains the primary assignment journey. **Extensions** unifies MCP and Skills inventory while exposing per-Agent compatibility and scope. **System** explains infrastructure in terms of affected Agents. **Settings** contains language, theme, local privacy/storage controls, and application preferences.

## 5. Model Management

ProviderProfile → ModelDefinition / ModelReference → ModelProfile → AgentModelAssignment.

A Provider is a connection to a service; a Model is a selectable identity offered through that connection. A Model Profile is reusable configuration intent, such as “Coding — High Quality” or “Coding — Fast.” An assignment binds an Agent installation to a direct ModelReference, a Model Profile revision, or its existing native configuration. Native configuration is a legitimate state; Nexus must not force account-based Agents into an API-key flow.

Potential Provider categories include OpenAI, Anthropic, OpenRouter, DeepSeek, GLM, Moonshot, and custom compatible endpoints. This is a design direction, not a promise that every category works with every Agent or ships in Beta. Provider-specific schemas and adapter compatibility govern actual support.

Combine built-in known metadata, discovery where officially supported, and manual Model entries. Every capability and availability claim has provenance and freshness. Unknown stays unknown. A custom identifier is not invalid merely because it is absent from a catalog.

Profile changes show affected Agents and compatibility before application. They never silently rewrite every Agent. Desired, applied, and effective state are separate; environment or policy overrides remain visible. Deleted referenced Providers/Profiles require reassignment or a clearly reported broken reference, never cascading deletion of working native configuration.

## 6. Differentiated MVP and non-goals

Public Beta is reached only at the **Phase 7** gate, after discovery, lifecycle operations, Models/Providers/assignments, basic MCP/Skills, Agent Health, structured Diagnostics, and useful Safe Repair work together. All four initial Agents appear in the validated capability matrix on macOS and Windows; unavailable upstream capabilities show a reason. Unsupported badges cannot substitute for implementing available capabilities without an explicit scope decision.

MVP manages local, current-user installations and configuration. It detects relevant enforced policy and respects it, without implementing an organization policy engine. Native Windows and macOS are first-class; WSL instances are separately identified and are not transparently treated as native Windows installations. Automated WSL management is deferred pending its own trust-boundary decision.

Non-goals through Beta: replacing Agent execution UIs; universal package/environment management; arbitrary terminal automation; every Provider or model option; background auto-repair; paid inference for routine health; Workspaces; AI Doctor; session migration; usage/cost dashboards; accounts; cloud sync; billing; marketplace; organization control plane. No silent toolchain replacement, forced credential migration, or copied product identity.

## 7. Primary journeys and outcomes

| Journey | User-visible flow | Required outcome |
| --- | --- | --- |
| Adopt an existing Agent | Discover → inspect installation/source → adopt → evaluate | No reinstall or configuration overwrite; ambiguous copies require selection |
| Install a missing Agent | Inspect requirements/source → review exact plan → confirm → install → verify | Partial installation is explicit; installation alone does not claim Ready |
| Assign a Model | Agent Detail → choose Provider/Model or Profile → inspect compatibility/diff → confirm → apply → verify | Existing unrelated keys survive; effective overrides remain visible |
| Add an Extension | Choose MCP/Skill → inspect source/permissions/scope → choose compatible Agent → confirm | No automatic execution while browsing; no implication that all Agents share permissions |
| Recover a blocked Agent | Health issue → finding/evidence → trusted plan/impact/backup → confirm → execute → verify | Finding resolves only from fresh evidence; rollback/manual recovery if unsuccessful |
| Work offline | Open Home → inspect last-known state and timestamps → run local checks | No login or cloud dependency; remote checks show unknown/stale, not false failure |
| Update/uninstall | Select installation → show ownership, affected paths and retention → confirm → verify | Use the originating installer when safe; preserve user configuration by default |

An Agent can still launch with a non-blocking warning. A known unsafe/incompatible launch is blocked with a reason. Nexus does not weaken the Agent’s own permissions or pretend to sandbox its subsequent work.

## 8. Success criteria

Release gates are requirements, not measured results. Acceptance evidence is local tests, usability sessions, and platform validation; telemetry is not needed to establish them.

- Every initial Agent has documented tested installation/OS/architecture combinations, supported capability boundaries, and source links. Unsupported combinations have actionable explanations.
- In deterministic fixtures, every Ready result has all applicable required checks freshly satisfied; missing evidence, denied access, timeout, and stale state never become success.
- Beta uses an explicit versioned readiness policy covering all implemented local domains for each supported Agent/context. Limited Phase 3 coverage cannot be presented as the finished Beta assessment; optional remote reachability remains separately labeled.
- For each initial Agent, a seeded recoverable issue is discovered, explained, confirmed, repaired, and independently verified. The Beta suite also includes shared configuration-conflict, missing-credential, and interrupted-operation cases.
- Beta ships at least three useful repair classes across its compatibility matrix: restore an eligible known-good configuration backup; reapply a supported assignment displaced by an editable configuration change; recover an interrupted Nexus-owned installation where the installer supports recovery. Manual guidance is required for unsupported cases. Re-authentication may be guided, never silently automated by scraping credentials.
- All destructive/configuration/lifecycle changes show impact and need explicit confirmation. Failure injection proves preservation or a precise recoverable partial state; no silent loss of unrelated configuration is accepted.
- In usability testing, at least 4 of 5 representative users in each locale can identify the blocked Agent and its next safe action without reading raw logs. This is a proposed release target, not existing research.
- Both platforms pass keyboard, screen-reader, Light/Dark, bilingual expansion, and narrow desktop gates. Unknown model metadata is never displayed as a supported capability.
- Quality feedback measures time to a verified ready state and successful recovery, not activity counts or token consumption. Any later analytics proposal needs explicit privacy review and opt-in design.

## 9. Commercial and post-MVP direction

Potential **Free** core: discovery, installation, updates, configuration, Models/Providers, basic Extensions, Health, basic Diagnostics, and essential Safe Repair. Safety confirmations, rollback needed for current operations, recovery records, and security updates must not be paywalled.

Potential **Pro**: AI Doctor, advanced repair workflows, extended repair/configuration history, environment backup, cross-device recovery, Workspace sync, and advanced Workspace workflows. Paid history means additional retention/convenience; basic recovery evidence remains available to every user. Pricing and entitlement design are deferred.

Potential **Team**: approved Agents/Providers/Models/Extensions, policy, version/configuration compliance, Health visibility, Workspace templates, and central administration. Local users must understand what information is shared.

**Project Workspaces** later associate a project path, preferred/fallback Agent, Provider, Model/Profile, MCP, Skills, instructions, permissions, and continuity. The user returns to a project, not merely a CLI. Reserve a context identifier; do not build a Workspace framework early.

**AI Doctor** interprets evidence and recommends registered repair actions above deterministic Discovery, Health, Diagnostics, and Repair. It never generates executable authority. **Session Continuity** and contextual usage/cost insights follow, with unknown cost shown as unknown. Accounts/sync/Pro, a verified registry/marketplace, and Team follow the canonical roadmap.

## 10. Risks, assumptions, and deferred decisions

| Risk or assumption | Mitigation / decision boundary |
| --- | --- |
| Agent behavior changes faster than Nexus | Version-qualified adapters, dated official evidence, unsupported-version behavior and fixtures |
| Existing config, managed policy, or login is complex | Preserve native mode; read-only adoption; never force a universal Provider schema |
| Ready is mistaken for guaranteed remote service access | Show check coverage, context, freshness, and remote-check limitations |
| Installer or local binary is compromised | Independent native action policy, provenance/integrity controls, no automatic execution of unknown candidates |
| Safe merge cannot preserve a format | Refuse mutation; show manual guidance rather than rewrite lossily |
| Shared Model Profile updates affect many Agents | Versioned proposals, impact review, per-Agent outcomes; no implied cross-file atomicity |
| Premium bilingual desktop UX costs effort | Equal locale/platform release gates; no post-release translation cleanup phase |
| Exact Nexus minimum OS / CPU distribution targets unknown | Decide from current Tauri/runtime evidence plus actual Phase 1 packaging tests; never inherit the strictest Agent minimum |

Before Phase 4, select verified installer channels and supply-chain policy per Agent. Before Phase 5, validate Provider/Agent coverage and secure credential handoff. Before Beta, set concrete evidence freshness/retention limits and distribution signing operations. These are intentional implementation-stage decisions, not permission to weaken the Phase 0 contracts.
