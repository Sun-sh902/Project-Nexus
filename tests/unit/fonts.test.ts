import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';

it('ships the verified font bytes, pinned provenance and OFL licenses locally', () => {
  const records = JSON.parse(readFileSync('public/fonts/sources.json', 'utf8')) as { file: string; sha256: string; bytes: number; revision: string }[];
  const css = readFileSync('src/styles.css', 'utf8');
  expect(records.map(record => record.file).sort()).toEqual(['plex-mono.woff2', 'plex-sans.woff2', 'source-han-serif.woff2', 'source-han.woff2', 'source-serif-italic.woff2', 'source-serif.woff2']);
  for (const record of records) {
    const bytes = readFileSync(`public/fonts/${record.file}`);
    expect(bytes.length).toBe(record.bytes);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(record.sha256);
    expect(record.revision).toMatch(/^[a-f0-9]{40}$/);
    expect(css).toContain(`url('/fonts/${record.file}')`);
  }
  for (const license of ['han-serif.txt', 'han.txt', 'plex-mono.txt', 'plex-sans.txt', 'source-serif.txt']) expect(readFileSync(`public/fonts/${license}`, 'utf8')).toContain('SIL OPEN FONT LICENSE Version 1.1');
  expect(css).not.toMatch(/url\(['"]?https?:/);
  expect(css).toContain('font-synthesis: none');
});
