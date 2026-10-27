import { useCallback, useState } from 'react';
import { backend, NexusError, type ActionRequest, type OperationResult, type PlanView } from './backend';
import { useNexus } from './nexus-context';

// Preview → native confirmation → verified result. The renderer never approves anything itself:
// "Confirm" asks Rust to show an OS dialog built from the same immutable plan.
export function useActionFlow() {
  const { refresh, reloadModels } = useNexus();
  const [plan, setPlan] = useState<PlanView | null>(null);
  const [result, setResult] = useState<OperationResult | null>(null);
  const [error, setError] = useState<NexusError | null>(null);
  const [busy, setBusy] = useState<'preparing' | 'confirming' | null>(null);
  const prepare = useCallback(async (request: ActionRequest) => {
    setBusy('preparing'); setError(null); setResult(null);
    try { setPlan(await backend.prepare(request)); }
    catch (caught) { setPlan(null); setError(caught instanceof NexusError ? caught : null); }
    finally { setBusy(null); }
  }, []);
  const confirm = useCallback(async () => {
    if (!plan) return;
    setBusy('confirming'); setError(null);
    try { setResult(await backend.confirm(plan)); setPlan(null); await refresh(); await reloadModels(); }
    catch (caught) { setError(caught instanceof NexusError ? caught : null); setPlan(null); }
    finally { setBusy(null); }
  }, [plan, refresh, reloadModels]);
  const discard = useCallback(async () => {
    if (plan) await backend.discard(plan.id).catch(() => undefined);
    setPlan(null); setError(null); setResult(null);
  }, [plan]);
  return { plan, result, error, busy, prepare, confirm, discard };
}
export type ActionFlow = ReturnType<typeof useActionFlow>;
