import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { initializeI18n } from './app/i18n';
import { readAppMetadata, type AppMetadata } from './app/native';
import { PreferenceProvider } from './app/PreferenceProvider';
import { NexusDataProvider } from './app/nexus-data';
import { LoadingState } from './components/ui';
import './styles.css';

const root = createRoot(document.getElementById('root')!);
await initializeI18n('en-US');
root.render(<LoadingState />);
let metadata: AppMetadata;
try { metadata = await readAppMetadata(); }
catch { metadata = { contractVersion: 1, version: __APP_VERSION__, preferredLocale: null }; }
root.render(<StrictMode><PreferenceProvider metadata={metadata}><NexusDataProvider><App /></NexusDataProvider></PreferenceProvider></StrictMode>);
