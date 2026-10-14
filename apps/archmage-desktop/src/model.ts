export type RunRequest = { repository: string; goal: string; teamId?: string };
export type ProviderKind = "codex" | "claude" | "openclaw" | "hermes";
export type ConnectionState = "unverified" | "ready" | "failed";
export type CapabilityKey = "designer" | "coder" | "tester" | "reviewer";

export type TeamNode = { capability: CapabilityKey; x: number; y: number };
export type TeamEdge = { from: CapabilityKey; to: CapabilityKey };
export type TeamGraph = { nodes: TeamNode[]; edges: TeamEdge[]; mode?: "legacy" | "flow"; entry?: CapabilityKey | null; terminal?: CapabilityKey | null };
export type Team = { id: string; name: string; revision: number; graph: TeamGraph };
export type AppView = "runs" | "agents" | "capabilities" | "teams";

export function shouldConfirmCapabilityExit(current: AppView, next: AppView, dirty: boolean): boolean {
  return current === "capabilities" && next !== "capabilities" && dirty;
}

export type AdapterDescriptor = {
  kind: ProviderKind;
  displayName: string;
  supported: boolean;
};

export type AgentInstance = {
  id: string;
  providerKind: ProviderKind;
  displayName: string;
  executablePath: string | null;
  model: string | null;
  enabled: boolean;
  connectionState: ConnectionState;
  discoveredVersion: string | null;
  lastVerifiedAt: string | null;
  lastError: string | null;
  verificationThreadId: string | null;
};

export type AgentInstanceInput = Pick<
  AgentInstance,
  "id" | "providerKind" | "displayName" | "executablePath" | "model" | "enabled"
>;

export type CapabilityBinding = {
  capability: CapabilityKey;
  agentInstanceId: string;
};

export type CapabilityGuidance = { name: string; purpose: string };

export type CapabilityConfiguration = {
  key: CapabilityKey;
  displayName: string;
  description: string;
  extraInstructions: string;
  skills: CapabilityGuidance[];
  tools: CapabilityGuidance[];
  agentInstanceId: string;
  revision: number;
  promptVersion: string;
};

export type CapabilityConfigurationInput = Omit<CapabilityConfiguration, "promptVersion">;

export function capabilityDraft(current: CapabilityConfiguration): CapabilityConfigurationInput {
  const { promptVersion: _promptVersion, ...input } = current;
  return { ...input, skills: current.skills.map((item) => ({ ...item })), tools: current.tools.map((item) => ({ ...item })) };
}

export function isCapabilityDirty(current: CapabilityConfiguration, draft: CapabilityConfigurationInput): boolean {
  return JSON.stringify(capabilityDraft(current)) !== JSON.stringify(draft);
}

export function capabilityDisplayName(configuration: Pick<CapabilityConfiguration, "displayName"> | undefined, key: CapabilityKey): string {
  return configuration?.displayName || key[0].toUpperCase() + key.slice(1);
}

export type TopologyEdge = {
  from: CapabilityKey;
  to: CapabilityKey;
  kind: "execution" | "context";
};

export function groupTopologyEdges(edges: TopologyEdge[]) {
  return {
    execution: edges.filter((edge) => edge.kind === "execution"),
    context: edges.filter((edge) => edge.kind === "context"),
  };
}

export type CapabilityManagementSnapshot = {
  configurations: CapabilityConfiguration[];
  instances: AgentInstance[];
  edges: TopologyEdge[];
};

export type AgentManagementSnapshot = {
  adapters: AdapterDescriptor[];
  instances: AgentInstance[];
  bindings: CapabilityBinding[];
};

export type FrozenAgentBinding = {
  agentInstanceId: string;
  displayName: string;
  providerKind: ProviderKind;
  executablePath: string | null;
  model: string | null;
};

export type NodeSnapshot = {
  id: string;
  key: CapabilityKey;
  state: string;
  revision: number;
  result: {
    attemptId: string;
    threadId: string;
    resultArtifactId: string;
    changesetArtifactId: string | null;
    output: Record<string, unknown>;
  } | null;
};

export type RunSnapshot = {
  id: string;
  goal: string;
  repository: string;
  checks: { checkId: string; command: string }[];
  agentBindings: Record<NodeSnapshot["key"], FrozenAgentBinding>;
  capabilityConfigurations: Record<CapabilityKey, CapabilityConfiguration>;
  team: Team | null;
  state: string;
  outcome: string | null;
  revision: number;
  createdAt: string;
  lastEventSequence: number;
  nodes: NodeSnapshot[];
  consultations: {
    id: string;
    runId: string;
    sourceNodeId: string;
    sourceAttemptId: string;
    targetCapability: CapabilityKey;
    round: number;
    decision: "approved" | "changes_requested";
    summary: string;
    suggestions: string[];
    result: Record<string, unknown>;
    resultSha256: string;
    threadId: string;
  }[];
  consultationProcesses: { sourceNodeId: string; sourceAttemptId: string; targetCapability: CapabilityKey }[];
  flowDeliveries?: {
    id: string; runId: string; sequence: number; sourceAttemptId: string | null;
    target: CapabilityKey; payloadArtifactId: string | null; state: string;
    attemptId: string | null; routing: { decision: string; next: CapabilityKey[] } | null;
  }[];
  workspace: string | null;
};

export type RunSummary = Pick<
  RunSnapshot,
  "id" | "goal" | "state" | "createdAt" | "lastEventSequence"
>;

export type RunEvent = {
  run_id: string;
  sequence: number;
  event_type: string;
  node_id: string | null;
  attempt_id: string | null;
  schema_version: number;
  payload: Record<string, unknown>;
  created_at: string;
};

export type TimelineState = {
  events: RunEvent[];
  lastSequence: number;
  needsRefresh: boolean;
};

export const initialTimelineState: TimelineState = {
  events: [],
  lastSequence: 0,
  needsRefresh: false,
};

export function toRunRequest(repository: string, goal: string, teamId?: string): RunRequest {
  return { repository: repository.trim(), goal: goal.trim(), ...(teamId ? { teamId } : {}) };
}

export const connectionLabel = (state: ConnectionState) =>
  ({ unverified: "连接未验证", ready: "可用", failed: "连接失败" })[state];

export function runBindingIssues(snapshot: AgentManagementSnapshot): string[] {
  const capabilities: NodeSnapshot["key"][] = ["designer", "coder", "tester", "reviewer"];
  return capabilities.flatMap((capability) => {
    const binding = snapshot.bindings.find((item) => item.capability === capability);
    if (!binding) return [`${capability}: 未绑定 Agent`];
    const instance = snapshot.instances.find((item) => item.id === binding.agentInstanceId);
    if (!instance) return [`${capability}: Agent 实例不存在`];
    if (!instance.enabled) return [`${capability}: Agent 实例已禁用`];
    if (instance.providerKind !== "codex") return [`${capability}: Provider 尚未支持`];
    return [];
  });
}

export function reduceEvent(state: TimelineState, event: RunEvent): TimelineState {
  if (event.sequence <= state.lastSequence) return state;
  return {
    events: [...state.events, event],
    lastSequence: event.sequence,
    needsRefresh: state.lastSequence > 0 && event.sequence !== state.lastSequence + 1,
  };
}

export const isActive = (state?: string) =>
  state === "created" || state === "provisioning" || state === "running";
