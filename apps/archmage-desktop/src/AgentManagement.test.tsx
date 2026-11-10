import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => false }));

import { AgentManagement, AgentManagementContent } from "./AgentManagement";
import type { AgentInstallGuidance, AgentManagementSnapshot } from "./model";

const guidance: AgentInstallGuidance[] = [
  {
    providerKind: "codex", platform: "macos", architecture: "aarch64",
    supported: true, detected: false, prerequisite: null,
    command: "curl -fsSL https://chatgpt.com/codex/install.sh | sh",
    documentationUrl: "https://github.com/openai/codex#quickstart", unsupportedReason: null,
  },
  {
    providerKind: "qoder", platform: "macos", architecture: "aarch64",
    supported: true, detected: true, prerequisite: null,
    command: "curl -fsSL https://qoder.com/install | bash",
    documentationUrl: "https://docs.qoder.com/cli/installation", unsupportedReason: null,
  },
  {
    providerKind: "deepseek_harness", platform: "windows", architecture: "x86_64",
    supported: false, detected: false, prerequisite: "Python 3.10 或更高版本",
    command: null, documentationUrl: "https://github.com/deepseek-ai/deepseek-harness",
    unsupportedReason: "当前 DeepSeek Harness Python SDK 暂不支持 Windows Agent。",
  },
];

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
    agentName: null,
    providerConfig: {},
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
        guidance={guidance}
        activeGuide={null}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).not.toContain("LOCAL AGENT CONNECTIONS");
    expect(html).toContain("添加 Codex");
    expect(html).toContain("添加 DeepSeek Harness");
    expect(html).toContain("添加 Qoder");
    expect(html).toContain("添加 DeepSeek Harness 实例");
    expect(html).not.toContain("添加 Codex 实例");
    expect(html).not.toContain("<select");
  });

  it("shows explicit Qoder ACP configuration fields", () => {
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={{ ...snapshot, adapters: [...snapshot.adapters, { kind: "qoder", displayName: "Qoder", supported: true }] }}
        busy={null}
        addingProvider="qoder"
        guidance={guidance}
        activeGuide={null}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).toContain("添加 Qoder 实例");
    expect(html).toContain("Qoder Agent");
    expect(html).toContain("显式扩展配置");
    expect(html).toContain("mcpServers");
    expect(html).toContain("Archmage 不保存 Token");
  });

  it("shows platform guidance for a missing provider without executing it", () => {
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={{ ...snapshot, instances: [], bindings: [] }}
        busy={null}
        addingProvider={null}
        guidance={guidance}
        activeGuide={{ provider: "codex" }}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).toContain("安装指引");
    expect(html).toContain("curl -fsSL https://chatgpt.com/codex/install.sh | sh");
    expect(html).toContain("复制命令");
    expect(html).toContain("重新检查");
  });

  it("does not flag a provider as missing when a custom-path instance is already discovered", () => {
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={snapshot}
        busy={null}
        addingProvider={null}
        guidance={guidance}
        activeGuide={null}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html.match(/>安装指引</g)).toHaveLength(1);
  });

  it("explains unsupported DSH platforms without offering an invalid command", () => {
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={snapshot}
        busy={null}
        addingProvider={null}
        guidance={guidance}
        activeGuide={{ provider: "deepseek_harness" }}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).toContain("暂不支持 Windows Agent");
    expect(html).not.toContain("复制命令");
    expect(html).not.toContain("重新检查");
  });

  it("offers guidance from a failed versionless instance", () => {
    const failedSnapshot: AgentManagementSnapshot = {
      ...snapshot,
      instances: [{ ...snapshot.instances[0], connectionState: "failed", discoveredVersion: null, lastError: "codex executable not found" }],
    };
    const html = renderToStaticMarkup(
      <AgentManagementContent
        snapshot={failedSnapshot}
        busy={null}
        addingProvider={null}
        guidance={guidance}
        activeGuide={null}
        onOpenGuidance={() => undefined}
        onCloseGuidance={() => undefined}
        onRefreshGuidance={async () => undefined}
        onToggleAdding={() => undefined}
        onRun={async () => true}
        onOpenCapabilities={() => undefined}
      />,
    );

    expect(html).toContain("查看安装指引");
  });

  it("uses provider section styles instead of a highlighted Hero action", () => {
    const css = readFileSync(new URL("./agents.css", import.meta.url), "utf8");

    expect(css).toContain(".agent-provider-section");
    expect(css).toContain(".agent-provider-heading");
    expect(css).not.toContain(".agents-view .management-hero-actions");
  });
});
