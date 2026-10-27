import { activity, agents, extensions, findModel, findProfile, findProvider, models, profiles, providers, system, type AgentFixture } from './fixtures';
import type { ActivityView, AgentView, ExtensionView, NodeKey, NodeState, Text, ToolView } from './view';

// Explicit demo mode only (browser preview, UI tests). Reuses the original Phase 1 fixture
// strings so the approved demo reads exactly as before.
const checkKeys: Partial<Record<NodeKey, string>> = { install: 'installCheck', runtime: 'runtimeCheck', config: 'configCheck' };

function nodes(agent: AgentFixture): AgentView['nodes'] {
  const profile = findProfile(agent.profileId);
  const model = findModel(agent.modelId);
  return (['install', 'runtime', 'config', 'model', 'online'] as NodeKey[]).map(key => {
    let state: NodeState;
    if (key === 'online') state = 'unchecked';
    else if (key === 'model') state = !agent.installed ? 'unreached' : profile && agent.appliedRevision !== profile.revision ? 'pending' : model ? 'set' : 'unreached';
    else {
      const check = agent.checks.find(item => item.key === checkKeys[key]);
      state = check ? check.result === 'pass' ? 'pass' : 'fail' : 'unreached';
    }
    const meta: Text = {
      install: agent.version ? { key: agent.version, literal: true } : { key: 'agents:noRecord' },
      runtime: agent.checks.some(check => check.key === 'runtimeCheck') ? { key: 'agents:runtimeCheck' } : { key: 'health:nodeUnreached' },
      config: { key: 'agents:configCheck' },
      model: profile ? { key: 'health:revisionChange', params: { applied: agent.appliedRevision ?? '?', desired: profile.revision } } : model ? { key: model.identifier, literal: true } : { key: 'agents:noRecord' },
      online: { key: 'health:remoteUnknown' },
    }[key];
    return { key, state, meta };
  });
}

function extensionView(item: typeof extensions[number]): ExtensionView {
  return {
    id: item.id, type: item.type, name: { key: `extensions:${item.nameKey}` }, literalName: false, readiness: item.readiness,
    status: { key: item.installed ? 'common:installed' : 'common:available' }, scope: { key: `common:${item.scope}` },
    permission: { key: `extensions:${item.permissionKey}` }, agents: item.agents,
  };
}

export function demoAgents(): AgentView[] {
  return agents.map(agent => {
    const model = findModel(agent.modelId);
    const profile = findProfile(agent.profileId);
    return {
      id: agent.id, name: agent.name, vendor: agent.vendor, installed: agent.installed, readiness: agent.readiness, version: agent.version,
      modelId: model?.identifier ?? null, modelSource: { key: `agents:${agent.assignment}` }, assignment: agent.assignment,
      providerName: findProvider(model?.providerId)?.name ?? null,
      profileName: profile ? { key: `models:${profile.nameKey}` } : null,
      desiredRevision: profile?.revision ?? null, appliedRevision: agent.appliedRevision, pending: profile ? agent.appliedRevision !== profile.revision : false,
      nodes: nodes(agent),
      checks: agent.checks.map(check => ({ id: check.key, label: { key: `agents:${check.key}` }, result: { key: check.result === 'pass' ? 'agents:checkPass' : 'agents:checkFail' }, state: check.result, required: true, evidence: [] })),
      requiredPassed: agent.checks.filter(check => check.result === 'pass').length, requiredTotal: agent.checks.length,
      headline: { key: `agents:${agent.id}Issue` }, summary: { key: `agents:${agent.id}Summary` }, next: { key: `agents:${agent.id}Next` }, context: { key: `home:${agent.id}Context` },
      findings: [], extensions: extensions.filter(item => agent.extensionIds.includes(item.id)).map(extensionView),
      observedAt: '2026-10-07T02:30:00Z', stale: false,
    };
  });
}

export const demoExtensions = (): ExtensionView[] => extensions.map(extensionView);
export const demoTools = (): ToolView[] => system.tools.map(tool => ({ key: tool.name, name: tool.name, version: tool.version, readiness: tool.readiness, status: { key: tool.readiness === 'unknown' ? 'system:evidenceMissing' : 'system:runtimeReady' }, agentIds: tool.agentIds, location: null }));
export const demoActivity = (): ActivityView[] => activity.map(item => ({ id: item.id, text: { key: `home:${item.key}` }, time: item.time }));
export const demoSystem = { os: system.os, arch: system.architecture };
export const demoCatalog = { models, profiles, providers };
