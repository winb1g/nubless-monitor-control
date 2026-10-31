import type { AppState, Feature, FeatureOutcome, HotkeyAction, HotkeysView, Outcome } from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Backend = {
  getState(): Promise<AppState>;
  rescan(): Promise<AppState>;
  useDemo(): Promise<AppState>;
  setLabs(on: boolean): Promise<AppState>;
  /** Make a monitor (by `MonitorCard.id`) the one the app controls. */
  selectMonitor(id: string): Promise<AppState>;
  /** Send every change to all connected monitors. */
  setLinked(on: boolean): Promise<AppState>;
  // `monitor` is the monitor the window showed (`MonitorCard.id`), so a change
  // never lands on one another window has just selected. `null` = the selected one.
  setControl(id: string, value: number, monitor: string | null): Promise<Outcome>;
  toggleFeature(feature: Feature, on: boolean, confirmed: boolean, monitor: string | null): Promise<FeatureOutcome>;
  simulatePbp(on: boolean): Promise<FeatureOutcome>;
  pickPictureMode(label: string, confirmed: boolean, monitor: string | null): Promise<FeatureOutcome>;
  undo(monitor: string | null): Promise<Outcome>;
  revertControl(id: string, value: number, monitor: string | null): Promise<Outcome>;
  resetOriginal(monitor: string | null): Promise<Outcome>;
  getHotkeys(): Promise<HotkeysView>;
  setHotkeys(enabled: boolean, bindings: Record<HotkeyAction, string>): Promise<HotkeysView>;
  getAutostart(): Promise<boolean>;
  setAutostart(on: boolean): Promise<boolean>;
  getAutoUpdate(): Promise<boolean>;
  setAutoUpdate(on: boolean): Promise<boolean>;
  /** Green pip and "Update x available" in the tray; `null` clears them. */
  setUpdateAvailable(version: string | null): Promise<void>;
  openMain(page?: string): Promise<void>;
  quit(): Promise<void>;
  onState(cb: (s: AppState) => void): void;
  onNavigate(cb: (page: string) => void): void;
};

async function tauriBackend(): Promise<Backend> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    getState: () => invoke("get_state"),
    rescan: () => invoke("rescan"),
    useDemo: () => invoke("use_demo"),
    setLabs: (on) => invoke("set_labs", { on }),
    selectMonitor: (id) => invoke("select_monitor", { id }),
    setLinked: (on) => invoke("set_linked", { on }),
    setControl: (id, value, monitor) => invoke("set_control", { id, value, monitor }),
    toggleFeature: (feature, on, confirmed, monitor) => invoke("toggle_feature", { feature, on, confirmed, monitor }),
    simulatePbp: (on) => invoke("simulate_pbp", { on }),
    pickPictureMode: (label, confirmed, monitor) => invoke("pick_picture_mode", { label, confirmed, monitor }),
    undo: (monitor) => invoke("undo", { monitor }),
    revertControl: (id, value, monitor) => invoke("revert_control", { id, value, monitor }),
    resetOriginal: (monitor) => invoke("reset_original", { monitor }),
    getHotkeys: () => invoke("get_hotkeys"),
    setHotkeys: (enabled, bindings) => invoke("set_hotkeys", { enabled, bindings }),
    getAutostart: () => invoke("get_autostart"),
    setAutostart: (on) => invoke("set_autostart", { on }),
    getAutoUpdate: () => invoke("get_auto_update"),
    setAutoUpdate: (on) => invoke("set_auto_update", { on }),
    setUpdateAvailable: (version) => invoke("set_update_available", { version }),
    openMain: (page) => invoke("open_main", { page: page ?? null }),
    quit: () => invoke("quit"),
    onState: (cb) => void listen<AppState>("state", (e) => cb(e.payload)),
    onNavigate: (cb) => void listen<string>("navigate", (e) => cb(e.payload)),
  };
}

let backend: Promise<Backend> | null = null;

/** Real backend inside Tauri; a browser-only preview backend otherwise. */
export function api(): Promise<Backend> {
  backend ??= inTauri ? tauriBackend() : import("./preview").then((m) => m.previewBackend());
  return backend;
}
