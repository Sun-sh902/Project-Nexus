import { useEffect, useRef, useState } from 'react';
import { Home as HomeIcon, Bot, Layers2, Puzzle, Cpu, Settings2, PanelLeftClose, PanelLeftOpen, SunMoon, Languages } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { navigation, parseRoute } from './app/routes';
import { usePreferences } from './app/preference-context';
import type { Language, Theme } from './app/preferences';
import { formatNumber, formatTimestamp } from './app/format';
import { agents, findAgent, fixtureTime, type Readiness } from './data/fixtures';
import { NexusIdentity } from './components/NexusIdentity';
import { ErrorState, Notice } from './components/ui';
import { Home } from './pages/Home';
import { Agents } from './pages/Agents';
import { AgentDetail } from './pages/AgentDetail';
import { Models } from './pages/Models';
import { Extensions } from './pages/Extensions';
import { System } from './pages/System';
import { Settings } from './pages/Settings';

const agentStateContext = (state: Readiness) => `health:${state}`;
const icons = { home: HomeIcon, agents: Bot, models: Layers2, extensions: Puzzle, system: Cpu, settings: Settings2 };
export default function App() {
  const { t } = useTranslation(['common', 'navigation', 'settings']);
  const { preferences, locale, version, setLanguage, setTheme, storageFailed, nativePreferenceFailed } = usePreferences();
  const [hash, setHash] = useState(location.hash);
  const [collapsed, setCollapsed] = useState(false);
  const main = useRef<HTMLElement>(null);
  const previous = useRef(hash);
  const route = parseRoute(hash);
  useEffect(() => {
    const update = () => setHash(location.hash);
    window.addEventListener('hashchange', update);
    return () => window.removeEventListener('hashchange', update);
  }, []);
  useEffect(() => {
    if (previous.current !== hash) {
      main.current?.scrollTo(0, 0);
      (main.current?.querySelector('h1') as HTMLElement | null)?.focus();
      previous.current = hash;
    }
  }, [hash]);
  let page;
  switch (route.page) {
    case 'home': page = <Home />; break;
    case 'agents': page = route.agentId ? <AgentDetail key={route.agentId} id={route.agentId} /> : <Agents />; break;
    case 'models': page = <Models />; break;
    case 'extensions': page = <Extensions />; break;
    case 'system': page = <System />; break;
    case 'settings': page = <Settings />; break;
    default: page = <ErrorState />;
  }
  const detailAgent = route.page === 'agents' && route.agentId ? findAgent(route.agentId) : undefined;
  return <div className={`app-shell ${collapsed ? 'nav-collapsed' : ''}`}>
    <a className="skip-link" href="#main-content" onClick={event => { event.preventDefault(); main.current?.focus(); }}>{t('skip')}</a>
    <aside className="sidebar">
      <div className="sidebar-head"><NexusIdentity /></div>
      <nav aria-label={t('navigation:label')}>{navigation.map((key, index) => { const Icon = icons[key]; return <a href={`#/${key}`} key={key} aria-current={route.page === key ? 'page' : undefined} title={t(`navigation:${key}`)}>
        <span className="nav-index mono" aria-hidden="true">{String(index + 1).padStart(2, '0')}</span><Icon className="nav-icon" size={16} strokeWidth={1.6} aria-hidden="true" /><span className="nav-label">{t(`navigation:${key}`)}</span>{key === 'agents' && <span className="nav-count mono" aria-hidden="true">{formatNumber(agents.length, locale)}</span>}
      </a>; })}</nav>
      <div className="sidebar-footer">
        <div className="demo-register"><p className="kicker">{t('fixtureShort')}</p><p className="demo-text">{t('fixtureNotice')}</p><p className="demo-detail">{t('fixtureDetail')}</p></div>
        <div className="sidebar-base"><span className="mono version">{t('versionLabel', { version })}</span><button className="icon-button" aria-label={t(collapsed ? 'expand' : 'collapse')} title={t(collapsed ? 'expand' : 'collapse')} aria-expanded={!collapsed} onClick={() => setCollapsed(value => !value)}>{collapsed ? <PanelLeftOpen size={15} strokeWidth={1.6} aria-hidden="true" /> : <PanelLeftClose size={15} strokeWidth={1.6} aria-hidden="true" />}</button></div>
      </div>
    </aside>
    <div className="workspace">
      <main id="main-content" ref={main} tabIndex={-1}>
        <header className="workspace-toolbar">
          <div className="toolbar-location">{detailAgent
            ? <><a href="#/agents">{t('navigation:agents')}</a><span className="crumb-sep" aria-hidden="true" /><span className="location-label" lang="en-US">{detailAgent.name}</span><span className="location-context">{t(agentStateContext(detailAgent.readiness))}</span></>
            : <><span className="location-label">{route.page === 'notFound' ? t('productName') : t(`navigation:${route.page}`)}</span>{route.page !== 'notFound' && <span className="location-context">{t(`navigation:${route.page}Context`)}</span>}</>}</div>
          <div className="shell-preferences">
            <span className="fixture-clock mono">{t('fixtureClock')}<span className="note-divider" aria-hidden="true" />{formatTimestamp(fixtureTime, locale)}</span>
            <label className="toolbar-select" title={t('settings:appearance')}><SunMoon size={14} strokeWidth={1.6} aria-hidden="true" /><span className="sr-only">{t('settings:appearance')}</span><select id="shell-theme" value={preferences.theme} onChange={event => setTheme(event.target.value as Theme)}>{(['light', 'dark', 'system'] as const).map(theme => <option value={theme} key={theme}>{t(`settings:${theme}`)}</option>)}</select></label>
            <label className="toolbar-select" title={t('settings:language')}><Languages size={14} strokeWidth={1.6} aria-hidden="true" /><span className="sr-only">{t('settings:language')}</span><select id="shell-language" value={preferences.language} onChange={event => setLanguage(event.target.value as Language)}>{(['system', 'zh-CN', 'en-US'] as const).map(language => <option value={language} key={language}>{t(`settings:${language}`)}</option>)}</select></label>
          </div>
        </header>
        <div className="main-inner">{storageFailed && <Notice tone="attention">{t('storageError')}</Notice>}{nativePreferenceFailed && <Notice tone="attention">{t('nativePreferenceError')}</Notice>}{page}</div>
      </main>
    </div>
  </div>;
}
