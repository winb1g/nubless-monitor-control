import type { AppState, Feature, FeatureOutcome, Outcome } from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Backend = {
  getState(): Promise<AppState>;
  rescan(): Promise<AppState>;
  useDemo(): Promise<AppState>;
  setLabs(on: boolean): Promise<AppState>;
  setControl(id: string, value: number): Promise<Outcome>;
  toggleFeature(feature: Feature, on: boolean, confirmed: boolean): Promise<FeatureOutcome>;
  simulatePbp(on: boolean): Promise<FeatureOutcome>;
  undo(): Promise<Outcome>;
  resetOriginal(): Promise<Outcome>;
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
    setControl: (id, value) => invoke("set_control", { id, value }),
    toggleFeature: (feature, on, confirmed) => invoke("toggle_feature", { feature, on, confirmed }),
    simulatePbp: (on) => invoke("simulate_pbp", { on }),
    undo: () => invoke("undo"),
    resetOriginal: () => invoke("reset_original"),
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
