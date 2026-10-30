import { isSupportedProvider, type AgentManagementSnapshot, type CapabilityManagementSnapshot, type TeamGraph } from "./model";
import type { PageMetricTone } from "./ManagementHero";

export type EnvironmentState = "checking" | "ready" | "failed" | "preview";
export type PageStatus = { label: string; value: string; detail: string; tone: PageMetricTone };
export type PageMetricData = { label: string; value: number; tone: PageMetricTone };

export function environmentSummary(state: EnvironmentState): PageStatus {
  const states: Record<EnvironmentState, Omit<PageStatus, "label">> = {
    checking: { value: "正在检查", detail: "正在读取本地 Agent 与团队配置", tone: "default" },
    ready: { value: "已就绪", detail: "Agent 与团队配置可用", tone: "ready" },
    failed: { value: "检查失败", detail: "处理错误后可重新检查", tone: "failed" },
    preview: { value: "浏览器预览", detail: "请在桌面窗口启动 Run", tone: "warning" },
  };
  return { label: "本地 Runtime", ...states[state] };
}

export function agentMetrics(snapshot: AgentManagementSnapshot): PageMetricData[] {
  const supported = snapshot.instances.filter((item) => isSupportedProvider(item.providerKind));
  const active = supported.filter((item) => item.enabled);
  return [
    { label: "已验证", value: active.filter((item) => item.connectionState === "ready").length, tone: "ready" },
    { label: "未验证", value: active.filter((item) => item.connectionState === "unverified").length, tone: "warning" },
    { label: "连接失败", value: active.filter((item) => item.connectionState === "failed").length, tone: "failed" },
    { label: "总实例", value: supported.length, tone: "default" },
  ];
}

export function capabilityMetrics(snapshot: CapabilityManagementSnapshot) {
  const bound = snapshot.configurations.filter((configuration) => {
    const instance = snapshot.instances.find((item) => item.id === configuration.agentInstanceId);
    return !!instance?.enabled && isSupportedProvider(instance.providerKind);
  }).length;
  return {
    capabilities: snapshot.configurations.length,
    bound,
    skills: snapshot.configurations.reduce((total, item) => total + item.skills.length, 0),
    tools: snapshot.configurations.reduce((total, item) => total + item.tools.length, 0),
  };
}

export function teamSummary(graph: TeamGraph) {
  return {
    capabilities: graph.nodes.length,
    routes: graph.edges.length,
    modeLabel: graph.mode === "flow" ? "消息流" : "历史咨询",
  };
}
