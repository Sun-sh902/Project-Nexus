import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { resources } from '../../src/app/resources';
import { navigation, parseRoute } from '../../src/app/routes';
import { agents, extensions, findModel, findProfile, findProvider } from '../../src/data/fixtures';
import { formatNumber, formatTimestamp } from '../../src/app/format';
import ts from 'typescript';

describe('offline translations', () => {
  it('keeps ordinary JSX text and accessible labels in localization resources', () => {
    const files = ['src/App.tsx', ...['src/pages', 'src/components'].flatMap(folder => readdirSync(folder).filter(file => file.endsWith('.tsx')).map(file => `${folder}/${file}`))];
    const violations: string[] = [];
    for (const file of files) {
      const source = ts.createSourceFile(file, readFileSync(file, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
      function visit(node: ts.Node) {
        if (ts.isJsxText(node) && node.text.trim() && !['N', 'Project Nexus'].includes(node.text.trim())) violations.push(`${file}: ${node.text.trim()}`);
        if (ts.isJsxAttribute(node) && ['aria-label', 'title', 'placeholder', 'alt'].includes(node.name.getText(source)) && node.initializer && ts.isStringLiteral(node.initializer)) violations.push(`${file}: ${node.name.getText(source)}`);
        ts.forEachChild(node, visit);
      }
      visit(source);
    }
    expect(violations).toEqual([]);
  });
  it('has exact namespace/key and interpolation parity with no empty messages', () => {
    expect(Object.keys(resources['zh-CN']).sort()).toEqual(Object.keys(resources['en-US']).sort());
    for (const namespace of Object.keys(resources['en-US']) as (keyof typeof resources['en-US'])[]) {
      const english = resources['en-US'][namespace];
      const chinese = resources['zh-CN'][namespace];
      expect(Object.keys(chinese).sort()).toEqual(Object.keys(english).sort());
      for (const key of Object.keys(english)) {
        const en = (english as Record<string, string>)[key];
        const zh = (chinese as Record<string, string>)[key];
        expect(en.trim().length).toBeGreaterThan(0);
        expect(zh.trim().length).toBeGreaterThan(0);
        expect(zh.match(/\{\{.*?\}\}/g)?.sort() ?? []).toEqual(en.match(/\{\{.*?\}\}/g)?.sort() ?? []);
      }
    }
  });
  it('provides every fixture content key in both languages', () => {
    for (const locale of ['en-US', 'zh-CN'] as const) {
      const agentStrings = resources[locale].agents as Record<string, string>;
      for (const agent of agents) for (const suffix of ['Summary', 'Issue', 'Next']) expect(agentStrings[`${agent.id}${suffix}`]).toBeTruthy();
      for (const extension of extensions) for (const key of [extension.nameKey, extension.permissionKey]) expect((resources[locale].extensions as Record<string, string>)[key]).toBeTruthy();
    }
  });
});
describe('navigation and fixture relationships', () => {
  it('handles primary/detail routes, browser hashes and unsupported routes explicitly', () => {
    expect(parseRoute('')).toEqual({ page: 'home' });
    for (const page of navigation) expect(parseRoute(`#/${page}`)).toEqual({ page });
    expect(parseRoute('#/agents/codex')).toEqual({ page: 'agents', agentId: 'codex' });
    for (const path of ['#/shell', '#/agents/codex/exec', '#/agents/../settings']) expect(parseRoute(path)).toEqual({ page: 'notFound' });
  });
  it('keeps installation independent and resolves model/profile/provider references', () => {
    expect(agents.find(agent => !agent.installed)?.readiness).toBe('blocked');
    for (const agent of agents) {
      if (agent.modelId) expect(findModel(agent.modelId)).toBeDefined();
      if (agent.profileId) expect(findProfile(agent.profileId)).toBeDefined();
      for (const id of agent.extensionIds) expect(extensions.find(extension => extension.id === id)).toBeDefined();
    }
    for (const agent of agents.filter(agent => agent.readiness === 'ready')) expect(findProvider(findModel(agent.modelId)?.providerId)?.state).toBe('connected');
  });
  it('formats dates/numbers with the chosen locale', () => {
    expect(formatNumber(1200, 'en-US')).toBe(new Intl.NumberFormat('en-US').format(1200));
    expect(formatTimestamp('2026-10-07T00:00:00Z', 'zh-CN')).not.toBe(formatTimestamp('2026-10-07T00:00:00Z', 'en-US'));
  });
});
describe('native scope (Phases 2–7 core backend)', () => {
  it('grants the main window only typed Nexus commands, with no broad core/plugin grants', () => {
    const capability = JSON.parse(readFileSync('src-tauri/capabilities/main.json', 'utf8'));
    expect(capability.windows).toEqual(['main']);
    const nexus = ['overview', 'agent-history', 'models', 'create-provider', 'set-provider-secret', 'delete-provider', 'create-model', 'delete-model', 'save-profile', 'set-assignment', 'prepare-action', 'confirm-plan', 'discard-plan', 'launch-agent', 'set-locale'];
    expect(capability.permissions).toEqual(['allow-app-metadata', 'allow-apply-app-preferences', ...nexus.map(name => `allow-nexus-${name}`)]);
    for (const permission of capability.permissions as string[]) expect(permission).not.toMatch(/:|shell|fs-|http|dialog|process|execute|write-file|fetch/);
    expect(capability.remote).toBeUndefined();
    const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
    expect(config.app.security.capabilities).toEqual(['main']);
    expect(config.app.security.csp).toContain("object-src 'none'");
    expect(config.app.security.csp).not.toContain('https:');
    expect(readdirSync('src-tauri/capabilities')).toEqual(['main.json']);
    const manifest = readFileSync('src-tauri/Cargo.toml', 'utf8');
    // Only the dialog plugin (used from Rust for native confirmation) is allowed; no shell, fs or HTTP plugins.
    expect(manifest.match(/tauri-plugin-[a-z-]+/g) ?? []).toEqual(['tauri-plugin-dialog']);
    expect(manifest).not.toMatch(/reqwest|tauri-plugin-shell|tauri-plugin-fs|tauri-plugin-http/);
  });
});
