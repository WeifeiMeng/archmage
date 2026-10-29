import { describe, expect, it } from "vitest";
import { applyTheme, DEFAULT_THEME, persistTheme, readTheme, THEME_STORAGE_KEY } from "./theme";

describe("theme preference", () => {
  it("defaults to light and restores only supported values", () => {
    expect(DEFAULT_THEME).toBe("light");
    expect(readTheme({ getItem: () => null })).toBe("light");
    expect(readTheme({ getItem: () => "light" })).toBe("light");
    expect(readTheme({ getItem: () => "dark" })).toBe("dark");
    expect(readTheme({ getItem: () => "unknown" })).toBe("light");
  });

  it("falls back when preference storage is unavailable", () => {
    expect(readTheme({ getItem: () => { throw new Error("blocked"); } })).toBe("light");
    expect(() => persistTheme("dark", { setItem: () => { throw new Error("blocked"); } })).not.toThrow();
  });

  it("persists the selected theme and applies it to the document root", () => {
    const writes: Array<[string, string]> = [];
    persistTheme("dark", { setItem: (key, value) => { writes.push([key, value]); } });
    expect(writes).toEqual([[THEME_STORAGE_KEY, "dark"]]);

    const root = { dataset: {} as DOMStringMap, style: { colorScheme: "" } };
    applyTheme("dark", root);
    expect(root.dataset.theme).toBe("dark");
    expect(root.style.colorScheme).toBe("dark");
  });
});
