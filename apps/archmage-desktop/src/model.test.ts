import { describe, expect, it } from "vitest";
import { initialTimelineState, reduceEvent, toRunRequest } from "./model";

describe("run request", () => {
  it("serializes nonblank checks with stable IDs", () => {
    expect(toRunRequest("/repo", "goal", ["npm test", " "]).checks).toEqual([
      { checkId: "check-1", command: "npm test" },
    ]);
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
