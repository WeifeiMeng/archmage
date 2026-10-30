import { describe, expect, it } from "vitest";
import type { AgentInstance, AgentManagementSnapshot, CapabilityConfiguration, CapabilityManagementSnapshot, TeamGraph } from "./model";
import { agentMetrics, capabilityMetrics, environmentSummary, teamSummary } from "./managementLayout";

const instance = (id: string, state: AgentInstance["connectionState"], enabled = true, providerKind: AgentInstance["providerKind"] = "codex"): AgentInstance => ({
  id,
  providerKind,
  displayName: id,
  executablePath: null,
  providerRoute: null,
  model: null,
  enabled,
  connectionState: state,
  discoveredVersion: null,
  lastVerifiedAt: null,
  lastError: null,
  verificationThreadId: null,
});

const configuration = (key: CapabilityConfiguration["key"], agentInstanceId: string, skills: number, tools: number): CapabilityConfiguration => ({
  key,
  displayName: key,
  description: "",
  extraInstructions: "",
  skills: Array.from({ length: skills }, (_, index) => ({ name: `skill-${index}`, purpose: "test" })),
  tools: Array.from({ length: tools }, (_, index) => ({ name: `tool-${index}`, purpose: "test" })),
  agentInstanceId,
  revision: 1,
  promptVersion: "v1",
});

describe("management layout summaries", () => {
  it("describes every Runtime environment state", () => {
    expect(environmentSummary("checking")).toEqual({ label: "本地 Runtime", value: "正在检查", detail: "正在读取本地 Agent 与团队配置", tone: "default" });
    expect(environmentSummary("ready")).toEqual({ label: "本地 Runtime", value: "已就绪", detail: "Agent 与团队配置可用", tone: "ready" });
    expect(environmentSummary("failed")).toEqual({ label: "本地 Runtime", value: "检查失败", detail: "处理错误后可重新检查", tone: "failed" });
    expect(environmentSummary("preview")).toEqual({ label: "本地 Runtime", value: "浏览器预览", detail: "请在桌面窗口启动 Run", tone: "warning" });
  });

  it("counts supported enabled Agent connection states", () => {
    const snapshot: AgentManagementSnapshot = {
      adapters: [{ kind: "codex", displayName: "Codex", supported: true }, { kind: "claude", displayName: "Claude", supported: false }],
      instances: [instance("ready", "ready"), instance("dsh", "ready", true, "deepseek_harness"), instance("unverified", "unverified"), instance("failed", "failed"), instance("disabled", "ready", false), instance("claude", "ready", true, "claude")],
      bindings: [],
    };

    expect(agentMetrics(snapshot)).toEqual([
      { label: "已验证", value: 2, tone: "ready" },
      { label: "未验证", value: 1, tone: "warning" },
      { label: "连接失败", value: 1, tone: "failed" },
      { label: "总实例", value: 5, tone: "default" },
    ]);
  });

  it("summarizes Capability bindings and guidance", () => {
    const snapshot: CapabilityManagementSnapshot = {
      configurations: [configuration("designer", "ready", 2, 1), configuration("coder", "disabled", 1, 2), configuration("tester", "missing", 0, 1), configuration("reviewer", "ready", 2, 0)],
      instances: [instance("ready", "ready"), instance("disabled", "ready", false)],
      edges: [],
    };

    expect(capabilityMetrics(snapshot)).toEqual({ capabilities: 4, bound: 2, skills: 5, tools: 4 });
  });

  it("summarizes flow and legacy Team graphs", () => {
    const flow: TeamGraph = {
      mode: "flow",
      nodes: [{ capability: "designer", x: 0, y: 0 }, { capability: "coder", x: 200, y: 0 }],
      edges: [{ from: "designer", to: "coder" }],
      entry: "designer",
      terminal: "coder",
    };
    expect(teamSummary(flow)).toEqual({ capabilities: 2, routes: 1, modeLabel: "消息流" });
    expect(teamSummary({ ...flow, mode: "legacy" })).toEqual({ capabilities: 2, routes: 1, modeLabel: "历史咨询" });
    expect(teamSummary({ mode: "flow", nodes: [], edges: [], entry: null, terminal: null })).toEqual({ capabilities: 0, routes: 0, modeLabel: "消息流" });
  });
});
