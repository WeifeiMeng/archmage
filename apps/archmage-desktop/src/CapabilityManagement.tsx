import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { ArrowRight, CircleAlert, LoaderCircle, Plus, RefreshCw, Save, Settings2, Trash2 } from "lucide-react";
import { api } from "./api";
import {
  capabilityDraft,
  connectionLabel,
  isCapabilityDirty,
  type CapabilityConfigurationInput,
  type CapabilityKey,
  type CapabilityManagementSnapshot,
} from "./model";

const order: CapabilityKey[] = ["designer", "coder", "tester", "reviewer"];

export function CapabilityManagement({ onError, onDirtyChange }: { onError: (message: string) => void; onDirtyChange: (dirty: boolean) => void }) {
  const [snapshot, setSnapshot] = useState<CapabilityManagementSnapshot | null>(null);
  const [selected, setSelected] = useState<CapabilityKey>("designer");
  const [draft, setDraft] = useState<CapabilityConfigurationInput | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function reload() {
    setBusy(true);
    try {
      const next = await api.getCapabilityManagement();
      setSnapshot(next);
      const configuration = next.configurations.find((item) => item.key === selected);
      setDraft(configuration ? capabilityDraft(configuration) : null);
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
      const configuration = next.configurations.find((item) => item.key === "designer");
      setDraft(configuration ? capabilityDraft(configuration) : null);
    }).catch((reason) => { setMessage(`读取 Capability 配置失败：${String(reason)}`); onError(String(reason)); });
  }, [onError]);

  const saved = snapshot?.configurations.find((item) => item.key === selected);
  const dirty = !!saved && !!draft && isCapabilityDirty(saved, draft);
  useEffect(() => { onDirtyChange(dirty); }, [dirty, onDirtyChange]);
  const instance = snapshot?.instances.find((item) => item.id === draft?.agentInstanceId);
  const availableInstances = snapshot?.instances.filter((item) => item.enabled && item.providerKind === "codex") ?? [];
  const canSave = !!instance?.enabled && instance.providerKind === "codex";

  async function select(key: CapabilityKey) {
    if (key === selected || !snapshot) return;
    if (dirty && !await confirm("当前 Capability 有未保存的修改，确定放弃吗？", { title: "切换 Capability", kind: "warning" })) return;
    setSelected(key);
    const configuration = snapshot.configurations.find((item) => item.key === key);
    setDraft(configuration ? capabilityDraft(configuration) : null);
    setMessage(null);
  }

  async function refresh() {
    if (dirty && !await confirm("重新加载会放弃当前未保存的修改，确定继续吗？", { title: "刷新 Capability", kind: "warning" })) return;
    await reload();
  }

  async function save() {
    if (!saved || !draft || !dirty || !canSave) return;
    setBusy(true);
    setMessage(null);
    try {
      const next = await api.saveCapabilityConfiguration(draft);
      setSnapshot(next);
      const configuration = next.configurations.find((item) => item.key === selected);
      setDraft(configuration ? capabilityDraft(configuration) : null);
      setMessage("Capability 配置已保存，仅影响之后创建的 Run。");
    } catch (reason) {
      const detail = String(reason);
      setMessage(detail.includes("refresh and retry") ? "配置已在别处更新。请刷新后重新编辑。" : `保存失败：${detail}`);
      onError(detail);
    } finally {
      setBusy(false);
    }
  }

  if (!isTauri()) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">当前是浏览器预览。请在 Tauri 桌面窗口查看真实 Agent 绑定。</div></section>;
  if (!snapshot) return <section className="capabilities-view"><header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1></header><div className="cap-feedback">{busy ? <LoaderCircle className="spin" size={18} /> : <CircleAlert size={18} />}{message ?? "正在读取 Capability 配置…"}{!busy && <button onClick={reload}>重试</button>}</div></section>;

  return <section className="capabilities-view">
    <header className="capabilities-header"><p className="eyebrow">CAPABILITY MANAGEMENT</p><h1>Capability 管理</h1><p>为每个能力配置角色指引、Skills、Tools 和执行 Agent。四个能力可以共用同一个本地 Agent 实例。</p></header>
    <div className="cap-section-heading"><div><h2>能力列表</h2><p>每种能力独立设置执行 Agent；协作关系在团队管理中配置。</p></div><div className="cap-heading-actions"><button disabled={busy} onClick={refresh}><RefreshCw size={14} />重新加载</button></div></div>
    <div className="capability-list" aria-label="能力列表">{order.map((key) => { const configuration = snapshot.configurations.find((item) => item.key === key); const agent = snapshot.instances.find((item) => item.id === configuration?.agentInstanceId); return <button key={key} type="button" className={selected === key ? "selected" : ""} aria-pressed={selected === key} disabled={busy} onClick={() => select(key)}><span>{key.toUpperCase()}</span><strong>{configuration?.displayName ?? key}</strong><small>{agent ? `${agent.displayName} · ${connectionLabel(agent.connectionState)}` : "Agent 未绑定"}</small><small>{configuration?.skills.length ?? 0} Skills · {configuration?.tools.length ?? 0} Tools</small></button>; })}</div>
    {saved && draft && <div className="cap-editor" key={selected}>
      <div className="cap-editor-heading"><span className="cap-editor-icon"><Settings2 size={19} /></span><div><p className="eyebrow">SELECTED CAPABILITY</p><h2>{saved.displayName}</h2><span>稳定标识：{selected} · 内置职责与输出契约保持固定</span></div></div>
      <p className="cap-role-description">{saved.description}</p>
      <div className="cap-fields">
        <label>执行 Agent<select disabled={busy} value={draft.agentInstanceId} onChange={(event) => setDraft({ ...draft, agentInstanceId: event.target.value })}>{!availableInstances.some((item) => item.id === draft.agentInstanceId) && <option value={draft.agentInstanceId} disabled>{instance ? `${instance.displayName} · 已禁用` : "当前绑定不可用"}</option>}{availableInstances.map((item) => <option key={item.id} value={item.id}>{item.displayName} · Codex</option>)}</select><small>{instance ? `当前连接：${connectionLabel(instance.connectionState)}` : "当前绑定不可用，请先到 Agent 管理添加或启用实例。"}</small></label>
        <label>角色补充提示词<textarea disabled={busy} rows={5} maxLength={8000} value={draft.extraInstructions} onChange={(event) => setDraft({ ...draft, extraInstructions: event.target.value })} placeholder="描述这个能力的工作风格、侧重点和交付偏好" /><small>会追加到内置职责之后；不替换内置输出契约。</small></label>
      </div>
      {(["skills", "tools"] as const).map((field) => <div className="cap-guidance" key={field}><div className="cap-guidance-header"><div><h3>{field === "skills" ? "Skills" : "Tools"}</h3><p>手动填写名称与用途，让执行 Agent 理解它们如何辅助当前能力。</p></div><button type="button" disabled={busy || draft[field].length >= 16} onClick={() => setDraft({ ...draft, [field]: [...draft[field], { name: "", purpose: "" }] })}><Plus size={14} />添加</button></div>{draft[field].length === 0 ? <p className="cap-guidance-empty">尚未配置</p> : draft[field].map((item, index) => <div className="cap-guidance-row" key={`${field}-${index}`}><input disabled={busy} aria-label={`${field} ${index + 1} 名称`} placeholder={field === "skills" ? "例如 brainstorming" : "例如 web_search"} maxLength={80} value={item.name} onChange={(event) => setDraft({ ...draft, [field]: draft[field].map((entry, entryIndex) => entryIndex === index ? { ...entry, name: event.target.value } : entry) })} /><input disabled={busy} aria-label={`${field} ${index + 1} 用途`} placeholder="用途说明" maxLength={500} value={item.purpose} onChange={(event) => setDraft({ ...draft, [field]: draft[field].map((entry, entryIndex) => entryIndex === index ? { ...entry, purpose: event.target.value } : entry) })} /><button type="button" disabled={busy} aria-label={`移除 ${field} ${item.name || index + 1}`} onClick={() => setDraft({ ...draft, [field]: draft[field].filter((_, entryIndex) => entryIndex !== index) })}><Trash2 size={15} /></button></div>)}</div>)}
      <p className="cap-advisory">Skill 和 Tool 是给 Agent 的提示词指引，不代表已安装或可用，也不会限制它使用其他工具。</p>
      {message && <p className={`cap-message ${message.startsWith("Capability 配置已保存") ? "success" : "error"}`} role="status">{message}</p>}
      <div className="cap-editor-actions"><p>配置修改只影响新 Run；已有 Run 保留当时冻结的配置。</p><button disabled={busy || !dirty || !canSave} onClick={save}>{busy ? <LoaderCircle className="spin" size={16} /> : <Save size={16} />}保存 Capability <ArrowRight size={14} /></button></div>
    </div>}
  </section>;
}
