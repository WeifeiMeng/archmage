import { describe, expect, it } from "vitest";
import { consultationEdgeState, isTeamDirty, moveTeamNode, OPTIONAL_TEAM_EDGES, REQUIRED_TEAM_EDGES, shouldConfirmTeamExit, toggleOptionalTeamEdge } from "./teamGraph";
import type { RunSnapshot, Team, TeamGraph } from "./model";

const graph: TeamGraph = {
  nodes: (["designer", "coder", "tester", "reviewer"] as const).map((capability, index) => ({ capability, x: 100 + index * 200, y: 120 })),
  edges: [...REQUIRED_TEAM_EDGES],
};

describe("team graph editor", () => {
  it("keeps required edges locked and toggles only optional edges", () => {
    expect(toggleOptionalTeamEdge(graph, REQUIRED_TEAM_EDGES[0])).toBe(graph);
    const withReview = toggleOptionalTeamEdge(graph, OPTIONAL_TEAM_EDGES[0]);
    expect(withReview.edges).toHaveLength(4);
    expect(toggleOptionalTeamEdge(withReview, OPTIONAL_TEAM_EDGES[0]).edges).toEqual(REQUIRED_TEAM_EDGES);
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
    const edge = REQUIRED_TEAM_EDGES[0];
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
