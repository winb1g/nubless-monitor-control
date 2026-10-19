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

/// Tiles LG Switch shows; raw values are `E_PICTUREMODE_NAME` indices found in the
/// supported-mode bitmasks. Whether VCP 0x15 takes these indices is NOT known yet.
const PICTURE_MODES: &[Choice] = &[
    Choice {
        raw: 60,
        label: "Personalized Picture",
    },
    Choice {
        raw: 39,
        label: "Gamer 1",
    },
    Choice {
        raw: 40,
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
        raw: 30,
        label: "Color Weakness",
    },
    Choice {
        raw: 25,
        label: "DCI-P3",
    },
    Choice {
        raw: 43,
        label: "FPS",
    },
];

const INPUTS: &[Choice] = &[
    Choice {
        raw: 0x0F,
        label: "DisplayPort",
    },
    Choice {
        raw: 0x10,
        label: "DisplayPort 2 / USB-C",
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

const ASPECT: &[Choice] = &[
    Choice {
        raw: 1,
        label: "16:9",
    },
    Choice {
        raw: 2,
        label: "21:9",
    },
];

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
        confidence: ReadConfirmed, standard: true, writable: true, risk: Risk::Visibility,
        note: "Standard MCCS contrast. Read-back 70 matched LG Switch. Safe range starts at 20 so the picture can never be dimmed into unreadability; lower original values are still restored.",
    },
    Control {
        id: "black-stabilizer", label: "Black Stabilizer", vcp: 0xF9, kind: range(0, 100),
        confidence: Strong, standard: false, writable: true, risk: Risk::Visibility,
        note: "LG MANUFACTURER_BLACK_STABILIZER. Read-back 50 matched. Monitor reports max 255; UI range assumed 0-100 until write-confirmed.",
    },
    Control {
        id: "response-time", label: "Response Time", vcp: 0xF7, kind: ValueKind::Choice { choices: RESPONSE_TIME },
        confidence: Strong, standard: false, writable: true, risk: Risk::Resync,
        note: "LG enum HIGH=1, MIDDLE=2, LOW=3; reads 2 while UI shows Fast. Label order inferred.",
    },
    Control {
        id: "adaptive-sync", label: "Adaptive-Sync/VRR", vcp: 0xF8, kind: ValueKind::Choice { choices: ADAPTIVE_SYNC },
        confidence: Strong, standard: false, writable: true, risk: Risk::Resync,
        note: "LG enum OFF=0, ON=1, BASIC=2, EXTENDED=3; reads 3 = Extended. Capabilities only declare 00 01, so declared sub-values are not trusted for this code.",
    },
    Control {
        id: "picture-mode", label: "Picture Mode", vcp: 0x15, kind: ValueKind::Choice { choices: PICTURE_MODES },
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "Supported list from bitmasks 0x4D/0x4E/0x4F matches all tiles. Current-value encoding unknown (reads 65 while UI shows Personalized Picture).",
    },
    Control {
        id: "color-temperature", label: "Color temperature", vcp: 0x69, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "LG COLOR_TEMP_VALUE reads 6500; list 0x6A = Warm, Medium, Cool, Custom, Manual. Encoding of the Manual slider unknown.",
    },
    Control {
        id: "red-gain", label: "Red", vcp: 0x16, kind: range(0, 100),
        confidence: Candidate, standard: true, writable: false, risk: Risk::Visibility,
        note: "Standard video gain; likely the Custom color sliders. Not matched to the UI yet.",
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
        note: "LG RATIO_VALUE reads 1 = 16:9; list 0xF4 = 16:9, 21:9.",
    },
    Control {
        id: "motion-blur", label: "Motion Blur Reduction", vcp: 0xFA, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "LG MOTION_BLUR reads 2; scale unknown.",
    },
    Control {
        id: "hdr", label: "HDR", vcp: 0xE2, kind: ValueKind::Info,
        confidence: Candidate, standard: false, writable: false, risk: Risk::Visibility,
        note: "LG HDR_VALUE reads 1; scale unknown. Read-only: HDR is driven by the PC.",
    },
    // ---- Sound / input ----------------------------------------------------
    Control {
        id: "volume", label: "Volume", vcp: 0x62, kind: range(0, 100),
        confidence: Strong, standard: true, writable: true, risk: Risk::None,
        note: "Standard speaker volume; reads 20. Not compared with the OSD yet.",
    },
    Control {
        id: "mute", label: "Mute", vcp: 0x8D, kind: ValueKind::Choice { choices: MUTE },
        confidence: Strong, standard: true, writable: true, risk: Risk::None,
        note: "Standard audio mute, 1 muted / 2 unmuted; reads 2.",
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
        note: "0x0305 = 3.05; equals the US number in LG's firmware file name.",
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
        let codes: HashSet<_> = CONTROLS.iter().map(|c| c.vcp).collect();
        assert_eq!(ids.len(), CONTROLS.len());
        assert_eq!(codes.len(), CONTROLS.len());
    }

    #[test]
    fn no_writable_control_touches_the_deny_list() {
        for c in CONTROLS.iter().filter(|c| c.writable) {
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
        for c in CONTROLS.iter().filter(|c| c.writable) {
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
