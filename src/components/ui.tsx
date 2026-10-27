import { CheckCircle2, CircleX, TriangleAlert, Info, ArrowRight, Inbox } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useId, type ReactNode, type KeyboardEvent } from 'react';
import type { Readiness, AgentId } from '../data/view';
import { useNexus } from '../app/nexus-context';

// Readiness glyphs share the Nexus signal dot. Each state has its own shape so
// meaning survives without color: disc, half disc, diamond, dashed ring, slashed ring.
export function StateGlyph({ state, size = 10 }: { state: Readiness; size?: number }) {
  return <svg className={`state-glyph glyph-${state}`} width={size} height={size} viewBox="0 0 12 12" aria-hidden="true">
    {state === 'ready' && <circle cx="6" cy="6" r="5" fill="currentColor" />}
    {state === 'attention' && <><circle cx="6" cy="6" r="4.6" fill="none" stroke="currentColor" strokeWidth="1.6" /><path d="M6 1.4a4.6 4.6 0 0 0 0 9.2Z" fill="currentColor" /></>}
    {state === 'blocked' && <path d="M6 .6 11.4 6 6 11.4.6 6Z" fill="currentColor" />}
    {state === 'unknown' && <circle cx="6" cy="6" r="4.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeDasharray="2.2 1.6" />}
    {state === 'unsupported' && <><circle cx="6" cy="6" r="4.6" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M2.9 9.1 9.1 2.9" stroke="currentColor" strokeWidth="1.5" /></>}
  </svg>;
}
export function Status({ state }: { state: Readiness }) {
  const { t } = useTranslation('health');
  return <span className={`status status-${state}`}><StateGlyph state={state} /><span>{t(state)}</span></span>;
}
// The seal is reserved for an Agent's overall readiness — never for check results.
export function Seal({ state, size }: { state: Readiness; size?: 'lg' }) {
  const { t } = useTranslation('health');
  return <span className="seal" data-s={state} data-size={size}><StateGlyph state={state} size={size ? 10 : 9} /><span>{t(state)}</span></span>;
}
export function Kicker({ children }: { children: ReactNode }) {
  return <p className="kicker">{children}</p>;
}
export function PageHeading({ title, description, kicker, children }: { title: string; description?: string; kicker?: ReactNode; children?: ReactNode }) {
  return <header className="page-heading"><div className="page-heading-text">{kicker && <Kicker>{kicker}</Kicker>}<h1 tabIndex={-1}>{title}</h1>{description && <p className="deck">{description}</p>}</div>{children}</header>;
}
export function Section({ title, children, aside, className, kicker }: { title: string; children: ReactNode; aside?: ReactNode; className?: string; kicker?: string }) {
  const id = useId();
  return <section aria-labelledby={id} className={`section ${className ?? ''}`}><div className="section-heading"><h2 id={id}>{kicker && <span className="section-index" aria-hidden="true">{kicker}</span>}{title}</h2>{aside}</div>{children}</section>;
}
export function Notice({ children, tone = 'info' }: { children: ReactNode; tone?: 'info' | 'attention' | 'error' | 'success' }) {
  const Icon = tone === 'success' ? CheckCircle2 : tone === 'error' ? CircleX : tone === 'attention' ? TriangleAlert : Info;
  return <div className={`notice notice-${tone}`}><Icon size={14} aria-hidden="true" /><div>{children}</div></div>;
}
export function EmptyState({ title, description, children }: { title?: string; description?: string; children?: ReactNode }) {
  const { t } = useTranslation('common');
  return <div className="empty-state"><Inbox size={18} strokeWidth={1.5} aria-hidden="true" /><div><strong>{title ?? t('empty')}</strong><p>{description ?? t('emptyHelp')}</p>{children}</div></div>;
}
export function LoadingState() {
  const { t } = useTranslation('common');
  return <div role="status" aria-live="polite" className="loading-state"><p>{t('loading')}</p><div className="skeleton" /><div className="skeleton" /></div>;
}
export function ErrorState() {
  const { t } = useTranslation('common');
  const { mode } = useNexus();
  return <div className="error-state"><Notice tone="error"><strong>{t('errorTitle')}</strong><p>{t(mode === 'live' ? 'errorHelpLive' : 'errorHelp')}</p></Notice><a className="button" href="#/agents">{t('backAgents')}</a></div>;
}
export function DataState() {
  const { t } = useTranslation(['common', 'errors']);
  const { status, error, refresh, refreshing } = useNexus();
  if (status === 'loading') return <div role="status" aria-live="polite" className="loading-state"><p>{t('scanning')}</p><div className="skeleton" /><div className="skeleton" /></div>;
  if (status === 'error') return <div className="error-state"><Notice tone="error"><strong>{t('scanFailed')}</strong><p>{t(`errors:${error?.code ?? 'ipc.unavailable'}`, error?.params)}</p></Notice><button className="button" disabled={refreshing} onClick={() => void refresh()}>{t('retryScan')}</button></div>;
  return null;
}
export function InfoList({ items }: { items: { label: string; value: ReactNode }[] }) {
  return <dl className="info-list">{items.map(item => <div className="info-row" key={item.label}><dt>{item.label}</dt><dd>{item.value}</dd></div>)}</dl>;
}
export function AgentLinks({ ids }: { ids: readonly AgentId[] }) {
  const { t } = useTranslation('common');
  const { findAgent } = useNexus();
  return <span className="agent-links">{ids.length === 0 ? t('none') : ids.map(id => <a href={`#/agents/${id}`} key={id} lang="en-US">{findAgent(id)?.name ?? id}</a>)}</span>;
}
export function DetailLink({ agentId, name, attention = false, compact = false, primary = false, label }: { agentId: string; name: string; attention?: boolean; compact?: boolean; primary?: boolean; label?: string }) {
  const { t } = useTranslation('common');
  const visibleLabel = label ?? t('view');
  const accessibleName = compact ? t(attention ? 'reviewAgent' : 'viewAgent', { agent: name }) : t('agentAction', { action: visibleLabel, agent: name });
  if (!primary) return <a className={`detail-link ${compact ? 'is-compact' : ''}`} href={`#/agents/${agentId}`} aria-label={accessibleName} title={compact ? accessibleName : undefined}><span className={compact ? 'sr-only' : undefined}>{visibleLabel}</span><ArrowRight size={13} aria-hidden="true" /></a>;
  return <a className="button button-primary" href={`#/agents/${agentId}`} aria-label={accessibleName}><span>{visibleLabel}</span><ArrowRight size={14} aria-hidden="true" /></a>;
}
export function Tabs<T extends string>({ label, options, value, onChange, children }: { label: string; options: { value: T; label: string; count?: string }[]; value: T; onChange: (value: T) => void; children: ReactNode }) {
  const id = useId();
  function handleKey(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let next: number;
    if (event.key === 'ArrowRight') next = (index + 1) % options.length;
    else if (event.key === 'ArrowLeft') next = (index - 1 + options.length) % options.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = options.length - 1;
    else return;
    event.preventDefault();
    onChange(options[next].value);
    document.getElementById(`${id}-${options[next].value}`)?.focus();
  }
  return <><div className="tabs" role="tablist" aria-label={label}>{options.map((option, index) => <button role="tab" key={option.value} id={`${id}-${option.value}`} aria-selected={value === option.value} aria-controls={`${id}-panel`} tabIndex={value === option.value ? 0 : -1} onClick={() => onChange(option.value)} onKeyDown={event => handleKey(event, index)}>{option.label}{option.count && <span className="tab-count" aria-hidden="true">{option.count}</span>}</button>)}</div><div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-${value}`} tabIndex={0} className="tab-panel">{children}</div></>;
}
