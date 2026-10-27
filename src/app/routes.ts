export const navigation = ['home', 'agents', 'models', 'extensions', 'system', 'settings'] as const;
export type Page = typeof navigation[number];
export type Route = { page: Page; agentId?: string } | { page: 'notFound' };
export function parseRoute(hash: string): Route {
  const path = hash.replace(/^#\/?/, '').split('/');
  if (!path[0]) return { page: 'home' };
  if (path[0] === 'agents' && path.length === 2 && /^[a-z-]+$/.test(path[1])) return { page: 'agents', agentId: path[1] };
  if (path.length === 1 && navigation.includes(path[0] as Page)) return { page: path[0] as Page };
  return { page: 'notFound' };
}
