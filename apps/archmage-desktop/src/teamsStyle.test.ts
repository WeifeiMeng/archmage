import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const teamsCss = readFileSync(new URL("./teams.css", import.meta.url), "utf8");

describe("Team selector summary card", () => {
  it("uses one card boundary and moves focus styling to the card", () => {
    expect(teamsCss).toMatch(/\.team-hero-selector\s*\{[^}]*position:\s*relative;[^}]*padding:\s*11px 12px;/s);
    expect(teamsCss).toMatch(/\.team-hero-selector select\s*\{[^}]*appearance:\s*none;[^}]*border:\s*0;[^}]*background:\s*transparent;/s);
    expect(teamsCss).toMatch(/\.team-hero-selector:focus-within\s*\{[^}]*outline:\s*2px solid var\(--accent\);/s);
    expect(teamsCss).toMatch(/\.teams-view \.team-hero-selector select:focus-visible\s*\{[^}]*outline:\s*none;/s);
  });

  it("stacks Team settings above the message-flow canvas", () => {
    expect(teamsCss).toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(0, 1fr\);/s);
    expect(teamsCss).not.toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(350px,/s);
  });
});
