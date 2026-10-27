import { describe, expect, it } from 'vitest';
import { defaults, parsePreferences, readPreferences, savePreferences, resolveLocale, resolveTheme } from '../../src/app/preferences';

describe('preference contract', () => {
  it('resolves first-launch Chinese language variants without treating unrelated tags as Chinese', () => {
    for (const locale of ['zh', 'zh-CN', 'zh-Hant-TW', 'ZH_hk']) expect(resolveLocale('system', locale)).toBe('zh-CN');
    for (const locale of ['en-GB', 'fr-FR', 'zho', undefined]) expect(resolveLocale('system', locale)).toBe('en-US');
  });
  it('honors explicit language independently of later OS changes', () => {
    expect(resolveLocale('en-US', 'zh-CN')).toBe('en-US');
    expect(resolveLocale('zh-CN', 'en-US')).toBe('zh-CN');
  });
  it('persists both preferences through serialization and a new read', () => {
    let record: string | null = null;
    const storage = { getItem: () => record, setItem: (_key: string, value: string) => { record = value; } };
    const preference = { version: 1 as const, language: 'zh-CN' as const, theme: 'dark' as const };
    expect(savePreferences(storage, preference)).toBe(true);
    expect(readPreferences(storage)).toEqual({ value: preference, failed: false });
  });
  it('rejects corrupt/foreign versions and validates each field', () => {
    for (const value of [null, 'invalid', '[]', 'null', '{"version":2,"theme":"dark"}']) expect(parsePreferences(value)).toEqual(defaults);
    expect(parsePreferences('{"version":1,"theme":"dark","language":"bad"}')).toEqual({ ...defaults, theme: 'dark' });
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
