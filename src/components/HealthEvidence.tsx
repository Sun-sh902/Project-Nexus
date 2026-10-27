import { useId } from 'react';
import { Check, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { type AgentFixture, fixtureTime } from '../data/fixtures';
import { usePreferences } from '../app/preference-context';
import { formatTimestamp } from '../app/format';

export function HealthCheckList({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation('agents');
  return <ul className="check-list">{agent.checks.map(check => <li key={check.key} className={`check-${check.result}`}>
    {check.result === 'pass' ? <Check className="success" size={14} strokeWidth={2} aria-hidden="true" /> : <X className="danger" size={14} strokeWidth={2} aria-hidden="true" />}
    <span>{t(check.key)}</span><span className="check-result mono">{t(check.result === 'pass' ? 'checkPass' : 'checkFail')}</span>
  </li>)}</ul>;
}

export function HealthEvidence({ agent }: { agent: AgentFixture }) {
  const { t } = useTranslation(['health', 'common']);
  const { locale } = usePreferences();
  const title = useId();
  return <section className="health-evidence sheet" aria-labelledby={title}>
    <header className="sheet-head"><h3 id={title} className="sheet-title">{t('evidenceTitle')}</h3><span className="kicker">{t('checksCount', { count: agent.checks.length })}</span></header>
    <div className="sheet-body">
      <p className="evidence-scope">{t('exampleScope')}</p>
      <HealthCheckList agent={agent} />
      <dl className="evidence-provenance">
        <div><dt>{t('time')}</dt><dd><time className="mono" dateTime={fixtureTime}>{formatTimestamp(fixtureTime, locale)}</time></dd></div>
        <div><dt>{t('source')}</dt><dd>{t('common:demoSource')}</dd></div>
      </dl>
    </div>
  </section>;
}

// What this example never assessed, drawn with the hatch that means "no conclusion".
export function NotChecked() {
  const { t } = useTranslation(['health', 'agents']);
  const title = useId();
  return <section className="sheet not-checked" aria-labelledby={title}>
    <header className="sheet-head"><h3 id={title} className="sheet-title">{t('notCheckedTitle')}</h3><span className="kicker">{t('hatchMeaning')}</span></header>
    <ul className="ledger-list">
      <li><strong>{t('lineOnline')}</strong><span>{t('remoteUnknown')}</span><span className="mono">{t('nodeUnchecked')}</span></li>
      <li><strong>{t('agents:auth')}</strong><span>{t('agents:agentOwned')}</span><span className="mono">{t('notRead')}</span></li>
      <li><strong>{t('deviceState')}</strong><span>{t('noDeviceScan')}</span><span className="mono">{t('notScanned')}</span></li>
    </ul>
  </section>;
}
