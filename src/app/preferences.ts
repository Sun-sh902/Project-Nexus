export const preferenceKey = 'nexus.preferences.v1';
export type Theme = 'light' | 'dark' | 'system';
// Nexus ships in English only. Localization still flows through i18n resources so components
// never hold user-facing strings.
export type Locale = 'en-US';
export const locale: Locale = 'en-US';
export interface Preferences { version: 1; theme: Theme }
export const defaults: Preferences = { version: 1, theme: 'system' };

export function parsePreferences(value: string | null): Preferences {
  try {
    const parsed: unknown = JSON.parse(value ?? 'null');
    if (!parsed || typeof parsed !== 'object' || !('version' in parsed) || parsed.version !== 1) return { ...defaults };
    const item = parsed as Record<string, unknown>;
    // Earlier builds also stored a `language` field; it is ignored.
    return { version: 1, theme: ['light', 'dark', 'system'].includes(String(item.theme)) ? item.theme as Theme : 'system' };
  } catch { return { ...defaults }; }
}

export function readPreferences(storage: Pick<Storage, 'getItem'>): { value: Preferences; failed: boolean } {
  try { return { value: parsePreferences(storage.getItem(preferenceKey)), failed: false }; }
  catch { return { value: { ...defaults }, failed: true }; }
}

export function savePreferences(storage: Pick<Storage, 'setItem'>, value: Preferences): boolean {
  try { storage.setItem(preferenceKey, JSON.stringify(value)); return true; }
  catch { return false; }
}

export function resolveTheme(theme: Theme, systemDark: boolean): 'light' | 'dark' {
  return theme === 'system' ? systemDark ? 'dark' : 'light' : theme;
}
