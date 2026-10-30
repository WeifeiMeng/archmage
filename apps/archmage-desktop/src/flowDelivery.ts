import {
  capabilityDisplayName,
  type CapabilityConfiguration,
  type CapabilityKey,
  type FlowDelivery,
  type Team,
} from "./model";

export type FlowLaneMember = { key: CapabilityKey; label: string };

export const FLOW_COMPLETE_LABEL = "完成";
export const FLOW_SUMMARY_FALLBACK = "已产生结构化交付，展开查看详情";

const STATE_LABELS: Record<string, string> = {
  queued: "等待执行",
  running: "执行中",
  completed: "已交付",
  failed: "失败",
  cancelled: "已取消",
  interrupted: "已中断",
};

const SKIP_REASONS: Record<string, string> = {
  skipped_due_terminal: "流程已完成，本步无需执行",
  skipped_due_failure: "上游失败，本步未执行",
  skipped_due_cancel: "运行被取消，本步未执行",
  skipped_due_interruption: "运行中断，本步未执行",
};

const FIELD_GROUPS: { key: string; title: string; keys: string[] }[] = [
  {
    key: "conclusion",
    title: "总体结论",
    keys: ["status", "summary_markdown", "summary", "verdict", "decision", "outcome"],
  },
  {
    key: "feedback",
    title: "审查与反馈",
    keys: ["issues", "suggestions", "requested_changes", "blocking_issues", "questions", "risks"],
  },
  {
    key: "implementation",
    title: "实现与验证",
    keys: ["changed_paths", "notes", "verification", "tests", "commands", "checks"],
  },
];

const FIELD_LABELS: Record<string, string> = {
  status: "状态",
  summary_markdown: "摘要",
  summary: "摘要",
  verdict: "结论",
  decision: "决策",
  outcome: "结果",
  issues: "问题",
  suggestions: "建议",
  requested_changes: "修改请求",
  blocking_issues: "阻塞问题",
  questions: "待确认问题",
  risks: "风险",
  changed_paths: "变更文件",
  notes: "说明",
  verification: "验证",
  tests: "测试",
  commands: "命令",
  checks: "检查项",
};

/** Frozen team members in their configured order, used as swimlane columns. */
export function flowLaneMembers(
  team: Team | null | undefined,
  configurations?: Record<CapabilityKey, CapabilityConfiguration>,
): FlowLaneMember[] {
  return (team?.graph.nodes ?? []).map((node) => ({
    key: node.capability,
    label: capabilityDisplayName(configurations?.[node.capability], node.capability),
  }));
}

export function deliveryStateLabel(state: string): string {
  if (STATE_LABELS[state]) return STATE_LABELS[state];
  if (isSkippedDelivery(state)) return "已跳过";
  return state;
}

export function deliverySkipReason(state: string): string | null {
  return SKIP_REASONS[state] ?? null;
}

export function isSkippedDelivery(state: string): boolean {
  return state.startsWith("skipped");
}

/**
 * The capability that produced this round is `delivery.target`: the delivery
 * records the attempt that was triggered for that capability. A delivery
 * without routing has not produced anything yet, so it has no sender.
 */
export function deliverySender(delivery: FlowDelivery): CapabilityKey | null {
  return delivery.routing ? delivery.target : null;
}

export function deliveryReceivers(delivery: FlowDelivery): CapabilityKey[] {
  if (!delivery.routing || delivery.routing.decision === "complete") return [];
  return delivery.routing.next;
}

export function deliveryIsComplete(delivery: FlowDelivery): boolean {
  return delivery.routing?.decision === "complete";
}

/** Column that carries the delivery card: always the capability being delivered to. */
export function deliveryCardLane(delivery: FlowDelivery): CapabilityKey {
  return delivery.target;
}

export function deliveryDirection(
  delivery: FlowDelivery,
  name: (key: CapabilityKey) => string,
): string {
  const sender = deliverySender(delivery);
  if (!sender) {
    const target = name(delivery.target);
    if (delivery.state === "running") return `执行中 · ${target}`;
    if (delivery.state === "queued") return `待执行 · ${target}`;
    return target;
  }
  if (deliveryIsComplete(delivery)) return `${name(sender)} → ${FLOW_COMPLETE_LABEL}`;
  const receivers = deliveryReceivers(delivery);
  if (receivers.length === 0) return name(sender);
  return `${name(sender)} → ${receivers.map(name).join("、")}`;
}

/** Whether this delivery has persisted structured content worth expanding. */
export function deliveryHasContent(delivery: FlowDelivery): boolean {
  const output = delivery.output;
  return output !== null && output !== undefined && typeof output === "object";
}

function cleanMarkdownBlock(block: string): string {
  return block
    .replace(/^[\s>#*`-]+/gm, "")
    .replace(/[*`]/g, "")
    .replace(/\s*\n\s*/g, " ")
    .trim();
}

/** First readable paragraph; a leading heading is paired with the next block. */
function firstParagraph(text: string): string {
  const blocks = text.replace(/\r\n?/g, "\n").split(/\n\s*\n/).map((block) => block.trim()).filter(Boolean);
  const head = blocks.length > 0 ? cleanMarkdownBlock(blocks[0]) : "";
  if (!head) return "";
  const rest = blocks.slice(1).map(cleanMarkdownBlock).find(Boolean);
  return blocks[0].startsWith("#") && rest ? `${head} · ${rest}` : head;
}

const NESTED_TEXT_KEYS = ["summary", "text", "detail", "description", "title", "message", "issue"];

function scalarText(value: unknown): string {
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value && typeof value === "object" && !Array.isArray(value)) {
    const record = value as Record<string, unknown>;
    for (const key of NESTED_TEXT_KEYS) {
      const nested = record[key];
      if (typeof nested === "string" && nested.trim()) return nested.trim();
    }
  }
  return "";
}

function textList(value: unknown): string[] {
  return Array.isArray(value) ? value.map(scalarText).filter(Boolean) : [];
}

/**
 * Display-only summary of the delivery output, fixed priority:
 * summary_markdown → issues → notes → changed_paths → generic fallback.
 */
export function deliverySummary(output?: Record<string, unknown> | null): string | null {
  if (!output || typeof output !== "object") return null;
  const markdown = output.summary_markdown;
  if (typeof markdown === "string") {
    const text = firstParagraph(markdown);
    if (text) return text;
  }
  const issues = textList(output.issues);
  if (issues.length > 0) return `${issues.length} 个问题 · 首个：${issues[0]}`;
  const notes = textList(output.notes);
  if (notes.length > 0) return notes[0];
  const changed = output.changed_paths;
  if (Array.isArray(changed) && changed.length > 0) return `修改了 ${changed.length} 个文件`;
  return FLOW_SUMMARY_FALLBACK;
}

export type FlowField = { key: string; value: unknown };
export type FlowFieldGroup = { key: string; title: string; fields: FlowField[] };

export function deliveryFieldLabel(key: string): string {
  return FIELD_LABELS[key] ?? key;
}

/** Friendly, stable ordering of the structured deliverable for the expanded detail. */
export function deliveryFieldGroups(output?: Record<string, unknown> | null): FlowFieldGroup[] {
  if (!output || typeof output !== "object") return [];
  const groups = FIELD_GROUPS.map((group) => ({
    key: group.key,
    title: group.title,
    fields: group.keys
      .filter((key) => output[key] !== undefined && output[key] !== null)
      .map((key) => ({ key, value: output[key] })),
  })).filter((group) => group.fields.length > 0);
  const used = new Set(groups.flatMap((group) => group.fields.map((field) => field.key)));
  const rest = Object.keys(output)
    .filter((key) => !used.has(key) && output[key] !== undefined)
    .sort();
  if (rest.length > 0) {
    groups.push({
      key: "other",
      title: "其他字段",
      fields: rest.map((key) => ({ key, value: output[key] })),
    });
  }
  return groups;
}

export type FlowSwimlaneRow = {
  delivery: FlowDelivery;
  cardLane: CapabilityKey;
  laneLabels: Partial<Record<CapabilityKey, string>>;
};

/**
 * Per-row lane status. Only the sending lane carries the card; receiving lanes
 * show a lightweight state instead of duplicating delivery content.
 */
export function flowSwimlaneRows(deliveries: FlowDelivery[]): FlowSwimlaneRow[] {
  const executed = new Set<CapabilityKey>();
  return deliveries.map((delivery) => {
    const laneLabels: Partial<Record<CapabilityKey, string>> = {};
    const pending = new Set(
      deliveries
        .filter(
          (item) =>
            item.state === "queued" &&
            item.sourceAttemptId !== null &&
            item.sourceAttemptId === delivery.attemptId,
        )
        .map((item) => item.target),
    );
    for (const receiver of deliveryReceivers(delivery)) {
      laneLabels[receiver] = pending.has(receiver)
        ? "等待接收"
        : executed.has(receiver)
          ? "收到反馈"
          : "收到交付";
    }
    const row: FlowSwimlaneRow = {
      delivery,
      cardLane: deliveryCardLane(delivery),
      laneLabels,
    };
    if (delivery.routing) executed.add(delivery.target);
    return row;
  });
}

/** Single-expansion toggle: opening one delivery closes the previously open one. */
export function nextExpandedDelivery(current: string | null, clicked: string): string | null {
  return current === clicked ? null : clicked;
}

export function fallbackLaneLabel(key: CapabilityKey): string {
  return capabilityDisplayName(undefined, key);
}
