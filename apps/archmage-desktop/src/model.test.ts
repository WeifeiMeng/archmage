import { describe, expect, it } from "vitest";
import { capabilityDraft, capabilityDisplayName, isCapabilityDirty, groupTopologyEdges, initialTimelineState, reduceEvent, runBindingIssues, shouldConfirmCapabilityExit, toRunRequest, type AgentManagementSnapshot, type CapabilityConfiguration } from "./model";

describe("capability topology", () => {
  it("keeps execution dependencies separate from extra context handoffs", () => {
    const edges = groupTopologyEdges([
      { from: "designer", to: "coder", kind: "execution" },
      { from: "designer", to: "tester", kind: "context" },
    ]);
    expect(edges.execution).toHaveLength(1);
    expect(edges.context).toEqual([{ from: "designer", to: "tester", kind: "context" }]);
  });
  it("uses the frozen Run label instead of the current global label", () => {
    expect(capabilityDisplayName({ displayName: "QA" }, "tester")).toBe("QA");
    expect(capabilityDisplayName(undefined, "tester")).toBe("Tester");
  });
  it("guards sidebar navigation away from a dirty Capability editor", () => {
    expect(shouldConfirmCapabilityExit("capabilities", "runs", true)).toBe(true);
    expect(shouldConfirmCapabilityExit("capabilities", "agents", false)).toBe(false);
    expect(shouldConfirmCapabilityExit("runs", "agents", true)).toBe(false);
  });
  it("keeps role, Skill, Tool, and Agent changes in one draft", () => {
    const current: CapabilityConfiguration = {
      key: "designer", displayName: "Designer", description: "内置职责",
      extraInstructions: "existing instruction", agentInstanceId: "codex-a",
      skills: [{ name: "brainstorming", purpose: "Explore ideas" }],
      tools: [],
      revision: 3, promptVersion: "v1",
    };
    const draft = capabilityDraft(current);
    expect(isCapabilityDirty(current, draft)).toBe(false);
    draft.skills[0].purpose = "Explore better ideas";
    expect(isCapabilityDirty(current, draft)).toBe(true);
    draft.skills[0].purpose = "Explore ideas";
    draft.extraInstructions = "New role guidance";
    expect(isCapabilityDirty(current, draft)).toBe(true);
    draft.extraInstructions = "existing instruction";
    draft.tools.push({ name: "web_search", purpose: "Research unfamiliar facts" });
    expect(isCapabilityDirty(current, draft)).toBe(true);
    draft.tools.pop();
    draft.agentInstanceId = "codex-b";
    expect(isCapabilityDirty(current, draft)).toBe(true);
    draft.extraInstructions = "New role guidance";
    draft.tools.push({ name: "web_search", purpose: "Research unfamiliar facts" });
    expect(isCapabilityDirty(current, draft)).toBe(true);
    expect(draft).toEqual({
      key: "designer", displayName: "Designer", description: "内置职责",
      extraInstructions: "New role guidance", agentInstanceId: "codex-b", revision: 3,
      skills: [{ name: "brainstorming", purpose: "Explore ideas" }],
      tools: [{ name: "web_search", purpose: "Research unfamiliar facts" }],
    });
  });
});

describe("run request", () => {
  it("serializes the selected team with the repository and goal", () => {
    expect(toRunRequest(" /repo ", " goal ", "team-1")).toEqual({
      repository: "/repo",
      goal: "goal",
      teamId: "team-1",
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
