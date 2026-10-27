import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import packageInfo from './package.json' with { type: 'json' };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  define: { __APP_VERSION__: JSON.stringify(packageInfo.version) },
  server: { port: 1420, strictPort: true },
  build: { target: ['es2022', 'safari15'], sourcemap: false },
});
