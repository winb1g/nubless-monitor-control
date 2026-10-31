//! LG UltraGear AI **27GM950B**.
//!
//! Evidence: `docs/research/04-ddc-vcp-27gm950b.md`, `05-screen-settings-panel.md`
//! and `08-lg-opcode-and-enum-tables.md`. Raise a control's [`Confidence`] only
//! with a recorded test card (see `docs/mapping-guide.md`).

use crate::profile::{Choice, Confidence::*, Control, Identity, Profile, Risk, ValueKind};

/// Capabilities string read from a real 27GM950B (firmware 3.05), whitespace as returned.
pub const CAPABILITIES_SAMPLE: &str = "(prot(monitor)type(lcd)model(WK95U)cmds(01 02 03 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B) 16 18 1A 52 60(11 12 0F 10) AC AE B2 B6 C0 C6 C8 C9 D6(01 04) DF 62 8D F4 F5(01 02 03 04) F6(00 01 02) 4D 4E 4F 15(01 06 11 13 14 15 18 19 20 22 23 24 28 29 32 48) F7(00 01 02 03) F8(00 01) F9 E4 E5 E6 E7 E8 E9 EA EB EF FA(00 01) FD(00 01) FE(00 01 02) FF)mccs_ver(2.1)mswhql(1))";

const RESPONSE_TIME: &[Choice] = &[
    Choice {
        raw: 0,
        label: "Off",
    },
    Choice {
        raw: 3,
        label: "Normal",
    },
    Choice {
        raw: 2,
        label: "Fast",
    },
    Choice {
        raw: 1,
        label: "Faster",
    },
];

const ADAPTIVE_SYNC: &[Choice] = &[
    Choice {
        raw: 0,
        label: "Off",
    },
    Choice {
        raw: 2,
        label: "Basic",
    },
    Choice {
        raw: 3,
        label: "Extended",
    },
];

/// Tiles LG Switch shows, with the values VCP 0x15 uses on the wire. These are
/// the monitor's own numbers, not LG's `E_PICTUREMODE_NAME` enum (LG Switch's
/// native service translates). All eight were read on a real unit (fw 3.05)
/// while switching tiles in LG Switch (docs/research/09).
const PICTURE_MODES: &[Choice] = &[
    Choice {
        raw: 65,
        label: "Personalized Picture",
    },
    Choice {
        raw: 45,
        label: "Gamer 1",
    },
    Choice {
        raw: 46,
        label: "Gamer 2",
    },
    Choice {
        raw: 18,
        label: "Calibration1",
    },
    Choice {
        raw: 19,
        label: "Calibration2",
    },
    Choice {
        raw: 6,
        label: "Color Weakness",
    },
    Choice {
        raw: 25,
        label: "DCI-P3",
    },
    Choice {
        raw: 30,
        label: "FPS",
    },
];

/// Picture modes in which AI Scene Optimization and AI Picture Mode work (LG
/// Switch's `PictureModeToAISupportedMap`: Personalized Picture, Custom, Gamer 1,
/// Gamer 2, Max Brightness, FPS, RTS, Vivid). In other modes LG Switch turns AI
/// off and the monitor refuses to switch while AI is on.
pub const AI_CAPABLE_PICTURE_MODES: &[&str] =
    &["Personalized Picture", "Gamer 1", "Gamer 2", "FPS"];

const OFF_ON: &[Choice] = &[
    Choice {
        raw: 1,
        label: "Off",
    },
    Choice {
        raw: 2,
        label: "On",
    },
];

/// Motion Blur Reduction (0xFA). Reads 1 with it On and 2 with it Off or hidden.
const MOTION_BLUR: &[Choice] = &[
    Choice {
        raw: 1,
        label: "On",
    },
    Choice {
        raw: 2,
        label: "Off",
    },
];

/// AI Upscaling. LG's enum has three values (1, 2, 4) and the OSD offers three
/// options (Off, Low, High); a real unit read 1 while LG Switch showed Off, so
/// 1 = Off, and LG's "Middle" (2) is the OSD's Low. Unconfirmed: read-only.
const UPSCALING: &[Choice] = &[
    Choice {
        raw: 1,
        label: "Off",
    },
    Choice {
        raw: 2,
        label: "Low",
    },
    Choice {
        raw: 4,
        label: "High",
    },
];

const INPUTS: &[Choice] = &[
    Choice {
        raw: 0x0F,
        label: "DisplayPort",
    },
    Choice {
        raw: 0x10,
        label: "USB-C",
    },
    Choice {
        raw: 0x11,
        label: "HDMI 1",
    },
    Choice {
        raw: 0x12,
        label: "HDMI 2",
    },
];

const MUTE: &[Choice] = &[
    Choice {
        raw: 1,
        label: "Muted",
    },
    Choice {
        raw: 2,
        label: "Unmuted",
    },
];

const ASPECT: &[Choice] = &[Choice {
    raw: 1,
    label: "Full Wide",
}];

const fn range(min: u16, max: u16) -> ValueKind {
    ValueKind::Range { min, max }
}

pub const CONTROLS: &[Control] = &[
    // ---- Picture ----------------------------------------------------------
    Control {
        id: "brightness", label: "Brightness", vcp: 0x10, kind: range(0, 100),
        confidence: ReadConfirmed, standard: true, writable: true, risk: Risk::None,
        note: "Standard MCCS luminance. Read-back 80 matched LG Switch. Write round-trip 80 -> 79 -> 80 verified in the OSD (fw 3.05, 2026-10-18).",
    },
    Control {
        id: "contrast", label: "Contrast", vcp: 0x12, kind: range(20, 100),
        confidence: WriteConfirmed, standard: true, writable: true, risk: Risk::Visibility,
        note: "Standard MCCS contrast. Read-back 70 matched LG Switch. Writes, Keep, Revert and the 5 s timeout agreed with the OSD (fw 3.05, 2026-10-30). Safe range starts at 20 so the picture can never be dimmed into unreadability; lower original values are still restored.",
    },
    Control {
        id: "black-stabilizer", label: "Black Stabilizer", vcp: 0xF9, kind: range(0, 100),
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::Visibility,
        note: "LG MANUFACTURER_BLACK_STABILIZER. Read-back 50 matched. Monitor reports max 255; the OSD slider is 0-100 (checked by hand 2026-10-20), so the UI range is 0-100. Writes agreed with the OSD, with one prompt per drag (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "response-time", label: "Response Time", vcp: 0xF7, kind: ValueKind::Choice { choices: RESPONSE_TIME },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::Resync,
        note: "LG enum OFF=0, HIGH=1, MIDDLE=2, LOW=3; the capabilities string declares F7(00 01 02 03) and the OSD offers Off, Normal, Fast, Faster. Reads 2 while UI shows Fast. Every option written from the app and agreed with the OSD, including Keep, Revert and the 5 s timeout (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "adaptive-sync", label: "Adaptive-Sync/VRR", vcp: 0xF8, kind: ValueKind::Choice { choices: ADAPTIVE_SYNC },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::Resync,
        note: "LG enum OFF=0, ON=1, BASIC=2, EXTENDED=3; reads 3 = Extended. Capabilities only declare 00 01, so declared sub-values are not trusted for this code. Off = 0, Basic = 2 and Extended = 3 each read back with the OSD showing the same option, and writes agreed with the OSD (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "picture-mode", label: "Picture Mode", vcp: 0x15, kind: ValueKind::Choice { choices: PICTURE_MODES },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::Visibility,
        note: "All eight tiles read on the wire while switching in LG Switch. Switching from nubless verified on the OSD, including Calibration1 through the AI interlock (fw 3.05, 2026-10-18). Non-AI modes need AI Scene Optimization off.",
    },
    Control {
        id: "color-temperature", label: "Color temperature", vcp: 0x69, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "Read-only. Preset selector is 0x14: Warm 3, Medium 5, Cool 9, Custom 11, Manual 65502 (0xFFDE), each read with the OSD on that option (fw 3.05, 2026-10-30). In Manual, 0x69 is the slider in kelvin: W8 4000, 0 6500, C1 6800, C10 10000 (uneven steps; the other notches are not mapped). In the presets 0x69 reports the preset (Warm 6500, Medium 7500, Cool 9300). Writes untested.",
    },
    Control {
        id: "red-gain", label: "Red", vcp: 0x16, kind: range(0, 100),
        confidence: Candidate, standard: true, writable: false, risk: Risk::Visibility,
        note: "Standard video gain. Choosing Custom unlocks the three sliders, which all read 50 here and in 0x16/0x18/0x1A (fw 3.05, 2026-10-30). Moving one slider to see which code follows is still to do.",
    },
    Control {
        id: "green-gain", label: "Green", vcp: 0x18, kind: range(0, 100),
        confidence: Candidate, standard: true, writable: false, risk: Risk::Visibility,
        note: "As red-gain.",
    },
    Control {
        id: "blue-gain", label: "Blue", vcp: 0x1A, kind: range(0, 100),
        confidence: Candidate, standard: true, writable: false, risk: Risk::Visibility,
        note: "As red-gain.",
    },
    Control {
        id: "aspect-ratio", label: "Aspect Ratio", vcp: 0xF5, kind: ValueKind::Choice { choices: ASPECT },
        confidence: Candidate, standard: false, writable: false, risk: Risk::Resync,
        note: "LG RATIO_VALUE; reads 1 (the OSD's Full Wide, by default). The capabilities string declares F5(01 02 03 04) and the OSD offers Full Wide, Original and Just Scan, so 2-4 are not named yet.",
    },
    Control {
        id: "motion-blur", label: "Motion Blur Reduction", vcp: 0xFA, kind: ValueKind::Choice { choices: MOTION_BLUR },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::Visibility,
        note: "1 = On, 2 = Off; both written and matched on the OSD. Only available while Adaptive-Sync is Off: with Basic or Extended the monitor ignores the write and 0xFA reads 2, so the controller reports it as not applied (docs/rules.md). 0xF4 reads 7 while Adaptive-Sync is Off and 6 otherwise (probable availability flag). Whether 0 also writes Off is untested (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "hdr", label: "HDR", vcp: 0xE2, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "LG HDR_VALUE reads 1; scale unknown. Read-only: HDR is driven by the PC.",
    },
    // ---- LG AI channel (VCP 0xA6 sub-commands, decoded from LG Switch) ------
    Control {
        id: "ai-scene-optimization", label: "AI Scene Optimization", vcp: 0xA6,
        kind: ValueKind::SubCommand { sub: 2, choices: OFF_ON },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::None,
        note: "Sub-command 2 (eSoC_AIMODE_ONOFF): 1 = off, 2 = on. LG Switch's own startup query answered 0x0202 while its toggle showed on. Toggling from nubless verified on the OSD (fw 3.05, 2026-10-18). Only works in AI-capable picture modes.",
    },
    Control {
        id: "ai-upscaling", label: "AI Upscaling", vcp: 0xA6,
        kind: ValueKind::SubCommand { sub: 4, choices: UPSCALING },
        confidence: ReadConfirmed, standard: false, writable: false, risk: Risk::Visibility,
        note: "Sub-command 4 (eSoC_AIMODE_UPSCALLING). Read matched the OSD on all three options: Off 1, Low 2, High 4 (fw 3.05, 2026-10-30). Not written yet. Unavailable while PBP/PIP is on.",
    },
    Control {
        id: "ai-sound", label: "AI Sound", vcp: 0xA6,
        kind: ValueKind::SubCommand { sub: 6, choices: OFF_ON },
        confidence: WriteConfirmed, standard: false, writable: true, risk: Risk::None,
        note: "Sub-command 6 (eAISOUND_ONOFF): 1 = off, 2 = on. LG Switch's startup query answered 0x0601 while its toggle showed off. Toggling from nubless verified on the OSD (fw 3.05, 2026-10-18).",
    },
    // ---- Sound / input ----------------------------------------------------
    Control {
        id: "volume", label: "Volume", vcp: 0x62, kind: range(0, 100),
        confidence: WriteConfirmed, standard: true, writable: true, risk: Risk::None,
        note: "Standard speaker volume; reads 20. Writes agreed with the OSD (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "mute", label: "Mute", vcp: 0x8D, kind: ValueKind::Choice { choices: MUTE },
        confidence: WriteConfirmed, standard: true, writable: true, risk: Risk::None,
        note: "Standard audio mute, 1 muted / 2 unmuted; reads 2. Muting and unmuting stopped and restarted the sound and agreed with the OSD (fw 3.05, 2026-10-30).",
    },
    Control {
        id: "input", label: "Input", vcp: 0x60, kind: ValueKind::Choice { choices: INPUTS },
        confidence: Candidate, standard: true, writable: false, risk: Risk::DeadEnd,
        note: "Standard input select; reads 0x0F on DisplayPort. Switching away may cut DDC/CI so the app could not switch back: blocked until proven recoverable.",
    },
    // ---- Info -------------------------------------------------------------
    Control {
        id: "firmware", label: "Firmware", vcp: 0xC9, kind: ValueKind::Info,
        confidence: ReadConfirmed, standard: true, writable: false, risk: Risk::None,
        note: "0x0305 = 3.05; matched the US number in LG's firmware file name and LG Switch's update history.",
    },
    Control {
        id: "usage-hours", label: "Usage time (hours)", vcp: 0xC0, kind: ValueKind::Info,
        confidence: Candidate, standard: true, writable: false, risk: Risk::None,
        note: "Standard display usage time.",
    },
    Control {
        id: "dual-mode", label: "Switch Dual-Mode", vcp: 0xB1, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Resync,
        note: "LG MONITOR_DUAL_MODE reads 0x2200 (high byte 0x22 = switched off). Sub-command protocol unknown: never written.",
    },
];

pub const PROFILE: Profile = Profile {
    model: "27GM950B",
    identity: Identity {
        edid_manufacturer: "GSM",
        edid_product_codes: &[0x5CF8],
        edid_names: &["LG ULTRAGEAR+"],
        caps_model: "WK95U",
        vcp_fingerprint: &[
            0x10, 0x12, 0x15, 0x4D, 0x4E, 0x4F, 0xC9, 0xF7, 0xF8, 0xF9, 0xFE,
        ],
    },
    controls: CONTROLS,
    mode_control: Some("picture-mode"),
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::Capabilities;
    use crate::safety;
    use std::collections::HashSet;

    #[test]
    fn ids_and_codes_are_unique() {
        let ids: HashSet<_> = CONTROLS.iter().map(|c| c.id).collect();
        let codes: HashSet<_> = CONTROLS
            .iter()
            .map(|c| match c.kind {
                ValueKind::SubCommand { sub, .. } => (c.vcp, Some(sub)),
                _ => (c.vcp, None),
            })
            .collect();
        assert_eq!(ids.len(), CONTROLS.len());
        assert_eq!(
            codes.len(),
            CONTROLS.len(),
            "only 0xA6 sub-commands may share a code"
        );
    }

    #[test]
    fn no_writable_control_touches_the_deny_list() {
        for c in CONTROLS.iter().filter(|c| c.writable) {
            if let ValueKind::SubCommand { sub, .. } = c.kind {
                assert!(
                    safety::AI_SUBCOMMANDS.contains(&(sub, true)),
                    "{} sub-command {sub} not allowed",
                    c.id
                );
                continue;
            }
            assert!(
                safety::deny_reason(c.vcp).is_none(),
                "{} uses a denied code",
                c.id
            );
        }
    }

    #[test]
    fn every_writable_control_is_declared_by_the_monitor() {
        let caps = Capabilities::parse(CAPABILITIES_SAMPLE).unwrap();
        // 0xA6 sub-commands are answered but not declared; see safety::check_write.
        for c in CONTROLS
            .iter()
            .filter(|c| c.writable && !matches!(c.kind, ValueKind::SubCommand { .. }))
        {
            assert!(
                caps.declares(c.vcp),
                "{} (0x{:02X}) not declared",
                c.id,
                c.vcp
            );
        }
    }

    #[test]
    fn identifies_by_edid_or_fingerprint() {
        use crate::profile::Match;
        let id = &PROFILE.identity;
        let caps = Capabilities::parse(CAPABILITIES_SAMPLE).unwrap();
        assert_eq!(
            id.matches(Some("GSM"), Some(0x5CF8), None, None),
            Match::Edid
        );
        assert_eq!(
            id.matches(Some("GSM"), None, Some("LG ULTRAGEAR+ "), None),
            Match::Edid
        );
        assert_eq!(
            id.matches(None, None, None, Some(&caps)),
            Match::Fingerprint
        );
        // A real 34WK95U also says model(WK95U), but its EDID gives it away.
        assert_eq!(
            id.matches(Some("GSM"), Some(0x7701), Some("LG HDR WQHD"), Some(&caps)),
            Match::No
        );
        let mut other = caps.clone();
        other.vcp.remove(&0xF9);
        assert_eq!(id.matches(None, None, None, Some(&other)), Match::No);
    }
}

/// Guards on how much the profile claims. Raising a control's [`Confidence`] is a
/// claim about real hardware, so it has to be a deliberate, reviewed edit: change
/// `PINNED` below in the same commit, tick the item in `docs/verify-on-hardware.md`
/// and put the evidence stamp `(fw X.XX, YYYY-MM-DD)` in the control's note.
#[cfg(test)]
mod evidence_tests {
    use super::*;
    use crate::profile::{Confidence, Risk, WritePolicy};
    use std::collections::HashSet;

    const PINNED: &[(&str, Confidence)] = &[
        ("brightness", ReadConfirmed),
        ("contrast", WriteConfirmed),
        ("black-stabilizer", WriteConfirmed),
        ("response-time", WriteConfirmed),
        ("adaptive-sync", WriteConfirmed),
        ("picture-mode", WriteConfirmed),
        ("color-temperature", Candidate),
        ("red-gain", Candidate),
        ("green-gain", Candidate),
        ("blue-gain", Candidate),
        ("aspect-ratio", Candidate),
        ("motion-blur", WriteConfirmed),
        ("hdr", Candidate),
        ("ai-scene-optimization", WriteConfirmed),
        ("ai-upscaling", ReadConfirmed),
        ("ai-sound", WriteConfirmed),
        ("volume", WriteConfirmed),
        ("mute", WriteConfirmed),
        ("input", Candidate),
        ("firmware", ReadConfirmed),
        ("usage-hours", Candidate),
        ("dual-mode", Candidate),
    ];

    #[test]
    fn confidence_only_changes_on_purpose() {
        for c in CONTROLS {
            let pinned = PINNED.iter().find(|(id, _)| *id == c.id);
            assert_eq!(
                pinned.map(|p| p.1),
                Some(c.confidence),
                "'{}' changed confidence (or is new): update PINNED together with the \
                 hardware evidence",
                c.id
            );
        }
        assert_eq!(
            PINNED.len(),
            CONTROLS.len(),
            "PINNED lists a removed control"
        );
    }

    /// `(fw 3.05, 2026-10-18)`: firmware, then the day a person saw it on the OSD.
    fn has_evidence_stamp(note: &str) -> bool {
        note.split("(fw ").skip(1).any(|rest| {
            let Some((inside, _)) = rest.split_once(')') else {
                return false;
            };
            let Some((fw, date)) = inside.split_once(", ") else {
                return false;
            };
            let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
            let mut d = date.split('-');
            let (y, m, day) = (d.next(), d.next(), d.next());
            fw.split('.').count() == 2
                && fw
                    .split('.')
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                && matches!((y, m, day), (Some(y), Some(m), Some(day))
                    if digits(y, 4) && digits(m, 2) && digits(day, 2) && d.next().is_none())
        })
    }

    #[test]
    fn the_stamp_checker_is_strict() {
        assert!(has_evidence_stamp(
            "verified on the OSD (fw 3.05, 2026-10-18)."
        ));
        assert!(!has_evidence_stamp("verified on the OSD"));
        assert!(!has_evidence_stamp("(fw 3.05)"));
        assert!(!has_evidence_stamp("(fw 3.05, 18-10-2026)"));
        assert!(!has_evidence_stamp("(fw three, 2026-10-18)"));
    }

    #[test]
    fn write_confirmed_means_a_person_saw_it_on_the_osd() {
        for c in CONTROLS.iter().filter(|c| c.confidence == WriteConfirmed) {
            assert!(
                has_evidence_stamp(c.note),
                "'{}' is WriteConfirmed but its note has no `(fw X.XX, YYYY-MM-DD)` stamp",
                c.id
            );
        }
    }

    #[test]
    fn read_confirmed_means_it_matched_lgs_ui() {
        for c in CONTROLS.iter().filter(|c| c.confidence == ReadConfirmed) {
            assert!(
                c.note.contains("matched") || has_evidence_stamp(c.note),
                "'{}' is ReadConfirmed but its note does not say what it matched",
                c.id
            );
        }
    }

    #[test]
    fn labs_is_exactly_the_documented_set() {
        let mut labs: Vec<_> = CONTROLS
            .iter()
            .filter(|c| WritePolicy::Experimental.allows(c) && !WritePolicy::Safe.allows(c))
            .map(|c| c.id)
            .collect();
        labs.sort_unstable();
        // README.md, "What works today". Keep the two in step. Every control that
        // was in Labs passed its hardware card on 2026-10-30; the next candidate to
        // become writable goes here until its card passes.
        assert!(labs.is_empty(), "controls in Labs: {labs:?}");
    }

    #[test]
    fn policy_never_trusts_less_than_it_says() {
        for c in CONTROLS {
            if WritePolicy::Safe.allows(c) {
                assert!(
                    c.confidence == WriteConfirmed || (c.standard && c.confidence == ReadConfirmed),
                    "{} is writable by default on thin evidence",
                    c.id
                );
            }
            if matches!(c.confidence, Candidate | Confidence::Unknown) || c.risk == Risk::DeadEnd {
                assert!(
                    !WritePolicy::Experimental.allows(c),
                    "{} must never be writable",
                    c.id
                );
            }
        }
    }

    #[test]
    fn every_value_table_is_well_formed() {
        for c in CONTROLS {
            match c.kind {
                ValueKind::Range { min, max } => assert!(min < max, "{}", c.id),
                ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => {
                    assert!(!choices.is_empty(), "{}", c.id);
                    let raws: HashSet<_> = choices.iter().map(|x| x.raw).collect();
                    let labels: HashSet<_> = choices.iter().map(|x| x.label).collect();
                    assert_eq!(raws.len(), choices.len(), "{} repeats a value", c.id);
                    assert_eq!(labels.len(), choices.len(), "{} repeats a label", c.id);
                }
                ValueKind::Info => assert!(!c.writable, "{} is Info but writable", c.id),
            }
            assert!(!c.note.is_empty(), "{} has no evidence note", c.id);
        }
    }
}
