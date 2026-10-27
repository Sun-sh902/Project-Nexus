import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';

function luminance(hex: string) {
  const rgb = hex.replace('#', '').match(/../g)!.map(value => Number.parseInt(value, 16) / 255).map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
}
function contrast(first: string, second: string) {
  const values = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return (values[0] + 0.05) / (values[1] + 0.05);
}
it('semantic text/status/focus colors meet composed theme thresholds', () => {
  const css = readFileSync('src/styles.css', 'utf8');
  for (const pattern of [/:root, \[data-theme='light'\] \{([^}]+)\}/, /:root\[data-theme='dark'\], \[data-theme='dark'\] \{([^}]+)\}/]) {
    const tokens = Object.fromEntries([...css.match(pattern)![1].matchAll(/--([\w-]+):\s*(#[\da-f]+)/gi)].map(match => [match[1], match[2]]));
    for (const surface of ['canvas', 'sidebar', 'sheet', 'raised', 'sunken', 'hover', 'selected']) {
      for (const foreground of ['ink', 'ink-2', 'ink-3', 'brand', 'ready-ink', 'attention-ink', 'blocked-ink', 'unknown-ink']) {
        expect(contrast(tokens[foreground], tokens[surface]), `${foreground} on ${surface}`).toBeGreaterThanOrEqual(4.5);
      }
      expect(contrast(tokens.focus, tokens[surface])).toBeGreaterThanOrEqual(3);
      expect(contrast(tokens.control, tokens[surface]), `control on ${surface}`).toBeGreaterThanOrEqual(3);
    }
    expect(contrast(tokens.canvas, tokens.ink), 'primary button').toBeGreaterThanOrEqual(4.5);
    expect(contrast(tokens.control, tokens.raised)).toBeGreaterThanOrEqual(3);
  }
});
