import { useTranslation } from 'react-i18next';
import { agents, system } from '../data/fixtures';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { AgentLinks, Notice, PageHeading, Section, Status } from '../components/ui';

export function System() {
  const { t } = useTranslation(['system', 'navigation', 'common']);
  const { locale } = usePreferences();
  return <div className="system-page"><PageHeading title={t('navigation:system')} description={t('description')} kicker={t('pageKicker')}>
      <dl className="fact-register" aria-label={t('environment')}>
        <div><dt>{t('environment')}</dt><dd>{t('fixtureHost')}</dd></div>
        <div><dt>{t('os')}</dt><dd lang="en-US">{system.os}</dd></div>
        <div><dt>{t('arch')}</dt><dd className="mono">{system.architecture}</dd></div>
        <div><dt>{t('toolCount')}</dt><dd className="mono">{formatNumber(system.tools.length, locale)}</dd></div>
      </dl>
    </PageHeading>
    <Notice>{t('notDetected')}</Notice>
    <Section title={t('toolsTitle')} aside={<span className="kicker">{t('matrixHelp')}</span>}>
      <div className="sheet tool-matrix-scroll"><table className="tool-matrix">
        <thead><tr>
          <th scope="col">{t('tool')}</th><th scope="col">{t('version')}</th><th scope="col">{t('status')}</th>
          {agents.map(agent => <th scope="col" key={agent.id} className="matrix-agent" lang="en-US"><a href={`#/agents/${agent.id}`}>{agent.name}</a></th>)}
          <th scope="col" className="matrix-list">{t('affected')}</th>
        </tr></thead>
        <tbody>{system.tools.map(tool => <tr key={tool.name}>
          <th scope="row" lang="en-US">{tool.name}</th>
          <td><span className={tool.version ? 'mono' : 'mono is-empty'}>{tool.version ?? t('common:unknown')}</span></td>
          <td><Status state={tool.readiness} /><span className="row-meta">{t(tool.readiness === 'unknown' ? 'evidenceMissing' : 'runtimeReady')}{tool.readiness === 'unknown' && <><span className="note-divider" aria-hidden="true" />{t('optional')}</>}</span></td>
          {agents.map(agent => { const affected = tool.agentIds.includes(agent.id); return <td key={agent.id} className={`matrix-agent ${affected ? 'is-affected' : ''}`}>
            <span className="matrix-mark" aria-hidden="true" /><span className="sr-only">{t(affected ? 'affectedMark' : 'notAffected')}</span>
          </td>; })}
          <td className="matrix-list"><AgentLinks ids={tool.agentIds} /></td>
        </tr>)}</tbody>
      </table>
      <p className="sheet-foot matrix-legend"><span><span className="matrix-mark is-on" aria-hidden="true" />{t('affectedMark')}</span><span><span className="matrix-mark" aria-hidden="true" />{t('notAffected')}</span></p></div>
    </Section>
  </div>;
}
