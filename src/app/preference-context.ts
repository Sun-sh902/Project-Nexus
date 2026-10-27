import { createContext, useContext } from 'react';
import type { Locale, Preferences, Theme } from './preferences';

export interface PreferenceContextValue {
  preferences: Preferences;
  locale: Locale;
  storageFailed: boolean;
  nativePreferenceFailed: boolean;
  version: string;
  setTheme: (theme: Theme) => void;
}
export const PreferenceContext = createContext<PreferenceContextValue | null>(null);
export function usePreferences() {
  const context = useContext(PreferenceContext);
  if (!context) throw new Error('app.preferences_context_missing');
  return context;
}
