import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunTeamPreview } from "./RunTeamPreview";
import type { Team } from "./model";

describe("new Run Team preview", () => {
  it("shows only selected Team members and its message routes", () => {
    const team: Team = { id: "t1", name: "审阅团队", revision: 1, graph: {
      mode: "flow", entry: "coder", terminal: "reviewer",
      nodes: [{ capability: "coder", x: 0, y: 0 }, { capability: "reviewer", x: 300, y: 0 }],
      edges: [{ from: "coder", to: "reviewer" }, { from: "reviewer", to: "coder" }],
    } };
    const html = renderToStaticMarkup(<RunTeamPreview team={team} />);
    expect(html).toContain("团队组成");
    expect(html).toContain("Coder");
    expect(html).toContain("Reviewer");
    expect(html).toContain("Coder → Reviewer");
    expect(html).toContain("Reviewer → Coder");
    expect(html).toContain("入口 Coder · 可完成 Reviewer");
    expect(html).not.toContain("Tester");
    expect(html).not.toContain("Designer");
  });
});
