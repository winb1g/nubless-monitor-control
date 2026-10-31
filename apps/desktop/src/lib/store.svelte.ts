import { untrack } from "svelte";
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
  tone?: "accent" | "danger" | "signal";
  resolve: (ok: boolean) => void;
}

/** A change that can hide the picture or resync the panel, waiting to be kept. */
export interface Keep {
  id: string;
  label: string;
  /** The new value as the monitor shows it ("Fast", "70"). */
  shown: string;
  /** What to put back if nobody answers. */
  before: number;
  /** Bumps when the same control is changed again, so the countdown restarts. */
  run: number;
}

/** The control whose change reconnects the whole monitor (see setControl). */
const BLANKS_SCREEN = "adaptive-sync";

/** How long "Keep this change?" waits. Matches --keep-window in app.css. */
export const KEEP_MS = 5000;

export const ui = $state({
  /** The change waiting for "Keep" (reverts when the time runs out). */
  keep: null as Keep | null,
  state: null as AppState | null,
  page: "displays" as Page,
  busy: false,
  /** A write the monitor is slow to answer (a resync can take seconds), shown after a short delay. */
  waiting: null as string | null,
  toasts: [] as Toast[],
  dialog: null as Dialog | null,
  updateBadge: false,
  /** Picture mode being switched to, shown at once while the monitor catches up. */
  switching: null as string | null,
  /** Smart feature being switched, shown flipped at once while the monitor confirms. */
  pendingFeature: null as { id: Feature; on: boolean } | null,
});

let toastId = 0;
export function toast(text: string, tone: Toast["tone"] = "info", undo?: () => void) {
  const t = { id: ++toastId, text, tone, undo };
  // Untracked: a toast fired from an $effect must not make that effect depend
  // on the toast list, or each toast re-runs it and fires another.
  ui.toasts = [...untrack(() => ui.toasts).slice(-3), t];
  // An error stays until it is dismissed: it may be the only clue to what went
  // wrong, and it has to outlast reading it, copying it, or taking a screenshot.
  if (tone !== "error") setTimeout(() => dismiss(t.id), undo ? 5000 : tone === "warn" ? 9000 : 3200);
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
  // Switching VRR makes the whole monitor reconnect: say so first, instead of
  // springing a black screen on the user. Putting it back (quiet) is not asked twice.
  if (id === BLANKS_SCREEN && !opts.quiet) {
    const ok = await ask({
      title: "Switch Adaptive-Sync?",
      body:
        "Your screen will go black for a few seconds while the monitor reconnects, and nubless can't talk to it until it's back. Nothing is wrong when that happens. Save your work if a game or video is running." +
        (value !== 0 && untrack(() => ui.state)?.controls.find((c) => c.id === "motion-blur")?.value === 1 ? " Motion Blur Reduction will switch off too; it only works with Adaptive-Sync off." : ""),
      confirm: "Switch",
      tone: "signal",
      cancel: "Not now",
    });
    if (!ok) return;
  }
  const name = untrack(() => ui.state)?.controls.find((c) => c.id === id)?.label ?? "Setting";
  // Most writes answer at once. If this one does not, say what we are waiting for.
  const slow = setTimeout(() => (ui.waiting = name), 600);
  const out = await guarded(async () => (await api()).setControl(id, value)).finally(() => {
    clearTimeout(slow);
    ui.waiting = null;
  });
  if (!out) return;
  ui.state = out.state;
  const before = opts.before;
  if (opts.quiet || before == null || before === value) return;
  const control = out.state.controls.find((c) => c.id === id);
  const label = control?.label ?? "Setting";
  // Decided by the control, not by this call: the last write of a drag can be a
  // no-op because a throttled one already sent the same value.
  // VRR can't strand you on a bad picture, so it gets the warning above and a plain Undo.
  if (control?.confirm && id !== BLANKS_SCREEN) {
    askToKeep(id, label, control.display ?? String(value), before);
    return;
  }
  toast(out.toast ?? `${label} updated`, "success", async () => {
    await setControl(id, before, { quiet: true });
    toast("Undone", "info");
  });
}

let keepTimer: ReturnType<typeof setTimeout> | undefined;
let keepRun = 0;

/**
 * Ask "Keep this change?" for a risky change that already went out. Changing
 * the same control again keeps the original `before`, so a run of tweaks is one
 * change that is kept or put back as a whole.
 */
export function askToKeep(id: string, label: string, shown: string, before: number) {
  const prior = untrack(() => ui.keep);
  ui.keep = { id, label, shown, before: prior?.id === id ? prior.before : before, run: ++keepRun };
  clearTimeout(keepTimer);
  keepTimer = setTimeout(() => void revertChange(), KEEP_MS);
}

export function keepChange() {
  clearTimeout(keepTimer);
  ui.keep = null;
}

/** "Revert", or no answer in time: put the control back to what it was. */
export async function revertChange() {
  const k = untrack(() => ui.keep);
  if (!k) return;
  keepChange();
  const out = await guarded(async () => (await api()).revertControl(k.id, k.before));
  if (out) {
    ui.state = out.state;
    toast(`${k.label} put back`, "info");
  }
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
  if (ui.pendingFeature) return;
  const b = await api();
  ui.pendingFeature = { id: feature, on };
  try {
    let out: FeatureOutcome | undefined = await guarded(() => b.toggleFeature(feature, on, false));
    if (out?.kind === "confirm") {
      ui.pendingFeature = null;
      const ok = await ask({ title: featureTitles[feature], body: out.question, confirm: "Turn on", cancel: "Not now" });
      if (!ok) return;
      ui.pendingFeature = { id: feature, on };
      out = await guarded(() => b.toggleFeature(feature, on, true));
    }
    handleFeature(out);
  } finally {
    ui.pendingFeature = null;
  }
}

/** Switch picture mode through the AI interlock, asking when the rules say so. */
export async function pickPictureMode(label: string) {
  if (ui.switching) return;
  const b = await api();
  ui.switching = label;
  try {
    let out: FeatureOutcome | undefined = await guarded(() => b.pickPictureMode(label, false));
    if (out?.kind === "confirm") {
      ui.switching = null;
      const ok = await ask({ title: `Switch to ${label}?`, body: out.question, confirm: "Switch", cancel: "Not now" });
      if (!ok) return;
      ui.switching = label;
      out = await guarded(() => b.pickPictureMode(label, true));
    }
    handleFeature(out);
  } finally {
    ui.switching = null;
  }
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
