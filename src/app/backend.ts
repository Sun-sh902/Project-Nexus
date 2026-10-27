import { invoke, isTauri } from '@tauri-apps/api/core';

// Typed mirror of the Rust IPC contract (src-tauri/src/ipc.rs). Rust validates every input;
// these types only describe the shapes. Pairs arrive as [key, value] tuples from Rust.
export type Pairs = [string, string][];
export type AgentId = 'codex' | 'claude' | 'gemini' | 'opencode';
export type Readiness = 'ready' | 'attention' | 'blocked' | 'unknown' | 'unsupported';
export type ObsStatus = 'observed' | 'notFound' | 'accessDenied' | 'timeout' | 'parseFailed' | 'notChecked' | 'unsafe';
export type Support = 'supported' | 'unverified' | 'unsupported';

export interface Observed<T> { value: T | null; status: ObsStatus; source: string; location: string | null; observedAt: string }
export interface Installation { id: string; executable: string; resolved: string | null; source: string; version: Observed<string>; pathRank: number | null; executableOk: boolean; nexusManaged: boolean; wrapper: boolean }
export interface ConfigSource { id: string; scope: 'user' | 'managed' | 'agentState'; format: string; location: string; status: ObsStatus; fingerprint: string | null; viaSymlink: boolean; tolerant: boolean }
export interface ModelLayer { kind: string; location: string; value: string | null; effective: boolean; uncertain: boolean }
export interface McpServer { name: string; transport: string; enabled: boolean | null; command: string | null; commandResolvable: boolean | null; endpointOrigin: string | null; envNames: string[]; scope: string; location: string; nexusManaged: boolean; canToggle: boolean }
export interface Skill { name: string; description: string | null; location: string; hasScripts: boolean; valid: boolean }
export interface Capability { key: string; support: Support; reason: string | null }
export interface AgentSnapshot {
  snapshotId: string; agentId: AgentId; name: string; vendor: string; observedAt: string; platform: Support;
  installations: Installation[]; primary: number | null; configSources: ConfigSource[];
  model: { effective: string | null; effectiveKind: string; layers: ModelLayer[]; providerHint: string | null };
  auth: { evidence: string[]; locations: string[] };
  extensions: { mcp: McpServer[]; mcpStatus: ObsStatus; skills: Skill[]; skillsStatus: ObsStatus };
  envOverrides: { variable: string; origin: string; location: string | null; affects: string }[];
  capabilities: Capability[];
  runtimeRequirement: { tool: string; minimum: string | null; evidence: string } | null;
}
export interface HealthCheck { id: string; category: string; requirement: 'required' | 'advisory'; outcome: 'pass' | 'fail' | 'unknown' | 'notApplicable'; detail: string | null; params: Pairs; evidence: string[]; source: string | null; evaluatedAt: string }
export interface AgentHealth { agentId: AgentId; readiness: Readiness; checks: HealthCheck[]; coverage: { required: number; requiredPassed: number; advisory: number; advisoryPassed: number; unknown: number; notChecked: string[] }; ruleSet: string; evaluatedAt: string; validUntil: string; stale: boolean }
export interface RepairOption { action: string; params: Pairs }
export interface Finding { id: string; code: string; agentId: AgentId; checkId: string | null; severity: 'blocking' | 'warning' | 'unknown'; cause: 'established' | 'possible'; params: Pairs; evidence: string[]; observedAt: string; stale: boolean; repairs: RepairOption[]; manual: string | null }
export interface ProviderProfile { id: string; displayName: string; kind: string; baseUrl: string | null; authMode: 'agentLogin' | 'nexusSecret' | 'environment'; secretRef: string | null; secretPresent: boolean | null; envVar: string | null; revision: number; builtin: boolean; connection: string }
export interface ModelDefinition { id: string; providerId: string; identifier: string; displayName: string | null; source: 'builtin' | 'observed' | 'manual'; sourceDetail: string | null; capabilities: { key: string; state: string; source: string }[] }
export interface ModelProfile { id: string; name: string; revision: number; modelId: string; reasoning: string | null }
export interface Compatibility { agentId: AgentId; support: Support; reason: string | null; nativeIdentifier: string | null }
export interface Assignment { agentId: AgentId; mode: 'native' | 'direct' | 'profile'; modelId: string | null; profileId: string | null; desiredRevision: number | null; applied: { identifier: string; profileRevision: number | null; providerRevision: number | null; appliedAt: string; location: string; fingerprint: string } | null; updatedAt: string }
export interface ResolvedAssignment { assignment: Assignment; provider: ProviderProfile; model: ModelDefinition; profile: ModelProfile | null; profileRevision: number | null; compatibility: Compatibility; pending: boolean }
export interface LifecycleOption { recipeId: string; op: 'install' | 'update' | 'uninstall'; package: string; available: boolean; installationId: string | null }
export interface AgentReport { snapshot: AgentSnapshot; health: AgentHealth; findings: Finding[]; assignment: ResolvedAssignment | null; lifecycle: LifecycleOption[] }
export interface ToolObservation { key: string; name: string; present: boolean; location: string | null; version: Observed<string>; candidates: number }
export interface SystemSnapshot { observedAt: string; os: 'macos' | 'windows' | 'linux' | 'other'; osVersion: Observed<string>; arch: string; shell: string | null; tools: ToolObservation[]; pathEntries: { location: string; exists: boolean; onPath: boolean }[]; dataDir: string; dataDirWritable: boolean; guiPathNote: boolean }
export interface OperationRecord { id: string; action: string; agentId: string | null; status: string; startedAt: string; finishedAt: string | null; errorCode: string | null; summary: Record<string, unknown>; steps: string[] }
export interface Overview { contractVersion: 1; generatedAt: string; system: SystemSnapshot; agents: AgentReport[]; recentOperations: OperationRecord[] }
export interface ModelsView { providers: ProviderProfile[]; models: ModelDefinition[]; profiles: ModelProfile[]; assignments: ResolvedAssignment[]; matrix: [string, Compatibility[]][] }
export interface BackupRecord { id: string; operationId: string; agentId: string | null; targetId: string; targetDisplay: string; sha256: string; size: number; createdAt: string }
export interface AgentHistory { backups: BackupRecord[]; operations: OperationRecord[]; health: { agentId: string; readiness: Readiness; issueCodes: string[]; evaluatedAt: string }[] }
export interface ConfigEdit { targetId: string; location: string; format: string; createsFile: boolean; beforeFingerprint: string | null; afterFingerprint: string; changes: { key: string; before: string | null; after: string | null }[]; diff: { kind: '+' | '-'; text: string }[] }
export interface PlanView {
  id: string; digest: string; kind: string; agentId: AgentId | null; createdAt: string; expiresAt: string; params: Pairs;
  config: ConfigEdit | null;
  process: { program: string; args: string[]; envNames: string[]; runsPackageScripts: boolean; usesNetwork: boolean; package: string; version: string | null } | null;
  privileges: string; backup: string; rollback: string; verification: string; warnings: string[]; preserves: string[];
}
export interface OperationResult {
  operationId: string; kind: string; agentId: AgentId | null; status: 'succeeded' | 'failed' | 'partial' | 'rolledBack' | 'rollbackFailed';
  verification: { outcome: 'passed' | 'failed' | 'inconclusive' | 'notRun'; checks: { key: string; outcome: string; params: Pairs; informational: boolean }[]; verifiedAt: string };
  error: NexusErrorShape | null; backupId: string | null; compensation: string | null; outputExcerpt: string | null; warnings: string[];
}
export type ActionRequest =
  | { kind: 'applyModel'; agentId: AgentId }
  | { kind: 'setMcpEnabled'; agentId: AgentId; name: string; enabled: boolean }
  | { kind: 'addRemoteMcp'; agentId: AgentId; name: string; url: string }
  | { kind: 'removeManagedMcp'; agentId: AgentId; name: string }
  | { kind: 'restoreBackup'; backupId: string }
  | { kind: 'install'; agentId: AgentId; recipeId: string }
  | { kind: 'update'; agentId: AgentId; installationId: string }
  | { kind: 'uninstall'; agentId: AgentId; installationId: string }
  | { kind: 'recoverInstall'; operationId: string };

export interface NexusErrorShape { code: string; params: Pairs; retryable: boolean }

/** Native errors keep their stable code; the UI translates `errors:<code>`. */
export class NexusError extends Error {
  readonly code: string;
  readonly params: Record<string, string>;
  readonly retryable: boolean;
  constructor(shape: NexusErrorShape) {
    super(shape.code);
    this.code = shape.code;
    this.params = Object.fromEntries(shape.params ?? []);
    this.retryable = shape.retryable;
  }
}

export const pairs = (value: Pairs | undefined | null): Record<string, string> => Object.fromEntries(value ?? []);
export const liveBackend = () => isTauri();

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && typeof (error as NexusErrorShape).code === 'string') throw new NexusError(error as NexusErrorShape);
    throw new NexusError({ code: 'ipc.unavailable', params: [], retryable: true });
  }
}

export const backend = {
  overview: (refresh: boolean) => call<Overview>('nexus_overview', { refresh }),
  agentHistory: (agentId: AgentId) => call<AgentHistory>('nexus_agent_history', { agentId }),
  models: () => call<ModelsView>('nexus_models'),
  createProvider: (input: { name: string; kind: string; baseUrl?: string; authMode: string; envVar?: string }) => call<ProviderProfile>('nexus_create_provider', { input }),
  setProviderSecret: (providerId: string, secret: string) => call<void>('nexus_set_provider_secret', { input: { providerId, secret } }),
  deleteProvider: (providerId: string) => call<void>('nexus_delete_provider', { providerId }),
  createModel: (input: { providerId: string; identifier: string; displayName?: string }) => call<ModelDefinition>('nexus_create_model', { input }),
  deleteModel: (modelId: string) => call<void>('nexus_delete_model', { modelId }),
  saveProfile: (input: { id?: string; name: string; modelId: string; reasoning?: string }) => call<ModelProfile>('nexus_save_profile', { input }),
  setAssignment: (input: { agentId: AgentId; mode: 'native' | 'direct' | 'profile'; modelId?: string; profileId?: string }) => call<ResolvedAssignment | null>('nexus_set_assignment', { input }),
  prepare: (request: ActionRequest) => call<PlanView>('nexus_prepare_action', { request }),
  confirm: (plan: PlanView) => call<OperationResult>('nexus_confirm_plan', { planId: plan.id, digest: plan.digest }),
  discard: (planId: string) => call<void>('nexus_discard_plan', { planId }),
  launch: (agentId: AgentId, installationId: string) => call<string>('nexus_launch_agent', { agentId, installationId }),
};
