// This is the only business-data boundary in Phase 1. Nothing here is detected,
// fetched, evaluated, or persisted. IDs/versions/capabilities are illustrative.
export type Readiness = 'ready' | 'attention' | 'blocked' | 'unknown' | 'unsupported';
export type AgentId = 'codex' | 'claude' | 'gemini' | 'opencode';
export interface AgentFixture {
  id: AgentId; name: string; vendor: string; installed: boolean; readiness: Readiness;
  version: string | null; modelId: string | null; assignment: 'native' | 'direct' | 'profile';
  profileId: string | null; appliedRevision: number | null; extensionIds: string[];
  checks: { key: string; result: 'pass' | 'fail' }[];
}
export const fixtureTime = '2026-10-07T02:30:00Z';
export const agents: readonly AgentFixture[] = [
  { id: 'codex', name: 'OpenAI Codex', vendor: 'OpenAI', installed: true, readiness: 'ready', version: '0.0.0-demo', modelId: 'coding', assignment: 'profile', profileId: 'quality', appliedRevision: 1, extensionIds: ['files', 'review'], checks: [{ key: 'installCheck', result: 'pass' }, { key: 'runtimeCheck', result: 'pass' }, { key: 'configCheck', result: 'pass' }] },
  { id: 'claude', name: 'Claude Code', vendor: 'Anthropic', installed: true, readiness: 'attention', version: '0.0.0-demo', modelId: 'sonnet', assignment: 'native', profileId: 'fast', appliedRevision: 1, extensionIds: ['review'], checks: [{ key: 'installCheck', result: 'pass' }, { key: 'runtimeCheck', result: 'pass' }, { key: 'configCheck', result: 'pass' }] },
  { id: 'gemini', name: 'Gemini CLI', vendor: 'Google', installed: false, readiness: 'blocked', version: null, modelId: null, assignment: 'native', profileId: null, appliedRevision: null, extensionIds: [], checks: [{ key: 'installCheck', result: 'fail' }] },
  { id: 'opencode', name: 'OpenCode', vendor: 'OpenCode', installed: true, readiness: 'ready', version: '0.0.0-demo', modelId: 'custom', assignment: 'direct', profileId: null, appliedRevision: null, extensionIds: ['docs', 'legacy'], checks: [{ key: 'installCheck', result: 'pass' }, { key: 'runtimeCheck', result: 'pass' }, { key: 'configCheck', result: 'pass' }] },
];
export const models = [
  { id: 'coding', identifier: 'demo-coding', providerId: 'openai', capabilities: ['coding', 'tools'], agents: ['codex'] as AgentId[] },
  { id: 'sonnet', identifier: 'demo-fast-coding', providerId: 'anthropic', capabilities: null, agents: ['claude'] as AgentId[] },
  { id: 'custom', identifier: 'example/custom-model', providerId: 'openrouter', capabilities: null, agents: ['opencode'] as AgentId[] },
] as const;
export const profiles = [
  { id: 'quality', nameKey: 'highQuality', modelId: 'coding', revision: 1, agents: ['codex'] as AgentId[] },
  { id: 'fast', nameKey: 'fast', modelId: 'sonnet', revision: 2, agents: ['claude'] as AgentId[] },
] as const;
export const providers = [
  { id: 'openai', name: 'OpenAI', state: 'connected', authKey: 'agentLogin', endpoint: 'openai.example.invalid', agents: ['codex'] as AgentId[] },
  { id: 'anthropic', name: 'Anthropic', state: 'connected', authKey: 'agentLogin', endpoint: 'anthropic.example.invalid', agents: ['claude'] as AgentId[] },
  { id: 'openrouter', name: 'OpenRouter', state: 'connected', authKey: 'secureReference', endpoint: 'openrouter.example.invalid', agents: ['opencode'] as AgentId[] },
  { id: 'example', name: 'Example', state: 'notConfigured', authKey: 'secureReference', endpoint: 'provider.example.invalid', agents: [] as AgentId[] },
] as const;
export const extensions: readonly { id: string; type: 'mcp' | 'skill'; nameKey: string; installed: boolean; readiness: Readiness; scope: 'localScope' | 'projectScope'; permissionKey: string; agents: AgentId[] }[] = [
  { id: 'files', type: 'mcp', nameKey: 'filesystemName', installed: true, readiness: 'ready', scope: 'projectScope', permissionKey: 'filesystemPermission', agents: ['codex'] },
  { id: 'docs', type: 'mcp', nameKey: 'docsName', installed: false, readiness: 'unknown', scope: 'localScope', permissionKey: 'docsPermission', agents: ['codex', 'opencode'] },
  { id: 'legacy', type: 'mcp', nameKey: 'legacyName', installed: false, readiness: 'unsupported', scope: 'localScope', permissionKey: 'legacyPermission', agents: ['opencode'] },
  { id: 'review', type: 'skill', nameKey: 'reviewName', installed: true, readiness: 'attention', scope: 'localScope', permissionKey: 'reviewPermission', agents: ['codex', 'claude'] },
];
export const system = {
  os: 'macOS', architecture: 'arm64',
  tools: [
    { name: 'Node.js', version: '22.0.0-demo', readiness: 'ready' as const, agentIds: ['codex', 'gemini'] as AgentId[] },
    { name: 'Git', version: '2.0.0-demo', readiness: 'ready' as const, agentIds: ['codex', 'claude', 'opencode'] as AgentId[] },
    { name: 'Python', version: null, readiness: 'unknown' as const, agentIds: ['codex', 'opencode'] as AgentId[] },
    { name: 'npm', version: '10.0.0-demo', readiness: 'ready' as const, agentIds: ['codex', 'gemini'] as AgentId[] },
  ],
};
export const activity = [
  { id: 'model', key: 'activityModel', time: '2026-10-07T02:25:00Z' },
  { id: 'extension', key: 'activityExtension', time: '2026-10-07T01:50:00Z' },
] as const;
export const findAgent = (id: string) => agents.find(agent => agent.id === id);
export const findModel = (id: string | null) => models.find(model => model.id === id);
export const findProfile = (id: string | null) => profiles.find(profile => profile.id === id);
export const findProvider = (id: string | undefined) => providers.find(provider => provider.id === id);
