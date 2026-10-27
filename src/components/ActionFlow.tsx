import { useId } from 'react';
import { ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { pairs, type OperationResult, type PlanView } from '../app/backend';
import type { ActionFlow } from '../app/action-flow';
import { usePreferences } from '../app/preference-context';
import { formatTimestamp } from '../app/format';
import { InfoList, Notice } from './ui';


export function ActionPanel({ flow }: { flow: ActionFlow }) {
  const { t } = useTranslation(['actions', 'errors']);
  return <div className="action-panel" aria-live="polite">
    {flow.busy === 'preparing' && <Notice>{t('preparing')}</Notice>}
    {flow.error && <Notice tone={flow.error.code === 'confirmation.denied' ? 'attention' : 'error'}><strong>{t(`errors:${flow.error.code}`, flow.error.params)}</strong><p>{t(flow.error.code === 'confirmation.denied' ? 'deniedHelp' : 'errorHelp')}</p></Notice>}
    {flow.plan && <PlanSheet flow={flow} plan={flow.plan} />}
    {flow.result && <ResultSheet result={flow.result} />}
  </div>;
}

function PlanSheet({ flow, plan }: { flow: ActionFlow; plan: PlanView }) {
  const { t } = useTranslation(['actions', 'common']);
  const { locale } = usePreferences();
  const id = useId();
  const params = pairs(plan.params);
  const rows = [
    plan.config && { label: t('target'), value: <span className="mono">{plan.config.location}</span> },
    plan.process && { label: t('command'), value: <span className="mono">{[plan.process.program, ...plan.process.args].join(' ')}</span> },
    plan.process?.version && { label: t('version'), value: <span className="mono">{plan.process.version}</span> },
    params.model && { label: t('model'), value: <span className="mono">{params.model}</span> },
    { label: t('backupLabel'), value: t(`backup.${plan.backup}`) },
    { label: t('rollbackLabel'), value: t(`rollback.${plan.rollback}`) },
    { label: t('privilegesLabel'), value: t(`privileges.${plan.privileges}`) },
    plan.verification && { label: t('verificationLabel'), value: t(plan.verification) },
  ].filter(Boolean) as { label: string; value: React.ReactNode }[];
  return <section className="sheet plan-sheet" aria-labelledby={id}>
    <header className="sheet-head"><h3 id={id} className="sheet-title">{t(`kind.${plan.kind}`)}</h3><span className="kicker">{t('previewKicker', { time: formatTimestamp(plan.expiresAt, locale) })}</span></header>
    <div className="sheet-body">
      <InfoList items={rows} />
      {plan.config && plan.config.changes.length > 0 && <ul className="plan-changes">{plan.config.changes.map(change => <li key={change.key}><span className="mono">{change.key}</span><span className="mono is-empty">{change.before ?? t('absent')}</span><span aria-hidden="true">{'→'}</span><span className="mono">{change.after ?? t('removed')}</span></li>)}</ul>}
      {plan.config && plan.config.diff.length > 0 && <pre className="plan-diff mono" aria-label={t('diffLabel')}>{plan.config.diff.map(line => `${line.kind} ${line.text}`).join('\n')}</pre>}
      {plan.config?.createsFile && <Notice>{t('createsFile')}</Notice>}
      {plan.warnings.map(w => <Notice key={w} tone="attention">{t(w)}</Notice>)}
      {plan.preserves.length > 0 && <p className="plan-preserves"><span className="field-label">{t('preserves')}</span>{plan.preserves.map(p => t(p)).join(' · ')}</p>}
      <div className="plan-actions">
        <button className="button button-primary" disabled={flow.busy !== null} onClick={() => void flow.confirm()}><ShieldCheck size={14} aria-hidden="true" />{t(flow.busy === 'confirming' ? 'confirming' : 'confirm')}</button>
        <button className="button" disabled={flow.busy === 'confirming'} onClick={() => void flow.discard()}>{t('discard')}</button>
      </div>
      <p className="field-help">{t('nativeConfirmHelp')}</p>
    </div>
  </section>;
}

function ResultSheet({ result }: { result: OperationResult }) {
  const { t } = useTranslation(['actions', 'errors']);
  const id = useId();
  const tone = result.status === 'succeeded' ? 'success' : result.status === 'partial' || result.status === 'rolledBack' ? 'attention' : 'error';
  return <section className="sheet plan-sheet" aria-labelledby={id}>
    <header className="sheet-head"><h3 id={id} className="sheet-title">{t(`status.${result.status}`)}</h3><span className="kicker">{t(`kind.${result.kind}`)}</span></header>
    <div className="sheet-body">
      <Notice tone={tone}><strong>{t(`verification.${result.verification.outcome}`)}</strong>{result.error && <p>{t(`errors:${result.error.code}`, pairs(result.error.params))}</p>}{result.compensation && <p>{t(`compensation.${result.compensation}`)}</p>}</Notice>
      {result.verification.checks.length > 0 && <ul className="check-list">{result.verification.checks.map(check => <li key={check.key} className={`check-${check.outcome === 'passed' ? 'pass' : check.outcome === 'failed' ? 'fail' : 'unknown'}`}>
        <span>{t(check.key, pairs(check.params))}{check.informational && <span className="row-meta">{t('informational')}</span>}</span><span className="check-result mono">{t(`outcome.${check.outcome}`)}</span>
      </li>)}</ul>}
      {result.backupId && <p className="row-meta">{t('backupCreated')}<span className="mono"> {result.backupId}</span></p>}
      {result.outputExcerpt && <details><summary>{t('output')}</summary><pre className="plan-diff mono">{result.outputExcerpt}</pre></details>}
      {result.warnings.filter(w => w === 'warning.effectiveOverridden' || w === 'warning.nextSession').map(w => <Notice key={w}>{t(w)}</Notice>)}
    </div>
  </section>;
}
