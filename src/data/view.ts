import type { AgentId, AgentReport, Finding, McpServer, Readiness, Skill } from '../app/backend';
export type { AgentId, Readiness } from '../app/backend';

// One view model for both sources. Live views come only from Rust reports; demo views come only
// from src/data/fixtures.ts and are shown only outside the desktop runtime (explicit demo mode).
export type Text = { key: string; params?: Record<string, string | number>; literal?: boolean };
export type NodeKey = 'install' | 'runtime' | 'config' | 'model' | 'online';
export type NodeState = 'pass' | 'fail' | 'pending' | 'set' | 'unchecked' | 'unreached';
export type AssignmentKind = 'native' | 'direct' | 'profile';

export interface CheckView { id: string; label: Text; result: Text; state: 'pass' | 'fail' | 'unknown' | 'notApplicable'; required: boolean; evidence: string[] }
export interface ExtensionView {
  id: string; type: 'mcp' | 'skill'; name: Text; literalName: boolean; readiness: Readiness; status: Text; scope: Text; permission: Text; agents: AgentId[];
  live?: { agentId: AgentId; server?: McpServer; skill?: Skill };
}
export interface AgentView {
  id: AgentId; name: string; vendor: string; installed: boolean; readiness: Readiness; version: string | null;
  modelId: string | null; modelSource: Text; assignment: AssignmentKind; providerName: string | null;
  profileName: Text | null; desiredRevision: number | null; appliedRevision: number | null; pending: boolean;
  nodes: { key: NodeKey; state: NodeState; meta: Text }[];
  checks: CheckView[]; requiredPassed: number; requiredTotal: number;
  headline: Text; summary: Text; next: Text; context: Text;
  findings: Finding[]; extensions: ExtensionView[];
  observedAt: string; stale: boolean;
  report?: AgentReport;
}
export interface ToolView { key: string; name: string; version: string | null; readiness: Readiness; status: Text; agentIds: AgentId[]; location: string | null }
export interface ActivityView { id: string; text: Text; time: string }
