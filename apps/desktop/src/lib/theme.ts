export type Theme = "light" | "dark";

export function normalizeTheme(t: string | undefined | null): Theme {
  return t === "light" ? "light" : "dark";
}

/** Applies theme tokens via `document.documentElement.dataset.theme`. */
export function applyTheme(theme: string): void {
  const t = normalizeTheme(theme);
  document.documentElement.dataset.theme = t;
  const meta = document.querySelector('meta[name="color-scheme"]');
  if (meta) {
    meta.setAttribute("content", t === "light" ? "light" : "dark");
  }
}
