import { useState } from 'react';
import { ArrowLeft, ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { findAgent, findModel, findProvider, extensions, type AgentFixture } from '../data/fixtures';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { ConfigurationSheet } from '../components/AgentConfiguration';
import { HealthCheckList, HealthEvidence, NotChecked } from '../components/HealthEvidence';
import { ExtensionRows } from '../components/ExtensionRows';
import { ReadinessProcession } from '../components/ReadinessLine';
import { EmptyState, ErrorState, InfoList, Kicker, Notice, Seal, Section, Tabs } from '../components/ui';

type DetailView = 'overview' | 'model' | 'extensions' | 'configuration' | 'diagnostics';
const detailViews: readonly DetailView[] = ['overview', 'model', 'extensions', 'configuration', 'diagnostics'];

function Overview({ agent, onOpen }: { agent: AgentFixture; onOpen: (view: DetailView) => void }) {
  const { t } = useTranslation(['agents', 'health', 'common']);
  const related = extensions.filter(item => agent.extensionIds.includes(item.id));
  return <div className="detail-layout"><div className="overview-main">
    <section className="sheet" aria-labelledby="overview-config">
      <header className="sheet-head"><h2 id="overview-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header>
      <ConfigurationSheet agent={agent} />
    </section>
    <Section title={t('importantExtensions')} className="detail-extensions"><ExtensionRows items={related} /></Section>
  </div>
  <aside className="evidence-pane" aria-label={t('health:evidenceTitle')}><HealthEvidence agent={agent} /><NotChecked /></aside>
  <div className="next-step">
    <p><strong>{t('next')}</strong>{t(`${agent.id}Next`)}</p>
    <button className="button button-primary" onClick={() => onOpen(agent.installed ? 'model' : 'diagnostics')}>{t(agent.id === 'claude' ? 'compareConfiguration' : agent.installed ? 'viewModel' : 'viewRequirement')}<ArrowRight size={14} aria-hidden="true" /></button>
  </div></div>;
}

function ModelArea({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation(['agents', 'common']);
  return <div className="model-area"><p className="view-intro">{t('modelHelp')}</p>
    <section className="sheet" aria-labelledby="model-config"><header className="sheet-head"><h2 id="model-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header><ConfigurationSheet agent={agent} /></section>
    <Notice>{t('recordLimit')}</Notice><Notice>{t('common:previewOnly')}</Notice>
  </div>;
}

function Configuration({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation(['agents', 'common']);
  return <div className="configuration-area"><p className="view-intro">{t('configurationHelp')}</p>
    {agent.installed ? <><section className="sheet" aria-labelledby="config-config"><header className="sheet-head"><h2 id="config-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header><ConfigurationSheet agent={agent} help={false} /></section>
      <Section title={t('configSource')}><InfoList items={[
        { label: t('configSource'), value: t('configOrigin') },
        { label: t('configScope'), value: t('common:localScope') },
        { label: t('source'), value: t(agent.assignment) },
        { label: t('auth'), value: t('agentOwned') },
      ]} /></Section></> : <EmptyState />}
    <Notice>{t('recordLimit')}</Notice><Notice>{t('common:previewOnly')}</Notice>
  </div>;
}

function Diagnostics({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation(['diagnostics', 'agents', 'common']);
  const finding = agent.readiness === 'attention' || agent.readiness === 'blocked';
  return <div className="diagnostics-area"><p className="view-intro">{t('help')}</p>{finding ? <section className={`sheet finding state-${agent.readiness}`} aria-labelledby="finding-title">
    <div className="finding-summary"><Seal state={agent.readiness} /><h2 id="finding-title">{t(`agents:${agent.id}Issue`)}</h2><p><span className="field-label">{t('impact')}</span>{t(`${agent.id}Impact`)}</p></div>
    <dl className="comparison">
      <div><dt className="kicker">{t('observed')}</dt><dd className="mono">{t(`${agent.id}Observed`)}</dd></div>
      <div className="comparison-op" aria-hidden="true"><svg width="12" height="12" viewBox="0 0 12 12"><path d="M2.5 4.4h7M2.5 7.6h7M8 1.8 4 10.2" stroke="currentColor" strokeWidth="1.4" /></svg></div>
      <div><dt className="kicker">{t('expected')}</dt><dd className="mono">{t(`${agent.id}Expected`)}</dd></div>
    </dl>
    <InfoList items={[
      { label: t('cause'), value: t('established') },
      { label: t('agents:next'), value: t(`agents:${agent.id}Next`) },
    ]} />
  </section> : <Notice tone="success"><strong>{t('noFindings')}</strong><p>{t('noFindingsHelp')}</p></Notice>}
    <details><summary>{t(agent.checks.some(check => check.result === 'fail') ? 'agents:exampleChecks' : 'agents:passed')}</summary><HealthCheckList agent={agent} /></details>
    <details><summary>{t('evidence')}</summary><p>{t('evidenceHelp')}</p><HealthEvidence agent={agent} /></details><Notice>{t('common:previewOnly')}</Notice>
  </div>;
}

export function AgentDetail({ id }: { id: string }) {
  const { t } = useTranslation(['agents', 'common', 'health']);
  const { locale } = usePreferences();
  const [view, setView] = useState<DetailView>('overview');
  const agent = findAgent(id);
  if (!agent) return <ErrorState />;
  const model = findModel(agent.modelId);
  const extensionCount = extensions.filter(item => agent.extensionIds.includes(item.id)).length;
  function openView(next: DetailView) {
    setView(next);
    // Match keyboard tab activation: put focus on the selected tab after the
    // Overview action replaces its own content. Route and browser history stay intact.
    requestAnimationFrame(() => document.querySelector<HTMLButtonElement>('.agent-detail [role="tab"][aria-selected="true"]')?.focus());
  }
  return <div className="agent-detail-page"><a className="back-link" href="#/agents"><ArrowLeft size={13} aria-hidden="true" />{t('common:backAgents')}</a>
    <article className="agent-detail">
      <header className="hero detail-hero">
        <div className="hero-text">
          <div className="hero-seal"><Seal state={agent.readiness} size="lg" /><Kicker>{t('detailKicker')}</Kicker></div>
          <h1 tabIndex={-1} lang="en-US">{agent.name}</h1>
          <p className="deck"><span className="lead">{t(`${agent.id}Issue`)}{t('common:sentenceEnd')}</span><span className="rest">{t(`${agent.id}Summary`)}</span></p>
        </div>
        <dl className="fact-register">
          <div><dt>{t('vendor')}</dt><dd lang="en-US">{agent.vendor}</dd></div>
          <div><dt>{t('installation')}</dt><dd>{t(agent.installed ? 'common:installed' : 'common:notInstalled')}</dd></div>
          <div><dt>{t('version')}</dt><dd className="mono">{agent.version ?? '—'}</dd></div>
          <div><dt>{t('configScope')}</dt><dd>{t('common:localScope')}</dd></div>
          <div><dt>{t('currentModel')}</dt><dd className={model ? 'mono' : 'is-empty'}>{model?.identifier ?? t('common:notConfigured')}</dd></div>
          <div><dt>{t('source')}</dt><dd>{t(agent.assignment)}</dd></div>
          <div><dt>{t('provider')}</dt><dd lang="en-US">{findProvider(model?.providerId)?.name ?? t('common:none')}</dd></div>
        </dl>
      </header>
      <ReadinessProcession agent={agent} />
      <Tabs label={t('detailTabs')} value={view} onChange={setView} options={detailViews.map(value => ({ value, label: t(value === 'model' ? 'modelTab' : value), count: value === 'extensions' ? formatNumber(extensionCount, locale) : undefined }))}>
        {view === 'overview' && <Overview agent={agent} onOpen={openView} />}
        {view === 'model' && <ModelArea agent={agent} />}
        {view === 'extensions' && <ExtensionRows items={extensions.filter(item => agent.extensionIds.includes(item.id))} />}
        {view === 'configuration' && <Configuration agent={agent} />}
        {view === 'diagnostics' && <Diagnostics agent={agent} />}
      </Tabs>
    </article>
  </div>;
}
