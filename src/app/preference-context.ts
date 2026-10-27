import { createContext, useContext } from 'react';
import type { Language, Locale, Preferences, Theme } from './preferences';

export interface PreferenceContextValue {
  preferences: Preferences;
  locale: Locale;
  storageFailed: boolean;
  localeFailed: boolean;
  nativePreferenceFailed: boolean;
  version: string;
  setTheme: (theme: Theme) => void;
  setLanguage: (language: Language) => void;
}
export const PreferenceContext = createContext<PreferenceContextValue | null>(null);
export function usePreferences() {
  const context = useContext(PreferenceContext);
  if (!context) throw new Error('app.preferences_context_missing');
  return context;
}
