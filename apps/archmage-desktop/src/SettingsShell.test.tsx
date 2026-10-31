import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SettingsShell } from "./SettingsShell";

describe("SettingsShell", () => {
  it("renders a full-screen settings navigator around the active management page", () => {
    const html = renderToStaticMarkup(
      <SettingsShell activeView="agents" onNavigate={() => undefined} onBack={() => undefined}>
        <div>Agent content</div>
      </SettingsShell>,
    );

    expect(html).toContain("返回主界面");
    expect(html).toContain("外观");
    expect(html).toContain("智能体管理");
    expect(html).toContain("能力管理");
    expect(html).toContain("团队管理");
    expect(html).toContain('aria-current="page"');
    expect(html).toContain("Agent content");
  });
});
