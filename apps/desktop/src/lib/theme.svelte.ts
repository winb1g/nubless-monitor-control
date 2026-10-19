// Theme preference is a per-device convenience, so localStorage is enough.
type Theme = "system" | "light" | "dark";

function load(): Theme {
  try {
    const t = localStorage.getItem("nubless.theme");
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}

export const theme = $state({ value: load() });

export function applyTheme() {
  const root = document.documentElement;
  if (theme.value === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme.value);
}

export function setTheme(t: Theme) {
  theme.value = t;
  try {
    localStorage.setItem("nubless.theme", t);
  } catch {
    /* private mode: keep it for this session */
  }
  applyTheme();
}
