export const preferenceKey = 'nexus.preferences.v1';
export type Theme = 'light' | 'dark' | 'system';
export type Locale = 'zh-CN' | 'en-US';
export type Language = Locale | 'system';
export interface Preferences { version: 1; theme: Theme; language: Language }
export const defaults: Preferences = { version: 1, theme: 'system', language: 'system' };

export function parsePreferences(value: string | null): Preferences {
  try {
    const parsed: unknown = JSON.parse(value ?? 'null');
    if (!parsed || typeof parsed !== 'object' || !('version' in parsed) || parsed.version !== 1) return { ...defaults };
    const item = parsed as Record<string, unknown>;
    return {
      version: 1,
      theme: ['light', 'dark', 'system'].includes(String(item.theme)) ? item.theme as Theme : 'system',
      language: ['zh-CN', 'en-US', 'system'].includes(String(item.language)) ? item.language as Language : 'system',
    };
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

export function resolveLocale(language: Language, preferred: string | undefined): Locale {
  if (language !== 'system') return language;
  return /^zh(?:[-_]|$)/i.test(preferred ?? '') ? 'zh-CN' : 'en-US';
}

export function resolveTheme(theme: Theme, systemDark: boolean): 'light' | 'dark' {
  return theme === 'system' ? systemDark ? 'dark' : 'light' : theme;
}
