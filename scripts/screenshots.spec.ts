import { test, type Browser, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';

// README screenshots of the real UI fed by IPC responses that the Rust backend produced from an
// isolated fake home (tests/fixtures/live, regenerate with
// `cargo test export_live_ui_fixture -- --ignored`). No personal machine data is shown.
const fixture = (name: string) => JSON.parse(readFileSync(`tests/fixtures/live/${name}.json`, 'utf8'));
const out = (name: string) => `docs/screenshots/${name}.png`;

async function open(browser: Browser, route: string, theme: 'light' | 'dark' = 'light'): Promise<Page> {
  // A fresh page per shot: hash-only navigation would not re-run init scripts.
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: theme });
  const data = { overview: fixture('overview'), models: fixture('models'), plan: fixture('plan'), result: fixture('result'), after: fixture('overview-after') };
  await page.addInitScript(({ data, theme }) => {
    localStorage.setItem('nexus.preferences.v1', JSON.stringify({ version: 1, theme }));
    let confirmed = false;
    Object.assign(window, { isTauri: true });
    Object.assign(window, { __TAURI_INTERNALS__: { transformCallback: () => 0, invoke: async (cmd: string, args: { request?: { requestId: string } }) => {
      switch (cmd) {
        case 'app_metadata': return { contractVersion: 1, version: '0.2.0', preferredLocale: 'en-US' };
        case 'apply_app_preferences': return { contractVersion: 1, requestId: args.request?.requestId, result: 'applied' };
        case 'nexus_overview': return confirmed ? data.after : data.overview;
        case 'nexus_models': return data.models;
        case 'nexus_agent_history': return { backups: [{ id: 'bak_0123456789abcdef0123456789abcdef', operationId: 'op_1', agentId: 'claude', targetId: 'claude.user', targetDisplay: '~/.claude/settings.json', sha256: 'f'.repeat(64), size: 42, createdAt: data.overview.generatedAt }], operations: [], health: [] };
        case 'nexus_prepare_action': return data.plan;
        case 'nexus_confirm_plan': confirmed = true; return data.result;
        default: return null;
      }
    } } });
  }, { data, theme });
  await page.goto(`/#/${route}`);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  return page;
}

test('README screenshots', async ({ browser }) => {
  let page: Page;
  page = await open(browser, 'home');
  await page.screenshot({ path: out('home') });
  page = await open(browser, 'home', 'dark');
  await page.screenshot({ path: out('home-dark') });

  page = await open(browser, 'agents/codex');
  await page.getByRole('tab', { name: 'Diagnostics' }).click();
  await page.locator('.finding').first().getByRole('button').first().click();
  await page.locator('.plan-sheet').waitFor();
  await page.locator('.finding').first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: out('diagnostics') });
  await page.locator('.plan-sheet').first().screenshot({ path: out('repair-plan') });
  await page.getByRole('button', { name: 'Confirm…' }).click();
  await page.locator('.plan-sheet').filter({ hasText: 'Completed and verified' }).waitFor();
  await page.locator('.plan-sheet').filter({ hasText: 'Completed and verified' }).screenshot({ path: out('verified-result') });

  page = await open(browser, 'agents/claude');
  await page.screenshot({ path: out('agent-detail') });
  await page.getByRole('tab', { name: 'Model', exact: true }).click();
  await page.locator('.layer-list').scrollIntoViewIfNeeded();
  await page.screenshot({ path: out('model-precedence') });
  await page.getByRole('tab', { name: 'Configuration' }).click();
  await page.waitForTimeout(200);
  await page.locator('.configuration-area').scrollIntoViewIfNeeded();
  await page.screenshot({ path: out('configuration') });

  page = await open(browser, 'models');
  await page.screenshot({ path: out('models') });
  page = await open(browser, 'extensions');
  await page.screenshot({ path: out('extensions') });
  page = await open(browser, 'system');
  await page.screenshot({ path: out('system') });
});
