import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readCss = (name: string) => readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
const swimlaneCss = readCss("flowSwimlane.css");

describe("delivery swimlane styles", () => {
  it("sizes the delivery panel from its content instead of the page", () => {
    expect(swimlaneCss).toMatch(/\.panel\.flow-panel\s*\{[^}]*min-height:\s*0;/s);
    expect(swimlaneCss).not.toMatch(/\.panel\.flow-panel\s*\{[^}]*min-height:\s*calc\(100vh/s);
    expect(swimlaneCss).toMatch(/\.flow-empty\s*\{[^}]*padding:\s*14px 17px/s);
  });

  it("degrades to vertical delivery cards on narrow viewports", () => {
    const narrow = swimlaneCss.slice(swimlaneCss.indexOf("@media (max-width: 1080px)"));
    expect(narrow).toMatch(/\.flow-swimlane,\s*\.flow-swimlane thead,\s*\.flow-swimlane tbody,\s*\.flow-swimlane tr,\s*\.flow-swimlane th,\s*\.flow-swimlane td\s*\{[^}]*display:\s*block;/s);
    expect(narrow).toMatch(/\.flow-swimlane thead\s*\{\s*display:\s*none;/s);
    expect(narrow).toMatch(/\.flow-lane-status\s*\{\s*display:\s*none;/s);
  });

  it("keeps status readable without colour alone and inside the lane cell", () => {
    expect(swimlaneCss).toMatch(/\.flow-delivery-state\s*\{[^}]*border-radius:\s*999px/s);
    expect(swimlaneCss).toMatch(/\.flow-delivery-detail\s*\{[^}]*border-top:\s*1px solid var\(--border\)/s);
    expect(swimlaneCss).toMatch(/\.flow-detail-raw pre\s*\{[^}]*white-space:\s*pre-wrap/s);
    expect(readCss("styles.css")).toMatch(/@media\(prefers-reduced-motion:reduce\)/);
  });
});
