# Project Nexus

Project Nexus is an AI Agent Lifecycle Manager that keeps local AI agents installed, configured, healthy, and ready to work.

AI Agent 管家 · Keep every AI Agent ready to work. · 让你的所有 AI Agent 始终可用。

For macOS and Windows, with first-class Simplified Chinese (zh-CN) and English (en-US).

Status: Core Backend Beta Candidate (Phases 2–7) on macOS arm64. Inside the desktop app every Agent, model, Extension and System value comes from a read-only scan of this computer; changes happen only through previewed plans confirmed in a native dialog and independently verified. Browser preview (`pnpm dev`) is an explicitly labeled demo using Phase 1 fixtures. Windows runtime validation is outstanding. Details: [backend progress](docs/BACKEND_IMPLEMENTATION_PROGRESS.md).

## Development

Use Node.js 26+, the pinned pnpm version in `package.json`, and stable Rust. Install native prerequisites from the [official Tauri guide](https://v2.tauri.app/start/prerequisites/): macOS desktop development can use Xcode Command Line Tools; Windows needs Microsoft C++ Build Tools and WebView2. The project does not install global/system prerequisites.

```sh
pnpm install --frozen-lockfile
pnpm dev                 # frontend at http://127.0.0.1:1420
pnpm desktop:dev         # Tauri desktop app (starts Vite itself)
```

Run `dev` and `desktop:dev` separately; they use the same frontend port. Both the frontend and desktop app work offline after dependencies are installed.

```sh
pnpm typecheck
pnpm lint
pnpm test
pnpm test:ui             # uses installed Google Chrome
pnpm build
pnpm native:check
pnpm native:test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm desktop:build -- --locked
```

For macOS application-only packaging, use `pnpm desktop:build --bundles app -- --locked`. The bundle is generated at `src-tauri/target/release/bundle/macos/Project Nexus.app`. Windows packaging must run on a Windows build environment; it has not been validated here.

### Installing a pre-release build

Pre-release DMGs (Apple Silicon) are **ad-hoc signed and not notarized**. macOS Gatekeeper will block the first launch: Control-click the app → Open, or remove the quarantine attribute after reviewing the source. Do not use pre-release builds on machines where you cannot accept that risk.

### What the app does

Inside the desktop app, Nexus performs a read-only scan of this computer: installed Agents (Codex, Claude Code, Gemini CLI, OpenCode), their versions and sources, configuration files, model settings and overrides, MCP servers and Skills, and supporting tools. Deterministic Health and structured Diagnostics explain readiness. Every change (model assignment, MCP enable/disable or add, lifecycle install/update/uninstall, backup restore, interrupted-install recovery) is previewed as an exact plan, confirmed in a native system dialog, journaled, backed up (encrypted) where it edits a file, and independently verified. Secrets are kept only in the macOS Keychain. Capability coverage, limits and test evidence: [backend implementation record](docs/BACKEND_IMPLEMENTATION_PROGRESS.md). Release notes: [CHANGELOG](CHANGELOG.md).

Browser preview (`pnpm dev`) has no native backend and shows the clearly labeled Phase 1 demo fixtures. Phase 1 UI history: [Phase 1 implementation record](docs/PHASE1.md). Browser screenshots and the HTML test report are generated under ignored `test-results/` and `playwright-report/` directories.

## Canonical specifications

- [Engineering constitution](AGENTS.md)
- [Product requirements](docs/PRD.md)
- [Architecture and official research](docs/ARCHITECTURE.md)
- [Security and threat model](docs/SECURITY.md)
- [Design system](docs/DESIGN_SYSTEM.md)
- [Internationalization](docs/I18N.md)
- [Agent Manifest specification](docs/AGENT_MANIFEST.md)
- [Canonical roadmap and Phase 0 review](docs/ROADMAP.md)

The PRD owns product scope; the roadmap owns sequencing. The differentiated public Beta boundary is Phase 7, including Agent Health, structured Diagnostics, and useful Safe Repair. Every later phase requires separate authorization.
