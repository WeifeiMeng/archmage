export type Theme = "light" | "dark";

export const DEFAULT_THEME: Theme = "light";
export const THEME_STORAGE_KEY = "archmage.theme";

type ThemeStorageReader = Pick<Storage, "getItem">;
type ThemeStorageWriter = Pick<Storage, "setItem">;
type ThemeRoot = {
  dataset: DOMStringMap;
  style: { colorScheme: string };
};

export function readTheme(storage?: ThemeStorageReader): Theme {
  try {
    const value = (storage ?? window.localStorage).getItem(THEME_STORAGE_KEY);
    return value === "light" || value === "dark" ? value : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

export function persistTheme(theme: Theme, storage?: ThemeStorageWriter): void {
  try {
    (storage ?? window.localStorage).setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // The in-memory selection remains active when storage is unavailable.
  }
}

export function applyTheme(theme: Theme, root?: ThemeRoot): void {
  const target = root ?? document.documentElement;
  target.dataset.theme = theme;
  target.style.colorScheme = theme;
}

export function initializeTheme(): Theme {
  const theme = typeof window === "undefined" ? DEFAULT_THEME : readTheme();
  if (typeof document !== "undefined") applyTheme(theme);
  return theme;
}
