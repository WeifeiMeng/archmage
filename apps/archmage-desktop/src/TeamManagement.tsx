import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { Plus, RefreshCw, Save, Trash2 } from "lucide-react";
import { api } from "./api";
import type { CapabilityKey, Team, TeamEdge, TeamGraph } from "./model";
import { createTeamEdge, isTeamDirty, moveTeamNode, TEAM_CAPABILITIES, teamEdgeId, teamEdgePath, teamGraphIssue, toggleTeamCapability, toggleTeamEdge } from "./teamGraph";
import "./teams.css";

const labels: Record<CapabilityKey, string> = { designer: "Designer", coder: "Coder", tester: "Tester", reviewer: "Reviewer" };
const copyGraph = (graph: TeamGraph): TeamGraph => ({ ...graph, nodes: graph.nodes.map((n) => ({ ...n })), edges: graph.edges.map((e) => ({ ...e })) });

function TeamCanvas({ graph, onChange }: { graph: TeamGraph; onChange: (graph: TeamGraph) => void }) {
  const canvas = useRef<HTMLDivElement>(null);
  const dragging = useRef<{ key: CapabilityKey; dx: number; dy: number } | null>(null);
  const linking = useRef<CapabilityKey | null>(null);
  const [source, setSource] = useState<CapabilityKey | null>(null);
  const [pointer, setPointer] = useState<{ x: number; y: number } | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const editable = graph.mode === "flow";
  const selected = graph.edges.find((edge) => teamEdgeId(edge) === selectedId);
  function point(clientX: number, clientY: number) {
    const rect = canvas.current!.getBoundingClientRect();
    return { x: (clientX - rect.left) * 1000 / rect.width, y: (clientY - rect.top) * 360 / rect.height };
  }
  function cancel() { linking.current = null; setSource(null); setPointer(null); }
  function connect(target: CapabilityKey | undefined) {
    const from = linking.current;
    if (from && target && from !== target) onChange(createTeamEdge(graph, { from, to: target }));
    cancel();
  }
  function remove(edge: TeamEdge) { onChange(toggleTeamEdge(graph, edge)); setSelectedId(null); }
  return <>
    <div className="team-canvas-scroll"><div ref={canvas} className="team-canvas" onKeyDown={(event) => {
      if (event.key === "Escape") { cancel(); setSelectedId(null); }
      if (editable && selected && (event.key === "Delete" || event.key === "Backspace")) remove(selected);
    }} onPointerMove={(event) => {
      if (!canvas.current) return;
      const position = point(event.clientX, event.clientY);
      if (dragging.current) onChange(moveTeamNode(graph, dragging.current.key, position.x - dragging.current.dx, position.y - dragging.current.dy));
      if (linking.current) setPointer(position);
    }} onPointerUp={(event) => {
      dragging.current = null;
      if (linking.current) {
        const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-team-input]")?.dataset.teamInput as CapabilityKey | undefined;
        connect(target);
      }
    }} onPointerCancel={() => { dragging.current = null; cancel(); }}>
      <svg viewBox="0 0 1000 360" aria-label="团队消息流向">
        <defs><marker id="team-arrow" markerWidth="9" markerHeight="9" refX="8" refY="4.5" orient="auto"><path d="M 0 0 L 9 4.5 L 0 9 Z" fill="#72d9a2" /></marker></defs>
        {graph.edges.map((edge) => <g key={teamEdgeId(edge)}>
          <path className={selectedId === teamEdgeId(edge) ? "team-edge selected" : "team-edge"} d={teamEdgePath(graph, edge)} markerEnd="url(#team-arrow)" />
          <path className="team-edge-hit" d={teamEdgePath(graph, edge)} onClick={() => setSelectedId(teamEdgeId(edge))} />
        </g>)}
        {source && pointer && <path className="team-edge-preview" d={`M ${(graph.nodes.find((n) => n.capability === source)?.x ?? 0) + 166} ${(graph.nodes.find((n) => n.capability === source)?.y ?? 0) + 44} L ${pointer.x} ${pointer.y}`} />}
      </svg>
      {graph.nodes.map((node) => <div key={node.capability} className="team-node" style={{ left: `${node.x / 10}%`, top: `${node.y / 3.6}%` }}>
        {editable && <button type="button" className="team-handle input" data-team-input={node.capability} onClick={() => connect(node.capability)} aria-label={`连接到 ${labels[node.capability]}`} />}
        <button type="button" className="team-node-body" onPointerDown={(event) => {
          if (!editable) return;
          const position = point(event.clientX, event.clientY);
          dragging.current = { key: node.capability, dx: position.x - node.x, dy: position.y - node.y };
          event.currentTarget.setPointerCapture(event.pointerId);
        }} onKeyDown={(event) => {
          if (!editable) return;
          const step = event.shiftKey ? 20 : 10;
          const offset: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
          if (offset[event.key]) { event.preventDefault(); onChange(moveTeamNode(graph, node.capability, node.x + offset[event.key][0], node.y + offset[event.key][1])); }
        }} aria-label={`${labels[node.capability]} 节点，可拖动或使用方向键移动`}>
          <span>{node.capability.toUpperCase()}</span><strong>{labels[node.capability]}</strong>
          <small>{graph.entry === node.capability ? "入口" : graph.terminal === node.capability ? "可终止" : "交付节点"}</small>
        </button>
        {editable && <button type="button" className="team-handle output" onPointerDown={(event) => {
          event.stopPropagation();
          linking.current = node.capability;
          setSource(node.capability);
          setPointer(point(event.clientX, event.clientY));
          event.currentTarget.setPointerCapture(event.pointerId);
        }} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); linking.current = node.capability; setSource(node.capability); } }} aria-label={`从 ${labels[node.capability]} 开始连接`} />}
      </div>)}
    </div></div>
    {selected && editable && <div className="team-selected-edge"><span>{labels[selected.from]} → {labels[selected.to]}</span><button type="button" onClick={() => remove(selected)}><Trash2 size={15} />删除连线</button></div>}
    {source && <p className="team-canvas-note" role="status">正在从 {labels[source]} 连线；拖到目标左侧圆点，或点击目标输入圆点。按 Escape 取消。</p>}
  </>;
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
  async function reload() {
    setBusy(true);
    try { const items = await api.listTeams(); setTeams(items); const next = items.find((item) => item.id === selectedId) ?? items[0]; if (next) select(next); }
    catch (reason) { onError(String(reason)); setMessage(`读取团队失败：${String(reason)}`); }
    finally { setBusy(false); }
  }
  useEffect(() => { if (isTauri()) api.listTeams().then((items) => { setTeams(items); if (items[0]) select(items[0]); }).catch((reason) => onError(String(reason))); }, [onError]);
  async function choose(team: Team) { if (dirty && !await confirm("当前团队有未保存修改，确定放弃吗？", { title: "切换团队", kind: "warning" })) return; select(team); }
  async function create() {
    if (dirty && !await confirm("当前团队有未保存修改，确定放弃并创建新团队吗？", { title: "创建团队", kind: "warning" })) return;
    setSelectedId(null); setName("新团队"); setGraph({ mode: "flow", nodes: [], edges: [], entry: null, terminal: null });
    setMessage("选择能力、指定入口与终止能力，然后在画布中连接消息流向。");
  }
  async function copyLegacy() {
    if (!graph) return;
    if (dirty && !await confirm("当前团队有未保存修改，确定放弃吗？", { title: "复制为消息流团队", kind: "warning" })) return;
    setSelectedId(null); setName(`${name} · 消息流`);
    setGraph({ ...copyGraph(graph), mode: "flow", entry: null, terminal: null });
    setMessage("这是尚未保存的副本；请指定入口和终止能力后保存。");
  }
  const selectedKeys = new Set(graph?.nodes.map((node) => node.capability) ?? []);
  const issue = graph ? teamGraphIssue(graph) : null;
  async function save() {
    if (!graph || !name.trim() || !dirty) return;
    setBusy(true); setMessage("");
    try {
      const saved = selected ? await api.saveTeam(selected.id, selected.revision, name.trim(), graph) : await api.createTeam(name.trim(), graph);
      setTeams(await api.listTeams()); select(saved); setMessage("团队已保存；新 Run 会冻结当前版本。");
    } catch (reason) { onError(String(reason)); setMessage(`保存失败：${String(reason)}`); }
    finally { setBusy(false); }
  }
  if (!isTauri()) return <section className="teams-view"><h1>团队管理</h1><p>请在 Tauri 桌面窗口连接本地 Runtime 后管理团队。</p></section>;
  return <section className="teams-view">
    <header className="teams-header"><p className="eyebrow">TEAM MANAGEMENT</p><h1>团队管理</h1><p>选择能力，在画布中绘制交付方向。每次交付触发目标能力的一次执行；终止能力可以发出完成信号。</p></header>
    <div className="teams-toolbar"><label>当前团队<select value={selectedId ?? "new"} onChange={(event) => { const team = teams.find((item) => item.id === event.target.value); if (team) void choose(team); }}><option value="new" disabled>新团队（未保存）</option>{teams.map((team) => <option key={team.id} value={team.id}>{team.name}{team.graph.mode === "flow" ? " · 消息流" : " · 历史咨询"}</option>)}</select></label>
      <button onClick={() => void create()} disabled={busy}><Plus size={16} />创建团队</button>
      <button onClick={() => { if (!dirty) void reload(); else void confirm("重新加载会放弃未保存修改，确定继续吗？", { title: "刷新团队", kind: "warning" }).then((yes) => { if (yes) void reload(); }); }} disabled={busy}><RefreshCw size={16} />刷新</button>
    </div>
    {!graph ? <div className="cap-feedback">正在读取团队…</div> : <>
      {graph.mode !== "flow" && <div className="team-legacy-note">此团队使用历史咨询语义，原团队保持不变。<button type="button" onClick={() => void copyLegacy()}>复制为消息流团队</button></div>}
      <div className="team-name-field"><label htmlFor="team-name">团队名称</label><input id="team-name" value={name} disabled={graph.mode !== "flow"} onChange={(event) => setName(event.target.value)} maxLength={80} /></div>
      <div className="team-members"><h2>团队能力</h2><p>至少选两个。取消选择时，相关连线也会移除。</p><div className="team-member-list">{TEAM_CAPABILITIES.map((key) => <label key={key}><input type="checkbox" checked={selectedKeys.has(key)} disabled={busy || graph.mode !== "flow"} onChange={() => setGraph(toggleTeamCapability(graph, key))} /><span>{labels[key]}</span></label>)}</div></div>
      {graph.mode === "flow" && <div className="team-endpoints"><label>入口能力<select value={graph.entry ?? ""} onChange={(event) => setGraph({ ...graph, entry: event.target.value ? event.target.value as CapabilityKey : null })}><option value="">请选择</option>{graph.nodes.map((node) => <option key={node.capability} value={node.capability}>{labels[node.capability]}</option>)}</select></label>
        <label>可发出完成信号的能力<select value={graph.terminal ?? ""} onChange={(event) => setGraph({ ...graph, terminal: event.target.value ? event.target.value as CapabilityKey : null })}><option value="">请选择</option>{graph.nodes.map((node) => <option key={node.capability} value={node.capability}>{labels[node.capability]}</option>)}</select></label></div>}
      <TeamCanvas graph={graph} onChange={setGraph} />
      <p className="team-canvas-note">{graph.mode === "flow" ? "从节点右侧圆点拖到目标左侧圆点添加连线；可反向再连一条。点击连线可删除，Escape 取消。节点仍可拖动或用方向键移动。" : "历史团队的拓扑仅供查看。"}</p>
      <div className="teams-footer"><span role="status">{issue ?? message}</span><button onClick={() => void save()} disabled={busy || graph.mode !== "flow" || !dirty || !name.trim() || !!issue}><Save size={16} />保存团队</button></div>
    </>}
  </section>;
}
