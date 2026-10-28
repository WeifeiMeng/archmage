import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => false }));

import { AgentManagement } from "./AgentManagement";

describe("Agent Management browser preview", () => {
  it("uses the shared page Hero and explains the preview boundary", () => {
    const html = renderToStaticMarkup(<AgentManagement onError={() => undefined} onOpenCapabilities={() => undefined} />);

    expect(html).toContain("management-hero");
    expect(html).toContain("Agent 管理");
    expect(html).toContain("当前是浏览器预览");
    expect(html).not.toContain("正在读取本地 Agent");
  });
});
