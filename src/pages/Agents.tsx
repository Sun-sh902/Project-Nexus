import { useState } from 'react';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import { AgentRows } from '../components/AgentRows';
import { DataState, EmptyState, PageHeading, StateGlyph } from '../components/ui';

export function Agents() {
  const { t } = useTranslation(['agents', 'navigation', 'health', 'common', 'home']);
  const [filter, setFilter] = useState('');
  const { agents, mode, status } = useNexus();
  const shown = agents.filter(agent => `${agent.name} ${agent.vendor}`.toLocaleLowerCase().includes(filter.toLocaleLowerCase().trim()));
  return <>
    <PageHeading title={t('navigation:agents')} description={t('description')} kicker={t(mode === 'live' ? 'pageKickerLive' : 'pageKicker')}>
      <div className="list-toolbar"><label className="search-field"><Search size={14} aria-hidden="true" /><input type="search" value={filter} onChange={event => setFilter(event.target.value)} placeholder={t('searchHint')} aria-label={t('search')} /></label><span className="kicker" aria-live="polite">{t('common:count', { count: shown.length })}</span></div>
    </PageHeading>
    {status !== 'ready' ? <DataState /> : shown.length ? <AgentRows agents={shown} /> : <EmptyState title={t('noResults')} description={t('searchHint')}><button className="button" onClick={() => setFilter('')}>{t('clearSearch')}</button></EmptyState>}
    <section className="state-reference" aria-labelledby="agents-legend"><h2 id="agents-legend" className="kicker">{t('health:legend')}</h2>
      <ul>{(['ready', 'attention', 'blocked', 'unknown', 'unsupported'] as const).map(state => <li key={state}><p className={`state-name state-${state}`}><StateGlyph state={state} size={11} />{t(`health:${state}`)}</p><p>{t(`health:${state}Help`)}</p></li>)}</ul>
    </section>
  </>;
}
