import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { RunEvent, RunRequest, RunSnapshot, RunSummary } from "./model";

export const api = {
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
