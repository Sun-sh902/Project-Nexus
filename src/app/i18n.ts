import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { resources } from './resources';

export async function initializeI18n(locale: string) {
  if (i18n.isInitialized) {
    await i18n.changeLanguage(locale);
    document.documentElement.lang = locale;
    return;
  }
  await i18n.use(initReactI18next).init({
    resources, lng: locale, fallbackLng: 'en-US', supportedLngs: ['en-US'],
    defaultNS: 'common', ns: Object.keys(resources['en-US']),
    interpolation: { escapeValue: false },
    returnEmptyString: false,
    parseMissingKeyHandler: () => resources['en-US'].common.unknown,
  });
  document.documentElement.lang = locale;
}
export default i18n;
