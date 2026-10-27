import { expect, it } from 'vitest';
import { readinessNodes } from '../../src/app/readiness';
import { agents } from '../../src/data/fixtures';

const states = (id: string) => readinessNodes(agents.find(agent => agent.id === id)!).map(node => node.state);

it('derives readiness-line nodes only from recorded checks and configuration records', () => {
  expect(states('codex')).toEqual(['pass', 'pass', 'pass', 'set', 'unchecked']);
  // A pending Profile revision is drawn as pending, not as a pass or a failure.
  expect(states('claude')).toEqual(['pass', 'pass', 'pass', 'pending', 'unchecked']);
  // Nothing after a failed installation is presumed to pass.
  expect(states('gemini')).toEqual(['fail', 'unreached', 'unreached', 'unreached', 'unchecked']);
  expect(states('opencode')).toEqual(['pass', 'pass', 'pass', 'set', 'unchecked']);
  for (const agent of agents) expect(readinessNodes(agent).at(-1)).toEqual({ key: 'online', state: 'unchecked' });
});
