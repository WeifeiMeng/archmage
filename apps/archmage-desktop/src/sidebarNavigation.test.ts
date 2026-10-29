import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

describe("sidebar brand and navigation", () => {
  it("uses the approved Chinese labels and a distinct team icon", () => {
    expect(appSource).toContain("智能体工作台");
    expect(appSource).toContain("新建任务");
    expect(appSource).toContain("智能体管理");
    expect(appSource).toContain("能力管理");
    expect(appSource).toContain("<UsersRound size={17} />团队管理");
  });

  it("uses the approved balanced brand size", () => {
    expect(styles).toContain(".brand-mark{width:44px;height:44px");
    expect(styles).toContain(".brand strong{font-size:18px}");
    expect(styles).toContain(".brand span{font-size:12px");
  });
});
