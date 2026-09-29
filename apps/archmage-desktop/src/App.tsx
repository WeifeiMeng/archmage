import { useCallback, useEffect, useMemo, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import {
  Box,
  Bot,
  Check,
  ChevronRight,
  Circle,
  Code2,
  FolderGit2,
  FolderOpen,
  LoaderCircle,
  Play,
  Plus,
  RefreshCw,
  ShieldCheck,
  Square,
  TestTube2,
  WandSparkles,
} from "lucide-react";
import { api } from "./api";
import { AgentManagement } from "./AgentManagement";
import "./agents.css";
import {
  initialTimelineState,
  isActive,
  reduceEvent,
  runBindingIssues,
  toRunRequest,
  type AgentManagementSnapshot,
  type RunEvent,
  type RunSnapshot,
  type RunSummary,
  type TimelineState,
} from "./model";

const nodeMeta = {
  designer: { label: "Designer", icon: WandSparkles },
  coder: { label: "Coder", icon: Code2 },
  tester: { label: "Tester", icon: TestTube2 },
  reviewer: { label: "Reviewer", icon: ShieldCheck },
};

function stateLabel(state: string, outcome?: string | null) {
  if (state === "completed") return outcome === "approved" ? "已批准" : "需要修改";
  return ({ pending: "等待", ready: "就绪", running: "运行中", succeeded: "完成", failed: "失败", interrupted: "已中断", cancelled: "已取消", created: "已创建", provisioning: "准备中" } as Record<string, string>)[state] ?? state;
}

export default function App() {
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<RunSnapshot | null>(null);
  const [timeline, setTimeline] = useState<TimelineState>(initialTimelineState);
  const [repository, setRepository] = useState("");
  const [goal, setGoal] = useState("");
  const [view, setView] = useState<"runs" | "agents">("runs");
  const [busy, setBusy] = useState(false);
  const [environment, setEnvironment] = useState<"checking" | "ready" | "failed" | "preview">("checking");
  const [agents, setAgents] = useState<AgentManagementSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshRuns = useCallback(async () => {
    const next = await api.listRuns();
    setRuns(next);
    return next;
  }, []);

  const refreshRun = useCallback(async (runId: string) => {
    const [nextSnapshot, events] = await Promise.all([
      api.snapshot(runId),
      api.events(runId, 0),
    ]);
    setSnapshot(nextSnapshot);
    setTimeline({
      events,
      lastSequence: events.at(-1)?.sequence ?? 0,
      needsRefresh: false,
    });
    await refreshRuns();
  }, [refreshRuns]);

  useEffect(() => {
    if (!isTauri()) return;
    refreshRuns()
      .then((items) => items[0] && setSelectedId(items[0].id))
      .catch((reason) => setError(String(reason)));
  }, [refreshRuns]);

  useEffect(() => {
    if (!isTauri()) {
      setEnvironment("preview");
      return;
    }
    checkEnvironment();
  }, []);

  useEffect(() => {
    if (!isTauri() || view !== "runs") return;
    api.getAgentManagement().then(setAgents).catch((reason) => setError(String(reason)));
  }, [view]);

  async function checkEnvironment() {
    setEnvironment("checking");
    try {
      setAgents(await api.getAgentManagement());
      setEnvironment("ready");
    } catch (reason) {
      setEnvironment("failed");
      setError(String(reason));
    }
  }

  useEffect(() => {
    if (!isTauri()) return;
    let stop: (() => void) | undefined;
    const window = getCurrentWindow();
    window.onCloseRequested(async (event) => {
      event.preventDefault();
      const active = await api.activeRunIds();
      if (active.length > 0) {
        const shouldClose = await confirm(
          `仍有 ${active.length} 个 Run 在执行。关闭会取消这些 Run，确定继续吗？`,
          { title: "关闭 Archmage", kind: "warning", okLabel: "取消 Run 并关闭", cancelLabel: "继续运行" },
        );
        if (!shouldClose) return;
        await Promise.allSettled(active.map((runId) => api.cancelRun(runId)));
      }
      await window.destroy();
    }).then((unlisten) => (stop = unlisten));
    return () => stop?.();
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setSnapshot(null);
      setTimeline(initialTimelineState);
      return;
    }
    refreshRun(selectedId).catch((reason) => setError(String(reason)));
  }, [selectedId, refreshRun]);

  useEffect(() => {
    if (!isTauri()) return;
    let stop: (() => void) | undefined;
    api.onRunEvent((runId) => {
      if (runId === selectedId) refreshRun(runId).catch((reason) => setError(String(reason)));
      else refreshRuns().catch(() => undefined);
    }).then((unlisten) => (stop = unlisten));
    return () => stop?.();
  }, [refreshRun, refreshRuns, selectedId]);

  useEffect(() => {
    if (!snapshot || !isActive(snapshot.state)) return;
    const timer = window.setInterval(() => refreshRun(snapshot.id).catch(() => undefined), 1200);
    return () => window.clearInterval(timer);
  }, [refreshRun, snapshot]);

  const request = useMemo(() => toRunRequest(repository, goal), [repository, goal]);
  const bindingIssues = agents ? runBindingIssues(agents) : ["正在读取能力绑定"];
  const canStart = environment === "ready" && agents !== null && bindingIssues.length === 0 && request.repository.length > 0 && request.goal.length > 0 && !busy;

  async function chooseRepository() {
    const chosen = await open({ directory: true, multiple: false, title: "选择 Git 仓库" });
    if (typeof chosen === "string") setRepository(chosen);
  }

  async function startRun() {
    setBusy(true);
    setError(null);
    try {
      const id = await api.startRun(request);
      setView("runs");
      setSelectedId(id);
      await refreshRun(id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><div className="brand-mark"><Box size={18} /></div><div><strong>Archmage</strong><span>Agent Workbench</span></div></div>
        <button className={`new-run ${view === "runs" && selectedId === null ? "active" : ""}`} onClick={() => { setView("runs"); setSelectedId(null); }}><Plus size={17} />新建 Run</button>
        <button className={`new-run ${view === "agents" ? "active" : ""}`} onClick={() => setView("agents")}><Bot size={17} />Agent 管理</button>
        <div className="section-label">最近运行</div>
        <nav className="run-list" aria-label="最近运行">
          {runs.map((run) => (
            <button key={run.id} className={view === "runs" && selectedId === run.id ? "active" : ""} onClick={() => { setView("runs"); setSelectedId(run.id); }}>
              <span className={`status-dot ${run.state}`} />
              <span className="run-copy"><strong>{run.goal}</strong><small>{stateLabel(run.state)} · {run.id.slice(0, 8)}</small></span>
              <ChevronRight size={15} />
            </button>
          ))}
          {runs.length === 0 && <p className="empty-copy">还没有 Run。先从右侧创建第一个真实协作流程。</p>}
        </nav>
        <div className="sidebar-foot"><span className="live-dot" />本地 Runtime</div>
      </aside>

      <main className="workspace">
        {error && <div className="error-banner" role="alert">{error}<button onClick={() => setError(null)}>关闭</button></div>}
        {view === "agents" ? <AgentManagement onError={setError} /> : !selectedId ? (
          <section className="new-run-view">
            <header><p className="eyebrow">REAL CODEX WORKFLOW</p><h1>把目标交给一组能力</h1><p>一个 Run 串联 Designer、Coder、Tester 和 Reviewer；每个节点都调用真实的本地 Codex。</p></header>
            <div className="form-card">
              <label>Git 仓库<span>必填</span></label>
              <div className="path-field"><FolderGit2 size={18} /><input value={repository} onChange={(event) => setRepository(event.target.value)} placeholder="选择一个已有 commit 的本地仓库" /><button onClick={chooseRepository}><FolderOpen size={17} />选择</button></div>
              <label htmlFor="goal">目标<span>必填</span></label>
              <textarea id="goal" value={goal} onChange={(event) => setGoal(event.target.value)} placeholder="描述希望四种能力协作完成的具体结果……" rows={5} />
              <div className="delegated-testing"><TestTube2 size={18} /><div><strong>Tester 自主验证</strong><span>测试任务会直接交给当前绑定的 Agent，由它分析项目并选择验证命令。</span></div></div>
              <div className="delegated-testing"><Bot size={18} /><div><strong>能力绑定</strong><span>{bindingIssues.length ? bindingIssues.join("；") : "Designer、Coder、Tester、Reviewer 均已绑定可用实例。"}</span>{agents && agents.bindings.map((binding) => { const instance = agents.instances.find((item) => item.id === binding.agentInstanceId); return instance?.connectionState !== "ready" ? <span key={binding.capability}>{binding.capability}: {instance?.displayName ?? "未知实例"} 尚未通过真实连接验证</span> : null; })}</div></div>
              <div className={`preflight ${environment}`}>
                {environment === "checking" ? <LoaderCircle className="spin" size={17} /> : environment === "ready" ? <Check size={17} /> : <Circle size={17} />}
                <div><strong>{environment === "checking" ? "正在检查本地环境" : environment === "ready" ? "本地 Runtime 已就绪" : environment === "preview" ? "当前是浏览器界面预览" : "环境检查未通过"}</strong><span>{environment === "ready" ? "数据库已就绪；具体 Agent 状态见上方能力绑定或 Agent 管理。" : environment === "failed" ? "请处理顶部错误后重试，未通过前不会创建 Run。" : environment === "preview" ? "请使用 Tauri 桌面窗口连接真实 Runtime 和 Codex。" : "正在读取 Runtime 和 Agent 配置。"}</span></div>
                {environment === "failed" && <button type="button" onClick={checkEnvironment}>重试</button>}
              </div>
              <button className="primary" disabled={!canStart} onClick={startRun}>{busy ? <LoaderCircle className="spin" size={18} /> : <Play size={18} fill="currentColor" />}{busy ? "正在准备…" : "启动真实 Run"}</button>
            </div>
          </section>
        ) : snapshot ? (
          <RunWorkbench snapshot={snapshot} timeline={timeline} onRefresh={() => refreshRun(snapshot.id)} onCancel={() => api.cancelRun(snapshot.id).then(() => refreshRun(snapshot.id)).catch((reason) => setError(String(reason)))} onReveal={() => api.revealWorkspace(snapshot.id).catch((reason) => setError(String(reason)))} />
        ) : <div className="loading"><LoaderCircle className="spin" />正在读取 Run…</div>}
      </main>
    </div>
  );
}

function RunWorkbench({ snapshot, timeline, onRefresh, onCancel, onReveal }: { snapshot: RunSnapshot; timeline: TimelineState; onRefresh: () => void; onCancel: () => void; onReveal: () => void }) {
  const [selectedNode, setSelectedNode] = useState<string>(snapshot.nodes.find((node) => node.state === "running")?.id ?? snapshot.nodes[0]?.id);
  const node = snapshot.nodes.find((item) => item.id === selectedNode) ?? snapshot.nodes[0];
  const nodeEvents = timeline.events.filter((event) => event.node_id === node?.id);
  return <div className="run-view">
    <header className="run-header"><div><div className="crumb"><span>Runs</span><ChevronRight size={14} />{snapshot.id.slice(0, 8)}</div><h1>{snapshot.goal}</h1><div className="run-meta"><span className={`state-pill ${snapshot.state}`}><Circle size={8} fill="currentColor" />{stateLabel(snapshot.state, snapshot.outcome)}</span><span>{snapshot.repository}</span><code>{snapshot.id}</code></div></div><div className="header-actions"><button onClick={onRefresh}><RefreshCw size={16} />刷新</button><button onClick={onReveal} disabled={!snapshot.workspace}><FolderOpen size={16} />工作区</button>{isActive(snapshot.state) && <button className="danger" onClick={onCancel}><Square size={14} fill="currentColor" />取消</button>}</div></header>
    <section className="node-strip" aria-label="能力进度">{snapshot.nodes.map((item, index) => { const meta = nodeMeta[item.key]; const binding = snapshot.agentBindings[item.key]; const Icon = meta.icon; return <button key={item.id} className={`${item.state} ${selectedNode === item.id ? "selected" : ""}`} onClick={() => setSelectedNode(item.id)}><span className="node-index">{item.state === "succeeded" ? <Check size={15} /> : item.state === "running" ? <LoaderCircle className="spin" size={15} /> : index + 1}</span><Icon size={18} /><span><strong>{meta.label}</strong><small>{stateLabel(item.state)} · {binding ? `${binding.displayName} (${binding.providerKind})` : "历史配置"}</small></span></button>; })}</section>
    <div className="run-grid"><section className="panel timeline-panel"><div className="panel-title"><div><span>运行时间线</span><small>{timeline.events.length} 个持久化事件</small></div><span className="sequence">SEQ {timeline.lastSequence}</span></div><div className="timeline">{timeline.events.map((event) => <EventRow key={event.sequence} event={event} />)}{timeline.events.length === 0 && <p className="empty-copy">等待 Runtime 事件……</p>}</div></section><section className="panel inspector"><div className="panel-title"><div><span>节点检查器</span><small>{node ? nodeMeta[node.key].label : "—"}</small></div></div>{node ? <><div className="inspect-state"><span className={`status-dot ${node.state}`} /><strong>{stateLabel(node.state)}</strong><code>{node.id}</code></div>{node.result ? <><div className="result-meta"><MetaItem label="Codex Thread" value={node.result.threadId} /><MetaItem label="Attempt" value={node.result.attemptId} /><MetaItem label="Result Artifact" value={node.result.resultArtifactId} />{node.result.changesetArtifactId && <MetaItem label="ChangeSet" value={node.result.changesetArtifactId} />}</div><div className="inspect-section output-section"><label>结构化结果</label><ResultSummary output={node.result.output} /><details><summary>查看原始 JSON</summary><pre>{JSON.stringify(node.result.output, null, 2)}</pre></details></div></> : <div className="waiting-result"><LoaderCircle className={node.state === "running" ? "spin" : ""} size={17} /><div><strong>尚无结构化结果</strong><span>{node.state === "running" ? "Codex 正在执行，完成后会在此显示结果和 Artifact。" : "节点开始并成功完成后，结果会持久化到这里。"}</span></div></div>}<div className="inspect-section"><label>运行记录</label>{nodeEvents.length ? nodeEvents.map((event) => <div className="compact-event" key={event.sequence}><span>#{event.sequence} · {event.event_type}</span>{Object.keys(event.payload).length > 0 && <pre>{JSON.stringify(event.payload, null, 2)}</pre>}</div>) : <p className="empty-copy">该节点还没有事件。</p>}</div></> : null}</section></div>
  </div>;
}

function MetaItem({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><code title={value}>{value}</code></div>;
}

function ResultSummary({ output }: { output: Record<string, unknown> }) {
  const preferred = ["status", "summary", "design", "implementation", "review", "notes", "verdict"];
  const entries = preferred.filter((key) => output[key] !== undefined).map((key) => [key, output[key]] as const);
  if (entries.length === 0) return null;
  return <div className="result-summary">{entries.map(([key, value]) => <div key={key}><span>{key}</span><p>{typeof value === "string" ? value : JSON.stringify(value, null, 2)}</p></div>)}</div>;
}

function EventRow({ event }: { event: RunEvent }) {
  const time = new Date(event.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  return <div className="event-row"><div className="event-rail"><span /><i /></div><div className="event-body"><div><strong>{event.event_type.replaceAll(".", " · ")}</strong><time>{time}</time></div>{Object.keys(event.payload).length > 0 && <code>{JSON.stringify(event.payload)}</code>}</div></div>;
}
