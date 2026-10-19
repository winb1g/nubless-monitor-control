export type Feature = "ai-scene-optimization" | "ai-picture-mode" | "my-display-preset";

export interface MonitorCard {
  id: number;
  name: string;
  detail: string;
  supported: boolean;
  selected: boolean;
  demo: boolean;
}

export interface ControlView {
  id: string;
  label: string;
  group: "mode" | "picture" | "game" | "sound" | "info";
  kind: "range" | "choice" | "info";
  value: number | null;
  display: string | null;
  min: number;
  max: number;
  choices: { raw: number; label: string }[];
  enabled: boolean;
  reason: string | null;
  unlock: "labs" | null;
}

export interface FeatureState {
  ai_scene_optimization: boolean;
  ai_picture_mode: boolean;
  my_display_preset: boolean;
  pbp: boolean;
  hdr: boolean;
  picture_mode_blocks_ai: boolean;
}

export type Availability = { state: "available" } | { state: "unavailable"; reason: { kind: string } };

export interface AppState {
  phase: "scanning" | "connected" | "empty";
  demo: boolean;
  monitors: MonitorCard[];
  model: string;
  controls: ControlView[];
  features: {
    state: FeatureState;
    availability: Record<string, Availability>;
    /** Feature id -> why it can't be switched on this connection. */
    locks: Record<string, string>;
    picture_mode: string | null;
    ai_capable_modes: string[];
  };
  baseline: { saved: boolean; values: number; just_saved: boolean; error: string | null; saved_in: string | null };
  labs: boolean;
  lg_switch_running: boolean;
  error: string | null;
  version: string;
  /** Values are being re-read after a picture-mode switch. */
  refreshing: boolean;
}

export interface Outcome {
  state: AppState;
  toast: string | null;
  undoable: boolean;
}

export type FeatureOutcome =
  | { kind: "applied"; state: AppState; notice: string | null }
  | { kind: "confirm"; question: string }
  | { kind: "blocked"; reason: string; state: AppState };

export type HotkeyAction = "brightness-up" | "brightness-down" | "next-picture-mode";

export interface HotkeyBinding {
  action: HotkeyAction;
  label: string;
  /** Accelerator string, e.g. "Ctrl+Alt+PageUp". */
  keys: string;
  default_keys: string;
  /** Why this one isn't active: taken by another app, or not a valid key. */
  error: string | null;
}

export interface HotkeysView {
  enabled: boolean;
  bindings: HotkeyBinding[];
}
