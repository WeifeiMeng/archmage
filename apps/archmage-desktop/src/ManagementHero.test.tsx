import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ManagementHero, PageMetric } from "./ManagementHero";

describe("management page hero", () => {
  it("renders page identity, actions, and textual status", () => {
    const html = renderToStaticMarkup(
      <ManagementHero
        eyebrow="TEAM RUN"
        title="把目标交给团队"
        description="定义任务并确认执行环境。"
        actions={<button>刷新</button>}
        summary={<PageMetric label="本地 Runtime" value="已就绪" detail="配置可用" tone="ready" />}
      />,
    );

    expect(html).toContain("management-hero");
    expect(html).toContain("management-hero-actions");
    expect(html).toContain("page-metric ready");
    expect(html).toContain("TEAM RUN");
    expect(html).toContain("把目标交给团队");
    expect(html).toContain("定义任务并确认执行环境。");
    expect(html).toContain("刷新");
    expect(html).toContain("本地 Runtime");
    expect(html).toContain("已就绪");
    expect(html).toContain("配置可用");
  });
});
