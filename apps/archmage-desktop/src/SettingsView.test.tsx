import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SettingsView } from "./SettingsView";

describe("Settings appearance view", () => {
  it("renders light and dark preview cards with the current theme selected", () => {
    const html = renderToStaticMarkup(<SettingsView theme="light" onThemeChange={() => undefined} />);

    expect(html).toContain("SETTINGS");
    expect(html).toContain("设置");
    expect(html).toContain("外观");
    expect(html).toContain("浅色");
    expect(html).toContain("暗色");
    expect(html.match(/class="theme-option(?: selected)?"/g)).toHaveLength(2);
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("已选择");
  });
});
