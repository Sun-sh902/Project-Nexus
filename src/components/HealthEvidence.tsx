import { useId } from 'react';
import { Check, CircleDashed, Minus, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { AgentView } from '../data/view';
import { usePreferences } from '../app/preference-context';
import { useNexus, useText } from '../app/nexus-context';
import { formatTimestamp } from '../app/format';

export function HealthCheckList({ agent }: { agent: AgentView }) {
  const text = useText();
  return <ul className="check-list">{agent.checks.map(check => <li key={check.id} className={`check-${check.state === 'pass' ? 'pass' : check.state === 'fail' ? 'fail' : 'unknown'}`}>
    {check.state === 'pass' ? <Check className="success" size={14} strokeWidth={2} aria-hidden="true" /> : check.state === 'fail' ? <X className="danger" size={14} strokeWidth={2} aria-hidden="true" /> : check.state === 'unknown' ? <CircleDashed size={14} strokeWidth={2} aria-hidden="true" /> : <Minus size={14} strokeWidth={2} aria-hidden="true" />}
    <span>{text(check.label)}</span><span className="check-result mono">{text(check.result)}</span>
  </li>)}</ul>;
}

export function HealthEvidence({ agent }: { agent: AgentView }) {
  const { t } = useTranslation(['health', 'common']);
  const { locale } = usePreferences();
  const { mode } = useNexus();
  const title = useId();
  return <section className="health-evidence sheet" aria-labelledby={title}>
    <header className="sheet-head"><h3 id={title} className="sheet-title">{t('evidenceTitle')}</h3><span className="kicker">{t('checksCount', { count: agent.checks.length })}</span></header>
    <div className="sheet-body">
      <p className="evidence-scope">{t(mode === 'live' ? agent.stale ? 'liveScopeStale' : 'liveScope' : 'exampleScope')}</p>
      <HealthCheckList agent={agent} />
      <dl className="evidence-provenance">
        <div><dt>{t(mode === 'live' ? 'liveTime' : 'time')}</dt><dd><time className="mono" dateTime={agent.observedAt}>{formatTimestamp(agent.observedAt, locale)}</time></dd></div>
        <div><dt>{t('source')}</dt><dd>{t(mode === 'live' ? 'liveSource' : 'common:demoSource')}</dd></div>
      </dl>
    </div>
  </section>;
}

// What was never assessed, drawn with the hatch that means "no conclusion".
export function NotChecked() {
  const { t } = useTranslation(['health', 'agents']);
  const { mode } = useNexus();
  const title = useId();
  return <section className="sheet not-checked" aria-labelledby={title}>
    <header className="sheet-head"><h3 id={title} className="sheet-title">{t(mode === 'live' ? 'notCheckedTitleLive' : 'notCheckedTitle')}</h3><span className="kicker">{t('hatchMeaning')}</span></header>
    <ul className="ledger-list">
      <li><strong>{t('lineOnline')}</strong><span>{t('remoteUnknown')}</span><span className="mono">{t('nodeUnchecked')}</span></li>
      <li><strong>{t('agents:auth')}</strong><span>{t('agents:agentOwned')}</span><span className="mono">{t('notRead')}</span></li>
      {mode === 'live'
        ? <li><strong>{t('credentialValidity')}</strong><span>{t('credentialValidityHelp')}</span><span className="mono">{t('nodeUnchecked')}</span></li>
        : <li><strong>{t('deviceState')}</strong><span>{t('noDeviceScan')}</span><span className="mono">{t('notScanned')}</span></li>}
    </ul>
  </section>;
}
