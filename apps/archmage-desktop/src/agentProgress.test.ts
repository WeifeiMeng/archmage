import { describe, expect, it } from "vitest";
import { progressForNode } from "./agentProgress";
import type { RunEvent } from "./model";

function event(overrides: Partial<RunEvent>): RunEvent {
  return {
    run_id: "run-1", sequence: 1, event_type: "agent.progress",
    node_id: "node-1", attempt_id: "attempt-1", schema_version: 1,
    payload: {}, created_at: "2026-11-11T12:00:00Z", ...overrides,
  };
}

describe("agent progress", () => {
  it("keeps supported records for the selected node in Runtime order", () => {
    const events = [
      event({ sequence: 13, payload: { kind: "message", label: "Agent message", text: "Done" } }),
      event({ sequence: 11, event_type: "node.started", payload: {} }),
      event({ sequence: 12, payload: { kind: "tool", label: "Run tests", status: "running", text: "cargo test", metadata: { exit_code: 0 } } }),
      event({ sequence: 14, node_id: "node-2", payload: { kind: "notice", label: "Other" } }),
    ];
    expect(progressForNode(events, "node-1")).toEqual([
      { sequence: 12, kind: "tool", label: "Run tests", status: "running", text: "cargo test", metadata: { exit_code: 0 } },
      { sequence: 13, kind: "message", label: "Agent message", status: null, text: "Done", metadata: {} },
    ]);
  });

  it("drops unknown kinds, malformed payloads, and nested metadata", () => {
    const events = [
      event({ payload: { kind: "reasoning", label: "Private", text: "secret" } }),
      event({ sequence: 2, payload: { kind: "tool", label: 42 } }),
      event({ sequence: 3, payload: { kind: "usage", label: "Usage", metadata: { input_tokens: 12, nested: { secret: true } } } }),
    ];
    expect(progressForNode(events, "node-1")).toEqual([
      { sequence: 3, kind: "usage", label: "Usage", status: null, text: null, metadata: { input_tokens: 12 } },
    ]);
  });

  it("shows only the latest attempt for a node", () => {
    const oldAttempt = event({ sequence: 3, attempt_id: "attempt-1", payload: {
      kind: "message", label: "Agent message", text: "old work",
    } });
    const currentAttempt = event({ sequence: 8, attempt_id: "attempt-2", payload: {
      kind: "message", label: "Agent message", text: "current work",
    } });
    expect(progressForNode([currentAttempt, oldAttempt], "node-1").map((item) => item.text)).toEqual(["current work"]);
  });

  it("does not show an old attempt while a new attempt is starting", () => {
    const oldAttempt = event({ sequence: 3, attempt_id: "attempt-1", payload: {
      kind: "message", label: "Agent message", text: "old work",
    } });
    const created = event({ sequence: 8, event_type: "attempt.created", attempt_id: "attempt-2", payload: { number: 2 } });
    expect(progressForNode([oldAttempt, created], "node-1")).toEqual([]);
  });
});
