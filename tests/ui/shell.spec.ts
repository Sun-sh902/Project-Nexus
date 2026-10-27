import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { resources } from '../../src/app/resources';

async function preferences(page: Page, locale: 'en-US' | 'zh-CN', theme: 'light' | 'dark' | 'system') {
  await page.addInitScript(({ locale, theme }) => localStorage.setItem('nexus.preferences.v1', JSON.stringify({ version: 1, language: locale, theme })), { locale, theme });
}
const routes = ['home', 'agents', 'agents/codex', 'models', 'extensions', 'system', 'settings'];
for (const locale of ['en-US', 'zh-CN'] as const) for (const theme of ['light', 'dark'] as const) for (const size of [{ width: 960, height: 640 }, { width: 1440, height: 1000 }]) {
  test(`${locale} ${theme} ${size.width}: primary surfaces and accessibility`, async ({ page }, testInfo) => {
    await preferences(page, locale, theme);
    await page.setViewportSize(size);
    const errors: string[] = [];
    page.on('pageerror', error => errors.push(error.message));
    for (const route of routes) {
      await page.goto(`/#/${route}`);
      await expect(page.locator('html')).toHaveAttribute('lang', locale);
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: resources[locale].navigation.label })).toBeVisible();
      const key = route.replace('/', '-');
      await page.screenshot({ path: testInfo.outputPath(`${key}.png`) });
      expect(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)).toBe(false);
      expect(await page.locator('main').evaluate(element => element.scrollWidth > element.clientWidth)).toBe(false);
      const scan = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
      expect(scan.violations, `${route} accessibility`).toEqual([]);
      if (await page.locator('main').evaluate(el => el.scrollHeight > el.clientHeight)) {
        await page.locator('main').evaluate(el => el.scrollTo(0, el.scrollHeight));
        await page.screenshot({ path: testInfo.outputPath(`${key}-lower.png`) });
      }
    }
    expect(errors).toEqual([]);
  });
  test(`${locale} ${theme} ${size.width}: subordinate views`, async ({ page }, testInfo) => {
    await preferences(page, locale, theme);
    await page.setViewportSize(size);
    const cases = [
      { route: 'agents/codex', labels: ['overview', 'model', 'extensions', 'configuration', 'diagnostics'], namespace: 'agents' as const },
      { route: 'models', labels: ['catalog', 'profiles', 'providers'], namespace: 'models' as const },
      { route: 'extensions', labels: ['mcp', 'skills'], namespace: 'extensions' as const },
    ];
    for (const item of cases) {
      await page.goto(`/#/${item.route}`);
      for (const key of item.labels) {
        const label = (resources[locale][item.namespace] as Record<string, string>)[item.namespace === 'agents' && key === 'model' ? 'modelTab' : key];
        await page.getByRole('tab', { name: label, exact: true }).click();
        await expect(page.getByRole('tab', { name: label, exact: true })).toHaveAttribute('aria-selected', 'true');
        expect(await page.locator('main').evaluate(el => el.scrollWidth > el.clientWidth)).toBe(false);
        await page.screenshot({ path: testInfo.outputPath(`${item.route.replace('/', '-')}-${key}.png`) });
        expect((await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze()).violations).toEqual([]);
      }
    }
  });
}
test('runtime preferences persist and system theme responds without restart', async ({ page }) => {
  await page.goto('/#/settings');
  await page.locator('#language-choice').selectOption('zh-CN');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('设置');
  await page.getByRole('radio', { name: '深色', exact: true }).check();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await expect(page.getByRole('radio', { name: '深色', exact: true })).toBeChecked();
  await page.getByRole('radio', { name: '跟随系统', exact: true }).check();
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.locator('#language-choice').selectOption('en-US');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Settings');
});
test('keyboard navigation, detail tabs, history, empty and error views', async ({ page }) => {
  await preferences(page, 'en-US', 'light');
  await page.goto('/');
  await page.keyboard.press('Tab');
  await expect(page.getByText('Skip to main content')).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
  await page.getByRole('navigation', { name: 'Main navigation' }).getByRole('link', { name: 'Agents', exact: true }).click();
  await expect(page.getByRole('heading', { level: 1 })).toBeFocused();
  await page.getByRole('searchbox').fill('no-such-agent');
  await expect(page.getByText('No matching Agents')).toBeVisible();
  await page.getByRole('button', { name: 'Clear filter' }).click();
  await page.getByRole('link', { name: 'View details — OpenAI Codex', exact: true }).click();
  await page.getByRole('tab', { name: 'Overview', exact: true }).focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Model', exact: true })).toBeFocused();
  await page.keyboard.press('End');
  await expect(page.getByRole('tab', { name: 'Diagnostics' })).toBeFocused();
  await page.goBack();
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Agents');
  await page.goto('/#/agents/missing');
  await expect(page.getByText('This view could not be found')).toBeVisible();
});
test('400% equivalent reflow and reduced motion preserve access', async ({ page }) => {
  await preferences(page, 'zh-CN', 'dark');
  await page.setViewportSize({ width: 320, height: 640 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const route of routes) {
    await page.goto(`/#/${route}`);
    expect(await page.locator('main').evaluate(el => el.scrollWidth > el.clientWidth)).toBe(false);
    await expect(page.getByRole('navigation', { name: '主导航' })).toBeVisible();
  }
  await page.getByRole('radio', { name: '浅色', exact: true }).check();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  expect(await page.getByRole('radio', { name: '浅色', exact: true }).evaluate(el => getComputedStyle(el).transitionDuration)).toBe('0s');
});

test('first launch uses native preferred locale; system language refresh and explicit choice are independent', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, 'isTauri', { value: true });
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {
      invoke: async (command: string, args: { request?: { requestId: string } }) => {
        if (command === 'apply_app_preferences') return { contractVersion: 1, requestId: args.request?.requestId, result: 'applied' };
        if (command !== 'app_metadata') throw new Error('denied');
        return { contractVersion: 1, version: '0.1.0', preferredLocale: document.documentElement.dataset.testLocale ?? 'zh-Hant-TW' };
      },
    } });
  });
  await page.goto('/#/settings');
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await page.locator('#language-choice').selectOption('en-US');
  await page.evaluate(() => { document.documentElement.dataset.testLocale = 'zh-CN'; window.dispatchEvent(new Event('focus')); });
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  await page.locator('#language-choice').selectOption('system');
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await page.evaluate(() => { document.documentElement.dataset.testLocale = 'fr-FR'; window.dispatchEvent(new Event('focus')); });
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
});

test('native locale failure and denied preference storage stay usable and visible', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, 'isTauri', { value: true });
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: { invoke: async () => { throw new Error('denied'); } } });
    Storage.prototype.setItem = () => { throw new Error('denied'); };
  });
  await page.goto('/#/settings');
  await expect(page.getByText(resources['en-US'].common.localeError)).toBeVisible();
  await page.getByRole('radio', { name: 'Dark', exact: true }).check();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.getByText(resources['en-US'].common.storageError).first()).toBeVisible();
  await page.locator('#language-choice').selectOption('zh-CN');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('设置');
});

test('attention/blocked diagnostics, empty inventory and expanded labels fit narrow desktop', async ({ page }, testInfo) => {
  await preferences(page, 'en-US', 'dark');
  await page.setViewportSize({ width: 960, height: 640 });
  for (const id of ['claude', 'gemini']) {
    await page.goto(`/#/agents/${id}`);
    await page.getByRole('tab', { name: 'Diagnostics' }).click();
    await expect(page.getByText('Cause certainty')).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`${id}-finding.png`) });
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  }
  await page.getByRole('tab', { name: 'Extensions' }).click();
  await expect(page.getByText('Nothing here yet')).toBeVisible();
  for (const route of routes) {
    await page.goto(`/#/${route}`);
    // Expanded-content simulation: full localized layout, no third product locale.
    await page.locator('h1, h2, p, button, summary, dt, .sidebar nav a span').evaluateAll(elements => {
      for (const element of elements) {
        for (const node of Array.from(element.childNodes)) {
          if (node.nodeType === Node.TEXT_NODE && node.textContent?.trim()) node.textContent += ` ${node.textContent.slice(0, Math.ceil(node.textContent.length * 0.4))}`;
        }
      }
    });
    expect(await page.locator('main').evaluate(el => el.scrollWidth > el.clientWidth)).toBe(false);
    await page.screenshot({ path: testInfo.outputPath(`expanded-${route.replace('/', '-')}.png`) });
  }
});
