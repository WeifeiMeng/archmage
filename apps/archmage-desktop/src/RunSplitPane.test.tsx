// @vitest-environment jsdom

import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunSplitPane } from "./RunSplitPane";

const storageKey = "archmage.run.inspectorRatio";

let resizeCallback: ResizeObserverCallback | null = null;
class ResizeObserverMock {
  constructor(callback: ResizeObserverCallback) { resizeCallback = callback; }
  observe() {}
  disconnect() {}
  unobserve() {}
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;
afterEach(() => {
  cleanup();
  resizeCallback = null;
});

function renderSplit(initialRatio?: number) {
  const view = render(<RunSplitPane
    initialRatio={initialRatio}
    inspector={<div data-panel="inspector">Inspector</div>}
    timeline={<div data-panel="timeline">Timeline</div>}
  />);
  const separator = view.getByRole("separator") as HTMLButtonElement;
  const container = separator.parentElement as HTMLDivElement;
  Object.defineProperty(container, "getBoundingClientRect", {
    value: () => ({ left: 0, width: 1000, right: 1000, top: 0, bottom: 500, height: 500, x: 0, y: 0, toJSON() {} }),
  });
  const captured = new Set<number>();
  separator.setPointerCapture = (id) => captured.add(id);
  separator.hasPointerCapture = (id) => captured.has(id);
  separator.releasePointerCapture = (id) => captured.delete(id);
  return { ...view, separator, container };
}

describe("RunSplitPane", () => {
  it("renders inspector first, separator second, and timeline last", () => {
    const html = renderToStaticMarkup(<RunSplitPane
      initialRatio={61}
      inspector={<div data-panel="inspector">Inspector</div>}
      timeline={<div data-panel="timeline">Timeline</div>}
    />);
    expect(html.indexOf('data-panel="inspector"')).toBeLessThan(html.indexOf('role="separator"'));
    expect(html.indexOf('role="separator"')).toBeLessThan(html.indexOf('data-panel="timeline"'));
    expect(html).toContain('aria-orientation="vertical"');
    expect(html).toContain('aria-valuenow="61"');
    expect(html).toContain('--inspector-ratio:61%');
  });

  it("drags, persists the visible ratio, and supports keyboard resizing", () => {
    localStorage.clear();
    const { separator, container } = renderSplit(43);
    fireEvent.pointerDown(separator, { pointerId: 7, clientX: 600 });
    fireEvent.pointerUp(separator, { pointerId: 7, clientX: 600 });
    expect(container.style.getPropertyValue("--inspector-ratio")).toBe("60%");
    expect(localStorage.getItem(storageKey)).toBe("60");

    fireEvent.keyDown(separator, { key: "ArrowRight" });
    expect(container.style.getPropertyValue("--inspector-ratio")).toBe("62%");
    expect(localStorage.getItem(storageKey)).toBe("62");
  });

  it("persists the last visible ratio when pointer interaction is cancelled", () => {
    localStorage.clear();
    const { separator, container } = renderSplit(43);
    fireEvent.pointerDown(separator, { pointerId: 8, clientX: 500 });
    fireEvent.pointerCancel(separator, { pointerId: 8 });
    expect(container.style.getPropertyValue("--inspector-ratio")).toBe("50%");
    expect(localStorage.getItem(storageKey)).toBe("50");
  });

  it("restores preference while passive resizing only changes the effective ratio", () => {
    localStorage.setItem(storageKey, "20");
    const { container } = renderSplit();
    expect(container.style.getPropertyValue("--inspector-ratio")).toBe("20%");
    act(() => resizeCallback?.([{ contentRect: { width: 800 } } as ResizeObserverEntry], {} as ResizeObserver));
    expect(container.style.getPropertyValue("--inspector-ratio")).toBe("40%");
    expect(localStorage.getItem(storageKey)).toBe("20");
  });
});
