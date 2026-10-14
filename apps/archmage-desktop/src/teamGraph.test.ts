import { describe, expect, it } from "vitest";
import { consultationEdgeState, isTeamDirty, moveTeamNode, shouldConfirmTeamExit, toggleTeamCapability, toggleTeamEdge, teamEdgePath, teamGraphIssue } from "./teamGraph";
import type { RunSnapshot, Team, TeamGraph } from "./model";

const graph: TeamGraph = {
  nodes: (["designer", "coder", "tester", "reviewer"] as const).map((capability, index) => ({ capability, x: 100 + index * 200, y: 120 })),
  edges: [{ from: "designer", to: "coder" }, { from: "coder", to: "tester" }, { from: "tester", to: "reviewer" }],
};

describe("team graph editor", () => {
  it("lets a new team select Coder and Reviewer before linking them", () => {
    const empty: TeamGraph = { nodes: [], edges: [] };
    const coder = toggleTeamCapability(empty, "coder");
    const members = toggleTeamCapability(coder, "reviewer");
    expect(teamGraphIssue(members)).toBeTruthy();
    const linked = toggleTeamEdge(members, { from: "coder", to: "reviewer" });
    expect(teamGraphIssue(linked)).toBeNull();
    expect(toggleTeamCapability(linked, "coder").edges).toEqual([]);
  });
  it("supports a flow feedback edge and clears a removed endpoint", () => {
    const empty: TeamGraph = { mode: "flow", entry: "coder", terminal: "reviewer", nodes: [], edges: [] };
    const members = toggleTeamCapability(toggleTeamCapability(empty, "coder"), "reviewer");
    const forward = toggleTeamEdge(members, { from: "coder", to: "reviewer" });
    const cycle = toggleTeamEdge(forward, { from: "reviewer", to: "coder" });
    expect(cycle.edges).toHaveLength(2);
    expect(teamGraphIssue(cycle)).toBeNull();
    const removed = toggleTeamCapability(cycle, "coder");
    expect(removed.edges).toEqual([]);
    expect(removed.entry).toBeNull();
    expect(teamEdgePath(cycle, { from: "coder", to: "reviewer" })).not.toBe(teamEdgePath(cycle, { from: "reviewer", to: "coder" }));
  });
  it("allows changing any selected forward relationship", () => {
    const withoutSpineEdge = toggleTeamEdge(graph, { from: "designer", to: "coder" });
    expect(withoutSpineEdge.edges).toHaveLength(2);
    expect(toggleTeamEdge(withoutSpineEdge, { from: "designer", to: "coder" }).edges).toEqual(expect.arrayContaining(graph.edges));
    expect(toggleTeamEdge(graph, { from: "reviewer", to: "coder" })).toBe(graph);
  });

  it("clamps dragged nodes and warns before leaving dirty edits", () => {
    const moved = moveTeamNode(graph, "designer", -10, 800);
    expect(moved.nodes[0]).toMatchObject({ x: 0, y: 240 });
    const saved: Team = { id: "default", name: "Default", revision: 1, graph };
    expect(isTeamDirty(saved, "Default", moved)).toBe(true);
    expect(shouldConfirmTeamExit("teams", "runs", true)).toBe(true);
    expect(shouldConfirmTeamExit("teams", "teams", true)).toBe(false);
  });

  it("does not present old consultation approval as current while producer reruns", () => {
    const edge = graph.edges[0];
    const snapshot = {
      nodes: [{ key: "designer", state: "pending", result: { attemptId: "old-attempt" } }],
      consultations: [{ sourceAttemptId: "old-attempt", targetCapability: "coder", decision: "approved" }],
      consultationProcesses: [],
    } as unknown as RunSnapshot;
    expect(consultationEdgeState(snapshot, edge)).toBe("等待上游重做");
    snapshot.nodes[0].state = "succeeded";
    expect(consultationEdgeState(snapshot, edge)).toBe("已认可");
  });
});
