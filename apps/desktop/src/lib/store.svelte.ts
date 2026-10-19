import { api } from "./api";
import type { AppState, Feature, FeatureOutcome } from "./types";

export type Page = "displays" | "settings" | "updates" | "about";

export interface Toast {
  id: number;
  text: string;
  tone: "info" | "success" | "warn" | "error";
  undo?: () => void;
}

export interface Dialog {
  title: string;
  body: string;
  confirm: string;
  cancel?: string;
  tone?: "accent" | "danger";
  resolve: (ok: boolean) => void;
}

export const ui = $state({
  state: null as AppState | null,
  page: "displays" as Page,
  busy: false,
  toasts: [] as Toast[],
  dialog: null as Dialog | null,
  updateBadge: false,
});

let toastId = 0;
export function toast(text: string, tone: Toast["tone"] = "info", undo?: () => void) {
  const t = { id: ++toastId, text, tone, undo };
  ui.toasts = [...ui.toasts.slice(-2), t];
  setTimeout(() => dismiss(t.id), undo ? 5000 : 3200);
}
export function dismiss(id: number) {
  ui.toasts = ui.toasts.filter((t) => t.id !== id);
}

export function ask(d: Omit<Dialog, "resolve">): Promise<boolean> {
  return new Promise((resolve) => {
    ui.dialog = {
      ...d,
      resolve: (ok) => {
        ui.dialog = null;
        resolve(ok);
      },
    };
  });
}

export async function init() {
  const b = await api();
  b.onState((s) => (ui.state = s));
  b.onNavigate((p) => (ui.page = p as Page));
  ui.state = await b.getState();
}

export async function refresh() {
  ui.state = await (await api()).getState();
}

/** Run a backend call; errors become a friendly toast instead of a dead end. */
async function guarded<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn();
  } catch (e) {
    toast(String(e), "error");
    await refresh().catch(() => {});
    return undefined;
  }
}

/**
 * Change a control. `before` is the value at the start of the gesture (a whole
 * slider drag counts as one change), so "Undo" puts back exactly that.
 */
export async function setControl(id: string, value: number, opts: { quiet?: boolean; before?: number | null } = {}) {
  const out = await guarded(async () => (await api()).setControl(id, value));
  if (!out) return;
  ui.state = out.state;
  const before = opts.before;
  if (opts.quiet || before == null || before === value) return;
  const label = out.state.controls.find((c) => c.id === id)?.label ?? "Setting";
  toast(`${label} updated`, "success", async () => {
    await setControl(id, before, { quiet: true });
    toast("Undone", "info");
  });
}

export async function undo() {
  const out = await guarded(async () => (await api()).undo());
  if (out) {
    ui.state = out.state;
    toast("Undone", "info");
  }
}

const featureTitles: Record<Feature, string> = {
  "ai-scene-optimization": "Turn on AI Scene Optimization?",
  "ai-picture-mode": "Turn on AI Picture Mode?",
  "my-display-preset": "Turn on My Display Preset?",
};

/** Toggle a smart feature through the interlock rules, asking when they say so. */
export async function toggleFeature(feature: Feature, on: boolean) {
  const b = await api();
  let out: FeatureOutcome | undefined = await guarded(() => b.toggleFeature(feature, on, false));
  if (out?.kind === "confirm") {
    const ok = await ask({ title: featureTitles[feature], body: out.question, confirm: "Turn on", cancel: "Not now" });
    if (!ok) return;
    out = await guarded(() => b.toggleFeature(feature, on, true));
  }
  handleFeature(out);
}

export async function simulatePbp(on: boolean) {
  handleFeature(await guarded(async () => (await api()).simulatePbp(on)));
}

function handleFeature(out: FeatureOutcome | undefined) {
  if (!out) return;
  if (out.kind === "applied") {
    ui.state = out.state;
    if (out.notice) toast(out.notice, "info");
  } else if (out.kind === "blocked") {
    ui.state = out.state;
    toast(out.reason, "warn");
  }
}

export async function resetOriginal() {
  const ok = await ask({
    title: "Reset to your original settings?",
    body: "nubless puts back the settings your monitor had before nubless changed anything.",
    confirm: "Reset",
    cancel: "Keep current",
  });
  if (!ok) return;
  ui.busy = true;
  const out = await guarded(async () => (await api()).resetOriginal());
  ui.busy = false;
  if (out) {
    ui.state = out.state;
    toast(out.toast ?? "Done", "success");
  }
}

export async function setLabs(on: boolean) {
  const s = await guarded(async () => (await api()).setLabs(on));
  if (s) ui.state = s;
}

export async function rescan() {
  ui.busy = true;
  const s = await guarded(async () => (await api()).rescan());
  ui.busy = false;
  if (s) ui.state = s;
}

export async function useDemo() {
  const s = await guarded(async () => (await api()).useDemo());
  if (s) {
    ui.state = s;
    toast("Demo monitor connected. Nothing is sent to real hardware.", "info");
  }
}

export function greeting(): string {
  const h = new Date().getHours();
  return h < 5 ? "Up late?" : h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
}
