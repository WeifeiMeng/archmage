import type { AppView, CapabilityKey, RunSnapshot, Team, TeamEdge, TeamGraph } from "./model";

export const TEAM_CAPABILITIES: CapabilityKey[] = ["designer", "coder", "tester", "reviewer"];
export const teamEdgeId = (edge: TeamEdge) => `${edge.from}-${edge.to}`;
export function teamEdgePath(graph: TeamGraph, edge: TeamEdge): string {
  const from = graph.nodes.find((node) => node.capability === edge.from);
  const to = graph.nodes.find((node) => node.capability === edge.to);
  if (!from || !to) return "";
  const forward = from.x <= to.x;
  const reciprocal = graph.edges.some((other) => other.from === edge.to && other.to === edge.from);
  const x1 = from.x + (forward ? 166 : 0);
  const x2 = to.x + (forward ? 0 : 166);
  const y1 = from.y + 44;
  const y2 = to.y + 44;
  const bend = reciprocal ? (forward ? -52 : 52) : 0;
  const middle = (x1 + x2) / 2;
  return `M ${x1} ${y1} Q ${middle} ${(y1 + y2) / 2 + bend} ${x2} ${y2}`;
}
const clamp = (value: number, max: number) => Math.max(0, Math.min(max, value));

export function toggleTeamCapability(graph: TeamGraph, key: CapabilityKey): TeamGraph {
  if (graph.nodes.some((node) => node.capability === key)) {
    return { ...graph, nodes: graph.nodes.filter((node) => node.capability !== key), edges: graph.edges.filter((edge) => edge.from !== key && edge.to !== key), entry: graph.entry === key ? null : graph.entry, terminal: graph.terminal === key ? null : graph.terminal };
  }
  const index = TEAM_CAPABILITIES.indexOf(key);
  return { ...graph, nodes: [...graph.nodes, { capability: key, x: 100 + index * 240, y: 120 }].sort((a, b) => TEAM_CAPABILITIES.indexOf(a.capability) - TEAM_CAPABILITIES.indexOf(b.capability)) };
}

export function toggleTeamEdge(graph: TeamGraph, edge: TeamEdge): TeamGraph {
  if (edge.from === edge.to || (graph.mode !== "flow" && TEAM_CAPABILITIES.indexOf(edge.from) >= TEAM_CAPABILITIES.indexOf(edge.to)) ||
    !graph.nodes.some((node) => node.capability === edge.from) ||
    !graph.nodes.some((node) => node.capability === edge.to)) return graph;
  const exists = graph.edges.some((item) => teamEdgeId(item) === teamEdgeId(edge));
  return { ...graph, edges: exists ? graph.edges.filter((item) => teamEdgeId(item) !== teamEdgeId(edge)) : [...graph.edges, edge] };
}

export function teamGraphIssue(graph: TeamGraph): string | null {
  if (graph.nodes.length < 2) return "至少选择两个能力。";
  const selected = new Set(graph.nodes.map((node) => node.capability));
  if (graph.mode === "flow") {
    if (!graph.entry || !selected.has(graph.entry)) return "请选择入口能力。";
    if (!graph.terminal || !selected.has(graph.terminal)) return "请选择可发出终止信号的能力。";
    const reachable = (start: CapabilityKey) => {
      const reached = new Set<CapabilityKey>([start]);
      for (let index = 0; index < graph.nodes.length; index++) {
        for (const edge of graph.edges) if (reached.has(edge.from)) reached.add(edge.to);
      }
      return reached;
    };
    if (reachable(graph.entry).size !== selected.size) return "入口能力必须能到达所有成员。";
    if ([...selected].some((key) => !reachable(key).has(graph.terminal!))) return "每个能力都必须能到达终止能力。";
    return null;
  }
  const roots = graph.nodes.filter((node) => !graph.edges.some((edge) => edge.to === node.capability));
  if (roots.length !== 1) return "请从一个入口连接所有能力。";
  const reached = new Set<CapabilityKey>([roots[0].capability]);
  for (const key of TEAM_CAPABILITIES) {
    if (reached.has(key)) graph.edges.filter((edge) => edge.from === key).forEach((edge) => reached.add(edge.to));
  }
  return reached.size === selected.size ? null : "团队中不能有孤立能力。";
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
