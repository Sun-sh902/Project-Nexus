# Agent Manifest specification

Status: Phase 0 normative design, proposed schema **1.0**; no parser, schema file, signer, remote service, or executable integration exists. See [ARCHITECTURE](ARCHITECTURE.md) and [SECURITY](SECURITY.md).

## 1. Purpose and authority

A Manifest describes an Agent and selects already shipped native capabilities. It supports ecosystem metadata evolution without remote code execution. Initially bundle reviewed manifests in the application. Remote refresh is optional later, after its signature/cache policy is implemented and tested; no Manifest service is needed for early discovery.

A Manifest is never a script, adapter binary, template language, package-manager command, or permission grant. Arbitrary shell commands are prohibited because remote metadata could otherwise turn a server compromise into execution on every developer machine. Structured parameters are safe only when a compiled recipe restricts their meaning; naming a dangerous generic executor does not make it safe.

Authority is the intersection of schema, compatible installed Nexus version, shipped adapter/recipe policy, platform, observed Agent version, and local policy. User confirmation cannot override a failed integrity or policy check.

## 2. Versioning and field rules

Use UTF-8 JSON, finite enumerations, bounded strings/arrays, and exact field definitions. Reject duplicate keys, invalid Unicode, excessive nesting, oversized payloads, and unknown semantic fields. Proposed limits: 128 KiB per Agent payload, depth 12, 32 targets, 64 capability entries; revisit only with evidence before parser implementation. Never evaluate interpolation, regex code, JavaScript, shell variables, or remote schema references.

`schemaVersion` is `major.minor`. Unsupported major or higher minor is rejected for activation; do not silently ignore a new security-relevant field. Additive fields get a new minor; removed/changed semantics require major. Application releases explicitly enumerate supported minors and migrations. Metadata cannot request an automatic parser/adapter download. Older supported schemas normalize through tested native migrations; malformed input is never repaired heuristically.

| Field | Required | Meaning / validation |
| --- | --- | --- |
| schemaVersion | yes | Supported schema version, initially `1.0` |
| manifestRevision | yes | Positive monotonically increasing integer per Agent/channel; distinct from Agent version |
| id | yes | Stable lowercase namespaced Agent ID, max 96 characters; rename displayName without changing identity |
| displayName | yes | Bounded plain text product name; no HTML/Markdown or code |
| vendor | yes | Bounded publisher label; a label alone is not publisher verification |
| officialWebsite | yes | HTTPS documentation/product URL; metadata, never an execution destination |
| adapterId | yes | Exact compiled adapter ID allowlisted for this Agent |
| minimumNexusVersion | yes | SemVer lower bound, checked against installed Nexus |
| channel | yes | Shipped allowed enum, initially `stable`; a remote document cannot create a channel |
| capabilities | yes | Array of known capability IDs; empty allowed for metadata-only records |
| targets | yes | Explicit OS/architecture/execution-environment combinations and strategy declarations |
| integrityPolicyId | yes | Shipped verification policy ID; cannot weaken minimum native policy |
| reviewedAt | yes | RFC 3339 evidence review timestamp; not a signature or activation expiry |
| sources | yes | Bounded list of official evidence URLs; informational, no automatic fetch/execute |

Identity is vendor/product integration identity, not local installation identity. The adapter registry reserves mappings for initial Agents; the exact machine-readable IDs are fixed when registry implementation begins. Never derive IDs from translated display names. Remote aliases cannot impersonate another Agent.

## 3. Capabilities and targets

Initial capability vocabulary: `detect`, `launch`, `install`, `update`, `uninstall`, `config.read`, `config.model.write`, `model.assign`, `extensions.mcp.read`, `extensions.mcp.write`, `extensions.skills.read`, `extensions.skills.write`, `health.contribute`. Capability absence means unavailable, not implicitly allowed. A target may narrow the top-level capability list with `capabilities`; it cannot widen it. Per-Agent/version restrictions still come from compiled policy.

Each target requires `os` (`macos` or `windows`), `arch` (`aarch64` or `x86_64`), `environment` (`native` in schema 1.0), and `capabilities`. These are vocabulary values, not a promise to ship all combinations. Optional `minimumOsVersion`/`maximumOsVersionExclusive` require verified official evidence and use an OS-specific native version parser. Omit unknown minima rather than invent them. CPU emulation is not inferred from architecture mismatch.

The target may contain the following declarations only when corresponding capabilities are advertised. Require a nonempty target array, unique target tuples, unique capability entries, and the declaration for every advertised `detect`, `install`, `update`, or `uninstall` capability. `launch` and configuration/health/extension capabilities resolve exclusively through the compiled adapter. Reject declarations without corresponding target capabilities. A manual uninstall declaration records a supported guidance path; it does not advertise native deletion authority.

| Declaration | Structured fields | Resolution |
| --- | --- | --- |
| detection | `recipeId` | Compiled detector determines candidate names, roots, safe probe parameters and output parser; remote content cannot provide paths/commands |
| install | `mechanism`, `recipeId`, `versionPolicy` | Compiled recipe binds package/source/target and resolves exact version/artifact before confirmation |
| update | `strategy`, optional `recipeId` | `same_source` requires reviewed recipe; `agent_managed` reports external behavior only and does not trigger it |
| uninstall | `strategy`, optional `recipeId` | `same_source` requires reviewed removal recipe; `manual` provides documentation and never implies automated deletion |

`versionPolicy` is an enum (`reviewed_stable` initially), not a shell/version-expression field. Supported concrete releases come from approved signed release metadata or bundled policy. `mechanism` accepts only implemented members of `official_binary`, `official_script`, `homebrew`, `winget`, `npm`, `github_release`, `native_package`. Merely being in this vocabulary does not enable an implementation. `official_script` remains disabled until Security's stronger controls are met.

Do not admit execution/download URL, package, repository, executable, command, args, working directory, environment-variable map, hook, postinstall, or path fields. The explicitly defined informational HTTPS links and target `environment` enum are allowed; neither grants execution authority. Recipe IDs select reviewed immutable policy; adding a new package identity, executable path pattern, source host, or installer family requires a reviewed Nexus release. Later schema changes may add structured parameters only with native finite policy and tests. This deliberately trades some remote flexibility for bounded authority.

Uninstall operates on a validated AgentInstallationId and detected source/ownership, never on a Manifest directory string. Retain user configuration, credentials, projects, sessions, and shared runtimes by default. No source match or uncertain ownership means no automatic removal. Update follows the original installer; switching source requires a separate migration plan and confirmation.

## 4. Integrity envelope and artifact integrity

The Agent payload above is carried inside a separate transport envelope. Production remote acceptance requires `keyId`, `algorithm`, exact `payloadBase64` bytes, `signature`, `issuedAt`, `expiresAt`, and a monotonic `catalogSequence`, all authenticated. Sign a domain-separated, length-delimited encoding of all envelope fields except signature, including exact payload bytes. The implementation must select one reviewed standard signing/encoding scheme and publish test vectors before enabling remote manifests. Do not invent custom cryptography or loosely reserialize JSON for verification.

Trust roots ship in Nexus. Manifest-serving credentials and signing keys are separate. A Manifest cannot supply a new trusted key, override algorithm, disable signature checks, or widen download policy. Algorithm negotiation is a closed native allowlist; unknown algorithms fail. Rotation uses a separately authenticated, monotonic root-update policy with overlap/revocation and tested recovery. Threshold/offline root requirements and concrete signing library are deferred security decisions that block remote activation until settled.

Manifest signature, download integrity, and executable authenticity are different checks. A valid Manifest signature establishes authorized metadata, not safe executable bytes. Artifact resolution records exact identity/version/architecture/source, digest, and required upstream signing/publisher evidence in the plan. Download and redirects must stay within the recipe's permitted origins. A digest from the same unsigned compromised source is insufficient. Archives cannot escape staging or create unsafe links. Verify before invocation and verify installed identity after execution.

## 5. Validation and failure behavior

Acceptance order: bound input → parse envelope → verify trusted signature/time/sequence → parse payload with supported schema → validate IDs/URLs/enums → resolve compiled adapter/recipes → intersect capabilities/platform/policy → cache authenticated payload. Every operation then rechecks current platform, installed Agent version/source, artifact identity, freshness/revocation and plan confirmation.

| Failure | Required behavior |
| --- | --- |
| Unknown schema/adapter/recipe or minimum Nexus unmet | Reject activation; show compatibility reason; do not fetch code to fix it |
| Bad signature/digest, wrong Agent mapping or forbidden field | Quarantine candidate; no mutation based on rejected metadata; retain safe diagnostic code and independently valid prior cache |
| Network unavailable | Use last accepted metadata for display with timestamp; apply freshness policy below |
| Expired/replayed/revoked remote data | Keep inventory visible, prohibit new operations that rely on it |
| Unsupported platform/architecture | Explain capability limitation; Nexus remains usable |
| Artifact mismatch or partial download | Delete/quarantine staging safely; leave existing installation untouched |
| Installed Agent differs from planned version/identity | Invalidate plan and rediscover; require a new preview |

Do not treat an unknown top-level field as a harmless extension. No remote `$ref` schema resolution. No rich markup from display fields. Official website/source links open only as explicit user actions through a restricted HTTPS opener; they are not download authority.

## 6. Cache, updates, and backward compatibility

Bundle a known-good baseline with each signed Nexus release. Store accepted remote payloads by digest with signature, origin, sequence, revision, expiry, and validation version. Activate updates atomically only after full validation; keep the previous accepted record for explanation/recovery. Cache writes cannot replace trusted root policy.

Refresh on explicit request or a bounded policy disclosed in Settings; no tight poll loop. Schema 1.0 uses a proposed maximum remote validity of 7 days; publisher expiry may be shorter. Reject future-issued metadata beyond a small documented clock-skew allowance; suspicious clock regression makes remote freshness unknown and blocks dependent mutation. Exact time/skew handling must have tests before remote release.

Anti-rollback state tracks highest accepted catalog sequence and per-Agent revision. Rejected new metadata must not erase accepted state. Signature verification alone is not freshness. Revocation/tombstones preserve inventory and existing installation data but disable affected Nexus actions; no automatic uninstall or secret deletion.

Expired remote records are display-only. Bundled records may support bounded read-only discovery and native manual launch under shipped policy; bundled install/update needs its own independently current reviewed artifact policy. Never fall back to a stale bundled installer to bypass a known remote revocation. Offline repair using a local protected backup is a native action and does not require remote metadata unless its own policy explicitly does.

Unknown newer minor versions remain inactive until Nexus supports them. A remote metadata change never migrates Agent configuration or auto-applies a Model/Profile. Human-visible diff/confirmation and configuration adapters own that separate workflow.

## 7. Illustrative manifests

These syntactically valid payloads are **fictional design fixtures**, not real Agent manifests, verified installation recipes, or signed production records. The example IDs and `example.invalid` URLs are intentionally nonproduction. They have no signature envelope and must fail production remote acceptance. No final Nexus release version or actual OS minimum is asserted.

Example A: fictional metadata with read-only detection on one target.

```json
{
  "schemaVersion": "1.0",
  "manifestRevision": 1,
  "id": "example.orbit",
  "displayName": "Orbit Example Agent",
  "vendor": "Example Vendor",
  "officialWebsite": "https://example.invalid/orbit",
  "adapterId": "example.orbit.v1",
  "minimumNexusVersion": "0.0.0",
  "channel": "stable",
  "capabilities": ["detect", "config.read", "health.contribute"],
  "targets": [
    {
      "os": "macos",
      "arch": "aarch64",
      "environment": "native",
      "capabilities": ["detect", "config.read", "health.contribute"],
      "detection": {"recipeId": "example.orbit.detect.v1"}
    }
  ],
  "integrityPolicyId": "example.readonly.v1",
  "reviewedAt": "2026-10-07T00:00:00Z",
  "sources": ["https://example.invalid/orbit/docs"]
}
```

Example B: fictional controlled lifecycle declarations. The recipe would have to exist in a test-only compiled registry and bind a specific package identity; the Manifest cannot name one.

```json
{
  "schemaVersion": "1.0",
  "manifestRevision": 2,
  "id": "example.orbit",
  "displayName": "Orbit Example Agent",
  "vendor": "Example Vendor",
  "officialWebsite": "https://example.invalid/orbit",
  "adapterId": "example.orbit.v1",
  "minimumNexusVersion": "0.0.0",
  "channel": "stable",
  "capabilities": ["detect", "launch", "install", "update", "uninstall"],
  "targets": [
    {
      "os": "windows",
      "arch": "x86_64",
      "environment": "native",
      "capabilities": ["detect", "launch", "install", "update", "uninstall"],
      "detection": {"recipeId": "example.orbit.detect.v1"},
      "install": {
        "mechanism": "winget",
        "recipeId": "example.orbit.winget.install.v1",
        "versionPolicy": "reviewed_stable"
      },
      "update": {
        "strategy": "same_source",
        "recipeId": "example.orbit.winget.update.v1"
      },
      "uninstall": {
        "strategy": "same_source",
        "recipeId": "example.orbit.winget.uninstall.v1"
      }
    }
  ],
  "integrityPolicyId": "example.package.v1",
  "reviewedAt": "2026-10-07T00:00:00Z",
  "sources": ["https://example.invalid/orbit/releases"]
}
```

Required negative fixtures when implemented: adding `command`, `args`, or `postinstall`; swapping recipe to another Agent; package/source injection through an identifier; unknown capabilities/schema; wrong signature; replay; expiry; revoked artifact; nested archive traversal; unknown adapter; unsupported target; cache corruption; and a signed but unauthorized source. All must deny relevant mutation without deleting working user state.
