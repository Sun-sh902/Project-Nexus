import { useEffect, useState } from 'react';
import { ArrowLeft, ArrowRight, ChevronDown, Play } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { backend, pairs, type AgentHistory, type Finding } from '../app/backend';
import { useNexus, useText } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber, formatTimestamp } from '../app/format';
import { findingNext, findingSummary, findingTitle, repairRequest } from '../data/live';
import type { AgentView } from '../data/view';
import { ConfigurationSheet } from '../components/AgentConfiguration';
import { HealthCheckList, HealthEvidence, NotChecked } from '../components/HealthEvidence';
import { ExtensionRows } from '../components/ExtensionRows';
import { ReadinessProcession } from '../components/ReadinessLine';
import { ActionPanel } from '../components/ActionFlow';
import { useActionFlow, type ActionFlow } from '../app/action-flow';
import { DataState, EmptyState, ErrorState, InfoList, Kicker, Notice, Seal, Section, Tabs } from '../components/ui';

type DetailView = 'overview' | 'model' | 'extensions' | 'configuration' | 'diagnostics';
const detailViews: readonly DetailView[] = ['overview', 'model', 'extensions', 'configuration', 'diagnostics'];

function Overview({ agent, onOpen, flow }: { agent: AgentView; onOpen: (view: DetailView) => void; flow: ActionFlow }) {
  const { t } = useTranslation(['agents', 'health', 'common', 'actions']);
  const text = useText();
  const top = agent.findings[0];
  const request = top?.repairs[0] ? repairRequest(agent, top.repairs[0]) : null;
  return <div className="detail-layout"><div className="overview-main">
    <section className="sheet" aria-labelledby="overview-config">
      <header className="sheet-head"><h2 id="overview-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header>
      <ConfigurationSheet agent={agent} />
    </section>
    <Section title={t('importantExtensions')} className="detail-extensions"><ExtensionRows items={agent.extensions.slice(0, 6)} /></Section>
  </div>
  <aside className="evidence-pane" aria-label={t('health:evidenceTitle')}><HealthEvidence agent={agent} /><NotChecked /></aside>
  <div className="next-step">
    <p><strong>{t('next')}</strong>{text(agent.next)}</p>
    {agent.report
      ? request ? <button className="button button-primary" disabled={flow.busy !== null} onClick={() => void flow.prepare(request)}>{t(`actions:repair.${top.repairs[0].action}`)}<ArrowRight size={14} aria-hidden="true" /></button>
        : <button className="button button-primary" onClick={() => onOpen(top ? 'diagnostics' : 'model')}>{t(top ? 'liveViewFinding' : 'viewModel')}<ArrowRight size={14} aria-hidden="true" /></button>
      : <button className="button button-primary" onClick={() => onOpen(agent.installed ? 'model' : 'diagnostics')}>{t(agent.id === 'claude' ? 'compareConfiguration' : agent.installed ? 'viewModel' : 'viewRequirement')}<ArrowRight size={14} aria-hidden="true" /></button>}
  </div>
  {agent.report && <div className="detail-actions"><ActionPanel flow={flow} /></div>}</div>;
}

function AssignmentEditor({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['agents', 'models', 'errors', 'common']);
  const { models, reloadModels, refresh } = useNexus();
  const current = agent.report?.assignment;
  const [mode, setMode] = useState<'native' | 'direct' | 'profile'>(current?.assignment.mode ?? 'native');
  const [modelId, setModelId] = useState(current?.assignment.modelId ?? '');
  const [profileId, setProfileId] = useState(current?.assignment.profileId ?? '');
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => { if (!models) void reloadModels(); }, [models, reloadModels]);
  const compat = (id: string) => models?.matrix.find(([mid]) => mid === id)?.[1].find(c => c.agentId === agent.id);
  const usable = (models?.models ?? []).filter(m => compat(m.id)?.support !== 'unsupported');
  async function save() {
    setMessage(null);
    try {
      await backend.setAssignment({ agentId: agent.id, mode, modelId: mode === 'direct' ? modelId : undefined, profileId: mode === 'profile' ? profileId : undefined });
      await refresh(); await reloadModels();
      setMessage(t('assignmentSaved'));
    } catch (caught) { setMessage(t(`errors:${(caught as { code?: string }).code ?? 'ipc.unavailable'}`)); }
  }
  const canWrite = agent.report?.snapshot.capabilities.find(c => c.key === 'model.write');
  return <section className="sheet" aria-labelledby="assign-title">
    <header className="sheet-head"><h2 id="assign-title" className="sheet-title">{t('assignTitle')}</h2><span className="kicker">{t('assignKicker')}</span></header>
    <div className="sheet-body form-stack">
      <div className="setting-row"><div><label className="setting-label" htmlFor="assign-mode">{t('source')}</label><p className="field-help">{t('assignModeHelp')}</p></div>
        <span className="select-control"><select id="assign-mode" value={mode} onChange={event => setMode(event.target.value as typeof mode)}>{(['native', 'direct', 'profile'] as const).map(m => <option key={m} value={m}>{t(m)}</option>)}</select><ChevronDown size={14} aria-hidden="true" /></span></div>
      {mode === 'direct' && <div className="setting-row"><div><label className="setting-label" htmlFor="assign-model">{t('models:identity')}</label><p className="field-help">{t('assignModelHelp')}</p></div>
        <span className="select-control"><select id="assign-model" value={modelId} onChange={event => setModelId(event.target.value)}><option value="">{t('choose')}</option>{usable.map(m => <option key={m.id} value={m.id}>{`${m.identifier} · ${t(`models:source.${m.source}`)}`}</option>)}</select><ChevronDown size={14} aria-hidden="true" /></span></div>}
      {mode === 'profile' && <div className="setting-row"><div><label className="setting-label" htmlFor="assign-profile">{t('profile')}</label><p className="field-help">{t('assignProfileHelp')}</p></div>
        <span className="select-control"><select id="assign-profile" value={profileId} onChange={event => setProfileId(event.target.value)}><option value="">{t('choose')}</option>{(models?.profiles ?? []).map(p => <option key={p.id} value={p.id}>{`${p.name} · ${t('revisionShort', { revision: p.revision })}`}</option>)}</select><ChevronDown size={14} aria-hidden="true" /></span></div>}
      <div className="plan-actions">
        <button className="button" disabled={(mode === 'direct' && !modelId) || (mode === 'profile' && !profileId)} onClick={() => void save()}>{t('saveIntent')}</button>
        {current && <button className="button button-primary" disabled={flow.busy !== null || canWrite?.support === 'unsupported'} onClick={() => void flow.prepare({ kind: 'applyModel', agentId: agent.id })}>{t('previewApply')}</button>}
      </div>
      {canWrite?.support === 'unsupported' && <Notice tone="attention">{t(`capability:${canWrite.reason ?? 'capability.unsupported'}`)}</Notice>}
      {current && current.compatibility.reason && <Notice tone={current.compatibility.support === 'unsupported' ? 'error' : 'attention'}>{t(`models:${current.compatibility.reason}`)}</Notice>}
      {message && <p className="preference-feedback" role="status">{message}</p>}
      <p className="field-help">{t('assignBoundary')}</p>
    </div>
  </section>;
}

function ModelLayers({ agent }: { agent: AgentView }) {
  const { t } = useTranslation(['agents']);
  const layers = agent.report?.snapshot.model.layers ?? [];
  return <Section title={t('layersTitle')} aside={<span className="kicker">{t('layersKicker')}</span>}>
    <div className="sheet"><ul className="ledger-list layer-list">{layers.map((layer, index) => <li key={`${layer.location}-${index}`} className={layer.effective ? 'is-effective' : undefined}>
      <strong>{t(`liveSource.${layer.kind}`)}</strong>
      <span className="mono">{layer.location}</span>
      <span className="mono">{layer.uncertain ? t('layerUncertain') : layer.value ?? t('layerUnset')}{layer.effective && <span className="chip">{t('layerEffective')}</span>}</span>
    </li>)}</ul><p className="sheet-foot">{t('layersFoot')}</p></div>
  </Section>;
}

function ModelArea({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['agents', 'common']);
  return <div className="model-area"><p className="view-intro">{t(agent.report ? 'modelHelpLive' : 'modelHelp')}</p>
    <section className="sheet" aria-labelledby="model-config"><header className="sheet-head"><h2 id="model-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header><ConfigurationSheet agent={agent} /></section>
    {agent.report ? <><ModelLayers agent={agent} /><AssignmentEditor agent={agent} flow={flow} /><ActionPanel flow={flow} /></>
      : <><Notice>{t('recordLimit')}</Notice><Notice>{t('common:previewOnly')}</Notice></>}
  </div>;
}

function ExtensionsArea({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['extensions', 'capability']);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const caps = agent.report?.snapshot.capabilities ?? [];
  const add = caps.find(c => c.key === 'mcp.addRemote');
  const managed = new Set(agent.report?.snapshot.extensions.mcp.filter(m => m.nexusManaged).map(m => m.name));
  return <div className="extensions-area">
    <ExtensionRows items={agent.extensions} actions={item => {
      const server = item.live?.server;
      if (!server) return null;
      return <div className="row-actions">
        {server.canToggle && server.enabled !== null && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'setMcpEnabled', agentId: agent.id, name: server.name, enabled: !server.enabled })}>{t(server.enabled ? 'liveDisable' : 'liveEnable')}</button>}
        {managed.has(server.name) && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'removeManagedMcp', agentId: agent.id, name: server.name })}>{t('liveRemoveManaged')}</button>}
      </div>;
    }} />
    {agent.report && <section className="sheet add-mcp" aria-labelledby="add-mcp-title">
      <header className="sheet-head"><h2 id="add-mcp-title" className="sheet-title">{t('liveAddTitle')}</h2><span className="kicker">{t('liveAddKicker')}</span></header>
      <div className="sheet-body form-stack">
        {add?.support === 'unsupported' ? <Notice>{t(`capability:${add.reason ?? 'capability.unsupported'}`)}</Notice> : <>
          <div className="setting-row"><div><label className="setting-label" htmlFor="mcp-name">{t('liveAddName')}</label><p className="field-help">{t('liveAddNameHelp')}</p></div><span className="search-field"><input id="mcp-name" value={name} maxLength={64} onChange={event => setName(event.target.value)} /></span></div>
          <div className="setting-row"><div><label className="setting-label" htmlFor="mcp-url">{t('liveAddUrl')}</label><p className="field-help">{t('liveAddUrlHelp')}</p></div><span className="search-field"><input id="mcp-url" value={url} maxLength={2048} inputMode="url" onChange={event => setUrl(event.target.value)} /></span></div>
          <div className="plan-actions"><button className="button" disabled={!name || !url || flow.busy !== null} onClick={() => void flow.prepare({ kind: 'addRemoteMcp', agentId: agent.id, name, url })}>{t('liveAddPreview')}</button></div>
          <p className="field-help">{t('liveStdioNotice')}</p>
        </>}
      </div>
    </section>}
    {agent.report && <ActionPanel flow={flow} />}
  </div>;
}

function Installations({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['agents', 'actions', 'errors', 'capability']);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const r = agent.report!;
  const options = r.lifecycle;
  const launchCap = r.snapshot.capabilities.find(c => c.key === 'launch');
  return <Section title={t('installationsTitle')} aside={<span className="kicker">{t('installationsKicker', { count: r.snapshot.installations.length })}</span>}>
    {r.snapshot.installations.length === 0 ? <EmptyState title={t('noInstallations')} description={t('noInstallationsHelp')}>
      <div className="row-actions">{options.filter(o => o.op === 'install').map(o => <button key={o.recipeId} className="button" disabled={!o.available || flow.busy !== null} onClick={() => void flow.prepare({ kind: 'install', agentId: agent.id, recipeId: o.recipeId })}>{t('installWith', { package: o.package })}</button>)}</div>
      {options.some(o => o.op === 'install' && !o.available) && <p className="row-meta">{t('installToolMissing')}</p>}
    </EmptyState> : <div className="sheet"><ul className="ledger-list install-list">{r.snapshot.installations.map((inst, index) => <li key={inst.id}>
      <div><p className="row-title mono">{inst.executable}</p>{inst.resolved && <p className="row-meta mono">{`→ ${inst.resolved}`}</p>}
        <p className="row-meta">{t(`installSource.${inst.source}`)}<span className="note-divider" aria-hidden="true" />{inst.version.value ? <span className="mono">{inst.version.value}</span> : t(`obs.${inst.version.status}`)}<span className="note-divider" aria-hidden="true" />{t(`evidenceSource.${inst.version.source}`)}{index === r.snapshot.primary && <><span className="note-divider" aria-hidden="true" />{t('primaryInstall')}</>}{inst.nexusManaged && <><span className="note-divider" aria-hidden="true" />{t('nexusManaged')}</>}</p></div>
      <div className="row-actions">
        {launchCap?.support !== 'unsupported' && <button className="button button-small" onClick={() => { setLaunchError(null); backend.launch(agent.id, inst.id).catch(e => setLaunchError(t(`errors:${e.code ?? 'ipc.unavailable'}`))); }}><Play size={12} aria-hidden="true" />{t('launch')}</button>}
        {options.filter(o => o.installationId === inst.id).map(o => <button key={o.op} className="button button-small" disabled={!o.available || flow.busy !== null} onClick={() => void flow.prepare({ kind: o.op as 'update' | 'uninstall', agentId: agent.id, installationId: inst.id })}>{t(`op.${o.op}`)}</button>)}
      </div>
    </li>)}</ul></div>}
    {launchError && <Notice tone="error">{launchError}</Notice>}
    {r.snapshot.capabilities.filter(c => c.key.startsWith('lifecycle.') && c.support === 'unsupported' && c.reason).map(c => <Notice key={c.key}>{t(`capability:${c.reason}`)}</Notice>)}
  </Section>;
}

function Configuration({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['agents', 'common', 'actions', 'capability']);
  const { locale } = usePreferences();
  const [history, setHistory] = useState<AgentHistory | null>(null);
  const r = agent.report;
  useEffect(() => { if (r) void backend.agentHistory(agent.id).then(setHistory).catch(() => setHistory(null)); }, [r, agent.id, flow.result]);
  if (!r) return <div className="configuration-area"><p className="view-intro">{t('configurationHelp')}</p>
    {agent.installed ? <><section className="sheet" aria-labelledby="config-config"><header className="sheet-head"><h2 id="config-config" className="sheet-title">{t('trackTitle')}</h2><span className="kicker">{t('trackKicker')}</span></header><ConfigurationSheet agent={agent} help={false} /></section>
      <Section title={t('configSource')}><InfoList items={[
        { label: t('configSource'), value: t('configOrigin') },
        { label: t('configScope'), value: t('common:localScope') },
        { label: t('source'), value: t(agent.assignment) },
        { label: t('auth'), value: t('agentOwned') },
      ]} /></Section></> : <EmptyState />}
    <Notice>{t('recordLimit')}</Notice><Notice>{t('common:previewOnly')}</Notice>
  </div>;
  return <div className="configuration-area"><p className="view-intro">{t('configurationHelpLive')}</p>
    <Installations agent={agent} flow={flow} />
    <Section title={t('configSourcesTitle')}><div className="sheet"><ul className="ledger-list">{r.snapshot.configSources.map(source => <li key={source.id}>
      <strong>{t(`scope.${source.scope}`)}</strong><span className="mono">{source.location}</span><span className="mono">{t(`obs.${source.status}`)}{source.viaSymlink && ` · ${t('viaSymlink')}`}</span>
    </li>)}</ul></div></Section>
    <Section title={t('authTitle')}><InfoList items={[
      { label: t('auth'), value: r.snapshot.auth.evidence.length ? r.snapshot.auth.evidence.map(e => t(`authEvidence.${e}`)).join(' · ') : t('authNone') },
      { label: t('authLocations'), value: r.snapshot.auth.locations.length ? <span className="mono">{r.snapshot.auth.locations.join(', ')}</span> : t('common:none') },
    ]} /><Notice>{t('authBoundary')}</Notice></Section>
    <Section title={t('capabilitiesTitle')}><div className="sheet"><ul className="ledger-list">{r.snapshot.capabilities.map(c => <li key={c.key}><strong>{t(`capabilityName.${c.key}`)}</strong><span>{t(`support.${c.support}`)}</span><span className="row-meta">{c.reason ? t(`capability:${c.reason}`) : ''}</span></li>)}</ul></div></Section>
    <Section title={t('backupsTitle')} aside={<span className="kicker">{t('backupsKicker')}</span>}>
      {history && history.backups.length ? <div className="sheet"><ul className="ledger-list">{history.backups.map(b => <li key={b.id}>
        <strong className="mono">{b.targetDisplay}</strong><time className="mono" dateTime={b.createdAt}>{formatTimestamp(b.createdAt, locale)}</time>
        <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'restoreBackup', backupId: b.id })}>{t('actions:repair.restoreBackup')}</button>
      </li>)}</ul></div> : <EmptyState title={t('noBackups')} description={t('noBackupsHelp')} />}
    </Section>
    <ActionPanel flow={flow} />
  </div>;
}

function FindingSheet({ agent, finding, flow }: { agent: AgentView; finding: Finding; flow: ActionFlow }) {
  const { t } = useTranslation(['diagnostics', 'agents', 'actions', 'findings']);
  const text = useText();
  const { locale } = usePreferences();
  const params = pairs(finding.params);
  const state = finding.severity === 'blocking' ? 'blocked' : finding.severity === 'unknown' ? 'unknown' : 'attention';
  return <section className={`sheet finding state-${state}`} aria-labelledby={`finding-${finding.id}`}>
    <div className="finding-summary"><Seal state={state} /><h2 id={`finding-${finding.id}`}>{text(findingTitle(finding))}</h2><p><span className="field-label">{t('impact')}</span>{text(findingSummary(finding))}</p></div>
    <dl className="comparison">
      <div><dt className="kicker">{t('observed')}</dt><dd className="mono">{t(`findings:${finding.code}.observed`, params)}</dd></div>
      <div className="comparison-op" aria-hidden="true"><svg width="12" height="12" viewBox="0 0 12 12"><path d="M2.5 4.4h7M2.5 7.6h7M8 1.8 4 10.2" stroke="currentColor" strokeWidth="1.4" /></svg></div>
      <div><dt className="kicker">{t('expected')}</dt><dd className="mono">{t(`findings:${finding.code}.expected`, params)}</dd></div>
    </dl>
    <InfoList items={[
      { label: t('cause'), value: t(`cause.${finding.cause}`) },
      { label: t('agents:next'), value: text(findingNext(finding)) },
      { label: t('freshness'), value: <time className="mono" dateTime={finding.observedAt}>{formatTimestamp(finding.observedAt, locale)}{finding.stale ? ` · ${t('stale')}` : ''}</time> },
      ...(finding.evidence.length ? [{ label: t('evidenceLive'), value: <span className="mono">{finding.evidence.join(', ')}</span> }] : []),
    ]} />
    {finding.repairs.length > 0 && <div className="plan-actions">{finding.repairs.map((repair, i) => { const request = repairRequest(agent, repair); return request && <button key={i} className="button button-primary" disabled={flow.busy !== null} onClick={() => void flow.prepare(request)}>{t(`actions:repair.${repair.action}`)}</button>; })}</div>}
  </section>;
}

function Diagnostics({ agent, flow }: { agent: AgentView; flow: ActionFlow }) {
  const { t } = useTranslation(['diagnostics', 'agents', 'common']);
  const text = useText();
  if (agent.report) {
    return <div className="diagnostics-area"><p className="view-intro">{t('helpLive')}</p>
      {agent.findings.length ? agent.findings.map(f => <FindingSheet key={f.id} agent={agent} finding={f} flow={flow} />) : <Notice tone="success"><strong>{t('noFindingsLive')}</strong><p>{t('noFindingsHelpLive')}</p></Notice>}
      <ActionPanel flow={flow} />
      <details><summary>{t('agents:liveAllChecks')}</summary><HealthCheckList agent={agent} /></details>
      <details><summary>{t('evidenceLive')}</summary><p>{t('evidenceHelpLive')}</p><HealthEvidence agent={agent} /></details>
    </div>;
  }
  const finding = agent.readiness === 'attention' || agent.readiness === 'blocked';
  return <div className="diagnostics-area"><p className="view-intro">{t('help')}</p>{finding ? <section className={`sheet finding state-${agent.readiness}`} aria-labelledby="finding-title">
    <div className="finding-summary"><Seal state={agent.readiness} /><h2 id="finding-title">{text(agent.headline)}</h2><p><span className="field-label">{t('impact')}</span>{t(`${agent.id}Impact`)}</p></div>
    <dl className="comparison">
      <div><dt className="kicker">{t('observed')}</dt><dd className="mono">{t(`${agent.id}Observed`)}</dd></div>
      <div className="comparison-op" aria-hidden="true"><svg width="12" height="12" viewBox="0 0 12 12"><path d="M2.5 4.4h7M2.5 7.6h7M8 1.8 4 10.2" stroke="currentColor" strokeWidth="1.4" /></svg></div>
      <div><dt className="kicker">{t('expected')}</dt><dd className="mono">{t(`${agent.id}Expected`)}</dd></div>
    </dl>
    <InfoList items={[
      { label: t('cause'), value: t('established') },
      { label: t('agents:next'), value: text(agent.next) },
    ]} />
  </section> : <Notice tone="success"><strong>{t('noFindings')}</strong><p>{t('noFindingsHelp')}</p></Notice>}
    <details><summary>{t(agent.checks.some(check => check.state === 'fail') ? 'agents:exampleChecks' : 'agents:passed')}</summary><HealthCheckList agent={agent} /></details>
    <details><summary>{t('evidence')}</summary><p>{t('evidenceHelp')}</p><HealthEvidence agent={agent} /></details><Notice>{t('common:previewOnly')}</Notice>
  </div>;
}

export function AgentDetail({ id }: { id: string }) {
  const { t } = useTranslation(['agents', 'common', 'health']);
  const { locale } = usePreferences();
  const { findAgent, status, mode } = useNexus();
  const text = useText();
  const flow = useActionFlow();
  const [view, setView] = useState<DetailView>('overview');
  if (status !== 'ready') return <DataState />;
  const agent = findAgent(id);
  if (!agent) return <ErrorState />;
  const primary = agent.report?.snapshot.primary != null ? agent.report.snapshot.installations[agent.report.snapshot.primary] : undefined;
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
          <div className="hero-seal"><Seal state={agent.readiness} size="lg" /><Kicker>{t(mode === 'live' ? 'detailKickerLive' : 'detailKicker')}</Kicker></div>
          <h1 tabIndex={-1} lang="en-US">{agent.name}</h1>
          <p className="deck"><span className="lead">{text(agent.headline)}{t('common:sentenceEnd')}</span><span className="rest">{text(agent.summary)}</span></p>
        </div>
        <dl className="fact-register">
          <div><dt>{t('vendor')}</dt><dd lang="en-US">{agent.vendor}</dd></div>
          <div><dt>{t('installation')}</dt><dd>{primary ? t(`installSource.${primary.source}`) : t(agent.installed ? 'common:installed' : 'common:notInstalled')}</dd></div>
          <div><dt>{t('version')}</dt><dd className="mono">{agent.version ?? '—'}</dd></div>
          <div><dt>{t('configScope')}</dt><dd>{t('common:localScope')}</dd></div>
          <div><dt>{t('currentModel')}</dt><dd className={agent.modelId ? 'mono' : 'is-empty'}>{agent.modelId ?? t(agent.report && agent.installed ? 'liveAgentDefault' : 'common:notConfigured')}</dd></div>
          <div><dt>{t('source')}</dt><dd>{agent.report ? text(agent.modelSource) : t(agent.assignment)}</dd></div>
          <div><dt>{t(agent.report ? 'scannedAt' : 'provider')}</dt><dd lang="en-US">{agent.report ? <time className="mono" dateTime={agent.observedAt}>{formatTimestamp(agent.observedAt, locale)}</time> : agent.providerName ?? t('common:none')}</dd></div>
        </dl>
      </header>
      <ReadinessProcession agent={agent} />
      <Tabs label={t('detailTabs')} value={view} onChange={setView} options={detailViews.map(value => ({ value, label: t(value === 'model' ? 'modelTab' : value), count: value === 'extensions' ? formatNumber(agent.extensions.length, locale) : value === 'diagnostics' && agent.report ? formatNumber(agent.findings.length, locale) : undefined }))}>
        {view === 'overview' && <Overview agent={agent} onOpen={openView} flow={flow} />}
        {view === 'model' && <ModelArea agent={agent} flow={flow} />}
        {view === 'extensions' && <ExtensionsArea agent={agent} flow={flow} />}
        {view === 'configuration' && <Configuration agent={agent} flow={flow} />}
        {view === 'diagnostics' && <Diagnostics agent={agent} flow={flow} />}
      </Tabs>
    </article>
  </div>;
}
