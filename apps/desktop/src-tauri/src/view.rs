//! What the UI sees. Everything the frontend renders comes from [`AppState`], so
//! the frontend never decides on its own whether something is allowed.

use nubless_core::profile::{Confidence, Control, Risk, ValueKind, WritePolicy};
use nubless_core::profiles::gm950b::PROFILE;
use nubless_core::rules::{Availabilities, FeatureState};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MonitorCard {
    pub id: usize,
    pub name: String,
    pub detail: String,
    pub supported: bool,
    pub selected: bool,
    pub demo: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChoiceView {
    pub raw: u16,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlView {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    /// `range`, `choice` or `info`.
    pub kind: &'static str,
    pub value: Option<u16>,
    pub display: Option<String>,
    pub min: u16,
    pub max: u16,
    pub choices: Vec<ChoiceView>,
    pub enabled: bool,
    /// Why it can't be changed right now, in plain words.
    pub reason: Option<&'static str>,
    /// `labs` when turning on Labs would unlock it.
    pub unlock: Option<&'static str>,
    /// A change can hide the picture or resync the panel: after setting it the UI
    /// asks "Keep this change?" and reverts if nobody answers (`Req::Revert`).
    pub confirm: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeaturesView {
    pub state: FeatureState,
    pub availability: Availabilities,
    /// Feature id -> why it can't be switched on this connection.
    pub locks: std::collections::BTreeMap<&'static str, String>,
    /// Current picture mode label, when known.
    pub picture_mode: Option<String>,
    /// Picture modes in which the AI features work.
    pub ai_capable_modes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaselineView {
    pub saved: bool,
    pub values: usize,
    /// Saved during this connection (first run, or any demo connect): the
    /// main window celebrates once.
    pub just_saved: bool,
    /// Picture mode the originals belong to ("Personalized Picture").
    pub saved_in: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppState {
    pub phase: &'static str, // "scanning" | "connected" | "empty"
    pub demo: bool,
    pub monitors: Vec<MonitorCard>,
    pub model: &'static str,
    pub controls: Vec<ControlView>,
    pub features: FeaturesView,
    pub baseline: BaselineView,
    pub labs: bool,
    pub lg_switch_running: bool,
    pub error: Option<String>,
    pub version: &'static str,
    /// Values are being re-read (after a picture-mode switch).
    pub refreshing: bool,
}

pub fn group_of(id: &str) -> &'static str {
    match id {
        "picture-mode" => "mode",
        "brightness" | "contrast" | "black-stabilizer" | "color-temperature" | "red-gain"
        | "green-gain" | "blue-gain" => "picture",
        "response-time" | "adaptive-sync" | "motion-blur" | "aspect-ratio" => "game",
        "volume" | "mute" | "ai-sound" => "sound",
        "ai-scene-optimization" => "smart",
        "ai-upscaling" => "picture",
        _ => "info",
    }
}

/// Why `c` can't be written under `policy`, if it can't.
pub fn lock_reason(
    c: &Control,
    policy: WritePolicy,
) -> (Option<&'static str>, Option<&'static str>) {
    if policy.allows(c) {
        return (None, None);
    }
    if c.risk == Risk::DeadEnd {
        return (Some("Changing this could disconnect nubless from your monitor, so it stays with the monitor's own menu."), None);
    }
    if matches!(c.kind, ValueKind::Info) && c.confidence >= Confidence::ReadConfirmed {
        return (Some("Information from your monitor."), None);
    }
    if WritePolicy::Experimental.allows(c) {
        return (
            Some("Looks right, but hasn't been tested on a real monitor yet."),
            Some("labs"),
        );
    }
    (
        Some("Not mapped yet. We know where it lives, but not how it's encoded."),
        None,
    )
}

pub fn view_control(c: &'static Control, value: Option<u16>, policy: WritePolicy) -> ControlView {
    let (kind, min, max, choices) = match c.kind {
        ValueKind::Range { min, max } => ("range", min, max, vec![]),
        ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => (
            "choice",
            0,
            0,
            choices
                .iter()
                .map(|ch| ChoiceView {
                    raw: ch.raw,
                    label: ch.label,
                })
                .collect(),
        ),
        ValueKind::Info => ("info", 0, 0, vec![]),
    };
    // An unconfirmed sub-command encoding shows the raw value, not a guessed label.
    let guessed =
        matches!(c.kind, ValueKind::SubCommand { .. }) && c.confidence < Confidence::Strong;
    let display = value.map(|v| match (c.choice_label(v), c.vcp) {
        (Some(_), _) if guessed => format!("raw {v} (not mapped yet)"),
        (Some(l), _) => l.to_string(),
        (None, 0xC9) => format!("{}.{:02X}", v >> 8, v & 0xFF),
        (None, 0xC0) => format!("{v} h"),
        _ => v.to_string(),
    });
    let (reason, unlock) = lock_reason(c, policy);
    let reason = if value.is_none() && reason.is_none() {
        Some("Your monitor didn't answer for this one. Try again in a moment.")
    } else {
        reason
    };
    ControlView {
        id: c.id,
        label: c.label,
        group: group_of(c.id),
        kind,
        value,
        display,
        min,
        max,
        choices,
        enabled: reason.is_none(),
        reason,
        unlock,
        // Picture mode is switched through the AI interlock, which asks its own question.
        confirm: c.needs_confirm_or_revert() && PROFILE.mode_control != Some(c.id),
    }
}
