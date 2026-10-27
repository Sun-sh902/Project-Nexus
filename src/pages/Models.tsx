import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { agents, models, profiles, providers, findModel, findProfile, findProvider } from '../data/fixtures';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { AgentLinks, Notice, PageHeading, Section, Tabs } from '../components/ui';

// Each row reads left to right as the chain the PRD keeps separate:
// Provider → Model → Model Profile → Agent assignment. Missing links stay explicit (dotted).
function AssignmentChain() {
  const { t } = useTranslation(['models', 'agents', 'common']);
  return <div className="sheet chain-sheet">
    <div className="ledger-head chain-grid" aria-hidden="true"><span>{t('provider')}</span><span>{t('identity')}</span><span>{t('agents:profile')}</span><span>{t('mapAgent')}</span></div>
    <ul className="chain-list">{agents.map(agent => {
      const model = findModel(agent.modelId);
      const provider = findProvider(model?.providerId);
      const profile = findProfile(agent.profileId);
      return <li key={agent.id} className={`chain-grid ${model ? '' : 'is-broken'}`}>
        <span className="chain-cell" data-node="provider"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('provider')}</span>{provider ? <span lang="en-US">{provider.name}</span> : <span className="is-empty">{t('common:none')}</span>}</span>
        <span className="chain-cell" data-node="model"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('identity')}</span>{model ? <span className="mono">{model.identifier}</span> : <span className="is-empty">{t('common:notConfigured')}</span>}</span>
        <span className={`chain-cell ${profile ? '' : 'is-skipped'}`} data-node="profile"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('agents:profile')}</span>{profile ? <span>{t(profile.nameKey)}<span className="chain-rev mono">{t('agents:revisionShort', { revision: profile.revision })}</span></span> : <span className="is-empty">{t('noProfile')}</span>}</span>
        <span className="chain-cell" data-node="agent"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('mapAgent')}</span><span className="chain-agent"><a href={`#/agents/${agent.id}`} lang="en-US">{agent.name}</a><span className="row-meta">{t(`agents:${agent.assignment}`)}</span></span></span>
      </li>;
    })}</ul>
    <p className="sheet-foot">{t('chainNote')}</p>
  </div>;
}

export function Models() {
  const { t } = useTranslation(['models', 'providers', 'agents', 'common', 'navigation']);
  const { locale } = usePreferences();
  const [view, setView] = useState<'catalog' | 'profiles' | 'providers'>('catalog');
  const counts = { catalog: models.length, profiles: profiles.length, providers: providers.length };
  return <><PageHeading title={t('navigation:models')} description={t('description')} kicker={t('pageKicker')} />
    <Section title={t('mapTitle')} className="map-section" aside={<span className="kicker">{t('mapHelp')}</span>}><AssignmentChain /></Section>
    <Section title={t('catalogTitle')} className="catalog-section">
    <Tabs label={t('tabs')} value={view} onChange={setView} options={(['catalog', 'profiles', 'providers'] as const).map(value => ({ value, label: t(value), count: formatNumber(counts[value], locale) }))}>
      {view === 'catalog' && <><div className="sheet">
        <div className="ledger-head model-grid" aria-hidden="true"><span>{t('identity')}</span><span>{t('provider')}</span><span>{t('metadata')}</span><span>{t('usedBy')}</span></div>
        <ul className="model-list">{models.map(model => <li key={model.id} className="model-grid">
          <div><p className="mono model-id">{model.identifier}</p><p className="row-meta">{t('common:demoSource')}</p></div>
          <div><span className="row-label">{t('provider')}</span><span lang="en-US">{findProvider(model.providerId)?.name}</span></div>
          <div><span className="row-label">{t('metadata')}</span>{model.capabilities ? <><span className="capabilities">{model.capabilities.map(key => <span className="chip" key={key}>{t(key)}</span>)}</span><p className="row-meta">{t('fixtureMetadata')}</p></> : <><span className="unknown-tag">{t('common:unknown')}</span><p className="row-meta">{t('unknownHelp')}</p></>}</div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={model.agents} /></div>
        </li>)}</ul>
      </div><Notice>{t('assignmentHelp')}</Notice></>}
      {view === 'profiles' && <><p className="view-intro">{t('profileHelp')}</p><div className="sheet">
        <div className="ledger-head profile-grid" aria-hidden="true"><span>{t('agents:profile')}</span><span>{t('profileIdentity')}</span><span>{t('identity')}</span><span>{t('usedBy')}</span></div>
        <ul className="profile-list">{profiles.map(profile => { const model = findModel(profile.modelId); return <li key={profile.id} className="profile-grid">
          <div><p className="row-title">{t(profile.nameKey)}</p><p className="row-meta">{t('agents:revision', { revision: profile.revision })}</p></div>
          <div><span className="row-label">{t('profileIdentity')}</span><span className="mono">{profile.id}</span></div>
          <div><span className="row-label">{t('identity')}</span><span className="mono">{model?.identifier}</span><p className="row-meta" lang="en-US">{findProvider(model?.providerId)?.name}</p></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={profile.agents} /></div>
        </li>; })}</ul>
      </div><Notice>{t('assignmentHelp')}</Notice></>}
      {view === 'providers' && <><p className="view-intro">{t('providers:help')}</p><div className="sheet">
        <div className="ledger-head provider-grid" aria-hidden="true"><span>{t('provider')}</span><span>{t('providers:status')}</span><span>{t('providers:authentication')}</span><span>{t('providers:destination')}</span><span>{t('usedBy')}</span></div>
        <ul className="provider-list">{providers.map(provider => <li key={provider.id} className="provider-grid">
          <div><p className="row-title" lang="en-US">{provider.name}</p></div>
          <div><span className="row-label">{t('providers:status')}</span><span className={`connection connection-${provider.state}`}><span className="connection-dot" aria-hidden="true" />{t(`providers:${provider.state}`)}</span></div>
          <div><span className="row-label">{t('providers:authentication')}</span>{t(`providers:${provider.authKey}`)}</div>
          <div><span className="row-label">{t('providers:destination')}</span><span className="mono">{provider.endpoint}</span></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={provider.agents} /></div>
        </li>)}</ul>
      </div><Notice>{t('providers:noSecrets')}</Notice></>}
    </Tabs>
    </Section>
  </>;
}
