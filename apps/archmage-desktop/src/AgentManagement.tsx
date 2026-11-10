import { useEffect, useMemo, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Bot, Check, CheckCircle2, CircleAlert, Clipboard, ExternalLink, LoaderCircle, Plus, RefreshCw, Save, Unplug, Wifi, X } from "lucide-react";
import { api } from "./api";
import { ManagementHero, PageMetric } from "./ManagementHero";
import { agentMetrics } from "./managementLayout";
import {
  connectionLabel,
  agentInputIsComplete,
  installGuidanceFor,
  providerLabel,
  type AgentInstallGuidance,
  type AgentInstance,
  type AgentInstanceInput,
  type AgentManagementSnapshot,
  type ProviderKind,
} from "./model";

export function AgentManagement({ onError, onOpenCapabilities }: { onError: (message: string) => void; onOpenCapabilities: () => void }) {
  const [snapshot, setSnapshot] = useState<AgentManagementSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [addingProvider, setAddingProvider] = useState<SupportedProvider | null>(null);
  const [guidance, setGuidance] = useState<AgentInstallGuidance[]>([]);
  const [activeGuide, setActiveGuide] = useState<InstallGuideTarget | null>(null);

  async function refreshGuidance() {
    try {
      setGuidance(await api.listAgentInstallGuidance());
    } catch (reason) {
      onError(String(reason));
      throw reason;
    }
  }

  useEffect(() => {
    if (!isTauri()) return;
    api.getAgentManagement().then(setSnapshot).catch((reason) => { setLoadError(String(reason)); onError(String(reason)); });
    void refreshGuidance().catch(() => undefined);
  }, [onError]);

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

  if (!isTauri()) return <section className="agents-view">
    <ManagementHero title="Agent 管理" description="管理真实的本地连接并验证可用性；每种能力使用哪个实例，请前往 Capability 管理配置。" />
    <div className="agent-preview-note">当前是浏览器预览。请在 Tauri 桌面窗口查看和验证真实 Agent 连接。</div>
  </section>;

  if (!snapshot) {
    if (loadError) return <div className="loading"><CircleAlert size={17} />无法读取 Agent 配置：{loadError}<button onClick={() => { setLoadError(null); api.getAgentManagement().then(setSnapshot).catch((reason) => setLoadError(String(reason))); }}>重试</button></div>;
    return <div className="loading"><LoaderCircle className="spin" />正在读取本地 Agent…</div>;
  }

  return <AgentManagementContent
    snapshot={snapshot}
    busy={busy}
    addingProvider={addingProvider}
    guidance={guidance}
    activeGuide={activeGuide}
    onOpenGuidance={(provider, instanceId) => setActiveGuide({ provider, instanceId })}
    onCloseGuidance={() => setActiveGuide(null)}
    onRefreshGuidance={refreshGuidance}
    onToggleAdding={(provider) => setAddingProvider((current) => current === provider ? null : provider)}
    onRun={run}
    onOpenCapabilities={onOpenCapabilities}
  />;
}

type SupportedProvider = Extract<ProviderKind, "codex" | "deepseek_harness" | "qoder">;
type InstallGuideTarget = { provider: SupportedProvider; instanceId?: string };

const providerSections: Array<{ kind: SupportedProvider; description: string; addLabel: string }> = [
  { kind: "codex", description: "Codex CLI 连接", addLabel: "添加 Codex" },
  { kind: "deepseek_harness", description: "官方 Python SDK 连接", addLabel: "添加 DeepSeek Harness" },
  { kind: "qoder", description: "本地 ACP Agent 连接", addLabel: "添加 Qoder" },
];

type AgentManagementContentProps = {
  snapshot: AgentManagementSnapshot;
  busy: string | null;
  addingProvider: SupportedProvider | null;
  guidance: AgentInstallGuidance[];
  activeGuide: InstallGuideTarget | null;
  onOpenGuidance: (provider: SupportedProvider, instanceId?: string) => void;
  onCloseGuidance: () => void;
  onRefreshGuidance: () => Promise<void>;
  onToggleAdding: (provider: SupportedProvider) => void;
  onRun: (id: string, action: () => Promise<AgentManagementSnapshot>) => Promise<boolean>;
  onOpenCapabilities: () => void;
};

export function AgentManagementContent({ snapshot, busy, addingProvider, guidance, activeGuide, onOpenGuidance, onCloseGuidance, onRefreshGuidance, onToggleAdding, onRun, onOpenCapabilities }: AgentManagementContentProps) {
  const metrics = agentMetrics(snapshot);
  const boundIds = new Set(snapshot.bindings.map((item) => item.agentInstanceId));
  return <section className="agents-view">
    <ManagementHero
      title="Agent 管理"
      description="管理真实的本地连接并验证可用性；每种能力使用哪个实例，请前往 Capability 管理配置。"
      summary={metrics.map((metric) => <PageMetric key={metric.label} {...metric} />)}
    />

    <div className="agent-provider-sections">
      {providerSections.map((section) => {
        const instances = snapshot.instances.filter((instance) => instance.providerKind === section.kind);
        const isAdding = addingProvider === section.kind;
        const install = installGuidanceFor(guidance, section.kind);
        const hasDetectedInstance = instances.some((instance) => Boolean(instance.discoveredVersion));
        return <section className="agent-provider-section" key={section.kind}>
          <div className="agent-provider-heading">
            <div><h2>{providerLabel(section.kind)}</h2><p>{section.description}</p></div>
            <div className="agent-provider-heading-actions">
              {install && !install.detected && !hasDetectedInstance && <button className="secondary-action install-guidance-trigger" onClick={() => onOpenGuidance(section.kind)}>安装指引</button>}
              <button className="secondary-action" onClick={() => onToggleAdding(section.kind)}><Plus size={16} />{isAdding ? "收起" : section.addLabel}</button>
            </div>
          </div>
          <div className="agent-grid">
            {activeGuide?.provider === section.kind && install && <AgentInstallGuide guidance={install} onClose={onCloseGuidance} onRecheck={async () => { if (activeGuide.instanceId) { const succeeded = await onRun(activeGuide.instanceId, () => api.discoverAgentInstance(activeGuide.instanceId!)); if (!succeeded) throw new Error("Agent 检查未通过"); } await onRefreshGuidance(); }} />}
            {instances.map((instance) => <AgentCard key={instance.id} instance={instance} bound={boundIds.has(instance.id)} busy={busy === instance.id} onInstallGuide={instance.connectionState === "failed" && !instance.discoveredVersion ? () => onOpenGuidance(section.kind, instance.id) : undefined} onSave={(input) => onRun(instance.id, () => api.saveAgentInstance(input))} onDiscover={() => onRun(instance.id, () => api.discoverAgentInstance(instance.id))} onVerify={() => onRun(instance.id, () => api.verifyAgentInstance(instance.id))} onDelete={async () => { if (await confirm(`删除 ${instance.displayName}？`, { title: "删除 Agent 实例", kind: "warning" })) await onRun(instance.id, () => api.deleteAgentInstance(instance.id)); }} />)}
            {instances.length === 0 && !isAdding && <div className="agent-provider-empty">尚未配置 {providerLabel(section.kind)} 实例。</div>}
            {isAdding && <NewAgentCard provider={section.kind} busy={busy === "new"} onCancel={() => onToggleAdding(section.kind)} onSave={async (input) => { if (await onRun("new", () => api.saveAgentInstance(input))) onToggleAdding(section.kind); }} />}
          </div>
        </section>;
      })}
    </div>

    <div className="agent-grid agent-unsupported-grid">
      {snapshot.adapters.filter((adapter) => !adapter.supported).map((adapter) => <article className="agent-card unsupported" key={adapter.kind}><div className="agent-card-top"><span className="agent-logo"><Unplug size={19} /></span><div><h3>{adapter.displayName}</h3><span>Provider Adapter</span></div><span className="status-chip unsupported">尚未支持</span></div><p>Adapter 类型已保留，但当前版本不会创建连接、执行探测或承载能力。</p></article>)}
    </div>

    <div className="agent-routing-handoff"><div><h2>下一步：配置能力路由</h2><p>为 Designer、Coder、Tester、Reviewer 分别选择执行 Agent。</p></div><button className="secondary-action" onClick={onOpenCapabilities}>打开 Capability 管理</button></div>
  </section>;
}

function AgentCard({ instance, bound, busy, onInstallGuide, onSave, onDiscover, onVerify, onDelete }: { instance: AgentInstance; bound: boolean; busy: boolean; onInstallGuide?: () => void; onSave: (input: AgentInstanceInput) => Promise<unknown>; onDiscover: () => Promise<unknown>; onVerify: () => Promise<unknown>; onDelete: () => Promise<void> }) {
  const isDsh = instance.providerKind === "deepseek_harness";
  const isQoder = instance.providerKind === "qoder";
  const [name, setName] = useState(instance.displayName);
  const [path, setPath] = useState(instance.executablePath ?? "");
  const [providerRoute, setProviderRoute] = useState(instance.providerRoute ?? "");
  const [model, setModel] = useState(instance.model ?? "");
  const [agentName, setAgentName] = useState(instance.agentName ?? "");
  const [providerConfig, setProviderConfig] = useState(JSON.stringify(instance.providerConfig, null, 2));
  const [enabled, setEnabled] = useState(instance.enabled);
  useEffect(() => { setName(instance.displayName); setPath(instance.executablePath ?? ""); setProviderRoute(instance.providerRoute ?? ""); setModel(instance.model ?? ""); setAgentName(instance.agentName ?? ""); setProviderConfig(JSON.stringify(instance.providerConfig, null, 2)); setEnabled(instance.enabled); }, [instance]);
  const parsedConfig = parseObject(providerConfig);
  const dirty = useMemo(() => name !== instance.displayName || path !== (instance.executablePath ?? "") || providerRoute !== (instance.providerRoute ?? "") || model !== (instance.model ?? "") || agentName !== (instance.agentName ?? "") || providerConfig !== JSON.stringify(instance.providerConfig, null, 2) || enabled !== instance.enabled, [agentName, enabled, instance, model, name, path, providerConfig, providerRoute]);
  const complete = agentInputIsComplete({ providerKind: instance.providerKind, providerRoute, model });
  const Icon = instance.connectionState === "ready" ? CheckCircle2 : instance.connectionState === "failed" ? CircleAlert : Wifi;
  return <article className="agent-card supported">
    <div className="agent-card-top"><span className="agent-logo"><Bot size={19} /></span><div><h3>{instance.displayName}</h3><span>{instance.id} · {providerLabel(instance.providerKind)}</span></div><span className={`status-chip ${instance.connectionState}`}><Icon size={13} />{!instance.enabled ? "已禁用" : instance.connectionState === "unverified" ? instance.discoveredVersion ? "CLI 可用 · 未验证" : "CLI 未检查" : connectionLabel(instance.connectionState)}</span></div>
    <div className="agent-fields">
      <label>实例名称<input value={name} onChange={(event) => setName(event.target.value)} /></label>
      <label>{isDsh ? "Python 路径" : isQoder ? "Qoder CLI 路径" : "Codex CLI 路径"}<input value={path} onChange={(event) => setPath(event.target.value)} placeholder={isDsh ? "留空则从 PATH 发现 python3" : "留空则从 PATH 自动发现"} /></label>
      {isDsh && <label>Provider route<input value={providerRoute} onChange={(event) => setProviderRoute(event.target.value)} placeholder="例如 deepseek-official" /></label>}
      <label>{isDsh ? "Model" : "模型覆盖"}<input value={model} onChange={(event) => setModel(event.target.value)} placeholder={isDsh ? "例如 deepseek-v4-flash" : "留空则使用 Codex 默认模型"} /></label>
      {isQoder && <label>Qoder Agent<input value={agentName} onChange={(event) => setAgentName(event.target.value)} placeholder="可选，例如 archmage-coder" /></label>}
      {isQoder && <label>显式扩展配置<textarea value={providerConfig} onChange={(event) => setProviderConfig(event.target.value)} rows={6} />{!parsedConfig && <span className="inline-agent-error">必须是一个 JSON 对象</span>}</label>}
      {isQoder && <p className="agent-provider-note">仅复用 Qoder CLI 登录状态；个人设置、默认 Agent、MCP、Plugin 和 Skill 不会自动继承。</p>}
      {isDsh && <p className="agent-provider-note">凭证、endpoint 与 DSH_HOME 继承 Archmage 启动环境；Provider route 与 model 由此实例明确配置。</p>}
      <label className="agent-enabled"><input type="checkbox" checked={enabled} disabled={bound} onChange={(event) => setEnabled(event.target.checked)} />启用实例{bound && <span>（已绑定能力）</span>}</label>
    </div>
    <div className="agent-facts"><span>版本<strong>{instance.discoveredVersion ?? "尚未读取"}</strong></span><span>上次验证<strong>{instance.lastVerifiedAt ? new Date(instance.lastVerifiedAt).toLocaleString() : "从未"}</strong></span></div>
    {instance.lastError && <p className="inline-agent-error"><CircleAlert size={14} />{instance.lastError}</p>}
    <div className="agent-actions">{onInstallGuide && <button className="install-guidance-trigger" disabled={busy} onClick={onInstallGuide}>查看安装指引</button>}<button disabled={!dirty || busy || !name.trim() || !complete || (isQoder && !parsedConfig)} onClick={() => onSave({ id: instance.id, providerKind: instance.providerKind, displayName: name.trim(), executablePath: path.trim() || null, providerRoute: isDsh ? providerRoute.trim() || null : null, model: model.trim() || null, agentName: isQoder ? agentName.trim() || null : null, providerConfig: isQoder ? parsedConfig! : {}, enabled })}><Save size={15} />保存配置</button><button disabled={busy || dirty || !enabled} onClick={onDiscover}>{isDsh ? "检查 SDK" : "检查 CLI"}</button><button className="verify" disabled={busy || dirty || !enabled} onClick={onVerify}>{busy ? <LoaderCircle className="spin" size={15} /> : <Wifi size={15} />}验证连接</button>{!bound && <button disabled={busy} onClick={onDelete}>删除</button>}</div>
  </article>;
}

function AgentInstallGuide({ guidance, onClose, onRecheck }: { guidance: AgentInstallGuidance; onClose: () => void; onRecheck: () => Promise<void> }) {
  const [feedback, setFeedback] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const platform = guidance.platform === "macos" ? "macOS" : guidance.platform === "windows" ? "Windows" : guidance.platform;

  async function copyCommand() {
    if (!guidance.command) return;
    try {
      await navigator.clipboard.writeText(guidance.command);
      setFeedback("安装命令已复制");
    } catch (reason) {
      setFeedback(`复制失败：${String(reason)}`);
    }
  }

  async function openDocumentation() {
    try {
      await openUrl(guidance.documentationUrl);
      setFeedback("已打开官方文档");
    } catch (reason) {
      setFeedback(`无法打开官方文档：${String(reason)}`);
    }
  }

  async function recheck() {
    setChecking(true);
    try {
      await onRecheck();
      setFeedback("检查完成；若仍未发现，请重新启动 Archmage 后再试。");
    } catch (reason) {
      setFeedback(`检查失败：${String(reason)}`);
    } finally {
      setChecking(false);
    }
  }

  return <article className={`agent-install-guide ${guidance.supported ? "supported" : "unsupported"}`}>
    <div className="agent-install-guide-heading"><div><span>{platform} · {guidance.architecture}</span><h3>{providerLabel(guidance.providerKind)} 安装指引</h3></div><button type="button" className="agent-guide-close" aria-label="关闭安装指引" onClick={onClose}><X size={17} /></button></div>
    {guidance.supported ? <>
      <p>请在系统终端中执行以下官方安装命令。Archmage 只提供指引，不会代替你执行。</p>
      {guidance.prerequisite && <p className="agent-install-prerequisite"><strong>前置条件</strong>{guidance.prerequisite}</p>}
      {guidance.command && <pre><code>{guidance.command}</code></pre>}
    </> : <p className="agent-install-unsupported"><CircleAlert size={16} />{guidance.unsupportedReason}</p>}
    <div className="agent-install-guide-actions">
      {guidance.command && <button type="button" onClick={copyCommand}>{feedback === "安装命令已复制" ? <Check size={15} /> : <Clipboard size={15} />}复制命令</button>}
      <button type="button" onClick={openDocumentation}><ExternalLink size={15} />打开官方文档</button>
      {guidance.supported && <button type="button" className="verify" disabled={checking} onClick={recheck}>{checking ? <LoaderCircle className="spin" size={15} /> : <RefreshCw size={15} />}重新检查</button>}
    </div>
    {feedback && <p className="agent-install-feedback" aria-live="polite">{feedback}</p>}
  </article>;
}

function NewAgentCard({ provider, busy, onCancel, onSave }: { provider: SupportedProvider; busy: boolean; onCancel: () => void; onSave: (input: AgentInstanceInput) => Promise<void> }) {
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [providerRoute, setProviderRoute] = useState("");
  const [model, setModel] = useState("");
  const [agentName, setAgentName] = useState("");
  const [providerConfig, setProviderConfig] = useState('{\n  "mcpServers": []\n}');
  const isDsh = provider === "deepseek_harness";
  const isQoder = provider === "qoder";
  const parsedConfig = parseObject(providerConfig);
  const complete = agentInputIsComplete({ providerKind: provider, providerRoute: isDsh ? providerRoute : null, model: model || null });
  return <article className="agent-card supported new-agent"><div className="agent-card-top"><span className="agent-logo"><Plus size={19} /></span><div><h3>添加 {providerLabel(provider)} 实例</h3><span>本地 Provider</span></div></div><div className="agent-fields"><label>实例 ID<input value={id} onChange={(event) => setId(event.target.value)} placeholder={provider === "codex" ? "例如 codex-work" : isQoder ? "例如 qoder-work" : "例如 dsh-work"} /></label><label>实例名称<input value={name} onChange={(event) => setName(event.target.value)} placeholder={providerLabel(provider)} /></label><label>{isDsh ? "Python 路径" : isQoder ? "Qoder CLI 路径" : "Codex CLI 路径"}<input value={path} onChange={(event) => setPath(event.target.value)} placeholder={isDsh ? "留空则从 PATH 发现 python3" : "留空则从 PATH 自动发现"} /></label>{isDsh && <label>Provider route<input value={providerRoute} onChange={(event) => setProviderRoute(event.target.value)} placeholder="例如 deepseek-official" /></label>}<label>{isDsh ? "Model" : "模型覆盖"}<input value={model} onChange={(event) => setModel(event.target.value)} placeholder={isDsh ? "例如 deepseek-v4-flash" : "留空则使用 Provider 默认模型"} /></label>{isQoder && <label>Qoder Agent<input value={agentName} onChange={(event) => setAgentName(event.target.value)} placeholder="可选，例如 archmage-coder" /></label>}{isQoder && <label>显式扩展配置<textarea value={providerConfig} onChange={(event) => setProviderConfig(event.target.value)} rows={6} />{!parsedConfig && <span className="inline-agent-error">必须是一个 JSON 对象</span>}</label>}{isDsh && <p className="agent-provider-note">凭证、endpoint 与 DSH_HOME 继承 Archmage 启动环境。</p>}{isQoder && <p className="agent-provider-note">认证复用 Qoder CLI；Archmage 不保存 Token。</p>}</div><div className="agent-actions"><button onClick={onCancel}>取消</button><button className="verify" disabled={busy || !id.trim() || !name.trim() || !complete || (isQoder && !parsedConfig)} onClick={() => onSave({ id: id.trim(), providerKind: provider, displayName: name.trim(), executablePath: path.trim() || null, providerRoute: isDsh ? providerRoute.trim() || null : null, model: model.trim() || null, agentName: isQoder ? agentName.trim() || null : null, providerConfig: isQoder ? parsedConfig! : {}, enabled: true })}>{busy ? <LoaderCircle className="spin" size={15} /> : <Plus size={15} />}创建实例</button></div></article>;
}

function parseObject(text: string): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(text);
    return value !== null && typeof value === "object" && !Array.isArray(value)
      ? value as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}
