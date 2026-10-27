import { useEffect, useState, type ReactNode } from 'react';
import i18n from './i18n';
import { readPreferences, resolveLocale, resolveTheme, savePreferences, type Language, type Theme } from './preferences';
import { PreferenceContext } from './preference-context';
import { readAppMetadata, applyAppPreferences, type AppMetadata } from './native';

export function PreferenceProvider({ children, metadata, initialLocaleFailed }: { children: ReactNode; metadata: AppMetadata; initialLocaleFailed: boolean }) {
  const [initial] = useState(() => readPreferences(localStorage));
  const [preferences, setPreferences] = useState(initial.value);
  const [storageFailed, setStorageFailed] = useState(initial.failed);
  const [preferredLocale, setPreferredLocale] = useState(metadata.preferredLocale ?? undefined);
  const [localeFailed, setLocaleFailed] = useState(initialLocaleFailed);
  const [nativePreferenceFailed, setNativePreferenceFailed] = useState(false);
  const locale = resolveLocale(preferences.language, preferredLocale);

  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      const theme = resolveTheme(preferences.theme, media.matches);
      document.documentElement.dataset.theme = theme;
      document.documentElement.style.colorScheme = theme;
    };
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [preferences.theme]);

  useEffect(() => {
    document.documentElement.lang = locale;
    void i18n.changeLanguage(locale);
  }, [locale]);

  useEffect(() => {
    let disposed = false;
    void applyAppPreferences(preferences.theme, locale).then(
      () => { if (!disposed) setNativePreferenceFailed(false); },
      () => { if (!disposed) setNativePreferenceFailed(true); },
    );
    return () => { disposed = true; };
  }, [preferences.theme, locale]);

  useEffect(() => {
    if (preferences.language !== 'system') return;
    let disposed = false;
    const refresh = async () => {
      if (document.visibilityState === 'hidden') return;
      try {
        const result = await readAppMetadata();
        if (!disposed) { setPreferredLocale(result.preferredLocale ?? undefined); setLocaleFailed(false); }
      } catch { if (!disposed) setLocaleFailed(true); }
    };
    void refresh();
    window.addEventListener('focus', refresh);
    window.addEventListener('languagechange', refresh);
    document.addEventListener('visibilitychange', refresh);
    return () => {
      disposed = true;
      window.removeEventListener('focus', refresh);
      window.removeEventListener('languagechange', refresh);
      document.removeEventListener('visibilitychange', refresh);
    };
  }, [preferences.language]);

  function update(change: { theme?: Theme; language?: Language }) {
    const next = { ...preferences, ...change };
    setStorageFailed(!savePreferences(localStorage, next));
    setPreferences(next);
  }
  return <PreferenceContext.Provider value={{ preferences, locale, storageFailed, localeFailed, nativePreferenceFailed, version: metadata.version,
    setTheme: theme => update({ theme }), setLanguage: language => update({ language }),
  }}>{children}</PreferenceContext.Provider>;
}
