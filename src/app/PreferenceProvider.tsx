import { useEffect, useState, type ReactNode } from 'react';
import { locale, readPreferences, resolveTheme, savePreferences, type Theme } from './preferences';
import { PreferenceContext } from './preference-context';
import { applyAppPreferences, type AppMetadata } from './native';

export function PreferenceProvider({ children, metadata }: { children: ReactNode; metadata: AppMetadata }) {
  const [initial] = useState(() => readPreferences(localStorage));
  const [preferences, setPreferences] = useState(initial.value);
  const [storageFailed, setStorageFailed] = useState(initial.failed);
  const [nativePreferenceFailed, setNativePreferenceFailed] = useState(false);

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
    let disposed = false;
    void applyAppPreferences(preferences.theme, locale).then(
      () => { if (!disposed) setNativePreferenceFailed(false); },
      () => { if (!disposed) setNativePreferenceFailed(true); },
    );
    return () => { disposed = true; };
  }, [preferences.theme]);

  function setTheme(theme: Theme) {
    const next = { ...preferences, theme };
    setStorageFailed(!savePreferences(localStorage, next));
    setPreferences(next);
  }
  return <PreferenceContext.Provider value={{ preferences, locale, storageFailed, nativePreferenceFailed, version: metadata.version, setTheme }}>{children}</PreferenceContext.Provider>;
}
