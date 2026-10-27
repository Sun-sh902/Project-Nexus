import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { ExtensionView } from '../data/view';
import { useText } from '../app/nexus-context';
import { AgentLinks, EmptyState, Status } from './ui';

export function ExtensionRows({ items, actions }: { items: readonly ExtensionView[]; actions?: (item: ExtensionView) => ReactNode }) {
  const { t } = useTranslation(['extensions', 'common']);
  const text = useText();
  if (items.length === 0) return <EmptyState />;
  return <div className="sheet">
    <div className="ledger-head extension-grid" aria-hidden="true"><span>{t('name')}</span><span>{t('availability')}</span><span>{t('scope')}</span><span>{t('compatible')}</span></div>
    <ul className="extension-list">{items.map(item => <li key={item.id} className="extension-grid">
      <div className="extension-identity"><p className="extension-name"><span className="type-tag">{t(item.type === 'mcp' ? 'mcp' : 'skills')}</span><strong className={item.literalName ? 'mono' : undefined}>{text(item.name)}</strong></p><p className="row-meta">{text(item.permission)}</p>{item.live?.server?.location && <p className="row-meta mono">{item.live.server.location}</p>}{actions?.(item)}</div>
      <div className="extension-state"><span className="row-label">{t('availability')}</span><Status state={item.readiness} /><span className="row-meta">{text(item.status)}</span></div>
      <div className="extension-scope"><span className="row-label">{t('scope')}</span>{text(item.scope)}</div>
      <div className="extension-agents"><span className="row-label">{t('compatible')}</span><AgentLinks ids={item.agents} /></div>
    </li>)}</ul>
  </div>;
}
