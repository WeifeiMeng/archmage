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

    const bodyStart = html.indexOf('<div class="settings-shell-body">');
    const navStart = html.indexOf('<nav class="settings-sections" aria-label="设置分类">');
    const navEnd = html.indexOf("</nav>", navStart);
    const contentStart = html.indexOf('<div class="settings-shell-content">');
    expect(bodyStart).toBeLessThan(navStart);
    expect(navStart).toBeLessThan(navEnd);
    expect(navEnd).toBeLessThan(contentStart);

    const navHtml = html.slice(navStart, navEnd + "</nav>".length);
    expect(navHtml).toMatch(/^<nav class="settings-sections" aria-label="设置分类"><span class="settings-sidebar-brand">ARCHMAGE<\/span>/);
    expect(navHtml).toMatch(/<button[^>]*class="settings-back"[^>]*>[\s\S]*返回主界面<\/button><\/nav>$/);
    expect(html).toMatch(/^<section class="settings-shell"><div class="settings-shell-body"><nav class="settings-sections" aria-label="设置分类">[\s\S]*<\/nav><div class="settings-shell-content"><div>Agent content<\/div><\/div><\/div><\/section>$/);
    expect(html).not.toContain("settings-shell-header");
    expect(html).not.toContain("settings-shell-title");
    expect(html).not.toContain("<h1");
  });

  it("scopes the flat settings styles and keeps the responsive navigation", () => {
    expect(settingsStyles).toContain(".settings-sections button.active::before");
    expect(settingsStyles).toContain(".settings-shell-content .management-hero");
    expect(settingsStyles).toContain(".settings-shell-content .management-hero::after");
    expect(settingsStyles).toContain("@media (max-width: 960px)");
    expect(settingsStyles).not.toContain(".settings-shell-header");
    expect(settingsStyles).not.toContain("box-shadow: var(--shadow)");
  });
});
