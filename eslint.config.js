import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import globals from 'globals';
import hooks from 'eslint-plugin-react-hooks';
import refresh from 'eslint-plugin-react-refresh';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'src-tauri', 'test-results', 'playwright-report'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  { files: ['**/*.{ts,tsx,js}'], languageOptions: { globals: { ...globals.browser, ...globals.node } } },
  { files: ['src/**/*.tsx'], plugins: { 'react-hooks': hooks, 'react-refresh': refresh }, rules: {
    ...hooks.configs.recommended.rules,
    'react-refresh/only-export-components': ['error', { allowConstantExport: true }],
  } },
);
