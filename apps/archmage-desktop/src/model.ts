export type RunRequest = { repository: string; goal: string };
export type ProviderKind = "codex" | "claude" | "openclaw" | "hermes";
export type ConnectionState = "unverified" | "ready" | "failed";
export type CapabilityKey = "designer" | "coder" | "tester" | "reviewer";

export type TeamNode = { capability: CapabilityKey; x: number; y: number };
export type TeamEdge = { from: CapabilityKey; to: CapabilityKey };
export type TeamGraph = { nodes: TeamNode[]; edges: TeamEdge[] };
export type Team = { id: string; name: string; revision: number; graph: TeamGraph };
export type AppView = "runs" | "agents" | "capabilities";

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

export type CapabilityConfiguration = {
  key: CapabilityKey;
  displayName: string;
  description: string;
  extraInstructions: string;
  agentInstanceId: string;
  revision: number;
  promptVersion: string;
};

export type CapabilityConfigurationInput = Omit<CapabilityConfiguration, "promptVersion">;

export function bindingOnlyUpdate(current: CapabilityConfiguration, agentInstanceId: string): CapabilityConfigurationInput {
  const { promptVersion: _promptVersion, ...input } = current;
  return { ...input, agentInstanceId };
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
  state: string;
  outcome: string | null;
  revision: number;
  createdAt: string;
  lastEventSequence: number;
  nodes: NodeSnapshot[];
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

export function toRunRequest(repository: string, goal: string): RunRequest {
  return { repository: repository.trim(), goal: goal.trim() };
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
