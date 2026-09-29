import { describe, expect, it } from "vitest";
import { initialTimelineState, reduceEvent, runBindingIssues, toRunRequest, type AgentManagementSnapshot } from "./model";

describe("run request", () => {
  it("serializes a run from repository and goal only", () => {
    expect(toRunRequest(" /repo ", " goal ")).toEqual({
      repository: "/repo",
      goal: "goal",
    });
  });
});

describe("agent routing", () => {
  it("blocks a run when a capability has no enabled supported instance", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [],
      instances: [{ id: "codex-default", providerKind: "codex", displayName: "Codex", executablePath: null, model: null, enabled: false, connectionState: "unverified", discoveredVersion: null, lastVerifiedAt: null, lastError: null, verificationThreadId: null }],
      bindings: [{ capability: "designer", agentInstanceId: "codex-default" }],
    };
    expect(runBindingIssues(snapshot)).toEqual(["designer: Agent 实例已禁用", "coder: 未绑定 Agent", "tester: 未绑定 Agent", "reviewer: 未绑定 Agent"]);
  });
});

describe("timeline reducer", () => {
  it("requests a snapshot refresh when an event sequence has a gap", () => {
    const state = { ...initialTimelineState, lastSequence: 4 };
    const next = reduceEvent(state, {
      run_id: "run-1",
      sequence: 6,
      event_type: "node.started",
      node_id: "node-1",
      attempt_id: "attempt-1",
      schema_version: 1,
      payload: {},
      created_at: "2026-09-28T00:00:00Z",
    });
    expect(next.needsRefresh).toBe(true);
  });
});
