import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunWorkbench } from "./App";
import type { RunSnapshot, TimelineState } from "./model";

const snapshot = {
  id: "run-1", goal: "Implement live output", repository: "/repo", checks: [],
  state: "running", outcome: null, revision: 1, createdAt: "2026-11-11T12:00:00Z",
  lastEventSequence: 12, workspace: "/repo", team: null, consultations: [], consultationProcesses: [],
  flowDeliveries: [], flowStepLimit: 24,
  nodes: [{ id: "node-1", key: "coder", state: "running", revision: 1, result: null }],
  agentBindings: { coder: { agentInstanceId: "codex", displayName: "Codex Work", providerKind: "codex", executablePath: null, providerRoute: null, model: null, discoveredVersion: null, agentName: null, providerConfig: {} } },
  capabilityConfigurations: { coder: { key: "coder", displayName: "Coder", description: "", extraInstructions: "", agentInstanceId: "codex", skills: [], tools: [], revision: 1, promptVersion: "v1" } },
} as unknown as RunSnapshot;

const timeline: TimelineState = {
  lastSequence: 12, needsRefresh: false,
  events: [{
    run_id: "run-1", sequence: 12, event_type: "agent.progress", node_id: "node-1", attempt_id: "attempt-1",
    schema_version: 1, payload: { kind: "message", label: "Agent message", text: "Inspecting provider events" }, created_at: "2026-11-11T12:00:01Z",
  }],
};

describe("RunWorkbench", () => {
  it("renders inspector first with Capability identity and timeline last", () => {
    const html = renderToStaticMarkup(<RunWorkbench snapshot={snapshot} timeline={timeline} onRefresh={() => undefined} onCancel={() => undefined} onReveal={() => undefined} />);
    expect(html.indexOf("节点检查器")).toBeLessThan(html.indexOf('role="separator"'));
    expect(html.indexOf('role="separator"')).toBeLessThan(html.indexOf("运行时间线"));
    expect(html).toContain("Coder 正在运行");
    expect(html).toContain("Codex · codex");
    expect(html).toContain("Inspecting provider events");
    expect(html).not.toContain("Codex 正在执行");
  });
});
