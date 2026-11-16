import { LoaderCircle } from "lucide-react";
import type { AgentProgress } from "./agentProgress";

const kindLabel: Record<AgentProgress["kind"], string> = {
  message: "Agent 消息",
  plan: "计划",
  tool: "工具",
  notice: "Provider 通知",
  usage: "用量",
  status: "状态",
};

function capabilityState(capability: string, state: string) {
  if (state === "running") return `${capability} 正在运行`;
  if (state === "succeeded") return `${capability} 已完成`;
  if (state === "blocked") return `${capability} 已阻塞`;
  if (state === "failed") return `${capability} 执行失败`;
  if (state === "cancelled") return `${capability} 已取消`;
  if (state === "interrupted") return `${capability} 已中断`;
  return `${capability} 尚未运行`;
}

function metadataText(metadata: AgentProgress["metadata"]) {
  return Object.entries(metadata).map(([key, value]) => `${key}: ${String(value)}`).join(" · ");
}

export function AgentLiveOutput({ capability, provider, state, progress }: {
  capability: string;
  provider: string;
  state: string;
  progress: AgentProgress[];
}) {
  return <section className="agent-live-output" aria-label="Agent 实时输出">
    <div className="agent-live-heading">
      <div className="agent-live-title">
        {state === "running" && <LoaderCircle className="spin" size={15} />}
        <div><strong>{capabilityState(capability, state)}</strong><span>{provider}</span></div>
      </div>
      {progress.length > 0 && <span className="agent-live-count">{progress.length} 条</span>}
    </div>
    {progress.length > 0 ? <ol className="agent-progress-feed" aria-live="polite">
      {progress.map((item) => <li key={item.sequence}>
        <div className="agent-progress-meta"><span>{kindLabel[item.kind]}</span>{item.status && <code>{item.status}</code>}</div>
        <strong>{item.label}</strong>
        {item.text && <pre>{item.text}</pre>}
        {Object.keys(item.metadata).length > 0 && <small>{metadataText(item.metadata)}</small>}
      </li>)}
    </ol> : <p className="agent-live-empty">{state === "running" ? "尚未产生可展示的 Agent 输出，正在等待公开消息、计划或工具活动。" : "此 Attempt 没有可展示的 Agent 输出。"}</p>}
  </section>;
}
