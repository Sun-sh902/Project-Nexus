import { expect, it } from 'vitest';
import { demoAgents } from '../../src/data/demo';

const states = (id: string) => demoAgents().find(agent => agent.id === id)!.nodes.map(node => node.state);

it('derives demo readiness-line nodes only from recorded fixture checks and configuration records', () => {
  expect(states('codex')).toEqual(['pass', 'pass', 'pass', 'set', 'unchecked']);
  // A pending Profile revision is drawn as pending, not as a pass or a failure.
  expect(states('claude')).toEqual(['pass', 'pass', 'pass', 'pending', 'unchecked']);
  // Nothing after a failed installation is presumed to pass.
  expect(states('gemini')).toEqual(['fail', 'unreached', 'unreached', 'unreached', 'unchecked']);
  expect(states('opencode')).toEqual(['pass', 'pass', 'pass', 'set', 'unchecked']);
  for (const agent of demoAgents()) expect(agent.nodes.at(-1)?.state).toEqual('unchecked');
});
