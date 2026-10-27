import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Locale, Theme } from './preferences';

export interface AppMetadata { contractVersion: 1; version: string; preferredLocale: string | null }
export async function readAppMetadata(): Promise<AppMetadata> {
  if (!isTauri()) return { contractVersion: 1, version: __APP_VERSION__, preferredLocale: navigator.languages[0] ?? navigator.language };
  let timeout: ReturnType<typeof setTimeout> | undefined;
  const result = await Promise.race([
    invoke<AppMetadata>('app_metadata'),
    new Promise<never>((_resolve, reject) => { timeout = setTimeout(() => reject(new Error('app.metadata_timeout')), 3000); }),
  ]).finally(() => clearTimeout(timeout));
  if (result.contractVersion !== 1 || typeof result.version !== 'string' || (result.preferredLocale !== null && typeof result.preferredLocale !== 'string')) {
    throw new Error('app.metadata_invalid');
  }
  return result;
}

export async function applyAppPreferences(theme: Theme, locale: Locale): Promise<void> {
  if (!isTauri()) return;
  const requestId = Array.from(crypto.getRandomValues(new Uint8Array(16)), value => value.toString(16).padStart(2, '0')).join('');
  const response = await invoke<{ contractVersion: 1; requestId: string; result: 'applied' }>('apply_app_preferences', { request: { requestId, theme, locale } });
  if (response.contractVersion !== 1 || response.requestId !== requestId || response.result !== 'applied') throw new Error('app.preferences_invalid');
}
