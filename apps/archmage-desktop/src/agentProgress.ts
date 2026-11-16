import type { RunEvent } from "./model";

export type AgentProgressKind = "message" | "plan" | "tool" | "notice" | "usage" | "status";

export type AgentProgress = {
  sequence: number;
  kind: AgentProgressKind;
  label: string;
  status: string | null;
  text: string | null;
  metadata: Record<string, string | number | boolean>;
};

const kinds = new Set<AgentProgressKind>(["message", "plan", "tool", "notice", "usage", "status"]);

function progressFromEvent(event: RunEvent): AgentProgress | null {
  if (event.event_type !== "agent.progress") return null;
  const { kind, label, status, text, metadata } = event.payload;
  if (typeof kind !== "string" || !kinds.has(kind as AgentProgressKind) || typeof label !== "string") return null;
  if (status !== undefined && typeof status !== "string") return null;
  if (text !== undefined && typeof text !== "string") return null;
  const safeMetadata: Record<string, string | number | boolean> = {};
  if (metadata && typeof metadata === "object" && !Array.isArray(metadata)) {
    Object.entries(metadata).forEach(([key, value]) => {
      if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") safeMetadata[key] = value;
    });
  }
  return {
    sequence: event.sequence,
    kind: kind as AgentProgressKind,
    label,
    status: typeof status === "string" ? status : null,
    text: typeof text === "string" ? text : null,
    metadata: safeMetadata,
  };
}

export function progressForNode(events: RunEvent[], nodeId: string): AgentProgress[] {
  const latestAttempt = events
    .filter((event) => event.node_id === nodeId && event.attempt_id)
    .reduce<RunEvent | null>((latest, event) => !latest || event.sequence > latest.sequence ? event : latest, null)
    ?.attempt_id;
  if (!latestAttempt) return [];
  return events
    .filter((event) => event.node_id === nodeId && event.attempt_id === latestAttempt)
    .map(progressFromEvent)
    .filter((progress): progress is AgentProgress => progress !== null)
    .sort((left, right) => left.sequence - right.sequence);
}
