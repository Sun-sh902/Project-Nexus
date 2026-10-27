# Changelog

## Unreleased

### Changed
- English-only interface: removed Simplified Chinese, the language selector in the toolbar and Settings, and the Chinese fonts (smaller app bundle).
- Detailed README with screenshots and a feature guide.

## 0.2.0-beta.1 — 2026-10-27 (pre-release, Core Backend Beta Candidate)

First build with a functional backend underneath the approved Phase 1 interface. Validated on macOS arm64 only. **Not a public Beta**: Windows is unvalidated and the build is not notarized.

### Added
- Read-only Agent and System discovery for OpenAI Codex, Claude Code, Gemini CLI and OpenCode: installations, versions with evidence source, install source, duplicates, configuration files, model precedence (including environment and shell-profile overrides), login evidence (presence only), MCP servers, Skills, Node/npm/Git/Python/Homebrew.
- Deterministic Health engine (`nexus-readiness-1`) with explicit coverage and staleness; structured Diagnostics with cause certainty and evidence.
- Action Registry with immutable, digest-bound, single-use plans; Rust-owned native confirmation; journaled execution; independent verification; automatic compensation only when no external edit intervened.
- Safe Repair: restore an encrypted known-good backup, reapply a displaced model assignment, recover an interrupted Nexus installation, disable an MCP server whose command is missing.
- Lifecycle: npm and Homebrew install/update/uninstall recipes with pinned versions; Claude Code `claude update`; launch in Terminal. Script installers are never executed.
- Models: Providers, Models (built-in aliases, found-in-config, manual), revisioned Model Profiles, per-Agent assignments with desired/applied/effective states; Keychain-only secrets.
- Extensions: MCP enable/disable, add remote HTTPS server, remove Nexus-added servers (Codex, Gemini CLI, OpenCode); Skills read-only.
- Local SQLite persistence with versioned migrations; no secrets stored.
- Live data across Home, Agents, Agent Detail, Models, Extensions, System and Settings.

### Known limitations
- Windows: implemented paths are unverified; Windows npm recipes are inactive.
- No online Provider/credential validation or model discovery; custom endpoints cannot be assigned to Agents.
- No Skills installation, no stdio MCP creation, no Claude Code MCP editing.
- Removing a JSONC entry can drop an adjacent comment line.
- See `docs/BACKEND_IMPLEMENTATION_PROGRESS.md` for the full capability matrix and open review items.
