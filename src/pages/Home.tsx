import { useId } from 'react';
import { ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Readiness } from '../data/view';
import { useNexus, useText } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber, formatTimestamp } from '../app/format';
import { AgentRows } from '../components/AgentRows';
import { DataState, DetailLink, Kicker, Notice, Section, StateGlyph, Status } from '../components/ui';

const registerStates: readonly Readiness[] = ['ready', 'attention', 'blocked', 'unknown'];
const severity: Record<Readiness, number> = { blocked: 0, attention: 1, unknown: 2, unsupported: 3, ready: 4 };
const numerals = ['I', 'II', 'III', 'IV'];

export function Home() {
  const { t } = useTranslation(['home', 'agents', 'health', 'common', 'system']);
  const { locale } = usePreferences();
  const { agents, tools, activity, scannedAt, mode, status, overview } = useNexus();
  const text = useText();
  const registerTitle = useId();
  if (status !== 'ready') return <DataState />;
  const live = mode === 'live';
  const attentionItems = agents.filter(agent => agent.readiness !== 'ready').sort((a, b) => severity[a.readiness] - severity[b.readiness]);
  const count = (state: Readiness) => agents.filter(agent => agent.readiness === state).length;
  const ready = count('ready');
  const time = scannedAt ?? '';
  const os = live && overview ? `${t(`system:osName.${overview.system.os}`)}${overview.system.osVersion.value ? ` ${overview.system.osVersion.value}` : ''}` : 'macOS';
  const arch = live && overview ? overview.system.arch : 'arm64';

  return <div className="home-page">
    <header className="hero">
      <div className="hero-text">
        <Kicker>{t(live ? 'heroKickerLive' : 'heroKicker')}<span className="note-divider" aria-hidden="true" /><time className="mono" dateTime={time}>{formatTimestamp(time, locale)}</time></Kicker>
        <h1 tabIndex={-1}><span className="lead">{t('headlineLead', { count: agents.length, ready: formatNumber(ready, locale), total: formatNumber(agents.length, locale) })}</span><span className="rest">{live ? attentionItems.length ? t('headlineRestLive', { agent: attentionItems[0].name, finding: text(attentionItems[0].headline) }) : t('headlineRestLiveClear') : t('headlineRest')}</span></h1>
        <p className="lede">{t(live ? 'ledeLive' : 'lede')}</p>
      </div>
      <section className="register" aria-labelledby={registerTitle}>
        <h2 id={registerTitle} className="kicker register-title">{t('registerTitle')}<span className="note-divider" aria-hidden="true" />{t('common:count', { count: agents.length })}</h2>
        <dl className="register-rows">{registerStates.map(state => <div key={state} className={`register-row state-${state}`}>
          <dt><StateGlyph state={state} />{t(`health:${state}`)}</dt><dd className="numeral">{formatNumber(count(state), locale)}</dd>
        </div>)}</dl>
        <p className="register-note">{t(live ? 'registerNoteLive' : 'registerNote')}</p>
      </section>
    </header>

    <Section title={t('myAgents')} className="home-agents" aside={<a className="text-action" href="#/agents">{t('allAgents')}<ArrowRight size={13} aria-hidden="true" /></a>}>
      <AgentRows agents={agents} />
    </Section>

    <Section title={t('attention')} className="home-attention" aside={<span className="kicker">{t('exhibitsNote')}</span>}>
      <div className="attention-grid">
        {attentionItems.length ? <ol className="sheet exhibits">{attentionItems.slice(0, 4).map((agent, index) => <li key={agent.id} className={`exhibit state-${agent.readiness}`}>
          <span className="exhibit-numeral" aria-hidden="true">{numerals[index]}</span>
          <div className="exhibit-body">
            <div className="exhibit-head"><h3>{text(agent.headline)}</h3><Status state={agent.readiness} /></div>
            <p className="exhibit-context"><span lang="en-US" className="exhibit-agent">{agent.name}</span><span className="note-divider" aria-hidden="true" />{text(agent.context)}</p>
            <div className="exhibit-next"><p><span className="field-label">{t('agents:next')}</span>{text(agent.next)}</p><DetailLink agentId={agent.id} name={agent.name} attention label={t('openOverview')} /></div>
          </div>
        </li>)}</ol> : <Notice tone="success"><strong>{t('allClear')}</strong><p>{t('allClearHelp')}</p></Notice>}
        <div className="support-column">
          <section className="sheet" aria-labelledby={`${registerTitle}-system`}>
            <header className="sheet-head"><h3 id={`${registerTitle}-system`} className="sheet-title">{t('systemTitle')}</h3><a className="text-action" href="#/system">{os}<span className="note-divider" aria-hidden="true" /><span className="mono">{arch}</span><ArrowRight size={13} aria-hidden="true" /></a></header>
            <ul className="tool-register">{tools.map(tool => <li key={tool.key}><StateGlyph state={tool.readiness} /><span lang="en-US">{tool.name}</span><span className={tool.version ? 'mono' : 'mono is-empty'}>{tool.version ?? text(tool.status)}</span></li>)}</ul>
            <p className="sheet-foot">{t(live ? 'systemImpactLive' : 'systemImpact')}</p>
          </section>
          <section className="sheet" aria-labelledby={`${registerTitle}-activity`}>
            <header className="sheet-head"><h3 id={`${registerTitle}-activity`} className="sheet-title">{t('activity')}</h3><span className="kicker">{t(live ? 'activityKickerLive' : 'activityKicker')}</span></header>
            {activity.length ? <ul className="activity-list">{activity.map(item => <li key={item.id}><span>{text(item.text)}</span><time className="mono" dateTime={item.time}>{formatTimestamp(item.time, locale)}</time></li>)}</ul> : <p className="sheet-body row-meta">{t('activityEmpty')}</p>}
            <p className="sheet-foot">{t(live ? 'activityNoteLive' : 'activityNote')}</p>
          </section>
        </div>
      </div>
    </Section>

    <section className="state-reference" aria-labelledby={`${registerTitle}-states`}>
      <h2 id={`${registerTitle}-states`} className="kicker">{t('statesTitle')}</h2>
      <ul>{(['ready', 'attention', 'blocked', 'unknown', 'unsupported'] as const).map(state => <li key={state}><p className={`state-name state-${state}`}><StateGlyph state={state} size={11} />{t(`health:${state}`)}</p><p>{t(`health:${state}Help`)}</p></li>)}</ul>
    </section>
  </div>;
}
