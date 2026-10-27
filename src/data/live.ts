import { pairs, type ActionRequest, type AgentReport, type Finding, type HealthCheck, type Overview, type RepairOption } from '../app/backend';
import type { ActivityView, AgentView, CheckView, ExtensionView, NodeKey, NodeState, Text, ToolView } from './view';

// Maps Rust reports to the shared view model. No value is invented: anything Rust did not
// observe stays unknown / not checked.
const severityRank: Record<Finding['severity'], number> = { blocking: 0, unknown: 1, warning: 2 };

export const findingTitle = (f: Finding): Text => ({ key: `findings:${f.code}.title`, params: pairs(f.params) });
export const findingSummary = (f: Finding): Text => ({ key: `findings:${f.code}.summary`, params: pairs(f.params) });
export function findingNext(f: Finding): Text {
  if (f.repairs[0]) return { key: `actions:next.${f.repairs[0].action}` };
  if (f.manual) return { key: `findings:${f.manual}` };
  return { key: 'findings:manual.review' };
}

function checkView(c: HealthCheck): CheckView {
  const params = pairs(c.params);
  return {
    id: c.id,
    label: { key: `checks:${c.id}.name` },
    result: { key: `checks:${c.id}.${c.detail ?? c.outcome}`, params },
    state: c.outcome,
    required: c.requirement === 'required',
    evidence: c.evidence,
  };
}

function node(checks: HealthCheck[], ids: string[]): NodeState {
  const relevant = checks.filter(c => ids.includes(c.id));
  if (relevant.some(c => c.outcome === 'fail' && c.requirement === 'required')) return 'fail';
  if (relevant.some(c => c.outcome === 'fail')) return 'pending';
  if (relevant.some(c => c.outcome === 'unknown')) return 'unchecked';
  if (relevant.length && relevant.every(c => c.outcome === 'notApplicable' && c.detail === 'requiresInstall')) return 'unreached';
  return relevant.length ? 'pass' : 'unchecked';
}

export function agentView(r: AgentReport): AgentView {
  const s = r.snapshot;
  const h = r.health;
  const primary = s.primary !== null ? s.installations[s.primary] : undefined;
  const findings = [...r.findings].sort((a, b) => severityRank[a.severity] - severityRank[b.severity]);
  const top = findings[0];
  const a = r.assignment;
  const installed = s.installations.length > 0;
  const modelState: NodeState = !installed ? 'unreached'
    : a ? (a.compatibility.support === 'unsupported' ? 'fail' : a.pending || h.checks.some(c => c.id === 'model.drift' && c.outcome === 'fail') ? 'pending' : 'set')
    : h.checks.some(c => c.id === 'model.effective' && c.outcome === 'unknown') ? 'unchecked' : 'set';
  const runtimeCheck = h.checks.find(c => c.id === 'runtime.prerequisite');
  const runtimeMeta: Text = runtimeCheck?.detail === 'noRequirement' ? { key: 'health:liveNoRuntime' } : { key: `checks:runtime.prerequisite.${runtimeCheck?.detail ?? runtimeCheck?.outcome ?? 'unknown'}`, params: pairs(runtimeCheck?.params) };
  const nodes: AgentView['nodes'] = ([
    ['install', node(h.checks, ['install.present', 'install.executable', 'install.version']), primary?.version.value ? { key: 'health:liveVersion', params: { version: primary.version.value } } : { key: 'agents:noRecord' }],
    ['runtime', node(h.checks, ['runtime.prerequisite']), runtimeMeta],
    ['config', node(h.checks, ['config.valid', 'config.validTolerated', 'config.unambiguous']), { key: 'health:liveConfigSources', params: { count: s.configSources.filter(c => c.status !== 'notFound').length } }],
    ['model', modelState, s.model.effective ? { key: 'health:liveModel', params: { model: s.model.effective } } : { key: 'agents:liveAgentDefault' }],
    ['online', 'unchecked', { key: 'health:remoteUnknown' }],
  ] as [NodeKey, NodeState, Text][]).map(([key, state, meta]) => ({ key, state, meta }));
  const required = h.checks.filter(c => c.requirement === 'required' && c.outcome !== 'notApplicable');
  const extensions: ExtensionView[] = [
    ...s.extensions.mcp.map(m => ({
      id: `${s.agentId}:mcp:${m.name}`, type: 'mcp' as const, name: { key: m.name, literal: true }, literalName: true,
      readiness: m.enabled === false ? 'unknown' as const : m.commandResolvable === false ? 'attention' as const : 'ready' as const,
      status: { key: m.enabled === false ? 'extensions:liveDisabled' : m.enabled === null ? 'extensions:liveEnabledUnknown' : 'extensions:liveEnabled' },
      scope: { key: 'common:localScope' },
      permission: { key: `extensions:liveTransport.${m.transport}`, params: { command: m.command ?? '', origin: m.endpointOrigin ?? '', env: m.envNames.join(', ') } },
      agents: [s.agentId], live: { agentId: s.agentId, server: m },
    })),
    ...s.extensions.skills.map(k => ({
      id: `${s.agentId}:skill:${k.name}`, type: 'skill' as const, name: { key: k.name, literal: true }, literalName: true,
      readiness: k.valid ? 'ready' as const : 'attention' as const,
      status: { key: k.valid ? 'extensions:liveSkillValid' : 'extensions:liveSkillInvalid' },
      scope: { key: 'common:localScope' },
      permission: { key: k.hasScripts ? 'extensions:liveSkillScripts' : 'extensions:liveSkillInstructions' },
      agents: [s.agentId], live: { agentId: s.agentId, skill: k },
    })),
  ];
  return {
    id: s.agentId, name: s.name, vendor: s.vendor, installed, readiness: h.readiness,
    version: primary?.version.value ?? null,
    modelId: s.model.effective,
    modelSource: { key: `agents:liveSource.${s.model.effectiveKind}` },
    assignment: a?.assignment.mode ?? 'native',
    providerName: a?.provider.displayName ?? null,
    profileName: a?.profile ? { key: a.profile.name, literal: true } : null,
    desiredRevision: a?.profileRevision ?? null,
    appliedRevision: a?.assignment.applied?.profileRevision ?? null,
    pending: a?.pending ?? false,
    nodes,
    checks: h.checks.filter(c => c.outcome !== 'notApplicable' || c.requirement === 'required').map(checkView),
    requiredPassed: required.filter(c => c.outcome === 'pass').length,
    requiredTotal: required.length,
    headline: top ? findingTitle(top) : { key: 'agents:liveReadyIssue' },
    summary: top ? findingSummary(top) : { key: 'agents:liveReadySummary' },
    next: top ? findingNext(top) : { key: 'agents:liveReadyNext' },
    context: top ? findingSummary(top) : { key: 'agents:liveReadySummary' },
    findings, extensions,
    observedAt: s.observedAt, stale: h.stale,
    report: r,
  };
}

export function toolViews(o: Overview): ToolView[] {
  return o.system.tools.map(tool => {
    const dependents = o.agents.filter(a => a.snapshot.runtimeRequirement?.tool === tool.key || (tool.key === 'npm' && a.lifecycle.some(l => l.package.startsWith('npm:'))) || (tool.key === 'homebrew' && a.lifecycle.some(l => l.package.startsWith('brew'))));
    const readiness = !tool.present ? (dependents.some(a => a.snapshot.runtimeRequirement?.tool === tool.key) ? 'blocked' : 'unknown') : tool.version.status === 'observed' ? 'ready' : 'unknown';
    return {
      key: tool.key, name: tool.name, version: tool.version.value, readiness,
      status: { key: !tool.present ? 'system:liveMissing' : tool.version.status === 'observed' ? 'system:liveFound' : `system:liveVersion.${tool.version.status}` },
      agentIds: dependents.map(a => a.snapshot.agentId), location: tool.location,
    };
  });
}

export function activityViews(o: Overview): ActivityView[] {
  return o.recentOperations.slice(0, 4).map(op => ({ id: op.id, text: { key: `actions:activity.${op.status}`, params: { action: `$t:actions:kind.${op.action}`, agent: op.agentId ?? '' } }, time: op.finishedAt ?? op.startedAt }));
}

/** Turns a Diagnostics repair option into a typed Action Registry request. */
export function repairRequest(agent: AgentView, repair: RepairOption): ActionRequest | null {
  const p = pairs(repair.params);
  switch (repair.action) {
    case 'applyModel': return { kind: 'applyModel', agentId: agent.id };
    case 'setMcpEnabled': return { kind: 'setMcpEnabled', agentId: agent.id, name: p.name, enabled: p.enabled === 'true' };
    case 'restoreBackup': return { kind: 'restoreBackup', backupId: p.backupId };
    case 'recoverInstall': return { kind: 'recoverInstall', operationId: p.operationId };
    case 'install': {
      const option = agent.report?.lifecycle.find(o => o.op === 'install' && o.available);
      return option ? { kind: 'install', agentId: agent.id, recipeId: option.recipeId } : null;
    }
    default: return null;
  }
}

