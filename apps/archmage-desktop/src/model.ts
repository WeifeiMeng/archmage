export type CheckSpec = { checkId: string; command: string };
export type RunRequest = { repository: string; goal: string; checks: CheckSpec[] };

export type NodeSnapshot = {
  id: string;
  key: "designer" | "coder" | "tester" | "reviewer";
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
  checks: CheckSpec[];
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

export function toRunRequest(repository: string, goal: string, rows: string[]): RunRequest {
  const checks = rows
    .map((command) => command.trim())
    .filter(Boolean)
    .map((command, index) => ({ checkId: `check-${index + 1}`, command }));
  return { repository: repository.trim(), goal: goal.trim(), checks };
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
