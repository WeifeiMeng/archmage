import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentInstanceInput,
  AgentManagementSnapshot,
  NodeSnapshot,
  RunEvent,
  RunRequest,
  RunSnapshot,
  RunSummary,
} from "./model";

export const api = {
  doctor: (online = false) => invoke<Record<string, unknown>>("doctor", { online }),
  getAgentManagement: () =>
    invoke<AgentManagementSnapshot>("get_agent_management"),
  saveAgentInstance: (input: AgentInstanceInput) =>
    invoke<AgentManagementSnapshot>("save_agent_instance", { input }),
  discoverAgentInstance: (agentInstanceId: string) =>
    invoke<AgentManagementSnapshot>("discover_agent_instance", { agentInstanceId }),
  verifyAgentInstance: (agentInstanceId: string) =>
    invoke<AgentManagementSnapshot>("verify_agent_instance", { agentInstanceId }),
  deleteAgentInstance: (agentInstanceId: string) =>
    invoke<AgentManagementSnapshot>("delete_agent_instance", { agentInstanceId }),
  bindCapability: (capability: NodeSnapshot["key"], agentInstanceId: string) =>
    invoke<AgentManagementSnapshot>("bind_capability", { capability, agentInstanceId }),
  listRuns: () => invoke<RunSummary[]>("list_runs"),
  activeRunIds: () => invoke<string[]>("active_run_ids"),
  snapshot: (runId: string) => invoke<RunSnapshot>("get_run_snapshot", { runId }),
  events: (runId: string, afterSequence = 0) =>
    invoke<RunEvent[]>("list_events", { runId, afterSequence }),
  startRun: (request: RunRequest) => invoke<string>("start_run", { request }),
  cancelRun: (runId: string) => invoke<void>("cancel_run", { runId }),
  revealWorkspace: (runId: string) => invoke<void>("reveal_workspace", { runId }),
  onRunEvent: (handler: (runId: string) => void): Promise<UnlistenFn> =>
    listen<{ runId: string; eventSequence: number }>("run-event", (event) =>
      handler(event.payload.runId),
    ),
};
