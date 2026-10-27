import { test, expect, type Page } from '@playwright/test';
import { resources } from '../../src/app/resources';
import { agents, findModel } from '../../src/data/fixtures';
import { readinessNodes } from '../../src/app/readiness';

async function start(page: Page, locale: 'zh-CN' | 'en-US', route = 'home') {
  await page.addInitScript(locale => {
    if (!localStorage.getItem('nexus.preferences.v1')) localStorage.setItem('nexus.preferences.v1', JSON.stringify({ version: 1, language: locale, theme: 'light' }));
  }, locale);
  const response = await page.goto(`/?production-ui=${route}#/${route}`);
  expect(response?.ok()).toBe(true);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await page.evaluate(() => document.fonts.ready);
}

async function assertNoHorizontalOverflow(page: Page, lists: string[]) {
  const dimensions = await page.evaluate(selectors => {
    const elements = [document.documentElement, document.querySelector('main')!, ...selectors.flatMap(selector => Array.from(document.querySelectorAll(selector)))];
    return elements.map(element => ({
      name: element.tagName.toLowerCase() + (element.className ? `.${String(element.className).split(' ').join('.')}` : ''),
      content: element.scrollWidth,
      available: element.clientWidth,
    }));
  }, lists);
  for (const element of dimensions) expect(element.content, `${element.name} horizontal content fits`).toBeLessThanOrEqual(element.available + 1);
}

async function assertEssentialControlsFit(page: Page) {
  const controls = page.locator('nav a, .workspace-toolbar select, main .detail-link, main .button, main [role="tab"], main summary, .tool-matrix a');
  for (const control of await controls.all()) {
    if (!await control.isVisible()) continue;
    await control.scrollIntoViewIfNeeded();
    await expect(control).toBeInViewport({ ratio: 1 });
    const clipped = await control.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
      const labels: string[] = [];
      while (walker.nextNode()) {
        const node = walker.currentNode;
        if (!node.textContent?.trim() || node.parentElement?.closest('.sr-only, [aria-hidden="true"]')) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        for (const text of range.getClientRects()) {
          if (text.width > 0 && (text.left < bounds.left - 2 || text.right > bounds.right + 2)) labels.push(node.textContent.trim());
        }
      }
      return labels;
    });
    expect(clipped, 'essential control text fits without horizontal clipping').toEqual([]);
  }
}

const missionSizes = [{ width: 1440, height: 900 }, { width: 1280, height: 800 }, { width: 960, height: 640 }];
for (const locale of ['zh-CN', 'en-US'] as const) for (const theme of ['light', 'dark'] as const) for (const size of missionSizes) {
  test(`mission layout ${locale} ${theme} ${size.width}x${size.height}: Home, Claude and System remain readable`, async ({ page }, testInfo) => {
    await page.setViewportSize(size);
    await page.addInitScript(({ locale, theme }) => localStorage.setItem('nexus.preferences.v1', JSON.stringify({ version: 1, language: locale, theme })), { locale, theme });
    const messages = resources[locale];
    for (const route of ['home', 'agents/claude', 'system']) {
      const response = await page.goto(`/?mission-layout=${route}#/${route}`);
      expect(response?.ok()).toBe(true);
      await expect(page.locator('html')).toHaveAttribute('lang', locale);
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await page.evaluate(() => document.fonts.ready);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await expect(page.getByText(messages.common.fixtureNotice, { exact: true })).toBeVisible();
      await assertNoHorizontalOverflow(page, route === 'home' ? ['.agent-sheet', '.register', '.exhibits'] : route === 'system' ? ['.tool-matrix'] : ['.config-sheet', '.check-list', '.procession', '.fact-register']);
      if (route === 'home') {
        for (const agent of agents) {
          const row = page.locator('.agent-row').filter({ has: page.getByRole('link', { name: agent.name, exact: true }) });
          await expect(row).toContainText(messages.health[agent.readiness]);
          await expect(row).toContainText(findModel(agent.modelId)?.identifier ?? messages.common.notConfigured);
          if (size.width === 1440) await expect(row.locator('.seal')).toBeInViewport();
        }
      }
      const filename = `${route.replace('/', '-')}-${locale}-${theme}-${size.width}x${size.height}.png`;
      await page.screenshot({ path: testInfo.outputPath(filename) });
      await assertEssentialControlsFit(page);
    }
  });
}

for (const locale of ['zh-CN', 'en-US'] as const) {
  test(`${locale}: Home readiness lines, exhibits and configuration truth stay explicit`, async ({ page }) => {
    await start(page, locale);
    const messages = resources[locale];
    const health = messages.health as Record<string, string>;
    const labels: Record<string, string> = { install: health.lineInstall, runtime: health.lineRuntime, config: health.lineConfig, model: health.lineModel, online: health.lineOnline };
    const states: Record<string, string> = { pass: health.nodePass, fail: health.nodeFail, pending: health.nodePending, set: health.nodeSet, unchecked: health.nodeUnchecked, unreached: health.nodeUnreached };
    for (const agent of agents) {
      const row = page.locator('.agent-row').filter({ has: page.getByRole('link', { name: agent.name, exact: true }) });
      const line = row.getByRole('list', { name: health.lineTitle });
      // Every node is announced with its own label and state; online service is never a pass.
      for (const node of readinessNodes(agent)) await expect(line).toContainText(health.nodeAria.replace('{{label}}', labels[node.key]).replace('{{state}}', states[node.state]));
      await expect(line.locator('li').last()).toHaveAttribute('data-n', 'unchecked');
    }
    // Exhibits put blocking issues before attention items.
    const exhibits = page.locator('.exhibit');
    await expect(exhibits).toHaveCount(2);
    await expect(exhibits.first()).toContainText('Gemini CLI');
    await expect(exhibits.first()).toContainText(messages.agents.geminiIssue);
    await expect(exhibits.last()).toContainText(messages.agents.claudeIssue);
    const overview = exhibits.last().getByRole('link', { name: messages.common.agentAction.replace('{{action}}', messages.home.openOverview).replace('{{agent}}', 'Claude Code'), exact: true });
    await overview.focus();
    await page.keyboard.press('Enter');
    expect(new URL(page.url()).hash).toBe('#/agents/claude');
    await expect(page.getByRole('tab', { name: messages.agents.overview, exact: true })).toHaveAttribute('aria-selected', 'true');
    // Desired, applied and effective remain three separately labelled records.
    const sheet = page.getByRole('tabpanel').locator('.config-sheet');
    await expect(sheet.locator('.station-desired')).toContainText(messages.agents.desired);
    await expect(sheet.locator('.station-desired')).toContainText('v2');
    await expect(sheet.locator('.station-desired')).toContainText(messages.agents.pendingReview);
    await expect(sheet.locator('.station-applied')).toContainText(messages.agents.applied);
    await expect(sheet.locator('.station-applied')).toContainText('v1');
    await expect(sheet.locator('.station-applied')).not.toContainText('demo-fast-coding');
    await expect(sheet.locator('.station-effective')).toContainText('demo-fast-coding');
    await expect(sheet.locator('.station-source')).toContainText(messages.agents.native);
    await expect(sheet.locator('.cs-op').first()).toHaveAttribute('data-op', 'differ');
    await expect(sheet.locator('.cs-op').last()).toHaveAttribute('data-op', 'separate');
    await page.goto('/#/agents/gemini');
    await expect(page.locator('.station-desired')).toContainText(messages.agents.noRecord);
    await expect(page.locator('.station-effective')).toContainText(messages.common.notConfigured);
    await expect(page.locator('.check-list').first()).toContainText(messages.agents.checkFail);
    await page.getByRole('navigation', { name: messages.navigation.label }).getByRole('link', { name: messages.navigation.agents, exact: true }).click();
    for (const agent of agents) {
      const detail = page.locator('.agent-row').filter({ has: page.getByRole('link', { name: agent.name, exact: true }) }).getByRole('link', { name: new RegExp(messages.common.view) });
      await expect(detail).toHaveAccessibleName(messages.common.agentAction.replace('{{action}}', messages.common.view).replace('{{agent}}', agent.name));
    }
  });

  test(`${locale}: Overview action opens the existing Model area and language changes retain it`, async ({ page }) => {
    await start(page, locale, 'agents/claude');
    const messages = resources[locale];
    const action = page.getByRole('button', { name: messages.agents.compareConfiguration });
    await action.focus();
    expect(await action.evaluate(element => getComputedStyle(element).outlineStyle)).toBe('solid');
    await page.keyboard.press('Enter');
    const modelTab = page.getByRole('tab', { name: messages.agents.modelTab, exact: true });
    await expect(modelTab).toHaveAttribute('aria-selected', 'true');
    await expect(modelTab).toBeFocused();
    await expect(page.getByRole('tabpanel')).toContainText(messages.agents.recordLimit);
    const otherLocale = locale === 'zh-CN' ? 'en-US' : 'zh-CN';
    await page.locator('#shell-language').selectOption(otherLocale);
    await expect(page.getByRole('tab', { name: resources[otherLocale].agents.modelTab, exact: true })).toHaveAttribute('aria-selected', 'true');
    await expect(page.getByRole('tabpanel')).toContainText(resources[otherLocale].agents.recordLimit);
    expect(new URL(page.url()).hash).toBe('#/agents/claude');
  });
}

test('collapsed navigation retains named routes and both persistent shell preferences', async ({ page }) => {
  await start(page, 'en-US');
  await page.getByRole('button', { name: 'Collapse navigation', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Expand navigation', exact: true })).toBeVisible();
  await page.locator('#shell-theme').selectOption('dark');
  await page.locator('#shell-language').selectOption('zh-CN');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await page.getByRole('navigation', { name: '主导航' }).getByRole('link', { name: '模型', exact: true }).click();
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('模型');
  await page.reload();
  await expect(page.locator('#shell-theme')).toHaveValue('dark');
  await expect(page.locator('#shell-language')).toHaveValue('zh-CN');
  await page.locator('#shell-theme').selectOption('system');
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
});

test('both scripts and exact model values render with the bundled fonts', async ({ page }, testInfo) => {
  const remoteFonts: string[] = [];
  page.on('request', request => { if (request.resourceType() === 'font' && new URL(request.url()).hostname !== '127.0.0.1') remoteFonts.push(request.url()); });
  await start(page, 'zh-CN');
  const cjk = [...new Set(Object.values(resources['zh-CN']).flatMap(namespace => Object.values(namespace)).join('').match(/[\u3400-\u9fff]/g))].join('');
  await page.evaluate(cjk => {
    const specimen = document.createElement('p');
    specimen.id = 'font-coverage-evidence';
    specimen.textContent = cjk;
    specimen.style.fontFamily = '"Nexus Han"';
    document.querySelector('main')!.append(specimen);
  }, cjk);
  await page.evaluate(() => document.fonts.ready);
  const session = await page.context().newCDPSession(page);
  await session.send('DOM.enable');
  await session.send('CSS.enable');
  const { root } = await session.send('DOM.getDocument');
  const { nodeId } = await session.send('DOM.querySelector', { nodeId: root.nodeId, selector: '#font-coverage-evidence' });
  const { fonts } = await session.send('CSS.getPlatformFontsForNode', { nodeId });
  expect(fonts.length).toBe(1);
  expect(fonts[0].familyName).toMatch(/Source Han Sans/);
  expect(fonts[0].isCustomFont).toBe(true);
  expect(fonts[0].glyphCount).toBe(cjk.length);
  for (const font of ['Nexus Latin', 'Nexus Han', 'Nexus Mono']) expect(await page.evaluate(font => document.fonts.check(`400 14px "${font}"`), font)).toBe(true);
  expect(remoteFonts).toEqual([]);
  await testInfo.attach('font-evidence', { body: JSON.stringify({ uniqueCjk: cjk.length, fonts, remoteFonts }, null, 2), contentType: 'application/json' });
});
