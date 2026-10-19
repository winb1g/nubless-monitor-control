//! Feature interlockfor s in all_states().filter(valid), reproducing LG Switch's (0u8..64).map(|b| FeatureState {ehaviour (see `docs/rules.md`
//! for the evidence). Pure functions over a [`FeatureState`] so every
//! combination can be tested.
//!
//! The three "smart" features are mutually exclusive:
//!
//! | Turning on | Turns off | Asks first? |
//! |---|---|---|
//! | AI Scene Optimization | AI Picture Mode, My Display Preset | no, shows a notice |
//! | AI Picture Mode | AI Scene Optimization, My Display Preset | yes |
//! | My Display Preset | AI Scene Optimization, AI Picture Mode | yes |
//!
//! While PBP/PIP is on, none of them can be turned on, and AI Upscaling options
//! are unavailable. The Personalized Picture Wizard needs HDR and PBP/PIP off.
//!
//! AI Scene Optimization and AI Picture Mode only work in some picture modes
//! (LG Switch's `PictureModeToAISupportedMap`: Personalized Picture, Custom,
//! Gamer 1, Gamer 2, Max Brightness, FPS, RTS, Vivid). In any other mode LG
//! Switch turns both off and disables them, and the monitor refuses a switch to
//! such a mode while AI is on. nubless asks first and turns AI off for you
//! ([`decide_picture_mode`]) instead of letting the switch fail.
//!
//! nubless adds one rule of its own: **turning a feature off is always allowed,
//! immediately, without a question**, so no combination can trap the user.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FeatureState {
    /// On-monitor AI (the 27GM950B's SoC), `IsSoCAIModeSupported` in LG Switch.
    pub ai_scene_optimization: bool,
    /// PC-side AI picture mode (LG's encrypted model), `IsPCAIModeEnabled`.
    pub ai_picture_mode: bool,
    /// Per-application picture-mode presets.
    pub my_display_preset: bool,
    /// PBP/PIP active on the monitor (read from the monitor, not set here).
    pub pbp: bool,
    /// HDR active (read from the monitor/OS, not set here).
    pub hdr: bool,
    /// The current picture mode does not support the AI features.
    pub picture_mode_blocks_ai: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    AiSceneOptimization,
    AiPictureMode,
    MyDisplayPreset,
}

impl Feature {
    pub const ALL: [Feature; 3] = [
        Feature::AiSceneOptimization,
        Feature::AiPictureMode,
        Feature::MyDisplayPreset,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Feature::AiSceneOptimization => "AI Scene Optimization",
            Feature::AiPictureMode => "AI Picture Mode",
            Feature::MyDisplayPreset => "My Display Preset",
        }
    }

    fn get(self, s: &FeatureState) -> bool {
        match self {
            Feature::AiSceneOptimization => s.ai_scene_optimization,
            Feature::AiPictureMode => s.ai_picture_mode,
            Feature::MyDisplayPreset => s.my_display_preset,
        }
    }

    fn set(self, s: &mut FeatureState, on: bool) {
        match self {
            Feature::AiSceneOptimization => s.ai_scene_optimization = on,
            Feature::AiPictureMode => s.ai_picture_mode = on,
            Feature::MyDisplayPreset => s.my_display_preset = on,
        }
    }
}

/// Something to tell or ask the user. Wording is ours; meaning matches LG Switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Notice {
    /// Question before turning on `feature`, which will turn off `turns_off`.
    ConfirmExclusive {
        feature: Feature,
        turns_off: Vec<Feature>,
    },
    /// Information after turning on `feature` turned off `turned_off`.
    TurnedOff {
        feature: Feature,
        turned_off: Vec<Feature>,
    },
    /// `feature` cannot be turned on while PBP/PIP is on.
    BlockedByPbp {
        feature: Feature,
    },
    /// PBP/PIP came on and turned these off.
    PbpTurnedOff {
        turned_off: Vec<Feature>,
    },
    /// `feature` needs an AI-capable picture mode.
    NeedsAiPictureMode {
        feature: Feature,
    },
    /// Question before switching to `mode`, which turns off `turns_off`.
    LeaveAiForMode {
        mode: String,
        turns_off: Vec<Feature>,
    },
    AiUpscalingNeedsPbpOff,
    WizardNeedsHdrOff,
    WizardNeedsPbpOff,
}

fn list(fs: &[Feature]) -> String {
    let names: Vec<_> = fs.iter().map(|f| f.label()).collect();
    match names.as_slice() {
        [] => String::new(),
        [a] => a.to_string(),
        [rest @ .., last] => format!("{} and {}", rest.join(", "), last),
    }
}

impl Notice {
    pub fn text(&self) -> String {
        match self {
            Notice::ConfirmExclusive { feature, turns_off } => format!(
                "Turning on {} will turn off {}. Continue?",
                feature.label(),
                list(turns_off)
            ),
            Notice::TurnedOff { feature, turned_off } => {
                format!("{} is on, so {} turned off.", feature.label(), list(turned_off))
            }
            Notice::BlockedByPbp { feature } => format!(
                "{} is not available while PBP/PIP is on. Turn PBP/PIP off first.",
                feature.label()
            ),
            Notice::PbpTurnedOff { turned_off } => format!("PBP/PIP is on, so {} turned off.", list(turned_off)),
            Notice::NeedsAiPictureMode { feature } => format!(
                "{} isn't available in this picture mode. Switch to Personalized Picture, Gamer 1, Gamer 2 or FPS first.",
                feature.label()
            ),
            Notice::LeaveAiForMode { mode, turns_off } => format!(
                "{mode} doesn't work with {}. Turn {} off and switch to {mode}?",
                list(turns_off),
                if turns_off.len() == 1 { "it" } else { "them" }
            ),
            Notice::AiUpscalingNeedsPbpOff => "AI Upscaling is not available while PBP/PIP is on.".into(),
            Notice::WizardNeedsHdrOff => {
                "The Personalized Picture Wizard needs HDR off. Turn HDR off in the system display settings, then try again.".into()
            }
            Notice::WizardNeedsPbpOff => {
                "The Personalized Picture Wizard needs PBP/PIP off. Turn PBP/PIP off, then try again.".into()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "kebab-case")]
pub enum Decision {
    /// Apply `next` now; show `notice` if any.
    Apply {
        next: FeatureState,
        notice: Option<Notice>,
    },
    /// Ask `question`; on yes apply `next`, on no change nothing.
    Confirm {
        question: Notice,
        next: FeatureState,
    },
    /// Cannot be done now; `reason` says why and what to do.
    Blocked { reason: Notice },
}

/// What happens when the user flips `feature` to `on`.
pub fn decide(state: FeatureState, feature: Feature, on: bool) -> Decision {
    if feature.get(&state) == on {
        return Decision::Apply {
            next: state,
            notice: None,
        };
    }
    let mut next = state;
    feature.set(&mut next, on);
    if !on {
        // Turning off: always allowed, never asks.
        return Decision::Apply { next, notice: None };
    }
    if state.pbp {
        return Decision::Blocked {
            reason: Notice::BlockedByPbp { feature },
        };
    }
    if state.picture_mode_blocks_ai && feature != Feature::MyDisplayPreset {
        return Decision::Blocked {
            reason: Notice::NeedsAiPictureMode { feature },
        };
    }
    let others: Vec<Feature> = Feature::ALL
        .into_iter()
        .filter(|f| *f != feature && f.get(&state))
        .collect();
    for f in &others {
        f.set(&mut next, false);
    }
    if others.is_empty() {
        return Decision::Apply { next, notice: None };
    }
    match feature {
        Feature::AiSceneOptimization => Decision::Apply {
            next,
            notice: Some(Notice::TurnedOff {
                feature,
                turned_off: others,
            }),
        },
        Feature::AiPictureMode | Feature::MyDisplayPreset => Decision::Confirm {
            question: Notice::ConfirmExclusive {
                feature,
                turns_off: others,
            },
            next,
        },
    }
}

/// What happens when the user picks picture mode `mode`.
pub fn decide_picture_mode(state: FeatureState, mode: &str, mode_allows_ai: bool) -> Decision {
    let mut next = FeatureState {
        picture_mode_blocks_ai: !mode_allows_ai,
        ..state
    };
    if mode_allows_ai {
        return Decision::Apply { next, notice: None };
    }
    let turns_off: Vec<Feature> = [Feature::AiSceneOptimization, Feature::AiPictureMode]
        .into_iter()
        .filter(|f| f.get(&state))
        .collect();
    for f in &turns_off {
        f.set(&mut next, false);
    }
    if turns_off.is_empty() {
        Decision::Apply { next, notice: None }
    } else {
        Decision::Confirm {
            question: Notice::LeaveAiForMode {
                mode: mode.to_string(),
                turns_off,
            },
            next,
        }
    }
}

/// Reaction to the monitor reporting a PBP/PIP change.
pub fn on_pbp_changed(state: FeatureState, pbp: bool) -> (FeatureState, Option<Notice>) {
    let mut next = FeatureState { pbp, ..state };
    if !pbp {
        return (next, None);
    }
    let turned_off: Vec<Feature> = Feature::ALL.into_iter().filter(|f| f.get(&state)).collect();
    for f in &turned_off {
        f.set(&mut next, false);
    }
    let notice = (!turned_off.is_empty()).then_some(Notice::PbpTurnedOff { turned_off });
    (next, notice)
}

/// Whether a control can be used right now, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Unavailable { reason: Notice },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Availabilities {
    pub ai_scene_optimization: Availability,
    pub ai_picture_mode: Availability,
    pub my_display_preset: Availability,
    pub ai_upscaling: Availability,
    pub picture_wizard: Availability,
}

pub fn availability(state: FeatureState) -> Availabilities {
    let feature = |f: Feature| {
        if state.pbp {
            Availability::Unavailable {
                reason: Notice::BlockedByPbp { feature: f },
            }
        } else if state.picture_mode_blocks_ai && f != Feature::MyDisplayPreset {
            Availability::Unavailable {
                reason: Notice::NeedsAiPictureMode { feature: f },
            }
        } else {
            Availability::Available
        }
    };
    Availabilities {
        ai_scene_optimization: feature(Feature::AiSceneOptimization),
        ai_picture_mode: feature(Feature::AiPictureMode),
        my_display_preset: feature(Feature::MyDisplayPreset),
        ai_upscaling: if state.pbp {
            Availability::Unavailable {
                reason: Notice::AiUpscalingNeedsPbpOff,
            }
        } else {
            Availability::Available
        },
        picture_wizard: if state.hdr {
            Availability::Unavailable {
                reason: Notice::WizardNeedsHdrOff,
            }
        } else if state.pbp {
            Availability::Unavailable {
                reason: Notice::WizardNeedsPbpOff,
            }
        } else {
            Availability::Available
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_states() -> impl Iterator<Item = FeatureState> {
        (0u8..32).map(|b| FeatureState {
            ai_scene_optimization: b & 1 != 0,
            ai_picture_mode: b & 2 != 0,
            my_display_preset: b & 4 != 0,
            pbp: b & 8 != 0,
            hdr: b & 16 != 0,
            picture_mode_blocks_ai: b & 32 != 0,
        })
    }

    fn smart_count(s: &FeatureState) -> usize {
        Feature::ALL.iter().filter(|f| f.get(s)).count()
    }

    /// States the rules can produce: one smart feature at most, none under
    /// PBP/PIP, and no AI feature in a picture mode that blocks AI.
    fn valid(s: &FeatureState) -> bool {
        smart_count(s) <= 1
            && !(s.pbp && smart_count(s) > 0)
            && !(s.picture_mode_blocks_ai && (s.ai_scene_optimization || s.ai_picture_mode))
    }

    #[test]
    fn exclusivity_holds_from_every_valid_state() {
        for s in all_states().filter(|s| smart_count(s) <= 1 && !(s.pbp && smart_count(s) > 0)) {
            for f in Feature::ALL {
                for on in [true, false] {
                    let next = match decide(s, f, on) {
                        Decision::Apply { next, .. } | Decision::Confirm { next, .. } => next,
                        Decision::Blocked { .. } => continue,
                    };
                    assert!(valid(&next), "{s:?} {f:?} {on} -> {next:?}");
                    assert!(smart_count(&next) <= 1, "{s:?} {f:?} {on} -> {next:?}");
                    assert!(!(next.pbp && smart_count(&next) > 0));
                    assert_eq!(
                        (next.pbp, next.hdr),
                        (s.pbp, s.hdr),
                        "rules never change PBP/HDR"
                    );
                }
            }
        }
    }

    #[test]
    fn turning_off_is_always_immediate() {
        for s in all_states() {
            for f in Feature::ALL {
                match decide(s, f, false) {
                    Decision::Apply { next, notice: None } => assert!(!f.get(&next)),
                    other => panic!("turning off {f:?} from {s:?} gave {other:?}"),
                }
            }
        }
    }

    #[test]
    fn ai_picture_mode_asks_before_disabling_others() {
        let s = FeatureState {
            ai_scene_optimization: true,
            ..Default::default()
        };
        match decide(s, Feature::AiPictureMode, true) {
            Decision::Confirm { question, next } => {
                assert!(
                    next.ai_picture_mode && !next.ai_scene_optimization && !next.my_display_preset
                );
                assert_eq!(
                    question.text(),
                    "Turning on AI Picture Mode will turn off AI Scene Optimization. Continue?"
                );
            }
            d => panic!("{d:?}"),
        }
    }

    #[test]
    fn my_display_preset_asks_before_disabling_others() {
        let s = FeatureState {
            ai_picture_mode: true,
            ..Default::default()
        };
        assert!(matches!(
            decide(s, Feature::MyDisplayPreset, true),
            Decision::Confirm { .. }
        ));
    }

    #[test]
    fn ai_scene_optimization_informs_instead_of_asking() {
        let s = FeatureState {
            my_display_preset: true,
            ..Default::default()
        };
        match decide(s, Feature::AiSceneOptimization, true) {
            Decision::Apply {
                next,
                notice: Some(n),
            } => {
                assert!(next.ai_scene_optimization && !next.my_display_preset);
                assert_eq!(
                    n.text(),
                    "AI Scene Optimization is on, so My Display Preset turned off."
                );
            }
            d => panic!("{d:?}"),
        }
    }

    #[test]
    fn pbp_blocks_turning_on_and_turns_everything_off() {
        let s = FeatureState {
            pbp: true,
            ..Default::default()
        };
        for f in Feature::ALL {
            assert!(matches!(decide(s, f, true), Decision::Blocked { .. }));
        }
        let (next, notice) = on_pbp_changed(
            FeatureState {
                ai_picture_mode: true,
                ..Default::default()
            },
            true,
        );
        assert_eq!(smart_count(&next), 0);
        assert_eq!(
            notice.unwrap().text(),
            "PBP/PIP is on, so AI Picture Mode turned off."
        );
    }

    #[test]
    fn every_unavailable_control_explains_itself() {
        for s in all_states() {
            let a = availability(s);
            for av in [
                a.ai_scene_optimization,
                a.ai_picture_mode,
                a.my_display_preset,
                a.ai_upscaling,
                a.picture_wizard,
            ] {
                if let Availability::Unavailable { reason } = av {
                    assert!(!reason.text().is_empty());
                }
            }
        }
    }

    #[test]
    fn picture_modes_keep_ai_consistent_from_every_state() {
        for s in all_states().filter(valid) {
            for allows in [true, false] {
                let next = match decide_picture_mode(s, "Calibration1", allows) {
                    Decision::Apply { next, .. } | Decision::Confirm { next, .. } => next,
                    Decision::Blocked { .. } => {
                        unreachable!("switching picture mode is never blocked")
                    }
                };
                assert!(valid(&next), "{s:?} allows={allows} -> {next:?}");
                assert_eq!(
                    next.my_display_preset, s.my_display_preset,
                    "My Display Preset is not affected"
                );
            }
        }
    }

    #[test]
    fn leaving_ai_for_a_calibration_mode_asks_first() {
        let s = FeatureState {
            ai_scene_optimization: true,
            ..Default::default()
        };
        match decide_picture_mode(s, "Calibration1", false) {
            Decision::Confirm { question, next } => {
                assert!(!next.ai_scene_optimization && next.picture_mode_blocks_ai);
                assert_eq!(
                    question.text(),
                    "Calibration1 doesn't work with AI Scene Optimization. Turn it off and switch to Calibration1?"
                );
            }
            d => panic!("{d:?}"),
        }
        assert!(matches!(
            decide_picture_mode(s, "Gamer 1", true),
            Decision::Apply { notice: None, .. }
        ));
    }

    #[test]
    fn ai_needs_an_ai_capable_picture_mode() {
        let s = FeatureState {
            picture_mode_blocks_ai: true,
            ..Default::default()
        };
        assert!(matches!(
            decide(s, Feature::AiSceneOptimization, true),
            Decision::Blocked { .. }
        ));
        assert!(matches!(
            decide(s, Feature::AiPictureMode, true),
            Decision::Blocked { .. }
        ));
        assert!(matches!(
            decide(s, Feature::MyDisplayPreset, true),
            Decision::Apply { .. }
        ));
        assert!(matches!(
            availability(s).ai_scene_optimization,
            Availability::Unavailable { .. }
        ));
    }
}
