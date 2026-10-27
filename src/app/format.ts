import type { Locale } from './preferences';
export const formatTimestamp = (iso: string, locale: Locale) => new Intl.DateTimeFormat(locale, {
  year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', timeZoneName: 'short',
}).format(new Date(iso));
export const formatNumber = (value: number, locale: Locale) => new Intl.NumberFormat(locale).format(value);
