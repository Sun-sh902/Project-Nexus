import { useTranslation } from 'react-i18next';
import type { AgentView } from '../data/view';
import { useText } from '../app/nexus-context';

type Op = 'match' | 'differ' | 'none' | 'separate';
function Operator({ op }: { op: Op }) {
  const { t } = useTranslation('agents');
  const label = t({ match: 'opMatch', differ: 'opDiffer', none: 'opNone', separate: 'opSeparate' }[op]);
  return <div className="cs-op" data-op={op}>
    <span className="cs-op-mark" aria-hidden="true"><svg width="12" height="12" viewBox="0 0 12 12">
      {op === 'match' && <path d="M2.5 4.4h7M2.5 7.6h7" stroke="currentColor" strokeWidth="1.4" />}
      {op === 'differ' && <path d="M2.5 4.4h7M2.5 7.6h7M8 1.8 4 10.2" stroke="currentColor" strokeWidth="1.4" />}
      {op === 'none' && <path d="M3 6h6" stroke="currentColor" strokeWidth="1.4" />}
      {op === 'separate' && <path d="M1.8 6h3M7.2 6h3" stroke="currentColor" strokeWidth="1.4" />}
    </svg></span>
    <span className="cs-op-label">{label}</span>
  </div>;
}

// Desired → applied → effective are three separate records. Operators only compare what is
// recorded: a pending revision differs; an effective model owned by the Agent is a separate source.
export function ConfigurationSheet({ agent, help = true }: { agent: AgentView; help?: boolean }) {
  const { t } = useTranslation(['agents', 'models', 'common']);
  const text = useText();
  const hasProfile = agent.profileName !== null && agent.desiredRevision !== null;
  const assigned = agent.assignment !== 'native';
  const recordOp: Op = hasProfile || assigned ? agent.pending ? 'differ' : 'match' : 'none';
  const effectiveOp: Op = assigned && !agent.pending && agent.report?.assignment?.assignment.applied?.identifier === agent.modelId ? 'match' : agent.assignment === 'profile' && hasProfile && !agent.report ? 'match' : 'separate';
  const applied = agent.report?.assignment?.assignment.applied;
  return <div className="config-sheet">
    <div className="cs-grid">
      <div className="cs-col station station-desired">
        <p className="cs-kicker">{t('desired')}</p>
        <p className={`cs-value ${hasProfile || assigned ? 'mono' : 'is-empty'}`}>{hasProfile ? t('revisionShort', { revision: agent.desiredRevision }) : assigned && agent.report?.assignment ? agent.report.assignment.compatibility.nativeIdentifier ?? agent.report.assignment.model.identifier : t('noRecord')}</p>
        <p className="cs-note">{hasProfile ? <>{text(agent.profileName)}<span className="note-divider" aria-hidden="true" /><span className={agent.pending ? 'note-pending' : undefined}>{t(agent.pending ? 'pendingReview' : 'matchesRecord')}</span></> : t(agent.assignment === 'direct' ? agent.report ? 'directIntentLive' : 'directIntent' : 'nativeIntent')}</p>
        {help && <p className="cs-help">{t('desiredHelp')}</p>}
      </div>
      <Operator op={recordOp} />
      <div className="cs-col station station-applied">
        <p className="cs-kicker">{t('applied')}</p>
        <p className={`cs-value ${hasProfile || applied ? 'mono' : 'is-empty'}`}>{hasProfile ? agent.appliedRevision === null ? t('common:unknown') : t('revisionShort', { revision: agent.appliedRevision }) : applied ? applied.identifier : t('noRecord')}</p>
        <p className="cs-note">{applied ? <span className="mono">{applied.location}</span> : t(assigned ? 'notAppliedYet' : hasProfile ? 'recordOnly' : 'noProfileShort')}</p>
        {help && <p className="cs-help">{t('appliedHelp')}</p>}
      </div>
      <Operator op={effectiveOp} />
      <div className="cs-col station station-effective">
        <p className="cs-kicker">{t('effective')}</p>
        <p className={`cs-value ${agent.modelId ? 'mono' : 'is-empty'}`}>{agent.modelId ?? t(agent.installed && agent.report ? 'liveAgentDefault' : 'common:notConfigured')}</p>
        <p className="cs-note station-source"><span className="sr-only">{t('source')}</span>{text(agent.modelSource)}{agent.providerName && <><span className="note-divider" aria-hidden="true" /><span lang="en-US">{agent.providerName}</span></>}</p>
        {help && <p className="cs-help">{t('effectiveHelp')}</p>}
      </div>
    </div>
    <p className="cs-foot">{t('stateBoundary')}</p>
  </div>;
}
