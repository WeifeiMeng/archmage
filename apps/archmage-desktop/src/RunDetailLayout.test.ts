import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("Run detail layout", () => {
  it("does not repeat the frozen Team topology below capability progress", () => {
    expect(appSource).not.toContain("冻结的团队拓扑");
    expect(appSource).not.toContain("snapshot.team.graph.edges.map");
    expect(appSource).toContain("<ConsultationProgress snapshot={snapshot} />");
  });

  it("delegates delivery rendering to the swimlane component", () => {
    expect(appSource).toContain("<FlowDeliverySwimlane");
    expect(appSource).not.toContain("flow-deliveries");
    expect(appSource).not.toContain("function FlowProgress");
  });
});
