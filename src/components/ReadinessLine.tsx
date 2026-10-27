import { useTranslation } from 'react-i18next';
import { type AgentFixture, findModel, findProfile } from '../data/fixtures';
import { nodeKeys, readinessNodes, type NodeKey, type NodeState } from '../app/readiness';

// The readiness line is Nexus's signature drawing: one path per Agent from installation to
// online service. Nodes come only from fixture checks and configuration records; anything the
// example never assessed is drawn as "not checked" (hatched) or "not reached" (dotted), never as a pass.
const labelKey: Record<NodeKey, string> = { install: 'lineInstall', runtime: 'lineRuntime', config: 'lineConfig', model: 'lineModel', online: 'lineOnline' };
const stateKey: Record<NodeState, string> = { pass: 'nodePass', fail: 'nodeFail', pending: 'nodePending', set: 'nodeSet', unchecked: 'nodeUnchecked', unreached: 'nodeUnreached' };

export function NodeMark({ state }: { state: NodeState }) {
  return <span className="node-mark" data-n={state} aria-hidden="true" />;
}

export function ReadinessLine({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation('health');
  const nodes = readinessNodes(agent);
  const checks = agent.checks.length;
  const passed = agent.checks.filter(check => check.result === 'pass').length;
  return <div className="readiness-line">
    <ol className="line-track" aria-label={t('lineTitle')}>{nodes.map(node => <li key={node.key} data-n={node.state}>
      <NodeMark state={node.state} />
      <span className="line-label" aria-hidden="true">{t(labelKey[node.key])}</span>
      <span className="sr-only">{t('nodeAria', { label: t(labelKey[node.key]), state: t(stateKey[node.state]) })}</span>
    </li>)}</ol>
    <p className="line-caption mono">{t('lineCaption', { passed, total: checks })}<span className="note-divider" aria-hidden="true" />{t('nodeOnlineShort')}</p>
  </div>;
}

export function LineHead() {
  const { t } = useTranslation('health');
  return <ol className="line-head" aria-hidden="true">{nodeKeys.map(key => <li key={key}>{t(labelKey[key])}</li>)}</ol>;
}

export function LineLegend() {
  const { t } = useTranslation('health');
  return <div className="line-legend"><ul>{(['pass', 'fail', 'pending', 'set', 'unchecked', 'unreached'] as const).map(state => <li key={state}><NodeMark state={state} />{t(stateKey[state])}</li>)}</ul><p>{t('legendNote')}</p></div>;
}

// The same path, expanded into the five-step procession on Agent Detail.
export function ReadinessProcession({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation(['health', 'agents']);
  const nodes = readinessNodes(agent);
  const profile = findProfile(agent.profileId);
  const model = findModel(agent.modelId);
  const meta: Record<NodeKey, string> = {
    install: agent.version ?? t('agents:noRecord'),
    runtime: agent.checks.some(check => check.key === 'runtimeCheck') ? t('agents:runtimeCheck') : t('nodeUnreached'),
    config: t('agents:configCheck'),
    model: profile ? t('revisionChange', { applied: agent.appliedRevision ?? '?', desired: profile.revision }) : model?.identifier ?? t('agents:noRecord'),
    online: t('remoteUnknown'),
  };
  return <ol className="procession" aria-label={t('lineTitle')}>{nodes.map((node, index) => <li key={node.key} className="step" data-n={node.state}>
    <p className="step-kicker"><span className="mono">{String(index + 1).padStart(2, '0')}</span><span className="note-divider" aria-hidden="true" />{t(labelKey[node.key])}</p>
    <p className="step-title"><NodeMark state={node.state} />{t(stateKey[node.state])}</p>
    <p className="step-help">{t(`step${node.key[0].toUpperCase()}${node.key.slice(1)}Help`)}</p>
    <p className="step-meta mono">{meta[node.key]}</p>
  </li>)}</ol>;
}
