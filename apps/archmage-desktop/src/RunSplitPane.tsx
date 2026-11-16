import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import {
  MAX_PREFERENCE_RATIO,
  MIN_PREFERENCE_RATIO,
  adjustInspectorRatio,
  clampActiveInspectorRatio,
  clampInspectorRatio,
  parseStoredInspectorRatio,
} from "./runSplitRatio";

const STORAGE_KEY = "archmage.run.inspectorRatio";

function storedRatio() {
  if (typeof window === "undefined") return parseStoredInspectorRatio(null);
  try { return parseStoredInspectorRatio(window.localStorage.getItem(STORAGE_KEY)); }
  catch { return parseStoredInspectorRatio(null); }
}

function persistRatio(ratio: number) {
  try { window.localStorage.setItem(STORAGE_KEY, String(ratio)); }
  catch { /* Local storage may be unavailable in hardened WebViews. */ }
}

export function RunSplitPane({ inspector, timeline, initialRatio }: {
  inspector: ReactNode;
  timeline: ReactNode;
  initialRatio?: number;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const preferredRef = useRef(initialRatio ?? storedRatio());
  const [preferred, setPreferred] = useState(preferredRef.current);
  const [effective, setEffective] = useState(preferredRef.current);
  const [dragging, setDragging] = useState(false);

  const applyRatio = (ratio: number, persist = false) => {
    const containerWidth = containerRef.current?.getBoundingClientRect().width ?? 0;
    const nextPreferred = clampActiveInspectorRatio(ratio, containerWidth);
    preferredRef.current = nextPreferred;
    setPreferred(nextPreferred);
    setEffective(containerWidth ? clampInspectorRatio(nextPreferred, containerWidth) : nextPreferred);
    if (persist && typeof window !== "undefined") persistRatio(nextPreferred);
  };

  useEffect(() => {
    const container = containerRef.current;
    if (!container || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      setEffective(clampInspectorRatio(preferredRef.current, entry.contentRect.width));
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const moveFromPointer = (clientX: number) => {
    const bounds = containerRef.current?.getBoundingClientRect();
    if (!bounds || bounds.width <= 0) return;
    applyRatio((clientX - bounds.left) / bounds.width * 100);
  };

  const style = { "--inspector-ratio": `${effective}%` } as CSSProperties;
  return <div className="run-split-pane" ref={containerRef} style={style}>
    <div className="run-split-inspector">{inspector}</div>
    <button
      type="button"
      className={`run-splitter ${dragging ? "dragging" : ""}`}
      role="separator"
      aria-label="调整节点检查器和运行时间线宽度"
      aria-orientation="vertical"
      aria-valuemin={MIN_PREFERENCE_RATIO}
      aria-valuemax={MAX_PREFERENCE_RATIO}
      aria-valuenow={Math.round(effective)}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        setDragging(true);
        moveFromPointer(event.clientX);
      }}
      onPointerMove={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) moveFromPointer(event.clientX);
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
        setDragging(false);
        persistRatio(preferredRef.current);
      }}
      onPointerCancel={() => {
        setDragging(false);
        persistRatio(preferredRef.current);
      }}
      onKeyDown={(event) => {
        const next = adjustInspectorRatio(preferred, event.key);
        if (next === null) return;
        event.preventDefault();
        applyRatio(next, true);
      }}
    ><span aria-hidden="true" /></button>
    <div className="run-split-timeline">{timeline}</div>
  </div>;
}
