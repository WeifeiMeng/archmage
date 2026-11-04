import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SettingsShell } from "./SettingsShell";
import { readFileSync } from "node:fs";

const settingsStyles = readFileSync(new URL("./settings.css", import.meta.url), "utf8");

describe("SettingsShell", () => {
  it("renders flat settings navigation with the back action inside the sidebar", () => {
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
    expect(html).toContain('class="settings-sidebar-brand"');
    expect(html).toMatch(/<nav[^>]*class="settings-sections"[\s\S]*class="settings-back"/);
    expect(html).not.toContain("settings-shell-header");
    expect(html).not.toContain("settings-shell-title");
    expect(html).not.toContain("<h1");
    expect(settingsStyles).toContain("@media (max-width: 960px)");
  });
});
