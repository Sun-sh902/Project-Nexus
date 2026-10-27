# Project Nexus engineering constitution

## Product and authority

Project Nexus is an AI Agent Lifecycle Manager that keeps local AI agents installed, configured, healthy, and ready to work.

- Agents are the product center. Nexus manages their lifecycle; it does not replace them.
- Models and Providers are separate domains. Agent Health, structured Diagnostics, and Safe Repair are first-class domains. AI is not required for them.
- Read [PRD](docs/PRD.md) for product authority and [ROADMAP](docs/ROADMAP.md) for the sole active phase sequence. Phase 7 is the differentiated Nexus MVP / public Beta boundary.
- Phase 1 application shell and UI are implemented. The Phases 2–7 core backend (discovery, Health, lifecycle, Models/Providers, MCP/Skills, Diagnostics, Safe Repair, persistence) was explicitly authorized on 2026-10-27 and is implemented as a Core Backend Beta Candidate; it is not a public Beta until the Phase 7 acceptance criteria (including Windows validation) pass. Status and limits: [BACKEND_IMPLEMENTATION_PROGRESS](docs/BACKEND_IMPLEMENTATION_PROGRESS.md). Fixtures remain only for explicit browser demo mode and tests. Phase 8+ requires separate authorization.

## Architecture

- Default stack: Tauri 2, Rust, React, TypeScript, Vite, Tailwind CSS, selective Radix UI/shadcn/ui, i18next/react-i18next. Use Zustand only for real global UI state, TanStack Query only for asynchronous data, SQLite only when persistence requires it. No MVP cloud dependency.
- Follow [ARCHITECTURE](docs/ARCHITECTURE.md). Keep presentation in React; native access, credentials, validation, and execution in Rust. Typed IPC transports domain intents and sanitized results.
- Use small capability adapters. Keep Agent paths, formats, authentication, commands, Models, and Extensions behind adapters. Unsupported capabilities remain explicit; never invent support.
- Every native side effect maps to a typed, allowlisted Action Registry operation. No generic execute-command, script, arbitrary file-write, or unrestricted URL-fetch IPC.
- Follow [AGENT_MANIFEST](docs/AGENT_MANIFEST.md). Manifests are data, never code. They cannot extend native authority, load adapters, supply shell strings, or authorize new package identities.

## Safety

- Follow [SECURITY](docs/SECURITY.md). No unrestricted shell from the frontend; no remote or AI-generated command execution; no silent privilege escalation.
- Store Nexus-managed secrets through macOS Keychain / Windows Credential Manager. SQLite holds references only. No secrets in logs, diagnostics, telemetry, URLs, or command arguments. Never copy Agent-owned credentials into Nexus storage during adoption.
- Preserve existing configuration and toolchains. Read → Parse → Validate → Backup → Minimal merge → Atomic write → Re-read → Validate → Rollback where practical. Detect concurrent edits, symlinks/reparse points, and unsafe paths. Refuse mutation when safety cannot be established.
- Explain changes, affected Agents, credential destination, privileges, and rollback limits before explicit confirmation. Bind confirmation to the exact native plan; changed inputs require a new plan.
- A successful process exit is not successful repair. Verify postconditions. Report partial changes and rollback failures honestly; never blindly replay interrupted actions.
- Treat downloaded artifacts, local configurations, MCP/Skills content, process output, and AI output as untrusted. Configuration can itself enable executable behavior.

## Product quality

- Follow [DESIGN_SYSTEM](docs/DESIGN_SYSTEM.md): premium desktop ergonomics, calm dense lists, Agent readiness first, deliberate Light/Dark/Follow System themes. Avoid generic dashboards, decorative metrics, giant cards, copied product expression, and hidden future upsells.
- Follow [I18N](docs/I18N.md): the product ships in English (en-US) only; do not add a language selector or another locale without an explicit product decision. No ordinary user-facing strings in React components. Native errors use stable codes and safe parameters. Localize accessibility labels, validation, dates, numbers, and plural forms.
- Keep macOS and Windows first-class. Separate Nexus runtime requirements from Agent/capability requirements. Verify OS and architecture claims against official sources; do not invent minimum versions.
- Support keyboard navigation, visible focus, screen readers, non-color states, reduced motion, and text expansion.

## Delivery discipline

- Work only within the authorized phase. No unrelated refactors, speculative frameworks, premature marketplace, cloud, billing, or Workspace implementation.
- Before adding a dependency, establish the present need, platform support, maintenance, license, and security implications. Prefer an existing appropriate primitive; do not install dependencies during Phase 0.
- Verify current Agent behavior using official documentation/repositories. Date evidence and keep volatile details in adapter evidence, not shared UI assumptions.
- When implementation begins, test pure Health rules, adapter fixtures, typed IPC denial paths, configuration preservation/conflicts, redaction, interrupted operations, and both platforms. Add UI keyboard, localization, theme, and narrow-window checks appropriate to the change.
- Verify material completion claims from the current changed state. Phase 0 checks are document coverage, links, examples, security/roadmap consistency, and repository scope; do not fabricate application test results.
- Update the owning specification when behavior or a decision changes; update affected cross-references in the same change. Resolve contradictions rather than appending an alternative active definition or roadmap.
- Log stable operation/error identifiers and redacted outcomes, not raw configuration, credentials, arbitrary process streams, or full environment dumps. Diagnostic exports require preview and explicit user action.
