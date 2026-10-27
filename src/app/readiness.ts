import { type AgentFixture, findModel, findProfile } from '../data/fixtures';

// The readiness line is Nexus's signature drawing: one path per Agent from installation to
// online service. Nodes come only from fixture checks and configuration records; anything the
// example never assessed is "not checked" (hatched) or "not reached" (dotted), never a pass.
export type NodeKey = 'install' | 'runtime' | 'config' | 'model' | 'online';
export type NodeState = 'pass' | 'fail' | 'pending' | 'set' | 'unchecked' | 'unreached';
const checkKeys: Partial<Record<NodeKey, string>> = { install: 'installCheck', runtime: 'runtimeCheck', config: 'configCheck' };
export const nodeKeys: readonly NodeKey[] = ['install', 'runtime', 'config', 'model', 'online'];

export function readinessNodes(agent: AgentFixture): { key: NodeKey; state: NodeState }[] {
  const profile = findProfile(agent.profileId);
  return nodeKeys.map(key => {
    if (key === 'online') return { key, state: 'unchecked' };
    if (key === 'model') {
      if (!agent.installed) return { key, state: 'unreached' };
      if (profile && agent.appliedRevision !== profile.revision) return { key, state: 'pending' };
      return { key, state: findModel(agent.modelId) ? 'set' : 'unreached' };
    }
    const check = agent.checks.find(item => item.key === checkKeys[key]);
    return { key, state: check ? check.result === 'pass' ? 'pass' : 'fail' : 'unreached' };
  });
}

