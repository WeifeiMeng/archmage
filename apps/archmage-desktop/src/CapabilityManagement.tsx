import { useEffect, useMemo, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { ArrowRight, CircleAlert, LoaderCircle, RefreshCw, Save, Settings2 } from "lucide-react";
import { api } from "./api";
import {
  connectionLabel,
  groupTopologyEdges,
  type CapabilityConfiguration,
  type CapabilityKey,
  type CapabilityManagementSnapshot,
  type TopologyEdge,
} from "./model";

const order: CapabilityKey[] = ["designer", "coder", "tester", "reviewer"];
const positions: Record<CapabilityKey, number> = { designer: 100, coder: 340, tester: 580, reviewer: 820 };

function edgePath(edge: TopologyEdge): string {
  const from = positions[edge.from];
  const to = positions[edge.to];
  if (edge.kind === "execution") return `M ${from + 82} 170 L ${to - 86} 170`;
  const lower = edge.from === "coder";
  const lane = lower ? 282 : edge.to === "reviewer" ? 36 : 76;
  const startY = lower ? 222 : 118;
  return `M ${from} ${startY} V ${lane} H ${to} V ${startY}`;
}

function Topology({ snapshot, selected, onSelect, busy }: {
  snapshot: CapabilityManagementSnapshot;
  selected: CapabilityKey;
  onSelect: (key: CapabilityKey) => void;
  busy?: boolean;
}) {
  const edges = groupTopologyEdges(snapshot.edges);
  return <><div className="cap-topology-scroll">
    <div className="cap-topology" aria-label="固定 Capability 拓扑">
      <svg viewBox="0 0 920 320" aria-hidden="true" focusable="false">
        <defs>
          <marker id="cap-arrow-solid" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 L8 4 L0 8 Z" fill="#72d9a2" /></marker>
          <marker id="cap-arrow-context" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 L8 4 L0 8 Z" fill="#7f9eb7" /></marker>
        </defs>
        {edges.context.map((edge) => <path key={`${edge.from}-${edge.to}`} className="cap-edge-context" d={edgePath(edge)} markerEnd="url(#cap-arrow-context)" />)}
        {edges.execution.map((edge) => <path key={`${edge.from}-${edge.to}`} className="cap-edge-execution" d={edgePath(edge)} markerEnd="url(#cap-arrow-solid)" />)}
      </svg>
      {order.map((key, index) => {
        const configuration = snapshot.configurations.find((item) => item.key === key);
        const instance = snapshot.instances.find((item) => item.id === configuration?.agentInstanceId);
        return <button key={key} type="button" className={`cap-node ${selected === key ? "selected" : ""}`} style={{ left: 20 + index * 240 }} aria-pressed={selected === key} disabled={busy} onClick={() => onSelect(key)}>
          <span className="cap-node-key">{key.toUpperCase()}</span>
          <strong>{configuration?.displayName ?? key}</strong>
          <span className="cap-node-agent">{instance ? `${instance.displayName} · ${instance.providerKind}` : "Agent 未绑定"}</span>
          <span className={`cap-node-state ${instance?.connectionState ?? "failed"}`}>{instance ? connectionLabel(instance.connectionState) : "不可用"}</span>
        </button>;
      })}
    </div>
  </div><nav className="cap-node-list" aria-label="Capability 节点列表">
    {order.map((key) => <button key={key} type="button" aria-current={selected === key ? "true" : undefined} disabled={busy} onClick={() => onSelect(key)}>{snapshot.configurations.find((item) => item.key === key)?.displayName ?? key}</button>)}
  </nav></>;
}

export function CapabilityManagement({ onError, onDirtyChange }: { onError: (message: string) => void; onDirtyChange: (dirty: boolean) => void }) {
  const [snapshot, setSnapshot] = useState<CapabilityManagementSnapshot | null>(null);
  const [selected, setSelected] = useState<CapabilityKey>("designer");
  const [draft, setDraft] = useState<CapabilityConfiguration | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function reload() {
    setBusy(true);
    try {
      const next = await api.getCapabilityManagement();
      setSnapshot(next);
      setDraft(next.configurations.find((item) => item.key === selected) ?? null);
      setMessage(null);
    } catch (reason) {
      setMessage(`读取 Capability 配置失败：${String(reason)}`);
      onError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (!isTauri()) return;
    api.getCapabilityManagement().then((next) => {
      setSnapshot(next);
      setDraft(next.configurations.find((item) => item.key === "designer") ?? null);
    }).catch((reason) => { setMessage(`读取 Capability 配置失败：${String(reason)}`); onError(String(reason)); });
  }, [onError]);

  const saved = snapshot?.configurations.find((item) => item.key === selected);
  const dirty = useMemo(() => !!draft && !!saved && (
    draft.displayName !== saved.displayName || draft.description !== saved.description ||
    draft.extraInstructions !== saved.extraInstructions || draft.agentInstanceId !== saved.agentInstanceId
  ), [draft, saved]);
  useEffect(() => { onDirtyChange(dirty); }, [dirty, onDirtyChange]);
  const instance = snapshot?.instances.find((item) => item.id === draft?.agentInstanceId);

  async function select(key: CapabilityKey) {
    if (key === selected || !snapshot) return;
    if (dirty && !await confirm("当前 Capability 有未保存的修改，确定放弃吗？", { title: "切换 Capability", kind: "warning" })) return;
    setSelected(key);
    setDraft(snapshot.configurations.find((item) => item.key === key) ?? null);
    setMessage(null);
  }

  async function refresh() {
    if (dirty && !await confirm("重新加载会放弃当前未保存的修改，确定继续吗？", { title: "刷新 Capability", kind: "warning" })) return;
    await reload();
  }

  async function save() {
    if (!draft || !dirty) return;
    setBusy(true);
    setMessage(null);
    try {
      const next = await api.saveCapabilityConfiguration({
        key: draft.key,
        displayName: draft.displayName,
        description: draft.description,
        extraInstructions: draft.extraInstructions,
        agentInstanceId: draft.agentInstanceId,
        revision: draft.revision,
      });
      setSnapshot(next);
      setDraft(next.configurations.find((item) => item.key === selected) ?? null);
      setMessage("配置已保存，仅影响之后创建的 Run。");
    } catch (reason) {
      const detail = String(reason);
      setMessage(detail.includes("refresh and retry") ? "配置已在别处更新。请刷新后重新编辑。" : `保存失败：${detail}`);
      onError(detail);
    } finally {
      setBusy(false);
    }
  }

  if (!isTauri()) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">当前是浏览器预览。请在 Tauri 桌面窗口查看真实拓扑与 Agent 配置。</div></section>;
  if (!snapshot) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">{busy ? <LoaderCircle className="spin" size={18} /> : <CircleAlert size={18} />}{message ?? "正在读取 Capability 配置…"}{!busy && <button onClick={reload}>重试</button>}</div></section>;

  return <section className="capabilities-view">
    <header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1><p>四个能力组成当前固定流程。它们可以共用同一个本地 Agent 实例；这里配置的是职责，而不是创建四个进程。</p></header>
    <div className="cap-section-heading"><div><h2>协作拓扑</h2><p>点击节点查看和配置能力。当前拓扑只读，不能拖动或编辑连线。</p></div><div className="cap-heading-actions"><span className="cap-fixed-badge">内置固定拓扑</span><button disabled={busy} onClick={refresh}><RefreshCw size={14} />重新加载</button></div></div>
    <div className="cap-topology-card">
      <Topology snapshot={snapshot} selected={selected} onSelect={select} busy={busy} />
      <div className="cap-legend"><span><i className="cap-legend-solid" />执行依赖</span><span><i className="cap-legend-dashed" />额外上下文传递</span></div>
      <p className="cap-handoff-note">Designer 的设计会交给 Coder、Tester 和 Reviewer；Coder 的实现会交给 Tester 和 Reviewer；Tester 的结果会交给 Reviewer。实线相邻节点也传递上下文。</p>
    </div>
    {draft && <div className="cap-editor" key={selected}>
      <div className="cap-editor-heading"><span className="cap-editor-icon"><Settings2 size={19} /></span><div><p className="eyebrow">SELECTED CAPABILITY</p><h2>{saved?.displayName ?? selected}</h2><span>稳定标识：{selected} · 基础指令、输入和输出契约由 Runtime 固定</span></div></div>
      <div className="cap-fields">
        <label>显示名称<input maxLength={80} value={draft.displayName} onChange={(event) => setDraft({ ...draft, displayName: event.target.value })} /><small>仅用于界面和新 Run 的历史快照。</small></label>
        <label>职责说明<textarea maxLength={500} rows={3} value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} /><small>帮助团队理解该能力的任务，不会替换内置 Prompt。</small></label>
        <label className="cap-wide-field">补充指令<textarea maxLength={8000} rows={5} value={draft.extraInstructions} onChange={(event) => setDraft({ ...draft, extraInstructions: event.target.value })} placeholder="可选：补充这个能力的工作偏好" /><small>会作为独立字段加入之后创建的 Run；不能改变固定输出 Schema。</small></label>
        <label>执行 Agent<select value={draft.agentInstanceId} onChange={(event) => setDraft({ ...draft, agentInstanceId: event.target.value })}>{snapshot.instances.filter((item) => item.enabled && item.providerKind === "codex").map((item) => <option key={item.id} value={item.id}>{item.displayName} · Codex</option>)}</select><small>{instance ? `当前连接：${connectionLabel(instance.connectionState)}` : "当前绑定不可用，请选择其他 Agent。"}</small></label>
      </div>
      {message && <p className={`cap-message ${message.startsWith("配置已保存") ? "success" : "error"}`} role="status">{message}</p>}
      <div className="cap-editor-actions"><p>修改只影响新 Run；已有 Run 保留当时冻结的配置。</p><button disabled={busy || !dirty || !draft.displayName.trim()} onClick={save}>{busy ? <LoaderCircle className="spin" size={16} /> : <Save size={16} />}保存 Capability <ArrowRight size={14} /></button></div>
    </div>}
  </section>;
}
