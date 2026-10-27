import { useTranslation } from 'react-i18next';
import { useNexus, useText } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { demoSystem } from '../data/demo';
import { AgentLinks, DataState, Notice, PageHeading, Section, Status } from '../components/ui';

export function System() {
  const { t } = useTranslation(['system', 'navigation', 'common']);
  const { locale } = usePreferences();
  const { agents, tools, mode, status, overview } = useNexus();
  const text = useText();
  const live = mode === 'live';
  const sys = overview?.system;
  if (status !== 'ready') return <div className="system-page"><PageHeading title={t('navigation:system')} description={t('description')} kicker={t('pageKickerLive')} /><DataState /></div>;
  return <div className="system-page"><PageHeading title={t('navigation:system')} description={t('description')} kicker={t(live ? 'pageKickerLive' : 'pageKicker')}>
      <dl className="fact-register" aria-label={t(live ? 'environmentLive' : 'environment')}>
        <div><dt>{t(live ? 'environmentLive' : 'environment')}</dt><dd>{t(live ? 'thisComputer' : 'fixtureHost')}</dd></div>
        <div><dt>{t('os')}</dt><dd lang="en-US">{live && sys ? `${t(`osName.${sys.os}`)} ${sys.osVersion.value ?? ''}` : demoSystem.os}</dd></div>
        <div><dt>{t('arch')}</dt><dd className="mono">{live && sys ? sys.arch : demoSystem.arch}</dd></div>
        <div><dt>{t('toolCount')}</dt><dd className="mono">{formatNumber(tools.filter(tool => !live || tool.version !== null || tool.readiness !== 'unknown').length, locale)}</dd></div>
      </dl>
    </PageHeading>
    <Notice>{t(live ? 'detectedLive' : 'notDetected')}</Notice>
    <Section title={t('toolsTitle')} aside={<span className="kicker">{t('matrixHelp')}</span>}>
      <div className="sheet tool-matrix-scroll"><table className="tool-matrix">
        <thead><tr>
          <th scope="col">{t('tool')}</th><th scope="col">{t(live ? 'versionLive' : 'version')}</th><th scope="col">{t('status')}</th>
          {agents.map(agent => <th scope="col" key={agent.id} className="matrix-agent" lang="en-US"><a href={`#/agents/${agent.id}`}>{agent.name}</a></th>)}
          <th scope="col" className="matrix-list">{t('affected')}</th>
        </tr></thead>
        <tbody>{tools.map(tool => <tr key={tool.key}>
          <th scope="row" lang="en-US">{tool.name}{tool.location && <span className="row-meta mono">{tool.location}</span>}</th>
          <td><span className={tool.version ? 'mono' : 'mono is-empty'}>{tool.version ?? t('common:unknown')}</span></td>
          <td><Status state={tool.readiness} /><span className="row-meta">{text(tool.status)}{!live && tool.readiness === 'unknown' && <><span className="note-divider" aria-hidden="true" />{t('optional')}</>}</span></td>
          {agents.map(agent => { const affected = tool.agentIds.includes(agent.id); return <td key={agent.id} className={`matrix-agent ${affected ? 'is-affected' : ''}`}>
            <span className="matrix-mark" aria-hidden="true" /><span className="sr-only">{t(affected ? 'affectedMark' : 'notAffected')}</span>
          </td>; })}
          <td className="matrix-list"><AgentLinks ids={tool.agentIds} /></td>
        </tr>)}</tbody>
      </table>
      <p className="sheet-foot matrix-legend"><span><span className="matrix-mark is-on" aria-hidden="true" />{t('affectedMark')}</span><span><span className="matrix-mark" aria-hidden="true" />{t('notAffected')}</span></p></div>
    </Section>
    {live && sys && <Section title={t('pathTitle')} aside={<span className="kicker">{t('pathKicker')}</span>}>
      <div className="sheet"><ul className="ledger-list">{sys.pathEntries.map(entry => <li key={entry.location}><span className="mono">{entry.location}</span><span>{t(entry.onPath ? 'onPath' : 'knownLocation')}</span><span className="mono">{t(entry.exists ? 'exists' : 'missing')}</span></li>)}</ul>
      <p className="sheet-foot">{t(sys.guiPathNote ? 'guiPathNote' : 'pathNote')}</p></div>
      <Notice>{t('dataDir', { dir: sys.dataDir })}{!sys.dataDirWritable && ` ${t('dataDirNotWritable')}`}</Notice>
    </Section>}
  </div>;
}
