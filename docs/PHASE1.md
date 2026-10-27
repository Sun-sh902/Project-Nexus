# Phase 1 implementation record

> Historical record. Since 2026-10-27 the product is English-only (see [I18N](I18N.md)); references below to Chinese, both locales or language switching describe the Phase 1 state at the time.

Authorized scope: application shell and bilingual fixture UI only. Phase 0 specifications remain authoritative. No domain discovery, evaluation, mutation, credentials, database, or networking is implemented.

## Dependency decisions (2026-10-07)

The existing pnpm installation is used, with a committed lockfile. Node 26 is the local contributor baseline; it satisfies the selected Vite/Vitest engines. No global tools are installed by this project.

| Dependency | Present need / license / boundary |
| --- | --- |
| React, React DOM, TypeScript, Vite and React plugin | Required rendering and build stack; MIT except TypeScript (Apache-2.0). Desktop WebView/browser frontend, no native authority. |
| Tailwind CSS and Vite integration | Required semantic-token utility integration; MIT; local build only. |
| i18next, react-i18next | Required offline namespaces, pluralization and live locale changes; MIT. No remote loader. |
| Lucide React | Consistent accessible stroke icons, ISC; no copied vendor artwork. |
| Tauri API/CLI, Rust tauri/tauri-build | Required desktop host and typed invocation; MIT OR Apache-2.0. No shell, filesystem, network, store, updater or secret plugins. |
| Rust serde | Typed harmless IPC result serialization; MIT OR Apache-2.0. |
| Rust serde_json | Reads the same bundled translation catalog for native menus and tests enum-bound preference requests; MIT OR Apache-2.0. No Agent configuration parsing. |
| Rust sys-locale | Native read-only preferred-language lookup on macOS/Windows; MIT OR Apache-2.0. Only locale metadata; no Agent/System probing. |
| ESLint, typescript-eslint, globals, React hooks/refresh rules | TypeScript/React quality checks; MIT; development only. |
| Vitest | Pure preference, routing and translation-contract tests; MIT; development only. |
| Playwright and axe-core integration | Navigation/preferences, keyboard, bilingual/theme/reflow and accessibility checks; Apache-2.0 and MPL-2.0 respectively; development only. Uses installed Chrome, no global browser installation. |

These maintained ecosystems support the frontend or the selected macOS/Windows host. Lockfiles pin resolution. The selected frontend dependencies installed without adding lifecycle-script permissions or global tools; a frozen-lockfile install with scripts disabled also passed. No Zustand, TanStack Query, Radix, database or animation package is needed for Phase 1.

## Implementation choices

- Native title bar; 1200×800 initial window, 960×640 practical minimum bounded by monitor work area. CSS has narrower/zoom reflow independent of the native minimum.
- Plain hash routing works with bundled assets and browser history, without a server/router dependency.
- One fixture module owns domain examples and relationships. Fixture labels persist on every screen. No fixture is derived from the developer's environment checks.
- Only theme/language preferences are stored in versioned WebView localStorage. Storage failure is visible. No other data is persisted.
- Native commands are limited to application version/OS locale metadata and enum-bound application preferences (native menu language/window theme). Preferences use a versioned request/result, bounded request ID and stable error codes. A finite application-only action registry has exactly menu-language/window-appearance operations; no domain actions exist. Application-command ACL is explicit, main-window-only and local-origin-only. Locale refresh happens when the window regains focus / visibility. Native menu translations come from the same JSON catalog as React; unsupported Windows undo/redo/fullscreen menu primitives are omitted, while standard WebView editing remains available.
- No model assignment, credential entry, installation, diagnosis, repair or Extension action is represented as executable. Links only navigate to fixture details.

## Platform policy

Development target: native macOS arm64 on the checked host. Windows x86_64 and macOS x86_64 remain intended validation targets, not verified releases. No signed public distribution or final support/minimum-OS claim is made from this one host.

Tauri's macOS configuration default is a build deployment baseline, not proof that Nexus works on that release. Windows requires C++ Build Tools and WebView2. Follow [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), [macOS bundle guidance](https://v2.tauri.app/distribute/macos-application-bundle/) and [Windows installer guidance](https://v2.tauri.app/distribute/windows-installer/). Releasing a support matrix requires actual builds/runtime checks on each target.

## Verification

Executed on 2026-10-07 (Asia/Shanghai). Host: macOS 27.2, arm64; Node 26.10.0, pnpm 12.6.0, rustc 1.99.0, cargo 1.99.0; Command Line Tools/Clang/macOS SDK available. These host versions are evidence, not Nexus minimum requirements.

| Exact command / procedure | Result |
| --- | --- |
| `pnpm install --frozen-lockfile --ignore-scripts` | Passed; lockfile matches dependencies. |
| `pnpm typecheck` | Passed. |
| `pnpm lint` | Passed. |
| `pnpm test` | 14 tests passed across 3 files. Preference defaults/validation/storage denial/persistence, Chinese OS language variants, theme resolution, routes, fixture relationships, namespace/key/interpolation parity, JSX string discipline, formatting, permission scope and composed token contrast. |
| `pnpm test:ui` | 22 tests passed. Installed Chrome on macOS; all 7 primary screens in both locales/themes at 960×640 and 1440×1000, all detail/model/Extension subviews, keyboard tabs/history/focus, search/empty/error, live/persisted preferences, system-theme changes, mocked native locale refresh/denial, blocked/attention diagnostic examples, 40% expanded-label simulation, reduced motion and 320px reflow. |
| `pnpm build` | TypeScript and production frontend build passed; bundled offline translation resources. |
| `pnpm native:check` | Locked macOS Rust check passed. Initial resolver used temporary `CARGO_HTTP_MULTIPLEXING=false CARGO_HTTP_TIMEOUT=30` after a slow HTTP/2 index fetch; no registry/toolchain/system configuration was changed. |
| `pnpm native:test` | 3 Rust tests passed: application metadata, shared native catalog parity, preference request denial for extra fields/invalid enums/non-main windows/oversized and malformed IDs. These are schema/policy unit tests, not an end-to-end remote-origin attack test. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Passed. |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Passed after simplifying window-height clamping. |
| `pnpm desktop:dev --no-watch` | Started Vite, compiled and ran the macOS development application; intentionally stopped after smoke verification. |
| `pnpm desktop:build --bundles app -- --locked` | Release compilation and `.app` packaging passed; approximately 10.55 MiB bundle. |
| Production `.app` via native UI automation | Opened at `tauri://localhost`; all primary routes and Agent Detail visible. Chinese first-launch locale, live translated native menus, native dark window, explicit English, and close/reopen persistence verified. Returned the application to Follow System for language/appearance after testing. |
| `pnpm audit --registry=https://registry.npmjs.org` | No known npm vulnerabilities reported. The configured mirror's audit endpoint was unavailable; the official registry audit succeeded. This is not a complete supply-chain audit or a Rust advisory scan. |
| `git diff --check`, local Markdown links and repository scope inspection | Passed; original owning specifications unchanged. Only the AGENTS phase-status sentence changed. |

## Product-design and engineering review

Reviewed rendered screenshots of Home, Agents, Agent Detail, Models, Extensions, System and Settings for both locales and themes at both desktop sizes, including lower scroll positions. Reviewed subordinate views through the browser suite and native production WebView smoke checks. Primary screenshots are under `test-results/shell-{locale}-{theme}-{width}-primary-surfaces-and-accessibility/`; subordinate screenshots and expanded/error examples are adjacent. The HTML report is `playwright-report/index.html`.

The final visual hierarchy is Agent readiness → aligned Agent rows → actionable fixture findings; System is secondary. Models, Profiles and Providers are separate tabs with explicit Agent references; unknown metadata is never filled in. Settings uses content-driven controls, not decorative preference cards. Both palettes follow the canonical semantic tokens, with system fonts, coherent Lucide strokes, dividers, restrained blue and no score/charts/marketing headings/upsell surfaces. Narrow layout stacks secondary metadata, wraps tabs and collapses the optional evidence pane into a disclosure.

Issues corrected during review: the English Agent Detail model tab now uses the canonical “Model”; a diagnostic group with a failed check no longer says all checks passed; native Tauri default English menus were replaced with the shared locale catalog; native window appearance now follows the same preference as the renderer; inaccessible native preference results and storage denial are visible without discarding usable session preferences. Ready OpenCode fixture references a connected example Provider. No copied vendor logos or font assets are included.

Automated axe checks report zero violations for the scanned primary/subordinate surfaces. Contrast tests cover text/status on canvas/panel/subtle/hover/selected backgrounds and focus/control/button pairs. Keyboard skip links, route focus, radio/select controls and arrow/Home/End tabs passed. The native macOS accessibility tree exposes localized headings, navigation, status text, named Agent actions and radio/select/tab controls. This is **not** a claim of full WCAG conformance or VoiceOver spoken-output validation.

Review also confirmed: no shell/filesystem/network plugins, no wildcard/default core permissions, no real API-key fields, no real Agent paths/config reads, no SQLite, no fake backend, no fixture fetching or domain evaluation, no unused state/query/UI-framework dependency, and no Phase 2 implementation.

## Remaining limitations and release gates

- Windows build/runtime, Windows keyboard/NVDA/Narrator and high-contrast testing were not executed. macOS x86_64 and other macOS versions were not built/run. The roadmap's both-platform release gate remains open; this locally verified shell is not a validated cross-platform release.
- Native VoiceOver spoken-output testing and independent bilingual usability review remain pending. Browser 320px reflow approximates a 400% desktop reading layout; it is not a native OS zoom validation. No blanket accessibility certification is claimed.
- Signing/notarization, public installers and an evidence-backed final minimum-OS/distribution matrix are not completed. Do not infer platform support from the build configuration's deployment baseline.
- All business values, versions, Providers, model capability assertions, diagnostics and recent activity are deliberately fictional. “Connected”/“Ready” appear only within explicitly labeled fixture context. Native commands never determine domain health.
- Preference storage is WebView localStorage, limited to theme/language. Clearing application website data resets these preferences. Native menu/window synchronization may be reported as partial if the OS refuses an update; there is no secret or domain persistence.
- No Phase 0 scope/architecture/roadmap deviation was required. App-only native preference operations implement the roadmap's bounded preference allowance; they do not enable lifecycle/configuration/repair authority.

Phase 1 implementation and macOS-local verification delivered. Outstanding platform/assistive-tech distribution gates are explicitly retained. **Phase 2 has not been started.**
