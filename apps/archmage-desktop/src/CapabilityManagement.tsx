import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { ArrowRight, CircleAlert, LoaderCircle, RefreshCw, Save, Settings2 } from "lucide-react";
import { api } from "./api";
import {
  bindingOnlyUpdate,
  connectionLabel,
  type CapabilityKey,
  type CapabilityManagementSnapshot,
} from "./model";

const order: CapabilityKey[] = ["designer", "coder", "tester", "reviewer"];

export function CapabilityManagement({ onError, onDirtyChange }: { onError: (message: string) => void; onDirtyChange: (dirty: boolean) => void }) {
  const [snapshot, setSnapshot] = useState<CapabilityManagementSnapshot | null>(null);
  const [selected, setSelected] = useState<CapabilityKey>("designer");
  const [draftAgentId, setDraftAgentId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function reload() {
    setBusy(true);
    try {
      const next = await api.getCapabilityManagement();
      setSnapshot(next);
      setDraftAgentId(next.configurations.find((item) => item.key === selected)?.agentInstanceId ?? null);
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
      setDraftAgentId(next.configurations.find((item) => item.key === "designer")?.agentInstanceId ?? null);
    }).catch((reason) => { setMessage(`读取 Capability 配置失败：${String(reason)}`); onError(String(reason)); });
  }, [onError]);

  const saved = snapshot?.configurations.find((item) => item.key === selected);
  const dirty = !!saved && draftAgentId !== null && draftAgentId !== saved.agentInstanceId;
  useEffect(() => { onDirtyChange(dirty); }, [dirty, onDirtyChange]);
  const instance = snapshot?.instances.find((item) => item.id === draftAgentId);
  const availableInstances = snapshot?.instances.filter((item) => item.enabled && item.providerKind === "codex") ?? [];
  const canSave = !!instance?.enabled && instance.providerKind === "codex";

  async function select(key: CapabilityKey) {
    if (key === selected || !snapshot) return;
    if (dirty && !await confirm("当前 Capability 有未保存的修改，确定放弃吗？", { title: "切换 Capability", kind: "warning" })) return;
    setSelected(key);
    setDraftAgentId(snapshot.configurations.find((item) => item.key === key)?.agentInstanceId ?? null);
    setMessage(null);
  }

  async function refresh() {
    if (dirty && !await confirm("重新加载会放弃当前未保存的修改，确定继续吗？", { title: "刷新 Capability", kind: "warning" })) return;
    await reload();
  }

  async function save() {
    if (!saved || !draftAgentId || !dirty || !canSave) return;
    setBusy(true);
    setMessage(null);
    try {
      const next = await api.saveCapabilityConfiguration(bindingOnlyUpdate(saved, draftAgentId));
      setSnapshot(next);
      setDraftAgentId(next.configurations.find((item) => item.key === selected)?.agentInstanceId ?? null);
      setMessage("Agent 绑定已保存，仅影响之后创建的 Run。");
    } catch (reason) {
      const detail = String(reason);
      setMessage(detail.includes("refresh and retry") ? "绑定已在别处更新。请刷新后重新选择。" : `保存失败：${detail}`);
      onError(detail);
    } finally {
      setBusy(false);
    }
  }

  if (!isTauri()) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">当前是浏览器预览。请在 Tauri 桌面窗口查看真实 Agent 绑定。</div></section>;
  if (!snapshot) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">{busy ? <LoaderCircle className="spin" size={18} /> : <CircleAlert size={18} />}{message ?? "正在读取 Capability 配置…"}{!busy && <button onClick={reload}>重试</button>}</div></section>;

  return <section className="capabilities-view">
    <header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1><p>能力由职责提示词和执行它的 Agent 组成。四个能力可以共用同一个本地 Agent 实例；当前只配置 Agent 绑定。</p></header>
    <div className="cap-section-heading"><div><h2>能力列表</h2><p>每种能力独立设置执行 Agent；协作关系在团队管理中配置。</p></div><div className="cap-heading-actions"><button disabled={busy} onClick={refresh}><RefreshCw size={14} />重新加载</button></div></div>
    <div className="capability-list" aria-label="能力列表">{order.map((key) => { const configuration = snapshot.configurations.find((item) => item.key === key); const agent = snapshot.instances.find((item) => item.id === configuration?.agentInstanceId); return <button key={key} type="button" className={selected === key ? "selected" : ""} aria-pressed={selected === key} disabled={busy} onClick={() => select(key)}><span>{key.toUpperCase()}</span><strong>{configuration?.displayName ?? key}</strong><small>{agent ? `${agent.displayName} · ${connectionLabel(agent.connectionState)}` : "Agent 未绑定"}</small></button>; })}</div>
    {saved && <div className="cap-editor" key={selected}>
      <div className="cap-editor-heading"><span className="cap-editor-icon"><Settings2 size={19} /></span><div><p className="eyebrow">SELECTED CAPABILITY</p><h2>{saved.displayName}</h2><span>稳定标识：{selected} · 能力提示词由 Runtime 内置，当前不可编辑</span></div></div>
      <p className="cap-role-description">{saved.description}</p>
      <div className="cap-fields">
        <label>执行 Agent<select value={draftAgentId ?? ""} onChange={(event) => setDraftAgentId(event.target.value)}>{!availableInstances.some((item) => item.id === draftAgentId) && <option value={draftAgentId ?? ""} disabled>{instance ? `${instance.displayName} · 已禁用` : "当前绑定不可用"}</option>}{availableInstances.map((item) => <option key={item.id} value={item.id}>{item.displayName} · Codex</option>)}</select><small>{instance ? `当前连接：${connectionLabel(instance.connectionState)}` : "当前绑定不可用，请先到 Agent 管理添加或启用实例。"}</small></label>
      </div>
      {message && <p className={`cap-message ${message.startsWith("Agent 绑定已保存") ? "success" : "error"}`} role="status">{message}</p>}
      <div className="cap-editor-actions"><p>绑定修改只影响新 Run；已有 Run 保留当时冻结的配置。</p><button disabled={busy || !dirty || !canSave} onClick={save}>{busy ? <LoaderCircle className="spin" size={16} /> : <Save size={16} />}保存 Agent 绑定 <ArrowRight size={14} /></button></div>
    </div>}
  </section>;
}
