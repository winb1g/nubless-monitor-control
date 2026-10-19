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
    mapped: boolean;
  };
  baseline: { saved: boolean; values: number; just_saved: boolean; error: string | null };
  labs: boolean;
  lg_switch_running: boolean;
  error: string | null;
  version: string;
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
