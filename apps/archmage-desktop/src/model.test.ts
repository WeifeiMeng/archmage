import { describe, expect, it } from "vitest";
import { agentInputIsComplete, capabilityDraft, capabilityDisplayName, installGuidanceFor, isCapabilityDirty, groupTopologyEdges, initialTimelineState, isSupportedProvider, providerLabel, reduceEvent, runBindingIssues, selectedTeamForRun, shouldConfirmCapabilityExit, teamRunOverview, toRunRequest, type AgentInstallGuidance, type AgentManagementSnapshot, type CapabilityConfiguration, type Team } from "./model";

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
    expect(shouldConfirmCapabilityExit("capabilities", "settings", true)).toBe(true);
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
  it("looks up installation guidance by provider", () => {
    const guidance: AgentInstallGuidance[] = [{
      providerKind: "codex", platform: "macos", architecture: "aarch64",
      supported: true, detected: false, prerequisite: null,
      command: "install codex", documentationUrl: "https://example.com/codex",
      unsupportedReason: null,
    }];
    expect(installGuidanceFor(guidance, "codex")?.platform).toBe("macos");
    expect(installGuidanceFor(guidance, "hermes")).toBeUndefined();
  });
  it("requires an explicit provider route and model for DeepSeek Harness", () => {
    expect(agentInputIsComplete({ providerKind: "codex", providerRoute: null, model: null })).toBe(true);
    expect(agentInputIsComplete({ providerKind: "deepseek_harness", providerRoute: null, model: "deepseek-v4-flash" })).toBe(false);
    expect(agentInputIsComplete({ providerKind: "deepseek_harness", providerRoute: "deepseek-official", model: "deepseek-v4-flash" })).toBe(true);
  });
  it("treats Codex, DeepSeek Harness, and Qoder as supported providers", () => {
    expect(isSupportedProvider("codex")).toBe(true);
    expect(isSupportedProvider("deepseek_harness")).toBe(true);
    expect(isSupportedProvider("qoder")).toBe(true);
    expect(isSupportedProvider("hermes")).toBe(false);
    expect(providerLabel("deepseek_harness")).toBe("DeepSeek Harness");
  });
  it("blocks a run when a capability has no enabled supported instance", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [],
      instances: [{ id: "codex-default", providerKind: "codex", displayName: "Codex", executablePath: null, providerRoute: null, model: null, agentName: null, providerConfig: {}, enabled: false, connectionState: "unverified", discoveredVersion: null, lastVerifiedAt: null, lastError: null, verificationThreadId: null }],
      bindings: [{ capability: "designer", agentInstanceId: "codex-default" }],
    };
    expect(runBindingIssues(snapshot)).toEqual(["designer: Agent 实例已禁用", "coder: 未绑定 Agent", "tester: 未绑定 Agent", "reviewer: 未绑定 Agent"]);
  });
  it("checks only members of the selected team", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [],
      instances: [{ id: "codex", providerKind: "codex", displayName: "Codex", executablePath: null, providerRoute: null, model: null, agentName: null, providerConfig: {}, enabled: true, connectionState: "ready", discoveredVersion: null, lastVerifiedAt: null, lastError: null, verificationThreadId: null }],
      bindings: [{ capability: "coder", agentInstanceId: "codex" }, { capability: "reviewer", agentInstanceId: "codex" }],
    };
    expect(runBindingIssues(snapshot, ["coder", "reviewer"])).toEqual([]);
    expect(runBindingIssues(snapshot, ["coder", "tester"])).toEqual(["tester: 未绑定 Agent"]);
  });
  it("allows a capability bound to DeepSeek Harness", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [],
      instances: [{ id: "dsh", providerKind: "deepseek_harness", displayName: "DSH", executablePath: null, providerRoute: "deepseek-official", model: "deepseek-v4-flash", agentName: null, providerConfig: {}, enabled: true, connectionState: "ready", discoveredVersion: null, lastVerifiedAt: null, lastError: null, verificationThreadId: null }],
      bindings: [{ capability: "coder", agentInstanceId: "dsh" }],
    };
    expect(runBindingIssues(snapshot, ["coder"])).toEqual([]);
  });
  it("allows a capability bound to Qoder", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [],
      instances: [{ id: "qoder", providerKind: "qoder", displayName: "Qoder", executablePath: null, providerRoute: null, model: null, agentName: "coder", providerConfig: { mcpServers: [] }, enabled: true, connectionState: "ready", discoveredVersion: "1.1.38", lastVerifiedAt: null, lastError: null, verificationThreadId: "session-1" }],
      bindings: [{ capability: "coder", agentInstanceId: "qoder" }],
    };
    expect(runBindingIssues(snapshot, ["coder"])).toEqual([]);
  });
});

describe("new Run Team overview", () => {
  it("does not use a cached team while its list reloads or has failed", () => {
    const team: Team = { id: "t1", name: "旧版本", revision: 1, graph: { nodes: [], edges: [] } };
    expect(selectedTeamForRun([team], "t1", "loading")).toBeNull();
    expect(selectedTeamForRun([team], "t1", "failed")).toBeNull();
    expect(selectedTeamForRun([team], "t1", "ready")).toEqual(team);
  });
  it("describes selected flow members, routes, entry, and terminal", () => {
    const team: Team = { id: "team-1", name: "代码审阅", revision: 2, graph: {
      mode: "flow", entry: "coder", terminal: "reviewer",
      nodes: [{ capability: "coder", x: 0, y: 0 }, { capability: "reviewer", x: 300, y: 0 }],
      edges: [{ from: "coder", to: "reviewer" }, { from: "reviewer", to: "coder" }],
    } };
    expect(teamRunOverview(team)).toEqual({
      members: ["Coder", "Reviewer"],
      routes: ["Coder → Reviewer", "Reviewer → Coder"],
      detail: "入口 Coder · 可完成 Reviewer",
    });
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
