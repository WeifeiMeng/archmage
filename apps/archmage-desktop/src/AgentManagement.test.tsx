import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => false }));

import { AgentManagement, AgentManagementContent } from "./AgentManagement";
import type { AgentManagementSnapshot } from "./model";

const snapshot: AgentManagementSnapshot = {
  adapters: [
    { kind: "codex", displayName: "Codex", supported: true },
    { kind: "deepseek_harness", displayName: "DeepSeek Harness", supported: true },
    { kind: "hermes", displayName: "Hermes", supported: false },
  ],
  instances: [{
    id: "codex-default",
    providerKind: "codex",
    displayName: "Codex",
    executablePath: null,
    providerRoute: null,
    model: null,
    enabled: true,
    connectionState: "ready",
    discoveredVersion: "codex-cli 0.154.0",
    lastVerifiedAt: null,
    lastError: null,
    verificationThreadId: "thread-1",
  }],
  bindings: [{ capability: "coder", agentInstanceId: "codex-default" }],
};

describe("Agent Management browser preview", () => {
  it("uses the shared page Hero and explains the preview boundary", () => {
    const html = renderToStaticMarkup(<AgentManagement onError={() => undefined} onOpenCapabilities={() => undefined} />);

    expect(html).toContain("management-hero");
    expect(html).toContain("Agent 管理");
    expect(html).toContain("当前是浏览器预览");
    expect(html).not.toContain("正在读取本地 Agent");
  });

  it("groups supported instances and scopes creation to the selected provider", () => {
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={snapshot}
        busy={null}
        addingProvider="deepseek_harness"
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).not.toContain("LOCAL AGENT CONNECTIONS");
    expect(html).toContain("添加 Codex");
    expect(html).toContain("添加 DeepSeek Harness");
    expect(html).toContain("添加 DeepSeek Harness 实例");
    expect(html).not.toContain("添加 Codex 实例");
    expect(html).not.toContain("<select");
  });

  it("uses provider section styles instead of a highlighted Hero action", () => {
    const css = readFileSync(new URL("./agents.css", import.meta.url), "utf8");

    expect(css).toContain(".agent-provider-section");
    expect(css).toContain(".agent-provider-heading");
    expect(css).not.toContain(".agents-view .management-hero-actions");
  });
});
