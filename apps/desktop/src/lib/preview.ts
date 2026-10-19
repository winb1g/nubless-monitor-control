// Browser-only stand-in for the Rust backend, so the UI can be designed in a
// plain browser (`npm run dev`). It is NOT used inside the app: the real
// rules, safety checks and monitor access live in nubless-core.

import type { AppState, ControlView, Feature, FeatureOutcome, FeatureState, Outcome } from "./types";

const LABS = "Looks right, but hasn't been tested on a real monitor yet.";
const UNMAPPED = "Not mapped yet. We know where it lives, but not how it's encoded.";
const INFO = "Information from your monitor.";

function c(p: Partial<ControlView> & Pick<ControlView, "id" | "label" | "group" | "kind">): ControlView {
  return { value: null, display: null, min: 0, max: 0, choices: [], enabled: true, reason: null, unlock: null, ...p };
}

function controls(labs: boolean, v: Record<string, number>): ControlView[] {
  const strong = (x: ControlView) => (labs ? x : { ...x, enabled: false, reason: LABS, unlock: "labs" as const });
  const list: ControlView[] = [
    c({ id: "brightness", label: "Brightness", group: "picture", kind: "range", min: 0, max: 100 }),
    c({ id: "contrast", label: "Contrast", group: "picture", kind: "range", min: 20, max: 100 }),
    strong(c({ id: "black-stabilizer", label: "Black Stabilizer", group: "picture", kind: "range", min: 0, max: 100 })),
    strong(c({ id: "response-time", label: "Response Time", group: "game", kind: "choice", choices: [{ raw: 3, label: "Normal" }, { raw: 2, label: "Fast" }, { raw: 1, label: "Faster" }] })),
    strong(c({ id: "adaptive-sync", label: "Adaptive-Sync/VRR", group: "game", kind: "choice", choices: [{ raw: 0, label: "Off" }, { raw: 2, label: "Basic" }, { raw: 3, label: "Extended" }] })),
    c({ id: "picture-mode", label: "Picture Mode", group: "mode", kind: "choice", enabled: false, reason: UNMAPPED, choices: [
      { raw: 60, label: "Personalized Picture" }, { raw: 39, label: "Gamer 1" }, { raw: 40, label: "Gamer 2" }, { raw: 18, label: "Calibration1" },
      { raw: 19, label: "Calibration2" }, { raw: 30, label: "Color Weakness" }, { raw: 25, label: "DCI-P3" }, { raw: 43, label: "FPS" }] }),
    c({ id: "color-temperature", label: "Color temperature", group: "picture", kind: "info", enabled: false, reason: UNMAPPED }),
    strong(c({ id: "volume", label: "Volume", group: "sound", kind: "range", min: 0, max: 100 })),
    strong(c({ id: "mute", label: "Mute", group: "sound", kind: "choice", choices: [{ raw: 1, label: "Muted" }, { raw: 2, label: "Unmuted" }] })),
    c({ id: "input", label: "Input", group: "info", kind: "choice", enabled: false, reason: "Changing this could disconnect nubless from your monitor, so it stays with the monitor's own menu.", choices: [{ raw: 15, label: "DisplayPort" }] }),
    c({ id: "firmware", label: "Firmware", group: "info", kind: "info", enabled: false, reason: INFO }),
    c({ id: "usage-hours", label: "Usage time (hours)", group: "info", kind: "info", enabled: false, reason: UNMAPPED }),
  ];
  return list.map((x) => {
    const value = v[x.id] ?? null;
    const label = x.choices.find((ch) => ch.raw === value)?.label;
    const display = x.id === "firmware" ? "3.05" : x.id === "usage-hours" ? `${value} h` : label ?? (value === null ? null : String(value));
    return { ...x, value, display };
  });
}

const names: Record<Feature, string> = {
  "ai-scene-optimization": "AI Scene Optimization",
  "ai-picture-mode": "AI Picture Mode",
  "my-display-preset": "My Display Preset",
};
const key: Record<Feature, keyof FeatureState> = {
  "ai-scene-optimization": "ai_scene_optimization",
  "ai-picture-mode": "ai_picture_mode",
  "my-display-preset": "my_display_preset",
};
const list = (xs: string[]) => (xs.length < 2 ? xs.join("") : `${xs.slice(0, -1).join(", ")} and ${xs[xs.length - 1]}`);

export function previewBackend() {
  const values: Record<string, number> = { brightness: 80, contrast: 70, "black-stabilizer": 50, "response-time": 2, "adaptive-sync": 3, "picture-mode": 65, volume: 20, mute: 2, input: 15, firmware: 0x305, "usage-hours": 1000 };
  const original = { ...values };
  let labs = false;
  let connected = true;
  let features: FeatureState = { ai_scene_optimization: false, ai_picture_mode: false, my_display_preset: false, pbp: false, hdr: false };
  const history: [string, number][] = [];
  const listeners: ((s: AppState) => void)[] = [];
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

  const state = (): AppState => ({
    phase: connected ? "connected" : "empty",
    demo: true,
    monitors: connected ? [{ id: 0, name: "27GM950B (demo)", detail: "Browser preview", supported: true, selected: true, demo: true }] : [],
    model: "27GM950B",
    controls: connected ? controls(labs, values) : [],
    features: {
      state: features,
      availability: Object.fromEntries(["ai_scene_optimization", "ai_picture_mode", "my_display_preset", "ai_upscaling", "picture_wizard"].map((k) => [k, features.pbp ? { state: "unavailable", reason: { kind: "blocked-by-pbp" } } : { state: "available" }])),
      mapped: true,
    },
    baseline: { saved: true, values: Object.keys(original).length, just_saved: false, error: null },
    labs,
    lg_switch_running: false,
    error: null,
    version: "0.1.0",
  });
  const emit = () => listeners.forEach((l) => l(state()));

  return {
    async getState() { await wait(500); return state(); },
    async rescan() { await wait(900); connected = true; emit(); return state(); },
    async useDemo() { connected = true; emit(); return state(); },
    async setLabs(on: boolean) { labs = on; emit(); return state(); },
    async setControl(id: string, value: number): Promise<Outcome> {
      await wait(120);
      const label = controls(labs, values).find((x) => x.id === id)?.label ?? "Setting";
      if (values[id] === value) return { state: state(), toast: null, undoable: false };
      history.push([id, values[id]]);
      values[id] = value;
      emit();
      return { state: state(), toast: `${label} updated`, undoable: true };
    },
    async toggleFeature(f: Feature, on: boolean, confirmed: boolean): Promise<FeatureOutcome> {
      if (features[key[f]] === on) return { kind: "applied", state: state(), notice: null };
      if (!on) { features = { ...features, [key[f]]: false }; emit(); return { kind: "applied", state: state(), notice: null }; }
      if (features.pbp) return { kind: "blocked", reason: `${names[f]} is not available while PBP/PIP is on. Turn PBP/PIP off first.`, state: state() };
      const others = (Object.keys(key) as Feature[]).filter((o) => o !== f && features[key[o]]);
      if (others.length && f !== "ai-scene-optimization" && !confirmed)
        return { kind: "confirm", question: `Turning on ${names[f]} will turn off ${list(others.map((o) => names[o]))}. Continue?` };
      const next = { ...features, [key[f]]: true };
      others.forEach((o) => (next[key[o]] = false));
      features = next;
      emit();
      const notice = others.length && f === "ai-scene-optimization" ? `${names[f]} is on, so ${list(others.map((o) => names[o]))} turned off.` : null;
      return { kind: "applied", state: state(), notice };
    },
    async simulatePbp(on: boolean): Promise<FeatureOutcome> {
      const off = (Object.keys(key) as Feature[]).filter((o) => on && features[key[o]]);
      features = { ...features, pbp: on, ...(on ? { ai_scene_optimization: false, ai_picture_mode: false, my_display_preset: false } : {}) };
      emit();
      return { kind: "applied", state: state(), notice: off.length ? `PBP/PIP is on, so ${list(off.map((o) => names[o]))} turned off.` : null };
    },
    async undo(): Promise<Outcome> {
      const last = history.pop();
      if (!last) throw "Nothing to undo";
      values[last[0]] = last[1];
      emit();
      return { state: state(), toast: "Undone", undoable: false };
    },
    async resetOriginal(): Promise<Outcome> {
      await wait(600);
      Object.assign(values, original);
      features = { ...features, ai_scene_optimization: false, ai_picture_mode: false, my_display_preset: false };
      emit();
      return { state: state(), toast: "Your original settings are back", undoable: false };
    },
    async openMain() {},
    async quit() {},
    onState(cb: (s: AppState) => void) { listeners.push(cb); },
    onNavigate() {},
  };
}
