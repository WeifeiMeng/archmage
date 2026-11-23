import { readFileSync, statSync } from "node:fs";
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

  it("validates the mounted DMG and application bundle fail-closed", () => {
    const verifierUrl = new URL("../../../scripts/verify-macos-release.sh", import.meta.url);
    const verifier = readFileSync(verifierUrl, "utf8");
    expect(statSync(verifierUrl).mode & 0o111).not.toBe(0);
    expect(verifier).toContain("set -euo pipefail");
    expect(verifier).toContain("hdiutil verify");
    expect(verifier).toContain("hdiutil attach");
    expect(verifier).toContain("lipo -archs");
    expect(verifier).toContain("CFBundleIdentifier");
    expect(verifier).toContain("CFBundleShortVersionString");
    expect(verifier).toContain("LSMinimumSystemVersion");
    expect(verifier).toContain("dev.archmage.workbench");
  });

  it("builds manually but publishes previews only from version tags", () => {
    const workflow = readFileSync(
      new URL("../../../.github/workflows/release-macos.yml", import.meta.url),
      "utf8",
    );
    const releaseNotes = readFileSync(
      new URL("../../../.github/release-notes/v1.0.0.md", import.meta.url),
      "utf8",
    );

    expect(workflow).toContain("workflow_dispatch:");
    expect(workflow).toMatch(/tags:\s*\n\s*- ["']v\*["']/);
    expect(workflow).toContain("permissions:\n  contents: read");
    expect(workflow).toContain("runs-on: macos-15");
    expect(workflow).toContain("cargo clippy --workspace --all-targets -- -D warnings");
    expect(workflow).toContain(
      "npm run tauri -- build --target aarch64-apple-darwin --bundles dmg",
    );
    expect(workflow).toContain("scripts/verify-macos-release.sh");
    expect(workflow).toContain("shasum -a 256");
    expect(workflow).toMatch(/release:\s*[\s\S]*needs: build/);
    expect(workflow).toMatch(/release:\s*[\s\S]*contents: write/);
    expect(workflow).toContain("github.ref_type == 'tag'");
    expect(workflow).toContain("github.ref_name");
    expect(workflow).toContain("--prerelease");
    expect(workflow).toContain("archmage-macos-arm64-");

    for (const phrase of [
      "Apple Silicon",
      "macOS 14+",
      "unsigned",
      "not notarized",
      "Open Anyway",
      "Provider",
      "manual upgrade",
      "shasum -a 256",
      "Apple 芯片",
      "未签名",
      "未公证",
      "仍要打开",
      "手动升级",
    ]) {
      expect(releaseNotes).toContain(phrase);
    }
  });
});
