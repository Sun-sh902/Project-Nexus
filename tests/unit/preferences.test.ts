import { describe, expect, it } from 'vitest';
import { defaults, parsePreferences, readPreferences, savePreferences, resolveTheme } from '../../src/app/preferences';

describe('preference contract', () => {
  it('persists the theme through serialization and a new read', () => {
    let record: string | null = null;
    const storage = { getItem: () => record, setItem: (_key: string, value: string) => { record = value; } };
    const preference = { version: 1 as const, theme: 'dark' as const };
    expect(savePreferences(storage, preference)).toBe(true);
    expect(readPreferences(storage)).toEqual({ value: preference, failed: false });
  });
  it('keeps the theme from earlier records that also stored a language', () => {
    expect(parsePreferences('{"version":1,"theme":"dark","language":"system"}')).toEqual({ version: 1, theme: 'dark' });
  });
  it('rejects corrupt/foreign versions and validates each field', () => {
    for (const value of [null, 'invalid', '[]', 'null', '{"version":2,"theme":"dark"}']) expect(parsePreferences(value)).toEqual(defaults);
    expect(parsePreferences('{"version":1,"theme":"neon"}')).toEqual(defaults);
  });
  it('exposes storage denial and preserves safe defaults', () => {
    const denied = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };
    expect(readPreferences(denied)).toEqual({ value: defaults, failed: true });
    expect(savePreferences(denied, defaults)).toBe(false);
  });
  it('tracks OS theme only when system mode is selected', () => {
    expect(resolveTheme('system', false)).toBe('light');
    expect(resolveTheme('system', true)).toBe('dark');
    expect(resolveTheme('light', true)).toBe('light');
    expect(resolveTheme('dark', false)).toBe('dark');
  });
});
