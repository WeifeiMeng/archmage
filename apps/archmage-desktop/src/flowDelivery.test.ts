import { describe, expect, it } from "vitest";
import {
  deliveryDirection,
  deliveryFieldGroups,
  deliveryHasContent,
  deliverySkipReason,
  deliveryStateLabel,
  deliverySummary,
  flowLaneMembers,
  flowSwimlaneRows,
  type FlowLaneMember,
} from "./flowDelivery";
import { capabilityDisplayName, type CapabilityConfiguration, type FlowDelivery, type Team } from "./model";

const names: Record<string, string> = { coder: "Coder", reviewer: "Reviewer", designer: "Designer", tester: "Tester" };
const label = (key: string) => names[key] ?? key;

function delivery(overrides: Partial<FlowDelivery> & Pick<FlowDelivery, "sequence" | "target">): FlowDelivery {
  return {
    id: `d${overrides.sequence}`,
    runId: "run-1",
    sourceAttemptId: null,
    payloadArtifactId: null,
    state: "completed",
    attemptId: `a${overrides.sequence}`,
    routing: null,
    output: null,
    ...overrides,
  };
}

describe("delivery direction", () => {
  it("reads the sending capability from the executed target and the receivers from routing", () => {
    const implement = delivery({ sequence: 1, target: "coder", routing: { decision: "continue", next: ["reviewer"] } });
    const feedback = delivery({ sequence: 2, target: "reviewer", routing: { decision: "continue", next: ["coder"] } });
    const approved = delivery({ sequence: 3, target: "reviewer", routing: { decision: "complete", next: [] } });
    expect(deliveryDirection(implement, label)).toBe("Coder → Reviewer");
    expect(deliveryDirection(feedback, label)).toBe("Reviewer → Coder");
    expect(deliveryDirection(approved, label)).toBe("Reviewer → 完成");
  });

  it("keeps every recipient of a multi-receiver delivery", () => {
    const fanOut = delivery({ sequence: 1, target: "designer", routing: { decision: "continue", next: ["coder", "tester"] } });
    expect(deliveryDirection(fanOut, label)).toBe("Designer → Coder、Tester");
    expect(flowSwimlaneRows([fanOut])[0].laneLabels).toEqual({ coder: "收到交付", tester: "收到交付" });
  });

  it("describes a delivery that has not routed yet with its queue state", () => {
    expect(deliveryDirection(delivery({ sequence: 1, target: "reviewer", state: "running" }), label)).toBe("执行中 · Reviewer");
    expect(deliveryDirection(delivery({ sequence: 1, target: "coder", state: "failed" }), label)).toBe("Coder");
    expect(deliveryDirection(delivery({ sequence: 1, target: "coder", state: "skipped_due_terminal" }), label)).toBe("Coder");
  });

  it("never invents a sender for a record that has not routed yet", () => {
    const queued = delivery({ sequence: 1, target: "coder", state: "queued", attemptId: null });
    expect(deliveryDirection(queued, label)).toBe("待执行 · Coder");
    expect(flowSwimlaneRows([queued])[0]).toMatchObject({ cardLane: "coder", laneLabels: {} });
  });
});

describe("swimlane rows", () => {
  it("marks feedback loops and still-pending receivers per lane", () => {
    const rows = flowSwimlaneRows([
      delivery({ sequence: 1, target: "coder", routing: { decision: "continue", next: ["reviewer"] } }),
      delivery({ sequence: 2, target: "reviewer", routing: { decision: "continue", next: ["coder"] } }),
      delivery({ sequence: 3, target: "coder", state: "queued", attemptId: null }),
    ]);
    expect(rows.map((row) => row.cardLane)).toEqual(["coder", "reviewer", "coder"]);
    expect(rows[0].laneLabels).toEqual({ reviewer: "收到交付" });
    expect(rows[1].laneLabels).toEqual({ coder: "收到反馈" });
    expect(rows[2].laneLabels).toEqual({});
  });

  it("shows a waiting receiver while a follow-up delivery is still queued", () => {
    const rows = flowSwimlaneRows([
      delivery({ sequence: 1, target: "designer", attemptId: "a1", routing: { decision: "continue", next: ["coder", "tester"] } }),
      delivery({ sequence: 2, target: "coder", state: "queued", attemptId: null, sourceAttemptId: "a1" }),
      delivery({ sequence: 3, target: "tester", state: "queued", attemptId: null, sourceAttemptId: "a1" }),
    ]);
    expect(rows[0].laneLabels).toEqual({ coder: "等待接收", tester: "等待接收" });
  });

  it("uses frozen team members in their configured order as lanes", () => {
    const team: Team = {
      id: "t1", name: "团队", revision: 1,
      graph: { mode: "flow", entry: "reviewer", terminal: "coder", nodes: [{ capability: "reviewer", x: 0, y: 0 }, { capability: "coder", x: 1, y: 0 }], edges: [] },
    };
    const configurations = { coder: { displayName: "实现者" } as CapabilityConfiguration } as Record<"coder" | "designer" | "reviewer" | "tester", CapabilityConfiguration>;
    const members: FlowLaneMember[] = flowLaneMembers(team, configurations);
    expect(members).toEqual([{ key: "reviewer", label: "Reviewer" }, { key: "coder", label: "实现者" }]);
    expect(flowLaneMembers(null, configurations)).toEqual([]);
    expect(capabilityDisplayName(undefined, "tester")).toBe("Tester");
  });
});

describe("delivery summary", () => {
  it("follows the fixed priority order", () => {
    expect(deliverySummary({ summary_markdown: "## 通过\n\n实现已验证。\n\n后续说明", issues: ["忽略"] })).toBe("通过 · 实现已验证。");
    expect(deliverySummary({ summary_markdown: "实现已完成，等待审阅。", issues: ["忽略"] })).toBe("实现已完成，等待审阅。");
    expect(deliverySummary({ issues: ["缺少测试", "未覆盖回滚"], notes: ["忽略"] })).toBe("2 个问题 · 首个：缺少测试");
    expect(deliverySummary({ notes: ["已补充单元测试"], changed_paths: ["a", "b"] })).toBe("已补充单元测试");
    expect(deliverySummary({ changed_paths: ["src/a.rs", "src/b.rs"] })).toBe("修改了 2 个文件");
    expect(deliverySummary({ status: "approved" })).toBe("已产生结构化交付，展开查看详情");
    expect(deliverySummary(null)).toBeNull();
  });

  it("never treats empty collections as readable content", () => {
    expect(deliverySummary({ issues: [], notes: [], changed_paths: [] })).toBe("已产生结构化交付，展开查看详情");
  });
});

describe("friendly fields", () => {
  it("groups conclusion, feedback and implementation before other keys", () => {
    const groups = deliveryFieldGroups({
      status: "changes_requested",
      issues: ["缺少测试"],
      notes: ["已阅读需求"],
      changed_paths: ["src/a.rs"],
      reviewer_score: 3,
      zz_extra: true,
    });
    expect(groups.map((group) => group.key)).toEqual(["conclusion", "feedback", "implementation", "other"]);
    expect(groups[3].fields.map((field) => field.key)).toEqual(["reviewer_score", "zz_extra"]);
    expect(deliveryFieldGroups(null)).toEqual([]);
  });
});

describe("delivery states", () => {
  it("labels every terminal and skip state with text", () => {
    expect(deliveryStateLabel("queued")).toBe("等待执行");
    expect(deliveryStateLabel("running")).toBe("执行中");
    expect(deliveryStateLabel("completed")).toBe("已交付");
    expect(deliveryStateLabel("failed")).toBe("失败");
    expect(deliveryStateLabel("cancelled")).toBe("已取消");
    expect(deliveryStateLabel("interrupted")).toBe("已中断");
    expect(deliveryStateLabel("skipped_due_terminal")).toBe("已跳过");
    expect(deliverySkipReason("skipped_due_failure")).toBe("上游失败，本步未执行");
    expect(deliverySkipReason("skipped_due_cancel")).toBe("运行被取消，本步未执行");
    expect(deliverySkipReason("skipped_due_interruption")).toBe("运行中断，本步未执行");
    expect(deliverySkipReason("completed")).toBeNull();
  });

  it("only offers detail when output was actually persisted", () => {
    expect(deliveryHasContent(delivery({ sequence: 1, target: "coder", output: { status: "ok" } }))).toBe(true);
    expect(deliveryHasContent(delivery({ sequence: 1, target: "coder", output: {} }))).toBe(true);
    expect(deliveryHasContent(delivery({ sequence: 1, target: "coder", output: null }))).toBe(false);
    expect(deliveryHasContent(delivery({ sequence: 1, target: "coder", state: "failed" }))).toBe(false);
  });
});
