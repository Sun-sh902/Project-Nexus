import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { ExtensionRows } from '../components/ExtensionRows';
import { ActionPanel } from '../components/ActionFlow';
import { useActionFlow } from '../app/action-flow';
import { DataState, Notice, PageHeading, Tabs } from '../components/ui';

export function Extensions() {
  const { t } = useTranslation(['extensions', 'navigation', 'common']);
  const { locale } = usePreferences();
  const { extensions, mode, status } = useNexus();
  const flow = useActionFlow();
  const [view, setView] = useState<'mcp' | 'skills'>('mcp');
  const live = mode === 'live';
  const byView = (value: 'mcp' | 'skills') => extensions.filter(item => item.type === (value === 'mcp' ? 'mcp' : 'skill'));
  return <><PageHeading title={t('navigation:extensions')} description={t('description')} kicker={t(live ? 'pageKickerLive' : 'pageKicker')} />
    {status !== 'ready' ? <DataState /> : <Tabs label={t('tabs')} value={view} onChange={setView} options={(['mcp', 'skills'] as const).map(value => ({ value, label: t(value), count: formatNumber(byView(value).length, locale) }))}>
      <p className="view-intro">{t(`${view}Help${live ? 'Live' : ''}`)}</p>
      <ExtensionRows items={byView(view)} actions={live ? item => {
        const server = item.live?.server;
        if (!server || !item.live) return null;
        const agentId = item.live.agentId;
        return <div className="row-actions">
          {server.canToggle && server.enabled !== null && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'setMcpEnabled', agentId, name: server.name, enabled: !server.enabled })}>{t(server.enabled ? 'liveDisable' : 'liveEnable')}</button>}
          {server.nexusManaged && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'removeManagedMcp', agentId, name: server.name })}>{t('liveRemoveManaged')}</button>}
        </div>;
      } : undefined} />
      {live ? <><ActionPanel flow={flow} /><Notice>{t(view === 'mcp' ? 'liveMcpBoundary' : 'liveSkillsBoundary')}</Notice></> : <Notice>{t('common:previewOnly')}</Notice>}
    </Tabs>}
  </>;
}
