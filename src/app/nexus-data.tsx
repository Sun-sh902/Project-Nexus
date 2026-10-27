import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { backend, liveBackend, NexusError, type ModelsView, type Overview } from './backend';
import { agentView, activityViews, toolViews } from '../data/live';
import { demoActivity, demoAgents, demoExtensions, demoTools } from '../data/demo';
import { NexusContext, type DataMode, type NexusData } from './nexus-context';

// Single data boundary for pages. Inside the desktop runtime every value comes from Rust; the
// fixture set is used only in explicit demo mode (browser preview / UI tests), which the shell
// labels as such. A failed scan shows an error state, never fixture data.

export function NexusDataProvider({ children }: { children: ReactNode }) {
  const mode: DataMode = liveBackend() ? 'live' : 'demo';
  const [overview, setOverview] = useState<Overview | null>(null);
  const [models, setModels] = useState<ModelsView | null>(null);
  const [status, setStatus] = useState<NexusData['status']>(mode === 'live' ? 'loading' : 'ready');
  const [error, setError] = useState<NexusError | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const generation = useRef(0);

  const load = useCallback(async (refresh: boolean) => {
    if (mode !== 'live') return;
    const mine = ++generation.current;
    setRefreshing(true);
    try {
      const next = await backend.overview(refresh);
      if (mine !== generation.current) return;
      if (next.contractVersion !== 1) throw new NexusError({ code: 'ipc.contract_mismatch', params: [], retryable: false });
      setOverview(next);
      setStatus('ready');
      setError(null);
    } catch (caught) {
      if (mine !== generation.current) return;
      setError(caught instanceof NexusError ? caught : new NexusError({ code: 'ipc.unavailable', params: [], retryable: true }));
      setStatus(current => current === 'ready' ? 'ready' : 'error');
    } finally {
      if (mine === generation.current) setRefreshing(false);
    }
  }, [mode]);

  const reloadModels = useCallback(async () => {
    if (mode !== 'live') return;
    try { setModels(await backend.models()); } catch (caught) { setError(caught instanceof NexusError ? caught : null); }
  }, [mode]);

  // Deferred so the scan's state updates never run synchronously inside the effect body.
  useEffect(() => { void Promise.resolve().then(() => load(true)); }, [load]);

  const value = useMemo<NexusData>(() => {
    const agents = mode === 'demo' ? demoAgents() : overview ? overview.agents.map(agentView) : [];
    return {
      mode, status, error, overview, agents,
      extensions: mode === 'demo' ? demoExtensions() : agents.flatMap(agent => agent.extensions),
      tools: mode === 'demo' ? demoTools() : overview ? toolViews(overview) : [],
      activity: mode === 'demo' ? demoActivity() : overview ? activityViews(overview) : [],
      scannedAt: mode === 'demo' ? '2026-10-07T02:30:00Z' : overview?.generatedAt ?? null,
      refreshing,
      refresh: () => load(true),
      models, reloadModels,
      findAgent: (id: string) => agents.find(agent => agent.id === id),
    };
  }, [mode, status, error, overview, refreshing, load, models, reloadModels]);
  return <NexusContext.Provider value={value}>{children}</NexusContext.Provider>;
}

