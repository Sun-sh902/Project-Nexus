import { useEffect, useState, type ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { backend, NexusError, type ModelsView } from '../app/backend';
import { useNexus } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { demoCatalog } from '../data/demo';
import { agents as fixtureAgents, findModel, findProfile, findProvider } from '../data/fixtures';
import type { AgentId } from '../data/view';
import { AgentLinks, DataState, Notice, PageHeading, Section, Tabs } from '../components/ui';

type ChainRow = { id: AgentId; name: string; provider: string | null; model: string | null; profile: { name: string; revision: number } | null; source: string };

// Each row reads left to right as the chain the PRD keeps separate:
// Provider → Model → Model Profile → Agent assignment. Missing links stay explicit (dotted).
function AssignmentChain({ rows }: { rows: ChainRow[] }) {
  const { t } = useTranslation(['models', 'agents', 'common']);
  return <div className="sheet chain-sheet">
    <div className="ledger-head chain-grid" aria-hidden="true"><span>{t('provider')}</span><span>{t('identity')}</span><span>{t('agents:profile')}</span><span>{t('mapAgent')}</span></div>
    <ul className="chain-list">{rows.map(row => <li key={row.id} className={`chain-grid ${row.model ? '' : 'is-broken'}`}>
      <span className="chain-cell" data-node="provider"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('provider')}</span>{row.provider ? <span lang="en-US">{row.provider}</span> : <span className="is-empty">{t('common:none')}</span>}</span>
      <span className="chain-cell" data-node="model"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('identity')}</span>{row.model ? <span className="mono">{row.model}</span> : <span className="is-empty">{t('common:notConfigured')}</span>}</span>
      <span className={`chain-cell ${row.profile ? '' : 'is-skipped'}`} data-node="profile"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('agents:profile')}</span>{row.profile ? <span>{row.profile.name}<span className="chain-rev mono">{t('agents:revisionShort', { revision: row.profile.revision })}</span></span> : <span className="is-empty">{t('noProfile')}</span>}</span>
      <span className="chain-cell" data-node="agent"><span className="chain-node" aria-hidden="true" /><span className="row-label">{t('mapAgent')}</span><span className="chain-agent"><a href={`#/agents/${row.id}`} lang="en-US">{row.name}</a><span className="row-meta">{row.source}</span></span></span>
    </li>)}</ul>
    <p className="sheet-foot">{t('chainNote')}</p>
  </div>;
}

function Field({ id, label, help, children }: { id: string; label: string; help?: string; children: ReactNode }) {
  return <div className="setting-row"><div><label className="setting-label" htmlFor={id}>{label}</label>{help && <p className="field-help">{help}</p>}</div>{children}</div>;
}
function Select({ id, value, onChange, options }: { id: string; value: string; onChange: (v: string) => void; options: { value: string; label: string }[] }) {
  return <span className="select-control"><select id={id} value={value} onChange={event => onChange(event.target.value)}>{options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}</select><ChevronDown size={14} aria-hidden="true" /></span>;
}

function useSubmit() {
  const { t } = useTranslation(['models', 'errors']);
  const { reloadModels, refresh } = useNexus();
  const [message, setMessage] = useState<{ tone: 'success' | 'error'; text: string } | null>(null);
  const run = async (work: () => Promise<unknown>, success: string) => {
    setMessage(null);
    try { await work(); await reloadModels(); await refresh(); setMessage({ tone: 'success', text: t(success) }); return true; }
    catch (caught) { setMessage({ tone: 'error', text: t(`errors:${caught instanceof NexusError ? caught.code : 'ipc.unavailable'}`, caught instanceof NexusError ? caught.params : undefined) }); return false; }
  };
  return { message, run };
}

function ModelForm({ view }: { view: ModelsView }) {
  const { t } = useTranslation('models');
  const { message, run } = useSubmit();
  const [providerId, setProviderId] = useState(view.providers[0]?.id ?? '');
  const [identifier, setIdentifier] = useState('');
  return <section className="sheet" aria-labelledby="add-model"><header className="sheet-head"><h3 id="add-model" className="sheet-title">{t('addModel')}</h3><span className="kicker">{t('addModelKicker')}</span></header>
    <div className="sheet-body form-stack">
      <Field id="model-provider" label={t('provider')}><Select id="model-provider" value={providerId} onChange={setProviderId} options={view.providers.map(p => ({ value: p.id, label: p.displayName }))} /></Field>
      <Field id="model-identifier" label={t('identity')} help={t('identifierHelp')}><span className="search-field"><input id="model-identifier" className="mono" value={identifier} maxLength={200} onChange={event => setIdentifier(event.target.value)} /></span></Field>
      <div className="plan-actions"><button className="button" disabled={!identifier || !providerId} onClick={() => void run(() => backend.createModel({ providerId, identifier }), 'modelAdded').then(ok => ok && setIdentifier(''))}>{t('addModel')}</button></div>
      {message && <Notice tone={message.tone}>{message.text}</Notice>}
      <p className="field-help">{t('manualUnknown')}</p>
    </div>
  </section>;
}

function ProfileForm({ view }: { view: ModelsView }) {
  const { t } = useTranslation('models');
  const { message, run } = useSubmit();
  const [name, setName] = useState('');
  const [modelId, setModelId] = useState('');
  const [editing, setEditing] = useState('');
  return <section className="sheet" aria-labelledby="add-profile"><header className="sheet-head"><h3 id="add-profile" className="sheet-title">{t('saveProfile')}</h3><span className="kicker">{t('profileKicker')}</span></header>
    <div className="sheet-body form-stack">
      <Field id="profile-existing" label={t('profileTarget')} help={t('profileTargetHelp')}><Select id="profile-existing" value={editing} onChange={value => { setEditing(value); const p = view.profiles.find(x => x.id === value); if (p) { setName(p.name); setModelId(p.modelId); } }} options={[{ value: '', label: t('newProfile') }, ...view.profiles.map(p => ({ value: p.id, label: `${p.name} · ${t('agents:revisionShort', { revision: p.revision })}` }))]} /></Field>
      <Field id="profile-name" label={t('profileName')}><span className="search-field"><input id="profile-name" value={name} maxLength={80} onChange={event => setName(event.target.value)} /></span></Field>
      <Field id="profile-model" label={t('identity')}><Select id="profile-model" value={modelId} onChange={setModelId} options={[{ value: '', label: t('choose') }, ...view.models.map(m => ({ value: m.id, label: m.identifier }))]} /></Field>
      <div className="plan-actions"><button className="button" disabled={!name || !modelId} onClick={() => void run(() => backend.saveProfile({ id: editing || undefined, name, modelId }), editing ? 'profileRevised' : 'profileSaved')}>{t(editing ? 'saveRevision' : 'saveProfile')}</button></div>
      {message && <Notice tone={message.tone}>{message.text}</Notice>}
    </div>
  </section>;
}

function ProviderForm() {
  const { t } = useTranslation('models');
  const { message, run } = useSubmit();
  const [name, setName] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [authMode, setAuthMode] = useState('nexusSecret');
  const [envVar, setEnvVar] = useState('');
  return <section className="sheet" aria-labelledby="add-provider"><header className="sheet-head"><h3 id="add-provider" className="sheet-title">{t('addProvider')}</h3><span className="kicker">{t('addProviderKicker')}</span></header>
    <div className="sheet-body form-stack">
      <Field id="provider-name" label={t('providerName')}><span className="search-field"><input id="provider-name" value={name} maxLength={80} onChange={event => setName(event.target.value)} /></span></Field>
      <Field id="provider-url" label={t('baseUrl')} help={t('baseUrlHelp')}><span className="search-field"><input id="provider-url" className="mono" value={baseUrl} maxLength={2048} inputMode="url" onChange={event => setBaseUrl(event.target.value)} /></span></Field>
      <Field id="provider-auth" label={t('authMode')}><Select id="provider-auth" value={authMode} onChange={setAuthMode} options={['nexusSecret', 'environment'].map(v => ({ value: v, label: t(`auth.${v}`) }))} /></Field>
      {authMode === 'environment' && <Field id="provider-env" label={t('envVar')} help={t('envVarHelp')}><span className="search-field"><input id="provider-env" className="mono" value={envVar} maxLength={64} onChange={event => setEnvVar(event.target.value.toUpperCase())} /></span></Field>}
      <div className="plan-actions"><button className="button" disabled={!name || !baseUrl} onClick={() => void run(() => backend.createProvider({ name, kind: 'openaiCompatible', baseUrl, authMode, envVar: authMode === 'environment' ? envVar : undefined }), 'providerAdded')}>{t('addProvider')}</button></div>
      {message && <Notice tone={message.tone}>{message.text}</Notice>}
      <p className="field-help">{t('customProviderLimit')}</p>
    </div>
  </section>;
}

function SecretForm({ providerId }: { providerId: string }) {
  const { t } = useTranslation('models');
  const { message, run } = useSubmit();
  const [secret, setSecret] = useState('');
  // The value lives only in this transient input; it is cleared after submission and never read back.
  return <div className="secret-form">
    <span className="search-field"><input type="password" autoComplete="off" aria-label={t('secretLabel')} value={secret} onChange={event => setSecret(event.target.value)} /></span>
    <button className="button button-small" disabled={!secret} onClick={() => { const value = secret; setSecret(''); void run(() => backend.setProviderSecret(providerId, value), 'secretStored'); }}>{t('storeSecret')}</button>
    {message && <Notice tone={message.tone}>{message.text}</Notice>}
  </div>;
}

export function Models() {
  const { t } = useTranslation(['models', 'providers', 'agents', 'common', 'navigation']);
  const { locale } = usePreferences();
  const { mode, agents, models, reloadModels, status } = useNexus();
  const [view, setView] = useState<'catalog' | 'profiles' | 'providers'>('catalog');
  useEffect(() => { if (mode === 'live' && !models) void reloadModels(); }, [mode, models, reloadModels]);
  if (mode === 'live' && (status !== 'ready' || !models)) return <><PageHeading title={t('navigation:models')} description={t('description')} kicker={t('pageKickerLive')} /><DataState /></>;
  const live = mode === 'live' && models;
  const rows: ChainRow[] = live
    ? agents.map(agent => { const a = agent.report?.assignment; return { id: agent.id, name: agent.name, provider: a?.provider.displayName ?? null, model: a ? a.compatibility.nativeIdentifier ?? a.model.identifier : agent.modelId, profile: a?.profile ? { name: a.profile.name, revision: a.profile.revision } : null, source: t(`agents:${agent.assignment}`) }; })
    : fixtureAgents.map(agent => { const model = findModel(agent.modelId); const profile = findProfile(agent.profileId); return { id: agent.id, name: agent.name, provider: findProvider(model?.providerId)?.name ?? null, model: model?.identifier ?? null, profile: profile ? { name: t(profile.nameKey), revision: profile.revision } : null, source: t(`agents:${agent.assignment}`) }; });
  const counts = live ? { catalog: models.models.length, profiles: models.profiles.length, providers: models.providers.length } : { catalog: demoCatalog.models.length, profiles: demoCatalog.profiles.length, providers: demoCatalog.providers.length };
  const usedBy = (modelId: string): AgentId[] => live ? models.assignments.filter(a => a.model.id === modelId).map(a => a.assignment.agentId) : [];
  return <><PageHeading title={t('navigation:models')} description={t('description')} kicker={t(live ? 'pageKickerLive' : 'pageKicker')} />
    <Section title={t('mapTitle')} className="map-section" aside={<span className="kicker">{t('mapHelp')}</span>}><AssignmentChain rows={rows} /></Section>
    <Section title={t('catalogTitle')} className="catalog-section">
    <Tabs label={t('tabs')} value={view} onChange={setView} options={(['catalog', 'profiles', 'providers'] as const).map(value => ({ value, label: t(value), count: formatNumber(counts[value], locale) }))}>
      {view === 'catalog' && <><div className="sheet">
        <div className="ledger-head model-grid" aria-hidden="true"><span>{t('identity')}</span><span>{t('provider')}</span><span>{t('metadata')}</span><span>{t('usedBy')}</span></div>
        <ul className="model-list">{live ? models.models.map(model => { const provider = models.providers.find(p => p.id === model.providerId); const compatible = models.matrix.find(([id]) => id === model.id)?.[1].filter(c => c.support !== 'unsupported').map(c => c.agentId) ?? []; return <li key={model.id} className="model-grid">
          <div><p className="mono model-id">{model.identifier}</p><p className="row-meta">{t(`source.${model.source}`)}{model.sourceDetail && <><span className="note-divider" aria-hidden="true" /><span className="mono">{model.sourceDetail}</span></>}</p>{model.source === 'manual' && <button className="button button-small" onClick={() => void backend.deleteModel(model.id).then(reloadModels).catch(() => undefined)}>{t('remove')}</button>}</div>
          <div><span className="row-label">{t('provider')}</span><span lang="en-US">{provider?.displayName}</span></div>
          <div><span className="row-label">{t('metadata')}</span><span className="unknown-tag">{t('common:unknown')}</span><p className="row-meta">{t('compatibleWith')}</p><AgentLinks ids={compatible} /></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={usedBy(model.id)} /></div>
        </li>; }) : demoCatalog.models.map(model => <li key={model.id} className="model-grid">
          <div><p className="mono model-id">{model.identifier}</p><p className="row-meta">{t('common:demoSource')}</p></div>
          <div><span className="row-label">{t('provider')}</span><span lang="en-US">{findProvider(model.providerId)?.name}</span></div>
          <div><span className="row-label">{t('metadata')}</span>{model.capabilities ? <><span className="capabilities">{model.capabilities.map(key => <span className="chip" key={key}>{t(key)}</span>)}</span><p className="row-meta">{t('fixtureMetadata')}</p></> : <><span className="unknown-tag">{t('common:unknown')}</span><p className="row-meta">{t('unknownHelp')}</p></>}</div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={model.agents} /></div>
        </li>)}</ul>
      </div>{live ? <ModelForm view={models} /> : <Notice>{t('assignmentHelp')}</Notice>}</>}
      {view === 'profiles' && <><p className="view-intro">{t(live ? 'profileHelpLive' : 'profileHelp')}</p><div className="sheet">
        <div className="ledger-head profile-grid" aria-hidden="true"><span>{t('agents:profile')}</span><span>{t('profileIdentity')}</span><span>{t('identity')}</span><span>{t('usedBy')}</span></div>
        <ul className="profile-list">{live ? models.profiles.map(profile => { const model = models.models.find(m => m.id === profile.modelId); return <li key={profile.id} className="profile-grid">
          <div><p className="row-title">{profile.name}</p><p className="row-meta">{t('agents:revision', { revision: profile.revision })}</p></div>
          <div><span className="row-label">{t('profileIdentity')}</span><span className="mono">{profile.id}</span></div>
          <div><span className="row-label">{t('identity')}</span><span className="mono">{model?.identifier ?? profile.modelId}</span><p className="row-meta" lang="en-US">{models.providers.find(p => p.id === model?.providerId)?.displayName}</p></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={models.assignments.filter(a => a.profile?.id === profile.id).map(a => a.assignment.agentId)} /></div>
        </li>; }) : demoCatalog.profiles.map(profile => { const model = findModel(profile.modelId); return <li key={profile.id} className="profile-grid">
          <div><p className="row-title">{t(profile.nameKey)}</p><p className="row-meta">{t('agents:revision', { revision: profile.revision })}</p></div>
          <div><span className="row-label">{t('profileIdentity')}</span><span className="mono">{profile.id}</span></div>
          <div><span className="row-label">{t('identity')}</span><span className="mono">{model?.identifier}</span><p className="row-meta" lang="en-US">{findProvider(model?.providerId)?.name}</p></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={profile.agents} /></div>
        </li>; })}</ul>
      </div>{live ? <ProfileForm view={models} /> : <Notice>{t('assignmentHelp')}</Notice>}</>}
      {view === 'providers' && <><p className="view-intro">{t('providers:help')}</p><div className="sheet">
        <div className="ledger-head provider-grid" aria-hidden="true"><span>{t('provider')}</span><span>{t('providers:status')}</span><span>{t('providers:authentication')}</span><span>{t('providers:destination')}</span><span>{t('usedBy')}</span></div>
        <ul className="provider-list">{live ? models.providers.map(provider => <li key={provider.id} className="provider-grid">
          <div><p className="row-title" lang="en-US">{provider.displayName}</p><p className="row-meta">{t(provider.builtin ? 'providers:builtin' : 'providers:custom')}</p>{!provider.builtin && <button className="button button-small" onClick={() => void backend.deleteProvider(provider.id).then(reloadModels).catch(() => undefined)}>{t('remove')}</button>}</div>
          <div><span className="row-label">{t('providers:status')}</span><span className="connection connection-notConfigured"><span className="connection-dot" aria-hidden="true" />{t('providers:notTested')}</span></div>
          <div><span className="row-label">{t('providers:authentication')}</span>{t(`providers:${provider.authMode}`)}{provider.authMode === 'nexusSecret' && <p className="row-meta">{t(provider.secretPresent === true ? 'providers:secretStored' : provider.secretPresent === false ? 'providers:secretMissing' : 'providers:secretUnknown')}</p>}{provider.authMode === 'environment' && <p className="row-meta mono">{provider.envVar}</p>}{provider.authMode === 'nexusSecret' && <SecretForm providerId={provider.id} />}</div>
          <div><span className="row-label">{t('providers:destination')}</span><span className="mono">{provider.baseUrl ?? '—'}</span></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={models.assignments.filter(a => a.provider.id === provider.id).map(a => a.assignment.agentId)} /></div>
        </li>) : demoCatalog.providers.map(provider => <li key={provider.id} className="provider-grid">
          <div><p className="row-title" lang="en-US">{provider.name}</p></div>
          <div><span className="row-label">{t('providers:status')}</span><span className={`connection connection-${provider.state}`}><span className="connection-dot" aria-hidden="true" />{t(`providers:${provider.state}`)}</span></div>
          <div><span className="row-label">{t('providers:authentication')}</span>{t(`providers:${provider.authKey}`)}</div>
          <div><span className="row-label">{t('providers:destination')}</span><span className="mono">{provider.endpoint}</span></div>
          <div><span className="row-label">{t('usedBy')}</span><AgentLinks ids={provider.agents} /></div>
        </li>)}</ul>
      </div>{live ? <><ProviderForm /><Notice>{t('providers:noSecretsLive')}</Notice></> : <Notice>{t('providers:noSecrets')}</Notice>}</>}
    </Tabs>
    </Section>
  </>;
}
