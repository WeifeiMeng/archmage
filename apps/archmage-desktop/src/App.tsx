import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import {
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
  Settings as SettingsIcon,
  Share2,
  ShieldCheck,
  Square,
  TestTube2,
  UsersRound,
  WandSparkles,
} from "lucide-react";
import { api } from "./api";
import { AgentManagement } from "./AgentManagement";
import { CapabilityManagement } from "./CapabilityManagement";
import { TeamManagement } from "./TeamManagement";
import { SettingsView } from "./SettingsView";
import { RunTeamPreview } from "./RunTeamPreview";
import { ManagementHero, PageMetric } from "./ManagementHero";
import { environmentSummary } from "./managementLayout";
import { consultationEdgeState, shouldConfirmTeamExit } from "./teamGraph";
import { applyTheme, DEFAULT_THEME, persistTheme, type Theme } from "./theme";
import "./agents.css";
import "./capabilities.css";
import "./newRun.css";
import {
  initialTimelineState,
  capabilityDisplayName,
  isActive,
  reduceEvent,
  runBindingIssues,
  selectedTeamForRun,
  shouldConfirmCapabilityExit,
  toRunRequest,
  type AgentManagementSnapshot,
  type AppView,
  type RunEvent,
  type RunSnapshot,
  type RunSummary,
  type TimelineState,
  type Team,
  type TeamLoadState,
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

type AppProps = { initialTheme?: Theme };

export default function App({ initialTheme = DEFAULT_THEME }: AppProps) {
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<RunSnapshot | null>(null);
  const [timeline, setTimeline] = useState<TimelineState>(initialTimelineState);
  const [repository, setRepository] = useState("");
  const [goal, setGoal] = useState("");
  const [view, setView] = useState<AppView>("runs");
  const [capabilityDirty, setCapabilityDirty] = useState(false);
  const [teamDirty, setTeamDirty] = useState(false);
  const capabilityDirtyRef = useRef(false);
  const teamDirtyRef = useRef(false);
  const teamLoadGeneration = useRef(0);
  const environmentGeneration = useRef(0);
  const [teams, setTeams] = useState<Team[]>([]);
  const [teamLoadState, setTeamLoadState] = useState<TeamLoadState>("loading");
  const [teamId, setTeamId] = useState<string>("default");
  const [runSetupRevision, setRunSetupRevision] = useState(0);
  const [busy, setBusy] = useState(false);
  const [environment, setEnvironment] = useState<"checking" | "ready" | "failed" | "preview">("checking");
  const [agents, setAgents] = useState<AgentManagementSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>(initialTheme);

  useEffect(() => { capabilityDirtyRef.current = capabilityDirty; }, [capabilityDirty]);
  useEffect(() => { teamDirtyRef.current = teamDirty; }, [teamDirty]);

  function changeTheme(nextTheme: Theme) {
    setTheme(nextTheme);
    applyTheme(nextTheme);
    persistTheme(nextTheme);
  }

  async function navigate(nextView: AppView, nextRunId?: string | null) {
    if (shouldConfirmCapabilityExit(view, nextView, capabilityDirty)) {
      const discard = await confirm("Capability 有未保存的修改，确定离开并放弃吗？", { title: "离开 Capability 管理", kind: "warning" });
      if (!discard) return;
    }
    if (shouldConfirmTeamExit(view, nextView, teamDirty)) {
      const discard = await confirm("团队有未保存的修改，确定离开并放弃吗？", { title: "离开团队管理", kind: "warning" });
      if (!discard) return;
    }
    if (nextView !== "capabilities") setCapabilityDirty(false);
    if (nextView !== "teams") setTeamDirty(false);
    if (nextView === "runs" && nextRunId === null) {
      setTeamLoadState("loading"); setEnvironment("checking"); setAgents(null);
      setRunSetupRevision((current) => current + 1);
    }
    setView(nextView);
    if (nextRunId !== undefined) setSelectedId(nextRunId);
  }

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
    }
  }, []);

  async function loadTeams() {
    const generation = ++teamLoadGeneration.current;
    setTeamLoadState("loading");
    try {
      const items = await api.listTeams();
      if (generation !== teamLoadGeneration.current) return;
      setTeams(items);
      if (items.length > 0 && !items.some((item) => item.id === teamId)) setTeamId(items[0].id);
      setTeamLoadState("ready");
    } catch (reason) {
      if (generation !== teamLoadGeneration.current) return;
      setTeams([]);
      setTeamLoadState("failed");
      setError(String(reason));
    }
  }

  useEffect(() => {
    if (!isTauri() || view !== "runs") return;
    void checkEnvironment();
    void loadTeams();
    return () => { teamLoadGeneration.current += 1; environmentGeneration.current += 1; };
  }, [view, runSetupRevision]);

  async function checkEnvironment() {
    const generation = ++environmentGeneration.current;
    setEnvironment("checking");
    setAgents(null);
    try {
      const next = await api.getAgentManagement();
      if (generation !== environmentGeneration.current) return;
      setAgents(next);
      setEnvironment("ready");
    } catch (reason) {
      if (generation !== environmentGeneration.current) return;
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
      if (capabilityDirtyRef.current || teamDirtyRef.current) {
        const discard = await confirm("配置有未保存的修改，确定放弃并关闭吗？", { title: "关闭 Archmage", kind: "warning" });
        if (!discard) return;
      }
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

  const request = useMemo(() => toRunRequest(repository, goal, teamId), [repository, goal, teamId]);
  const selectedTeam = selectedTeamForRun(teams, teamId, teamLoadState);
  const bindingIssues = agents && selectedTeam ? runBindingIssues(agents, selectedTeam.graph.nodes.map((node) => node.capability)) : [];
  const canStart = environment === "ready" && agents !== null && bindingIssues.length === 0 && !!selectedTeam && request.repository.length > 0 && request.goal.length > 0 && !busy;
  const runtimeStatus = environmentSummary(environment);

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
        <div className="brand"><div className="brand-mark"><img src="/archmage-icon.png" alt="" /></div><div><strong>Archmage</strong><span>智能体工作台</span></div></div>
        <button className={`new-run ${view === "runs" && selectedId === null ? "active" : ""}`} onClick={() => navigate("runs", null)}><Plus size={17} />新建任务</button>
        <button className={`new-run ${view === "agents" ? "active" : ""}`} onClick={() => navigate("agents")}><Bot size={17} />智能体管理</button>
        <button className={`new-run ${view === "capabilities" ? "active" : ""}`} onClick={() => navigate("capabilities")}><Share2 size={17} />能力管理</button>
        <button className={`new-run ${view === "teams" ? "active" : ""}`} onClick={() => navigate("teams")}><UsersRound size={17} />团队管理</button>
        <div className="section-label">最近运行</div>
        <nav className="run-list" aria-label="最近运行">
          {runs.map((run) => (
            <button key={run.id} className={view === "runs" && selectedId === run.id ? "active" : ""} onClick={() => navigate("runs", run.id)}>
              <span className={`status-dot ${run.state}`} />
              <span className="run-copy"><strong>{run.goal}</strong><small>{stateLabel(run.state)} · {run.id.slice(0, 8)}</small></span>
              <ChevronRight size={15} />
            </button>
          ))}
          {runs.length === 0 && <p className="empty-copy">还没有 Run。先从右侧创建第一个真实协作流程。</p>}
        </nav>
        <div className="sidebar-bottom">
          <button className={`new-run settings-nav ${view === "settings" ? "active" : ""}`} onClick={() => navigate("settings")}><SettingsIcon size={17} />设置</button>
          <div className="sidebar-foot"><span className="live-dot" />本地 Runtime</div>
        </div>
      </aside>

      <main className="workspace">
        {error && <div className="error-banner" role="alert">{error}<button onClick={() => setError(null)}>关闭</button></div>}
        {view === "agents" ? <AgentManagement onError={setError} onOpenCapabilities={() => setView("capabilities")} /> : view === "capabilities" ? <CapabilityManagement onError={setError} onDirtyChange={setCapabilityDirty} /> : view === "teams" ? <TeamManagement onError={setError} onDirtyChange={setTeamDirty} /> : view === "settings" ? <SettingsView theme={theme} onThemeChange={changeTheme} /> : !selectedId ? (
          <section className="new-run-view">
            <ManagementHero eyebrow="TEAM RUN" title="把目标交给团队" description="定义任务、选择团队，并在启动前确认执行环境。" summary={<PageMetric {...runtimeStatus} />} />
            <div className="new-run-grid">
              <section className="form-card new-run-task-panel">
                <div className="new-run-panel-heading"><div><span>STEP 1</span><h2>任务定义</h2><p>告诉团队在哪里工作，以及需要交付什么。</p></div></div>
                <label htmlFor="repository">Git 仓库<span>必填</span></label>
                <div className="path-field"><FolderGit2 size={18} /><input id="repository" value={repository} onChange={(event) => setRepository(event.target.value)} placeholder="选择一个已有 commit 的本地仓库" /><button onClick={chooseRepository}><FolderOpen size={17} />选择</button></div>
                <label htmlFor="goal">目标<span>必填</span></label>
                <textarea id="goal" value={goal} onChange={(event) => setGoal(event.target.value)} placeholder="描述希望这个团队完成的具体结果……" rows={7} />
              </section>
              <section className="form-card new-run-team-panel">
                <div className="new-run-panel-heading"><div><span>STEP 2</span><h2>执行团队</h2><p>确认参与能力和消息流向。</p></div></div>
                <label htmlFor="run-team">选择团队<span>必填</span></label>
                <select id="run-team" className="run-team-select" value={selectedTeam?.id ?? ""} disabled={teamLoadState !== "ready"} onChange={(event) => setTeamId(event.target.value)}><option value="" disabled>{environment === "preview" ? "请在桌面窗口选择团队" : teamLoadState === "loading" ? "正在读取团队…" : teamLoadState === "failed" ? "团队读取失败" : "请选择团队"}</option>{teamLoadState === "ready" && teams.map((team) => <option key={team.id} value={team.id}>{team.name}</option>)}</select>
                {selectedTeam && <RunTeamPreview team={selectedTeam} />}
                <div className="new-run-issues">
                  {teamLoadState === "ready" && teams.length === 0 && <div className="run-form-issue" role="status"><span>还没有可执行的团队。</span><button type="button" onClick={() => setView("teams")}>前往团队管理</button></div>}
                  {teamLoadState === "failed" && <div className="run-form-issue" role="status"><span>团队列表读取失败，暂时无法启动 Run。</span><button type="button" onClick={() => void loadTeams()}>重新读取</button></div>}
                  {bindingIssues.length > 0 && <div className="run-form-issue" role="status"><span>所选团队有能力尚不可用：{bindingIssues.join("；")}</span><button type="button" onClick={() => setView("capabilities")}>配置 Capability</button></div>}
                  {environment === "failed" && <div className="run-form-issue" role="status"><span>本地 Runtime 检查未通过，请处理顶部错误后重试。</span><button type="button" onClick={checkEnvironment}>重新检查</button></div>}
                  {environment === "preview" && <div className="run-form-issue" role="status"><span>浏览器只提供界面预览；请在桌面窗口启动 Run。</span></div>}
                </div>
                <p className="new-run-freeze-note">启动后将冻结当前团队、能力配置和 Agent 绑定。</p>
                <button className="primary" disabled={!canStart} onClick={startRun}>{busy ? <LoaderCircle className="spin" size={18} /> : <Play size={18} fill="currentColor" />}{busy ? "正在准备…" : environment === "checking" ? "正在检查环境…" : "启动 Run"}</button>
              </section>
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
    <header className="run-header"><div><div className="crumb"><span>Runs</span><ChevronRight size={14} />{snapshot.id.slice(0, 8)}</div><h1>{snapshot.goal}</h1><div className="run-meta"><span className={`state-pill ${snapshot.state}`}><Circle size={8} fill="currentColor" />{stateLabel(snapshot.state, snapshot.outcome)}</span><span>{snapshot.repository}</span>{snapshot.team && <span>团队：{snapshot.team.name} · v{snapshot.team.revision}</span>}<code>{snapshot.id}</code></div></div><div className="header-actions"><button onClick={onRefresh}><RefreshCw size={16} />刷新</button><button onClick={onReveal} disabled={!snapshot.workspace}><FolderOpen size={16} />工作区</button>{isActive(snapshot.state) && <button className="danger" onClick={onCancel}><Square size={14} fill="currentColor" />取消</button>}</div></header>
    <section className="node-strip" aria-label="能力进度">{snapshot.nodes.map((item, index) => { const meta = nodeMeta[item.key]; const binding = snapshot.agentBindings[item.key]; const Icon = meta.icon; return <button key={item.id} className={`${item.state} ${selectedNode === item.id ? "selected" : ""}`} onClick={() => setSelectedNode(item.id)}><span className="node-index">{item.state === "succeeded" ? <Check size={15} /> : item.state === "running" ? <LoaderCircle className="spin" size={15} /> : index + 1}</span><Icon size={18} /><span><strong>{capabilityDisplayName(snapshot.capabilityConfigurations[item.key], item.key)}</strong><small>{stateLabel(item.state)} · {binding ? `${binding.displayName} (${binding.providerKind})` : "历史配置"}</small></span></button>; })}</section>
    {snapshot.team && <section className="panel team-run-panel"><div className="panel-title"><div><span>冻结的团队拓扑</span><small>{snapshot.team.name} · v{snapshot.team.revision}</small></div></div><div className="team-run-edges">{snapshot.team.graph.edges.map((edge) => <span key={`${edge.from}-${edge.to}`}>{edge.from} → {edge.to}</span>)}</div></section>}
    {snapshot.team?.graph.mode === "flow" ? <FlowProgress snapshot={snapshot} /> : snapshot.team && <ConsultationProgress snapshot={snapshot} />}
    <div className="run-grid"><section className="panel timeline-panel"><div className="panel-title"><div><span>运行时间线</span><small>{timeline.events.length} 个持久化事件</small></div><span className="sequence">SEQ {timeline.lastSequence}</span></div><div className="timeline">{timeline.events.map((event) => <EventRow key={event.sequence} event={event} />)}{timeline.events.length === 0 && <p className="empty-copy">等待 Runtime 事件……</p>}</div></section><section className="panel inspector"><div className="panel-title"><div><span>节点检查器</span><small>{node ? nodeMeta[node.key].label : "—"}</small></div></div>{node ? <><div className="inspect-state"><span className={`status-dot ${node.state}`} /><strong>{stateLabel(node.state)}</strong><code>{node.id}</code></div>{node.result ? <><div className="result-meta"><MetaItem label="Codex Thread" value={node.result.threadId} /><MetaItem label="Attempt" value={node.result.attemptId} /><MetaItem label="Result Artifact" value={node.result.resultArtifactId} />{node.result.changesetArtifactId && <MetaItem label="ChangeSet" value={node.result.changesetArtifactId} />}</div><div className="inspect-section output-section"><label>结构化结果</label><ResultSummary output={node.result.output} /><details><summary>查看原始 JSON</summary><pre>{JSON.stringify(node.result.output, null, 2)}</pre></details></div></> : <div className="waiting-result"><LoaderCircle className={node.state === "running" ? "spin" : ""} size={17} /><div><strong>尚无结构化结果</strong><span>{node.state === "running" ? "Codex 正在执行，完成后会在此显示结果和 Artifact。" : "节点开始并成功完成后，结果会持久化到这里。"}</span></div></div>}<div className="inspect-section"><label>下游咨询</label>{snapshot.consultations.filter((item) => item.sourceNodeId === node.id).map((item) => <div className="consultation-card" key={item.id}><strong>{item.targetCapability} · {item.decision === "approved" ? "认可" : "要求修改"} · 第 {item.round + 1} 轮</strong><p>{item.summary}</p>{item.suggestions.map((suggestion) => <p key={suggestion}>建议：{suggestion}</p>)}<code>{item.threadId}</code></div>)}{!snapshot.consultations.some((item) => item.sourceNodeId === node.id) && <p className="empty-copy">暂无咨询结果。</p>}</div><div className="inspect-section"><label>运行记录</label>{nodeEvents.length ? nodeEvents.map((event) => <div className="compact-event" key={event.sequence}><span>#{event.sequence} · {event.event_type}</span>{Object.keys(event.payload).length > 0 && <pre>{JSON.stringify(event.payload, null, 2)}</pre>}</div>) : <p className="empty-copy">该节点还没有事件。</p>}</div></> : null}</section></div>
  </div>;
}

function ConsultationProgress({ snapshot }: { snapshot: RunSnapshot }) {
  const team = snapshot.team;
  if (!team) return null;
  return <section className="panel team-run-panel"><div className="panel-title"><div><span>审阅进度</span><small>咨询与主任务分开；每条边都必须认可当前上游结果</small></div></div><div className="team-run-edges">{team.graph.edges.map((edge) => <span key={`${edge.from}-${edge.to}`}>{edge.from} → {edge.to} · {consultationEdgeState(snapshot, edge)}</span>)}</div></section>;
}

function FlowProgress({ snapshot }: { snapshot: RunSnapshot }) {
  const deliveries = snapshot.flowDeliveries ?? [];
  const stateLabels: Record<string, string> = {
    queued: "待执行", running: "执行中", completed: "已交付", failed: "失败",
    cancelled: "已取消", interrupted: "已中断", skipped_due_terminal: "流程完成，已跳过",
    skipped_due_failure: "流程失败，已跳过", skipped_due_cancel: "流程取消，已跳过",
    skipped_due_interruption: "流程中断，已跳过",
  };
  return <section className="panel team-run-panel"><div className="panel-title"><div><span>交付消息</span><small>每条消息独立触发一次目标能力；最多执行 24 次</small></div></div><div className="flow-deliveries">
    {deliveries.map((item) => <div key={item.id} className="flow-delivery"><strong>#{item.sequence} · {item.target}</strong><span>{stateLabels[item.state] ?? item.state}</span>{item.routing && <code>{item.routing.decision === "complete" ? "完成" : `→ ${item.routing.next.join(", ")}`}</code>}</div>)}
    {!deliveries.length && <p className="empty-copy">尚未产生交付消息。</p>}
  </div></section>;
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
