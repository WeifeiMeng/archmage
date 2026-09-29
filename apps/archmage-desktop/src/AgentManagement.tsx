import { useEffect, useMemo, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { Bot, CheckCircle2, CircleAlert, LoaderCircle, Plus, Save, Unplug, Wifi } from "lucide-react";
import { api } from "./api";
import {
  connectionLabel,
  type AgentInstance,
  type AgentInstanceInput,
  type AgentManagementSnapshot,
} from "./model";

export function AgentManagement({ onError, onOpenCapabilities }: { onError: (message: string) => void; onOpenCapabilities: () => void }) {
  const [snapshot, setSnapshot] = useState<AgentManagementSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    api.getAgentManagement().then(setSnapshot).catch((reason) => { setLoadError(String(reason)); onError(String(reason)); });
  }, [onError]);

  const readyCount = snapshot?.instances.filter((item) => item.enabled && item.connectionState === "ready").length ?? 0;
  const unverifiedCount = snapshot?.instances.filter((item) => item.enabled && item.connectionState === "unverified").length ?? 0;
  const failedCount = snapshot?.instances.filter((item) => item.enabled && item.connectionState === "failed").length ?? 0;
  const supportedInstances = snapshot?.instances.filter((item) => item.providerKind === "codex") ?? [];
  const boundIds = new Set(snapshot?.bindings.map((item) => item.agentInstanceId) ?? []);

  async function run(id: string, action: () => Promise<AgentManagementSnapshot>): Promise<boolean> {
    setBusy(id);
    try {
      setSnapshot(await action());
      return true;
    } catch (reason) {
      onError(String(reason));
      return false;
    } finally {
      setBusy(null);
    }
  }

  if (!snapshot) {
    if (loadError) return <div className="loading"><CircleAlert size={17} />无法读取 Agent 配置：{loadError}<button onClick={() => { setLoadError(null); api.getAgentManagement().then(setSnapshot).catch((reason) => setLoadError(String(reason))); }}>重试</button></div>;
    return <div className="loading"><LoaderCircle className="spin" />正在读取本地 Agent…</div>;
  }

  return <section className="agents-view">
    <header className="agents-header">
      <div><p className="eyebrow">LOCAL AGENT CONNECTIONS</p><h1>Agent 管理</h1><p>管理真实的本地连接并验证可用性；每种能力使用哪个实例，请前往 Capability 管理配置。</p></div>
      <div className="agent-summary"><strong>{readyCount}/{supportedInstances.length}</strong><span>已验证 · 未验证 {unverifiedCount} · 失败 {failedCount}</span></div>
    </header>

    <div className="agents-section-heading"><div><h2>本地连接</h2><p>CLI 可发现不等于账号已连通；使用“验证连接”发起一次真实只读 Codex 请求。</p></div><button className="secondary-action" onClick={() => setAdding((value) => !value)}><Plus size={16} />添加 Codex 实例</button></div>
    <div className="agent-grid">
      {supportedInstances.map((instance) => <AgentCard key={instance.id} instance={instance} bound={boundIds.has(instance.id)} busy={busy === instance.id} onSave={(input) => run(instance.id, () => api.saveAgentInstance(input))} onDiscover={() => run(instance.id, () => api.discoverAgentInstance(instance.id))} onVerify={() => run(instance.id, () => api.verifyAgentInstance(instance.id))} onDelete={async () => { if (await confirm(`删除 ${instance.displayName}？`, { title: "删除 Agent 实例", kind: "warning" })) await run(instance.id, () => api.deleteAgentInstance(instance.id)); }} />)}
      {adding && <NewAgentCard busy={busy === "new"} onCancel={() => setAdding(false)} onSave={async (input) => { if (await run("new", () => api.saveAgentInstance(input))) setAdding(false); }} />}
      {snapshot.adapters.filter((adapter) => !adapter.supported).map((adapter) => <article className="agent-card unsupported" key={adapter.kind}><div className="agent-card-top"><span className="agent-logo"><Unplug size={19} /></span><div><h3>{adapter.displayName}</h3><span>Provider Adapter</span></div><span className="status-chip unsupported">尚未支持</span></div><p>Adapter 类型已保留，但当前版本不会创建连接、执行探测或承载能力。</p></article>)}
    </div>

    <div className="agents-section-heading routing-heading"><div><h2>能力路由</h2><p>在独立的 Capability 管理页查看固定拓扑，并为四个能力分别绑定执行 Agent。</p></div><button className="secondary-action" onClick={onOpenCapabilities}>打开 Capability 管理</button></div>
  </section>;
}

function AgentCard({ instance, bound, busy, onSave, onDiscover, onVerify, onDelete }: { instance: AgentInstance; bound: boolean; busy: boolean; onSave: (input: AgentInstanceInput) => Promise<unknown>; onDiscover: () => Promise<unknown>; onVerify: () => Promise<unknown>; onDelete: () => Promise<void> }) {
  const [name, setName] = useState(instance.displayName);
  const [path, setPath] = useState(instance.executablePath ?? "");
  const [model, setModel] = useState(instance.model ?? "");
  const [enabled, setEnabled] = useState(instance.enabled);
  useEffect(() => { setName(instance.displayName); setPath(instance.executablePath ?? ""); setModel(instance.model ?? ""); setEnabled(instance.enabled); }, [instance]);
  const dirty = useMemo(() => name !== instance.displayName || path !== (instance.executablePath ?? "") || model !== (instance.model ?? "") || enabled !== instance.enabled, [instance, model, name, path, enabled]);
  const Icon = instance.connectionState === "ready" ? CheckCircle2 : instance.connectionState === "failed" ? CircleAlert : Wifi;
  return <article className="agent-card supported">
    <div className="agent-card-top"><span className="agent-logo"><Bot size={19} /></span><div><h3>{instance.displayName}</h3><span>{instance.id}</span></div><span className={`status-chip ${instance.connectionState}`}><Icon size={13} />{!instance.enabled ? "已禁用" : instance.connectionState === "unverified" ? instance.discoveredVersion ? "CLI 可用 · 未验证" : "CLI 未检查" : connectionLabel(instance.connectionState)}</span></div>
    <div className="agent-fields">
      <label>实例名称<input value={name} onChange={(event) => setName(event.target.value)} /></label>
      <label>Codex CLI 路径<input value={path} onChange={(event) => setPath(event.target.value)} placeholder="留空则从 PATH 自动发现" /></label>
      <label>模型覆盖<input value={model} onChange={(event) => setModel(event.target.value)} placeholder="留空则使用 Codex 默认模型" /></label>
      <label className="agent-enabled"><input type="checkbox" checked={enabled} disabled={bound} onChange={(event) => setEnabled(event.target.checked)} />启用实例{bound && <span>（已绑定能力）</span>}</label>
    </div>
    <div className="agent-facts"><span>版本<strong>{instance.discoveredVersion ?? "尚未读取"}</strong></span><span>上次验证<strong>{instance.lastVerifiedAt ? new Date(instance.lastVerifiedAt).toLocaleString() : "从未"}</strong></span></div>
    {instance.lastError && <p className="inline-agent-error"><CircleAlert size={14} />{instance.lastError}</p>}
    <div className="agent-actions"><button disabled={!dirty || busy || !name.trim()} onClick={() => onSave({ id: instance.id, providerKind: "codex", displayName: name.trim(), executablePath: path.trim() || null, model: model.trim() || null, enabled })}><Save size={15} />保存配置</button><button disabled={busy || dirty || !enabled} onClick={onDiscover}>检查 CLI</button><button className="verify" disabled={busy || dirty || !enabled} onClick={onVerify}>{busy ? <LoaderCircle className="spin" size={15} /> : <Wifi size={15} />}验证连接</button>{!bound && <button disabled={busy} onClick={onDelete}>删除</button>}</div>
  </article>;
}

function NewAgentCard({ busy, onCancel, onSave }: { busy: boolean; onCancel: () => void; onSave: (input: AgentInstanceInput) => Promise<void> }) {
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  return <article className="agent-card supported new-agent"><div className="agent-card-top"><span className="agent-logo"><Plus size={19} /></span><div><h3>新 Codex 实例</h3><span>高级配置</span></div></div><div className="agent-fields"><label>实例 ID<input value={id} onChange={(event) => setId(event.target.value)} placeholder="例如 codex-work" /></label><label>实例名称<input value={name} onChange={(event) => setName(event.target.value)} placeholder="例如 Codex Work" /></label></div><div className="agent-actions"><button onClick={onCancel}>取消</button><button className="verify" disabled={busy || !id.trim() || !name.trim()} onClick={() => onSave({ id: id.trim(), providerKind: "codex", displayName: name.trim(), executablePath: null, model: null, enabled: true })}>{busy ? <LoaderCircle className="spin" size={15} /> : <Plus size={15} />}创建实例</button></div></article>;
}
