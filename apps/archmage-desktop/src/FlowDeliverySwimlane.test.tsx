import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FlowDeliverySwimlaneView } from "./FlowDeliverySwimlane";
import { nextExpandedDelivery, type FlowLaneMember } from "./flowDelivery";
import type { CapabilityKey, FlowDelivery } from "./model";

const members: FlowLaneMember[] = [
  { key: "coder", label: "Coder" },
  { key: "reviewer", label: "Reviewer" },
];

function delivery(overrides: Partial<FlowDelivery> & Pick<FlowDelivery, "sequence" | "target">): FlowDelivery {
  return {
    id: `d${overrides.sequence}`,
    runId: "run-1",
    sourceAttemptId: null,
    payloadArtifactId: "artifact",
    state: "completed",
    attemptId: `a${overrides.sequence}`,
    routing: null,
    output: null,
    ...overrides,
  };
}

const loop: FlowDelivery[] = [
  delivery({
    sequence: 1, target: "coder", sourceAttemptId: null,
    routing: { decision: "continue", next: ["reviewer"] },
    output: { status: "implemented", summary_markdown: "# 实现完成\n\n补充了导出能力。", changed_paths: ["src/a.rs"], notes: ["已运行测试"] },
  }),
  delivery({
    sequence: 2, target: "reviewer", sourceAttemptId: "a1",
    routing: { decision: "continue", next: ["coder"] },
    output: { status: "changes_requested", issues: ["缺少回归测试"], suggestions: ["补一个 e2e"] },
  }),
  delivery({
    sequence: 3, target: "coder", sourceAttemptId: "a2",
    routing: { decision: "continue", next: ["reviewer"] },
    output: { status: "fixed", notes: ["补上了回归测试"] },
  }),
  delivery({
    sequence: 4, target: "reviewer", sourceAttemptId: "a3",
    routing: { decision: "complete", next: [] },
    output: { status: "approved", summary_markdown: "实现已验证通过" },
  }),
];

function render(deliveries: FlowDelivery[], expandedId: string | null = null, lanes = members) {
  return renderToStaticMarkup(
    <FlowDeliverySwimlaneView
      members={lanes}
      deliveries={deliveries}
      stepLimit={24}
      expandedId={expandedId}
      onToggle={() => undefined}
    />,
  );
}

function cells(html: string) {
  return html.match(/<td class="flow-delivery-cell">[\s\S]*?<\/td>/g) ?? [];
}

describe("delivery swimlanes", () => {
  it("orders the lanes by the frozen team members", () => {
    const html = render(loop, null, [
      { key: "reviewer", label: "Reviewer" },
      { key: "coder", label: "Coder" },
      { key: "tester", label: "Tester" },
    ]);
    const headers = Array.from(html.matchAll(/<th scope="col"[^>]*>([^<]*)<\/th>/g)).map((match) => match[1]);
    expect(headers).toEqual(["序号", "Reviewer", "Coder", "Tester"]);
  });

  it("renders every direction of the Coder/Reviewer loop", () => {
    const html = render(loop);
    expect(html).toContain("Coder → Reviewer");
    expect(html).toContain("Reviewer → Coder");
    expect(html).toContain("Reviewer → 完成");
    expect(html).toContain("#1");
    expect(html).toContain("#4");
  });

  it("shows all recipients of one delivery", () => {
    const html = render([
      delivery({ sequence: 1, target: "coder", routing: { decision: "continue", next: ["reviewer"] }, output: { status: "ok" } }),
    ], null, [{ key: "coder", label: "Coder" }, { key: "reviewer", label: "Reviewer" }, { key: "tester", label: "Tester" }]);
    expect(html).toContain("Coder → Reviewer");
    expect(html).toContain("收到交付");
  });

  it("keeps the expanded detail inside the same lane cell as its card", () => {
    const html = render(loop, "d2");
    const expanded = cells(html).find((cell) => cell.includes("flow-delivery-detail-d2"));
    expect(expanded).toBeDefined();
    expect(expanded).toContain("缺少回归测试");
    expect(expanded).toContain("查看原始 JSON");
    // Expanding one delivery leaves the others collapsed and hides their detail.
    expect(html.match(/class="flow-delivery-detail"/g) ?? []).toHaveLength(1);
    expect(html.match(/class="flow-detail-raw"/g) ?? []).toHaveLength(1);
    expect(html).not.toContain("flow-delivery-detail-d1");
    expect(html.match(/aria-expanded="true"/g)).toHaveLength(1);
    expect(html.match(/aria-expanded="false"/g)).toHaveLength(3);
    expect(expanded).not.toContain("flow-delivery-detail-d1");
  });

  it("gives every expandable card an accessible name", () => {
    const html = render(loop, "d1");
    expect(html).toContain('aria-label="交付 #1 Coder → Reviewer 已交付，收起详情"');
    expect(html).toContain('aria-label="交付 #2 Reviewer → Coder 已交付，展开详情"');
  });

  it("shows friendly fields before the collapsed raw JSON", () => {
    const html = render(loop, "d1");
    expect(html).toContain("总体结论");
    expect(html).toContain("实现与验证");
    expect(html).toContain("变更文件");
    expect(html).toContain("src/a.rs");
    expect(html).toMatch(/<details class="flow-detail-raw">\s*<summary>查看原始 JSON<\/summary>/);
    expect(html).toContain("&quot;status&quot;: &quot;implemented&quot;");
  });

  it("describes queued, running, failed, skipped and empty deliveries with text", () => {
    const html = render([
      delivery({ sequence: 1, target: "coder", state: "queued", attemptId: null }),
      delivery({ sequence: 2, target: "reviewer", state: "running" }),
      delivery({ sequence: 3, target: "coder", state: "failed", output: null }),
      delivery({ sequence: 4, target: "reviewer", state: "skipped_due_terminal", attemptId: null }),
      delivery({ sequence: 5, target: "coder", output: {} }),
    ]);
    expect(html).toContain("等待执行");
    expect(html).toContain("执行中");
    expect(html).toContain("失败");
    expect(html).toContain("已跳过");
    expect(html).toContain("流程已完成，本步无需执行");
    expect(html).toContain("待执行 · Coder");
    expect(html).toContain("执行中 · Reviewer");
    expect(html).toContain("流程已完成，本步无需执行");
    // No output means no expand affordance at all.
    expect(html.match(/flow-delivery-toggle/g) ?? []).toHaveLength(1);
    expect(html).not.toContain("flow-delivery-detail");
    expect(html).not.toContain("查看原始 JSON");
  });

  it("explains an empty structured deliverable and shows its raw JSON", () => {
    const html = render([delivery({ sequence: 5, target: "coder", output: {} })], "d5");
    expect(html).toContain("该交付没有可读字段");
    expect(html).toContain("查看原始 JSON");
    expect(html).toContain("已产生结构化交付，展开查看详情");
  });

  it("falls back to vertical cards when no team member list is available", () => {
    const html = render(loop.slice(0, 2), null, []);
    expect(html).toContain("Coder");
    expect(html).toContain("Reviewer");
  });

  it("renders a compact empty state and drives single-expansion toggling", () => {
    expect(render([])).toContain("尚未产生交付消息。");
    expect(nextExpandedDelivery(null, "d1")).toBe("d1");
    expect(nextExpandedDelivery("d1", "d1")).toBeNull();
    expect(nextExpandedDelivery("d1", "d2")).toBe("d2");
  });

  it("keeps capability keys type-safe for lanes", () => {
    const keys: CapabilityKey[] = members.map((member) => member.key);
    expect(keys).toEqual(["coder", "reviewer"]);
  });
});
