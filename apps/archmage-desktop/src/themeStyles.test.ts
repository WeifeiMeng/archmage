import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readCss = (name: string) => readFileSync(new URL(`./${name}`, import.meta.url), "utf8");

describe("application theme styles", () => {
  it("defines a light default and a complete dark override", () => {
    const css = readCss("theme.css");
    const variables = [
      "bg", "surface", "surface-2", "surface-raised", "panel-gradient", "border", "muted", "text",
      "accent", "accent-strong", "accent-soft", "input-bg", "sidebar-bg", "hero-gradient", "canvas-bg",
      "shadow", "danger", "danger-bg", "warning", "warning-bg", "info", "info-bg",
    ];
    expect(css).toMatch(/:root\s*\{/);
    expect(css).toMatch(/:root\[data-theme=["']dark["']\]\s*\{/);
    for (const variable of variables) {
      expect(css.match(new RegExp(`--${variable}:`, "g"))).toHaveLength(2);
    }
  });

  it("routes every primary page surface through semantic theme tokens", () => {
    expect(readCss("styles.css")).toMatch(/\.form-card,\s*\.panel\s*\{[^}]*background:\s*var\(--panel-gradient\)/s);
    expect(readCss("management.css")).toMatch(/\.management-hero\s*\{[^}]*background:\s*var\(--hero-gradient\)/s);
    expect(readCss("agents.css")).toMatch(/\.agent-card\s*\{[^}]*background:\s*var\(--panel-gradient\)/s);
    expect(readCss("capabilities.css")).toMatch(/\.capability-list,\s*\.cap-editor\s*\{[^}]*background:\s*var\(--panel-gradient\)/s);
    expect(readCss("teams.css")).toMatch(/\.team-config-panel,\s*\.team-canvas-panel\s*\{[^}]*background:\s*var\(--panel-gradient\)/s);
    expect(readCss("newRun.css")).toMatch(/\.run-team-select\s*\{[^}]*background:\s*var\(--input-bg\)/s);
    expect(readCss("runTeam.css")).toMatch(/\.run-team-preview\s*\{[^}]*background:\s*var\(--surface-2\)/s);
  });
});
