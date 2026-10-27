import { useTranslation } from 'react-i18next';
import { findModel, findProfile, findProvider, type AgentFixture } from '../data/fixtures';

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

// Desired → applied → effective are three separate records. Operators only compare what the
// fixture records: a pending revision differs; an effective model owned by the Agent is a separate source.
export function ConfigurationSheet({ agent, help = true }: { agent: AgentFixture; help?: boolean }) {
  const { t } = useTranslation(['agents', 'models', 'common']);
  const profile = findProfile(agent.profileId);
  const model = findModel(agent.modelId);
  const provider = findProvider(model?.providerId);
  const pending = profile ? agent.appliedRevision !== profile.revision : false;
  const recordOp: Op = profile ? pending ? 'differ' : 'match' : 'none';
  const effectiveOp: Op = agent.assignment === 'profile' && profile ? 'match' : 'separate';
  return <div className="config-sheet">
    <div className="cs-grid">
      <div className="cs-col station station-desired">
        <p className="cs-kicker">{t('desired')}</p>
        <p className={`cs-value ${profile ? 'mono' : 'is-empty'}`}>{profile ? t('revisionShort', { revision: profile.revision }) : t('noRecord')}</p>
        <p className="cs-note">{profile ? <>{t(`models:${profile.nameKey}`)}<span className="note-divider" aria-hidden="true" /><span className={pending ? 'note-pending' : undefined}>{t(pending ? 'pendingReview' : 'matchesRecord')}</span></> : t(agent.assignment === 'direct' ? 'directIntent' : 'nativeIntent')}</p>
        {help && <p className="cs-help">{t('desiredHelp')}</p>}
      </div>
      <Operator op={recordOp} />
      <div className="cs-col station station-applied">
        <p className="cs-kicker">{t('applied')}</p>
        <p className={`cs-value ${profile ? 'mono' : 'is-empty'}`}>{profile ? agent.appliedRevision === null ? t('common:unknown') : t('revisionShort', { revision: agent.appliedRevision }) : t('noRecord')}</p>
        <p className="cs-note">{t(profile ? 'recordOnly' : 'noProfileShort')}</p>
        {help && <p className="cs-help">{t('appliedHelp')}</p>}
      </div>
      <Operator op={effectiveOp} />
      <div className="cs-col station station-effective">
        <p className="cs-kicker">{t('effective')}</p>
        <p className={`cs-value ${model ? 'mono' : 'is-empty'}`}>{model?.identifier ?? t('common:notConfigured')}</p>
        <p className="cs-note station-source"><span className="sr-only">{t('source')}</span>{t(agent.assignment)}{provider && <><span className="note-divider" aria-hidden="true" /><span lang="en-US">{provider.name}</span></>}</p>
        {help && <p className="cs-help">{t('effectiveHelp')}</p>}
      </div>
    </div>
    <p className="cs-foot">{t('stateBoundary')}</p>
  </div>;
}
