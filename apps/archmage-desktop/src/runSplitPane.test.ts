import { describe, expect, it } from "vitest";
import {
  adjustInspectorRatio,
  clampActiveInspectorRatio,
  clampInspectorRatio,
  parseStoredInspectorRatio,
} from "./runSplitRatio";

describe("run split pane ratio", () => {
  it("restores valid preferences and falls back for malformed values", () => {
    expect(parseStoredInspectorRatio(null)).toBe(43);
    expect(parseStoredInspectorRatio("66")).toBe(66);
    expect(parseStoredInspectorRatio("bad")).toBe(43);
    expect(parseStoredInspectorRatio("5")).toBe(43);
  });

  it("clamps both panels to their minimum width", () => {
    expect(clampInspectorRatio(90, 1000, 320, 12)).toBeCloseTo(66.8);
    expect(clampInspectorRatio(10, 1000, 320, 12)).toBe(32);
    expect(clampInspectorRatio(43, 600, 320, 12)).toBe(43);
  });

  it("stores the ratio that is actually visible after an active resize", () => {
    expect(clampActiveInspectorRatio(10, 1000)).toBe(32);
    expect(clampActiveInspectorRatio(90, 1000)).toBeCloseTo(66.8);
    expect(clampActiveInspectorRatio(48, 1000)).toBe(48);
  });

  it("supports keyboard steps and boundary shortcuts", () => {
    expect(adjustInspectorRatio(43, "ArrowRight")).toBe(45);
    expect(adjustInspectorRatio(43, "ArrowLeft")).toBe(41);
    expect(adjustInspectorRatio(43, "Home")).toBe(20);
    expect(adjustInspectorRatio(43, "End")).toBe(80);
    expect(adjustInspectorRatio(43, "Enter")).toBeNull();
  });
});
