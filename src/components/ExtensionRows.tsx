import { useTranslation } from 'react-i18next';
import { extensions } from '../data/fixtures';
import { AgentLinks, EmptyState, Status } from './ui';

export function ExtensionRows({ items }: { items: typeof extensions }) {
  const { t } = useTranslation(['extensions', 'common']);
  if (items.length === 0) return <EmptyState />;
  return <div className="sheet">
    <div className="ledger-head extension-grid" aria-hidden="true"><span>{t('name')}</span><span>{t('availability')}</span><span>{t('scope')}</span><span>{t('compatible')}</span></div>
    <ul className="extension-list">{items.map(item => <li key={item.id} className="extension-grid">
      <div className="extension-identity"><p className="extension-name"><span className="type-tag">{t(item.type === 'mcp' ? 'mcp' : 'skills')}</span><strong>{t(item.nameKey)}</strong></p><p className="row-meta">{t(item.permissionKey)}</p></div>
      <div className="extension-state"><span className="row-label">{t('availability')}</span><Status state={item.readiness} /><span className="row-meta">{t(item.installed ? 'common:installed' : 'common:available')}</span></div>
      <div className="extension-scope"><span className="row-label">{t('scope')}</span>{t(`common:${item.scope}`)}</div>
      <div className="extension-agents"><span className="row-label">{t('compatible')}</span><AgentLinks ids={item.agents} /></div>
    </li>)}</ul>
  </div>;
}
