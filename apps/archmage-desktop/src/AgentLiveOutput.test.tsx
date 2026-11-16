import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentLiveOutput } from "./AgentLiveOutput";

describe("AgentLiveOutput", () => {
  it("makes Capability primary and Provider secondary", () => {
    const html = renderToStaticMarkup(<AgentLiveOutput
      capability="Coder"
      provider="Codex · codex"
      state="running"
      progress={[{ sequence: 1, kind: "tool", label: "Run tests", status: "running", text: "cargo test", metadata: {} }]}
    />);
    expect(html).toContain("Coder 正在运行");
    expect(html).toContain("Codex · codex");
    expect(html).toContain("Run tests");
    expect(html).toContain("cargo test");
    expect(html).not.toContain("Codex 正在运行");
  });

  it("explains a running node that has not emitted public progress", () => {
    const html = renderToStaticMarkup(<AgentLiveOutput capability="Reviewer" provider="Qoder · qoder" state="running" progress={[]} />);
    expect(html).toContain("Reviewer 正在运行");
    expect(html).toContain("尚未产生可展示的 Agent 输出");
  });

  it("keeps historical progress after completion", () => {
    const html = renderToStaticMarkup(<AgentLiveOutput
      capability="Designer"
      provider="DeepSeek Harness · deepseek_harness"
      state="succeeded"
      progress={[{ sequence: 4, kind: "message", label: "Agent message", status: null, text: "Design complete", metadata: {} }]}
    />);
    expect(html).toContain("Designer 已完成");
    expect(html).toContain("Design complete");
  });
});
