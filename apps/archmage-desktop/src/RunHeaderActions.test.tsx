import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { RunHeaderActions } from "./RunHeaderActions";

const callbacks = { onRefresh: vi.fn(), onReveal: vi.fn(), onCancel: vi.fn() };

describe("Run header actions", () => {
  it("groups the active Run actions and disables an unavailable workspace", () => {
    const html = renderToStaticMarkup(
      <RunHeaderActions {...callbacks} workspaceAvailable={false} cancellable />,
    );
    expect(html).toContain('aria-label="Run 操作"');
    expect(html).toContain("刷新");
    expect(html).toContain("工作区");
    expect(html).toContain("取消");
    expect(html).toContain("disabled");
  });

  it("omits cancel after the Run becomes inactive", () => {
    const html = renderToStaticMarkup(
      <RunHeaderActions {...callbacks} workspaceAvailable cancellable={false} />,
    );
    expect(html).not.toContain("取消");
  });
});
