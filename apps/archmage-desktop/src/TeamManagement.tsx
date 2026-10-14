import { useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { LoaderCircle, Plus, RefreshCw, Save } from "lucide-react";
import { api } from "./api";
import type { CapabilityKey, Team, TeamEdge, TeamGraph } from "./model";
import { isTeamDirty, moveTeamNode, TEAM_CAPABILITIES, teamEdgeId, teamGraphIssue, toggleTeamCapability, toggleTeamEdge } from "./teamGraph";
import "./teams.css";

const labels: Record<CapabilityKey, string> = { designer: "Designer", coder: "Coder", tester: "Tester", reviewer: "Reviewer" };
const copyGraph = (graph: TeamGraph): TeamGraph => ({ nodes: graph.nodes.map((node) => ({ ...node })), edges: graph.edges.map((edge) => ({ ...edge })) });

function TeamCanvas({ graph, onChange }: { graph: TeamGraph; onChange: (graph: TeamGraph) => void }) {
  const canvas = useRef<HTMLDivElement>(null);
  const drag = useRef<{ key: CapabilityKey; dx: number; dy: number } | null>(null);
  const nodes = useMemo(() => new Map(graph.nodes.map((node) => [node.capability, node])), [graph.nodes]);
  function move(key: CapabilityKey, x: number, y: number) {
    onChange(moveTeamNode(graph, key, x, y));
  }
  return <div className="team-canvas-scroll"><div ref={canvas} className="team-canvas" onPointerMove={(event) => {
    if (!drag.current || !canvas.current) return;
    const rect = canvas.current.getBoundingClientRect();
    move(drag.current.key, (event.clientX - rect.left) * 1000 / rect.width - drag.current.dx, (event.clientY - rect.top) * 360 / rect.height - drag.current.dy);
  }} onPointerUp={() => { drag.current = null; }} onPointerCancel={() => { drag.current = null; }}>
    <svg viewBox="0 0 1000 360" aria-hidden="true"><defs><marker id="team-arrow" markerWidth="9" markerHeight="9" refX="8" refY="4.5" orient="auto"><path d="M0 0 L9 4.5 L0 9 Z" fill="#72d9a2" /></marker></defs>{graph.edges.map((edge) => { const from = nodes.get(edge.from); const to = nodes.get(edge.to); if (!from || !to) return null; const x1 = from.x + 166; const y1 = from.y + 44; const x2 = to.x; const y2 = to.y + 44; return <path key={teamEdgeId(edge)} d={`M ${x1} ${y1} C ${x1 + 65} ${y1}, ${x2 - 65} ${y2}, ${x2} ${y2}`} markerEnd="url(#team-arrow)" />; })}</svg>
    {graph.nodes.map((node) => <button key={node.capability} type="button" className="team-node" style={{ left: `${node.x / 10}%`, top: `${node.y / 3.6}%` }} onPointerDown={(event) => { if (!canvas.current) return; const rect = canvas.current.getBoundingClientRect(); drag.current = { key: node.capability, dx: (event.clientX - rect.left) * 1000 / rect.width - node.x, dy: (event.clientY - rect.top) * 360 / rect.height - node.y }; event.currentTarget.setPointerCapture(event.pointerId); }} onKeyDown={(event) => { const step = event.shiftKey ? 20 : 10; const offsets: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }; const offset = offsets[event.key]; if (offset) { event.preventDefault(); move(node.capability, node.x + offset[0], node.y + offset[1]); } }} aria-label={`${labels[node.capability]} 节点；可拖动或使用方向键移动`}><span>{node.capability.toUpperCase()}</span><strong>{labels[node.capability]}</strong><small>审阅上游交付物</small></button>)}
  </div></div>;
}

export function TeamManagement({ onError, onDirtyChange }: { onError: (message: string) => void; onDirtyChange: (dirty: boolean) => void }) {
  const [teams, setTeams] = useState<Team[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [graph, setGraph] = useState<TeamGraph | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const selected = teams.find((team) => team.id === selectedId);
  const dirty = isTeamDirty(selected, name, graph);
  useEffect(() => onDirtyChange(dirty), [dirty, onDirtyChange]);
  function select(team: Team) { setSelectedId(team.id); setName(team.name); setGraph(copyGraph(team.graph)); setMessage(""); }
  async function reload() { setBusy(true); try { const items = await api.listTeams(); setTeams(items); const next = items.find((item) => item.id === selectedId) ?? items[0]; if (next) select(next); } catch (reason) { onError(String(reason)); setMessage(`读取团队失败：${String(reason)}`); } finally { setBusy(false); } }
  useEffect(() => { if (isTauri()) api.listTeams().then((items) => { setTeams(items); if (items[0]) select(items[0]); }).catch((reason) => onError(String(reason))); }, [onError]);
  async function choose(team: Team) { if (dirty && !await confirm("当前团队有未保存修改，确定放弃吗？", { title: "切换团队", kind: "warning" })) return; select(team); }
  async function create() { if (dirty && !await confirm("当前团队有未保存修改，确定放弃并创建新团队吗？", { title: "创建团队", kind: "warning" })) return; setSelectedId(null); setName("新团队"); setGraph({ nodes: [], edges: [] }); setMessage("先选择至少两个能力，再设置关系并保存。"); }
  function toggle(edge: TeamEdge) { if (graph) setGraph(toggleTeamEdge(graph, edge)); }
  const selectedKeys = new Set(graph?.nodes.map((node) => node.capability) ?? []);
  const availableEdges = TEAM_CAPABILITIES.flatMap((from, index) => TEAM_CAPABILITIES.slice(index + 1).filter((to) => selectedKeys.has(from) && selectedKeys.has(to)).map((to) => ({ from, to })));
  const graphIssue = graph ? teamGraphIssue(graph) : null;
  async function save() { if (!graph || !name.trim() || !dirty) return; setBusy(true); setMessage(""); try { const saved = selected ? await api.saveTeam(selected.id, selected.revision, name.trim(), graph) : await api.createTeam(name.trim(), graph); const items = await api.listTeams(); setTeams(items); select(saved); setMessage("团队已保存；新 Run 会冻结所选团队的当前版本。"); } catch (reason) { onError(String(reason)); setMessage(`保存失败：${String(reason)}`); } finally { setBusy(false); } }
  if (!isTauri()) return <section className="teams-view"><h1>团队管理</h1><p>请在 Tauri 桌面窗口连接本地 Runtime 后管理团队。</p></section>;
  return <section className="teams-view"><header className="teams-header"><p className="eyebrow">TEAM MANAGEMENT</p><h1>团队管理</h1><p>先选择团队能力，再连接关系。每条关系表示下游能力会先审阅上游结果；有修改意见时，上游会收到建议并重新执行。</p></header>
    <div className="teams-toolbar"><label>当前团队<select value={selectedId ?? "new"} onChange={(event) => { const team = teams.find((item) => item.id === event.target.value); if (team) void choose(team); }}><option value="new" disabled>新团队（未保存）</option>{teams.map((team) => <option key={team.id} value={team.id}>{team.name}</option>)}</select></label><button onClick={() => void create()} disabled={busy}><Plus size={16} />创建团队</button><button onClick={() => { if (!dirty) void reload(); else void confirm("重新加载会放弃未保存修改，确定继续吗？", { title: "刷新团队", kind: "warning" }).then((yes) => { if (yes) void reload(); }); }} disabled={busy}><RefreshCw size={16} />刷新</button></div>
    {!graph ? <div className="cap-feedback">{busy ? <LoaderCircle className="spin" size={18} /> : null}正在读取团队…</div> : <>
      <div className="team-name-field"><label htmlFor="team-name">团队名称</label><input id="team-name" value={name} onChange={(event) => setName(event.target.value)} maxLength={80} /></div>
      <div className="team-members"><h2>团队能力</h2><p>至少选两个。取消选择时，相关连线也会移除。</p><div className="team-member-list">{TEAM_CAPABILITIES.map((key) => <label key={key}><input type="checkbox" checked={selectedKeys.has(key)} disabled={busy} onChange={() => setGraph(toggleTeamCapability(graph, key))} /><span>{labels[key]}</span></label>)}</div></div>
      <TeamCanvas graph={graph} onChange={setGraph} />
      <p className="team-canvas-note">拖动节点整理画布；聚焦节点后也可用方向键移动。关系按 Designer → Coder → Tester → Reviewer 的方向连接。</p>
      <div className="team-edges"><h2>协作关系</h2><p>选择已加入团队的能力之间的关系。图中所有能力必须从同一个入口连通；下游咨询可并行，主任务按顺序执行。</p>
        <div className="team-edge-list">{availableEdges.map((edge) => <label key={teamEdgeId(edge)}><input type="checkbox" disabled={busy} checked={graph.edges.some((item) => teamEdgeId(item) === teamEdgeId(edge))} onChange={() => toggle(edge)} /><span>{labels[edge.from]} → {labels[edge.to]}</span></label>)}</div>
        {availableEdges.length === 0 && <p>请先选择至少两个能力。</p>}
      </div>
      <div className="teams-footer"><span role="status">{graphIssue ?? message}</span><button onClick={save} disabled={busy || !dirty || !name.trim() || !!graphIssue}>{busy ? <LoaderCircle className="spin" size={16} /> : <Save size={16} />}保存团队</button></div>
    </>}
  </section>;
}
