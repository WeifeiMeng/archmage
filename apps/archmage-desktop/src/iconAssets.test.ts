import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const text = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

describe("Archmage icon assets", () => {
  it("uses the wizard portrait in the sidebar and browser favicon", () => {
    expect(text("./App.tsx")).toContain('src="/archmage-icon.png"');
    expect(text("../index.html")).toContain('href="/archmage-icon.png"');
    expect(text("../public/archmage-icon.png").length).toBeGreaterThan(0);
  });

  it("configures non-empty Tauri desktop icons", () => {
    const config = text("../src-tauri/tauri.conf.json");
    for (const path of ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]) {
      expect(config).toContain(`"${path}"`);
      expect(text(`../src-tauri/${path}`).length).toBeGreaterThan(0);
    }
  });
});
