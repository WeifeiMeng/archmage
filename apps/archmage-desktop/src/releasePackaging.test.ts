import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

function json(relative: string) {
  return JSON.parse(readFileSync(new URL(relative, import.meta.url), "utf8"));
}

describe("macOS release packaging contract", () => {
  it("builds only a macOS 14+ DMG with the stable identity and icon", () => {
    const config = json("../src-tauri/tauri.conf.json");
    expect(config.version).toBe("1.0.0");
    expect(config.identifier).toBe("dev.archmage.workbench");
    expect(config.bundle.active).toBe(true);
    expect(config.bundle.targets).toEqual(["dmg"]);
    expect(config.bundle.macOS.minimumSystemVersion).toBe("14.0");
    expect(config.bundle.icon).toContain("icons/icon.icns");
    expect(config.plugins?.updater).toBeUndefined();
  });
});
