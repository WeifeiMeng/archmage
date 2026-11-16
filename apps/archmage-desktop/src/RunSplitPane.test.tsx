import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunSplitPane } from "./RunSplitPane";

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
});
