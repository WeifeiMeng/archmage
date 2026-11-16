export const DEFAULT_INSPECTOR_RATIO = 43;
export const MIN_PANEL_PX = 320;
export const SPLITTER_PX = 12;
export const MIN_PREFERENCE_RATIO = 20;
export const MAX_PREFERENCE_RATIO = 80;

export function parseStoredInspectorRatio(value: string | null): number {
  if (value === null) return DEFAULT_INSPECTOR_RATIO;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < MIN_PREFERENCE_RATIO || parsed > MAX_PREFERENCE_RATIO) return DEFAULT_INSPECTOR_RATIO;
  return parsed;
}

export function clampInspectorRatio(ratio: number, width: number, minimum = MIN_PANEL_PX, splitter = SPLITTER_PX): number {
  if (width < minimum * 2 + splitter) return ratio;
  const minimumRatio = minimum / width * 100;
  const maximumRatio = (width - splitter - minimum) / width * 100;
  return Math.max(minimumRatio, Math.min(maximumRatio, ratio));
}

export function clampActiveInspectorRatio(ratio: number, width: number): number {
  const preferred = Math.max(MIN_PREFERENCE_RATIO, Math.min(MAX_PREFERENCE_RATIO, ratio));
  return width > 0 ? clampInspectorRatio(preferred, width) : preferred;
}

export function adjustInspectorRatio(ratio: number, key: string): number | null {
  if (key === "ArrowLeft") return Math.max(MIN_PREFERENCE_RATIO, ratio - 2);
  if (key === "ArrowRight") return Math.min(MAX_PREFERENCE_RATIO, ratio + 2);
  if (key === "Home") return MIN_PREFERENCE_RATIO;
  if (key === "End") return MAX_PREFERENCE_RATIO;
  return null;
}
