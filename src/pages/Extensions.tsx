import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { extensions } from '../data/fixtures';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { ExtensionRows } from '../components/ExtensionRows';
import { Notice, PageHeading, Tabs } from '../components/ui';

export function Extensions() {
  const { t } = useTranslation(['extensions', 'navigation', 'common']);
  const { locale } = usePreferences();
  const [view, setView] = useState<'mcp' | 'skills'>('mcp');
  const byView = (value: 'mcp' | 'skills') => extensions.filter(item => item.type === (value === 'mcp' ? 'mcp' : 'skill'));
  return <><PageHeading title={t('navigation:extensions')} description={t('description')} kicker={t('pageKicker')} />
    <Tabs label={t('tabs')} value={view} onChange={setView} options={(['mcp', 'skills'] as const).map(value => ({ value, label: t(value), count: formatNumber(byView(value).length, locale) }))}>
      <p className="view-intro">{t(`${view}Help`)}</p><ExtensionRows items={byView(view)} /><Notice>{t('common:previewOnly')}</Notice>
    </Tabs>
  </>;
}
