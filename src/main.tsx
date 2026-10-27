import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { initializeI18n } from './app/i18n';
import { readAppMetadata, type AppMetadata } from './app/native';
import { readPreferences, resolveLocale } from './app/preferences';
import { PreferenceProvider } from './app/PreferenceProvider';
import { LoadingState } from './components/ui';
import './styles.css';

const root = createRoot(document.getElementById('root')!);
const { value } = readPreferences(localStorage);
await initializeI18n(resolveLocale(value.language, undefined));
root.render(<LoadingState />);
let metadata: AppMetadata;
let localeFailed = false;
try { metadata = await readAppMetadata(); }
catch { metadata = { contractVersion: 1, version: __APP_VERSION__, preferredLocale: null }; localeFailed = true; }
await initializeI18n(resolveLocale(value.language, metadata.preferredLocale ?? undefined));
root.render(<StrictMode><PreferenceProvider metadata={metadata} initialLocaleFailed={localeFailed}><App /></PreferenceProvider></StrictMode>);
