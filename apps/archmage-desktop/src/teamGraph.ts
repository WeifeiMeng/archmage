import type { AppView, CapabilityKey, RunSnapshot, Team, TeamEdge, TeamGraph } from "./model";

export const REQUIRED_TEAM_EDGES: TeamEdge[] = [{ from: "designer", to: "coder" }, { from: "coder", to: "tester" }, { from: "tester", to: "reviewer" }];
export const OPTIONAL_TEAM_EDGES: TeamEdge[] = [{ from: "designer", to: "tester" }, { from: "designer", to: "reviewer" }, { from: "coder", to: "reviewer" }];
export const teamEdgeId = (edge: TeamEdge) => `${edge.from}-${edge.to}`;
const clamp = (value: number, max: number) => Math.max(0, Math.min(max, value));

export function toggleOptionalTeamEdge(graph: TeamGraph, edge: TeamEdge): TeamGraph {
  if (!OPTIONAL_TEAM_EDGES.some((allowed) => teamEdgeId(allowed) === teamEdgeId(edge))) return graph;
  const exists = graph.edges.some((item) => teamEdgeId(item) === teamEdgeId(edge));
  return { ...graph, edges: exists ? graph.edges.filter((item) => teamEdgeId(item) !== teamEdgeId(edge)) : [...graph.edges, edge] };
}

export function moveTeamNode(graph: TeamGraph, key: CapabilityKey, x: number, y: number): TeamGraph {
  return { ...graph, nodes: graph.nodes.map((node) => node.capability === key ? { ...node, x: clamp(Math.round(x), 820), y: clamp(Math.round(y), 240) } : node) };
}

export function isTeamDirty(saved: Team | undefined, name: string, graph: TeamGraph | null): boolean {
  return !!graph && (!saved || saved.name !== name || JSON.stringify(saved.graph) !== JSON.stringify(graph));
}

export function shouldConfirmTeamExit(current: AppView, next: AppView, dirty: boolean): boolean {
  return current === "teams" && next !== "teams" && dirty;
}

export function consultationEdgeState(snapshot: RunSnapshot, edge: TeamEdge): string {
  const source = snapshot.nodes.find((node) => node.key === edge.from);
  if (!source) return "等待上游";
  if (source.state !== "succeeded") {
    if (source.state === "failed") return "上游失败";
    if (source.result) return source.state === "running" ? "上游重做中" : "等待上游重做";
    return "等待上游";
  }
  const attempt = source.result?.attemptId;
  if (!attempt) return "待咨询";
  const decision = snapshot.consultations.find((item) => item.sourceAttemptId === attempt && item.targetCapability === edge.to);
  if (decision) return decision.decision === "approved" ? "已认可" : "要求修改";
  if (snapshot.consultationProcesses.some((item) => item.sourceAttemptId === attempt && item.targetCapability === edge.to)) return "咨询中";
  return "待咨询";
}
