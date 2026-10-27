import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronDown } from 'lucide-react';
import { usePreferences } from '../app/preference-context';
import { useNexus } from '../app/nexus-context';
import { InfoList, Notice, PageHeading, StateGlyph } from '../components/ui';
import type { Language } from '../app/preferences';

const themes = ['light', 'dark', 'system'] as const;
const groups = ['appearance', 'language', 'about'] as const;

// A specimen is real Nexus UI scoped to one palette with data-theme, not a picture of one.
function Plate({ theme }: { theme: 'light' | 'dark' }) {
  const { t } = useTranslation(['health', 'home']);
  return <span className="specimen-plate" data-theme={theme}>
    <span className="plate-kicker">{t('home:specimenKicker')}</span>
    <span className="plate-line"><span className="lead">{t('home:specimenLead')}</span></span>
    <span className="plate-nodes" aria-hidden="true"><i data-n="pass" /><i data-n="pass" /><i data-n="pass" /><i data-n="pending" /><i data-n="unchecked" /></span>
    <span className="seal" data-s="attention"><StateGlyph state="attention" size={9} />{t('attention')}</span>
  </span>;
}

export function Settings() {
  const { t } = useTranslation(['settings', 'navigation', 'common']);
  const { preferences, setTheme, setLanguage, storageFailed, localeFailed, version } = usePreferences();
  const { mode } = useNexus();
  const live = mode === 'live';
  const id = useId();
  return <><PageHeading title={t('navigation:settings')} description={t('description')} kicker={t('pageKicker')} />
    <div className="settings-layout">
      <nav className="settings-categories" aria-label={t('contents')}><p className="kicker">{t('contents')}</p>{groups.map((group, index) => <button key={group} onClick={() => document.getElementById(`${id}-${group}`)?.scrollIntoView({ block: 'start' })}><span className="mono" aria-hidden="true">{t('common:sectionMark', { n: index + 1 })}</span>{t(group === 'about' && live ? 'aboutLive' : group)}</button>)}<p className="categories-note">{t('saved')}</p></nav>
      <div className="settings-content">
        <section className="settings-group" id={`${id}-appearance`} aria-labelledby={`${id}-appearance-title`}>
          <div className="group-head"><h2 id={`${id}-appearance-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 1 })}</span>{t('appearance')}</h2><span className="kicker">{t('appliesNow')}</span></div>
          <fieldset><legend className="sr-only">{t('theme')}</legend><p id="theme-help" className="field-help">{t('appearanceHelp')}</p>
            <div className="theme-options" aria-describedby="theme-help">{themes.map(theme => <label className={`theme-option ${preferences.theme === theme ? 'is-selected' : ''}`} key={theme}>
              <input type="radio" name="theme" value={theme} checked={preferences.theme === theme} onChange={() => setTheme(theme)} aria-labelledby={`${id}-${theme}-name`} aria-describedby={`${id}-${theme}-note`} />
              <span className={`specimen specimen-${theme}`} aria-hidden="true">{theme === 'system' ? <><Plate theme="light" /><Plate theme="dark" /></> : <Plate theme={theme} />}</span>
              <span className="theme-caption"><span className="theme-name" id={`${id}-${theme}-name`}>{t(theme)}</span><span className="theme-note" id={`${id}-${theme}-note`}>{t(`${theme}Caption`)}</span><span className="theme-radio" aria-hidden="true" /></span>
            </label>)}</div>
          </fieldset>
        </section>
        <section className="settings-group" id={`${id}-language`} aria-labelledby={`${id}-language-title`}>
          <div className="group-head"><h2 id={`${id}-language-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 2 })}</span>{t('language')}</h2></div>
          <div className="setting-row">
            <div><label className="setting-label" htmlFor="language-choice">{t('interfaceLanguage')}</label><p id="language-help" className="field-help">{t('languageHelp')}</p></div>
            <span className="select-control"><select id="language-choice" aria-describedby="language-help" value={preferences.language} onChange={event => setLanguage(event.target.value as Language)}><option value="system">{t('system')}</option><option value="zh-CN">{t('zh-CN')}</option><option value="en-US">{t('en-US')}</option></select><ChevronDown size={14} aria-hidden="true" /></span>
          </div>
          {localeFailed && preferences.language === 'system' && <Notice tone="attention">{t('common:localeError')}</Notice>}
          <p className="preference-feedback" role="status">{storageFailed ? t('common:storageError') : t('saved')}</p>
        </section>
        <section className="settings-group" id={`${id}-about`} aria-labelledby={`${id}-about-title`}>
          <div className="group-head"><h2 id={`${id}-about-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 3 })}</span>{t(live ? 'aboutLive' : 'about')}</h2><span className="kicker">{t(live ? 'common:phaseLive' : 'common:phase')}</span></div>
          <p className="field-help">{t(live ? 'aboutHelpLive' : 'aboutHelp')}</p>
          <InfoList items={[{ label: t('version'), value: <span className="mono">{version}</span> }, { label: t('build'), value: t(live ? 'common:phaseLive' : 'common:phase') }, ...(live ? [{ label: t('dataMode'), value: t('dataModeLive') }] : [])]} />
        </section>
      </div>
    </div>
  </>;
}
