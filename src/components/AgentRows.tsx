import { useTranslation } from 'react-i18next';
import { type AgentFixture, findModel } from '../data/fixtures';
import { DetailLink, Seal } from './ui';
import { LineHead, LineLegend, ReadinessLine } from './ReadinessLine';

export function AgentRows({ agents }: { agents: readonly AgentFixture[] }) {
  const { t } = useTranslation(['agents', 'common', 'health']);
  return <div className="sheet agent-sheet">
    <div className="agent-grid ledger-head" aria-hidden="true"><span>{t('agent')}</span><LineHead /><span>{t('currentModel')}</span><span>{t('readiness')}</span></div>
    <ul className="agent-ledger">{agents.map(agent => {
      const model = findModel(agent.modelId);
      return <li className={`agent-row agent-grid state-${agent.readiness}`} key={agent.id}>
        <div className="row-identity">
          <a className="agent-name" lang="en-US" href={`#/agents/${agent.id}`}>{agent.name}</a>
          <span className="row-meta"><span lang="en-US">{agent.vendor}</span><span className="note-divider" aria-hidden="true" />{agent.version ? <span className="mono">{agent.version}</span> : t('common:notInstalled')}</span>
        </div>
        <ReadinessLine agent={agent} />
        <div className="row-model">
          <span className={model ? 'mono model-id' : 'is-empty'}>{model?.identifier ?? t('common:notConfigured')}</span>
          <span className="row-meta">{t(agent.assignment)}</span>
        </div>
        <div className="row-verdict">
          <Seal state={agent.readiness} />
          <span className="row-finding">{t(`${agent.id}Issue`)}</span>
          <DetailLink agentId={agent.id} name={agent.name} attention={agent.readiness !== 'ready'} />
        </div>
      </li>;
    })}</ul>
    <LineLegend />
  </div>;
}
