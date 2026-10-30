import { useMemo, useState } from "react";
import { ChevronDown, LoaderCircle } from "lucide-react";
import type { CapabilityKey, FlowDelivery } from "./model";
import {
  deliveryDirection,
  deliveryFieldGroups,
  deliveryFieldLabel,
  deliveryHasContent,
  deliverySkipReason,
  deliveryStateLabel,
  deliverySummary,
  fallbackLaneLabel,
  flowSwimlaneRows,
  nextExpandedDelivery,
  type FlowLaneMember,
  type FlowSwimlaneRow,
} from "./flowDelivery";
import "./flowSwimlane.css";

export type FlowDeliverySwimlaneProps = {
  members: FlowLaneMember[];
  deliveries: FlowDelivery[];
  stepLimit?: number;
};

type SwimlaneViewProps = FlowDeliverySwimlaneProps & {
  expandedId: string | null;
  onToggle: (deliveryId: string) => void;
};

/** Delivery swimlanes for a frozen Run team; expands one delivery at a time. */
export function FlowDeliverySwimlane({ members, deliveries, stepLimit = 24 }: FlowDeliverySwimlaneProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  return (
    <FlowDeliverySwimlaneView
      members={members}
      deliveries={deliveries}
      stepLimit={stepLimit}
      expandedId={expandedId}
      onToggle={(deliveryId) => setExpandedId((current) => nextExpandedDelivery(current, deliveryId))}
    />
  );
}

/** Controlled presentation, exported so expansion states stay testable. */
export function FlowDeliverySwimlaneView({ members, deliveries, stepLimit = 24, expandedId, onToggle }: SwimlaneViewProps) {
  const rows = useMemo(() => flowSwimlaneRows(deliveries), [deliveries]);
  const columns: FlowLaneMember[] = members.length > 0
    ? members
    : Array.from(new Set(deliveries.map((delivery) => delivery.target))).map((key) => ({ key, label: fallbackLaneLabel(key) }));
  const label = (key: CapabilityKey) => columns.find((member) => member.key === key)?.label ?? fallbackLaneLabel(key);
  return <section className="panel flow-panel" aria-label="交付消息">
    <div className="panel-title"><div><span>交付消息</span><small>每条消息独立触发一次目标能力；最多执行 {stepLimit} 次</small></div><span className="sequence">{deliveries.length} 条</span></div>
    {rows.length === 0
      ? <p className="flow-empty">尚未产生交付消息。</p>
      : <div className="flow-swimlane-scroll">
        <table className="flow-swimlane">
          <caption className="flow-visually-hidden">团队交付泳道：每行一条交付，列为团队能力</caption>
          <thead><tr><th scope="col" className="flow-lane-index-head">序号</th>{columns.map((member) => <th scope="col" key={member.key}>{member.label}</th>)}</tr></thead>
          <tbody>
            {rows.map((row) => <tr key={row.delivery.id} data-state={row.delivery.state}>
              <th scope="row" className="flow-lane-index">#{row.delivery.sequence}</th>
              {columns.map((member) => member.key === row.cardLane
                ? <td className="flow-delivery-cell" key={member.key}>
                  <DeliveryCard row={row} label={label} expanded={expandedId === row.delivery.id} onToggle={() => onToggle(row.delivery.id)} />
                </td>
                : <td className="flow-lane-status" key={member.key}>{row.laneLabels[member.key] ?? ""}</td>)}
            </tr>)}
          </tbody>
        </table>
      </div>}
  </section>;
}

function DeliveryCard({ row, label, expanded, onToggle }: { row: FlowSwimlaneRow; label: (key: CapabilityKey) => string; expanded: boolean; onToggle: () => void }) {
  const { delivery } = row;
  const state = delivery.state;
  const direction = deliveryDirection(delivery, label);
  const stateLabel = deliveryStateLabel(state);
  const summary = deliverySummary(delivery.output);
  const skipReason = deliverySkipReason(state);
  const expandable = deliveryHasContent(delivery);
  const detailId = `flow-delivery-detail-${delivery.id}`;
  const collapsedSummary = state === "queued" || state === "running" ? null : summary;
  const header = <>
    <span className="flow-delivery-seq">#{delivery.sequence}</span>
    <span className="flow-delivery-direction">{direction}</span>
    <span className="flow-delivery-state">
      {state === "running" && <LoaderCircle className="spin flow-delivery-spinner" size={11} aria-hidden="true" />}
      {stateLabel}
    </span>
    {expandable && <ChevronDown className={`flow-delivery-chevron ${expanded ? "open" : ""}`} size={13} aria-hidden="true" />}
    {skipReason && <span className="flow-delivery-reason">{skipReason}</span>}
    {collapsedSummary && <span className="flow-delivery-summary">{collapsedSummary}</span>}
  </>;
  return <div className="flow-delivery-card" data-state={state}>
    {expandable
      ? <button
        type="button"
        className="flow-delivery-toggle"
        aria-expanded={expanded}
        aria-controls={expanded ? detailId : undefined}
        aria-label={`交付 #${delivery.sequence} ${direction} ${stateLabel}，${expanded ? "收起" : "展开"}详情`}
        onClick={onToggle}
      >{header}</button>
      : <div className="flow-delivery-static">{header}</div>}
    {expandable && expanded && <DeliveryDetail delivery={delivery} detailId={detailId} direction={direction} />}
  </div>;
}

function DeliveryDetail({ delivery, detailId, direction }: { delivery: FlowDelivery; detailId: string; direction: string }) {
  const groups = deliveryFieldGroups(delivery.output);
  return <div className="flow-delivery-detail" id={detailId} role="group" aria-label={`交付 #${delivery.sequence} ${direction} 详情`}>
    {groups.length === 0 && <p className="flow-detail-empty">该交付没有可读字段，可展开原始 JSON 查看。</p>}
    {groups.map((group) => <section className="flow-detail-group" key={group.key}>
      <h4>{group.title}</h4>
      <dl>
        {group.fields.map((field) => <div key={field.key}>
          <dt>{deliveryFieldLabel(field.key)}</dt>
          <dd><FieldValue value={field.value} /></dd>
        </div>)}
      </dl>
    </section>)}
    <details className="flow-detail-raw">
      <summary>查看原始 JSON</summary>
      <pre>{JSON.stringify(delivery.output, null, 2)}</pre>
    </details>
  </div>;
}

function FieldValue({ value }: { value: unknown }) {
  if (Array.isArray(value)) {
    return <ul>{value.map((item, index) => <li key={index}>{inlineText(item)}</li>)}</ul>;
  }
  if (value && typeof value === "object") {
    return <dl className="flow-detail-nested">
      {Object.entries(value as Record<string, unknown>).map(([key, nested]) => <div key={key}>
        <dt>{deliveryFieldLabel(key)}</dt>
        <dd>{inlineText(nested)}</dd>
      </div>)}
    </dl>;
  }
  return <p>{inlineText(value)}</p>;
}

function inlineText(value: unknown): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) return value.map(inlineText).join("、");
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .filter(([, nested]) => nested !== null && nested !== undefined && typeof nested !== "object")
      .map(([key, nested]) => `${deliveryFieldLabel(key)}：${String(nested)}`);
    return entries.length > 0 ? entries.join(" · ") : JSON.stringify(value);
  }
  return String(value);
}
