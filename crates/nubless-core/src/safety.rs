//! The write guard. [`check_write`] is the **only** constructor of
//! [`ValidatedWrite`], and [`crate::transport::Transport::set_vcp`] only accepts a
//! `ValidatedWrite`, so no code path can send a write that skipped these checks.

use crate::caps::Capabilities;
use crate::profile::{Control, Risk, ValueKind, WritePolicy};

/// Codes that are never written, whatever any profile, flag or user says.
/// Reasons are shown to the user verbatim.
pub fn deny_reason(code: u8) -> Option<&'static str> {
    Some(match code {
        0x01 => "degauss (CRT only)",
        0x04 | 0x05 | 0x06 | 0x08 | 0x0A => "factory restore: wipes the user's settings",
        0x0B | 0x0C => "color temperature increment/request: undocumented on this model",
        0x1E | 0x1F => "auto setup: unpredictable result",
        0x73..=0x76 => "LUT / remote procedure call: can corrupt the picture pipeline",
        0x78 => "EDID operation: can make the PC lose the monitor",
        0xA6 | 0xA7 => "LG AI mode channel: protocol unknown",
        0xB0 => "save/restore preset: can overwrite stored settings",
        0xB1 => "LG Dual-Mode sub-command channel: protocol unknown",
        0xC6 | 0xC7 => "enable keys",
        0xCA | 0xCC => "OSD lock/language: could lock the user out of the on-screen menu",
        0xD6 | 0xD8 => "power mode: can switch the screen off with no way back from the app",
        0xDB => "LG picture wizard sub-command channel: protocol unknown",
        0xE4..=0xEB => "LG calibration / gamma LUT / color matrix channel: can corrupt calibration",
        0xEF | 0xF0 => "LG display id / heartbeat",
        0xFD..=0xFF => "LG power LED / gamma preset / wizard option: not mapped",
        _ => return None,
    })
}

/// A write that passed every check. Cannot be built outside this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedWrite {
    code: u8,
    value: u16,
}

impl ValidatedWrite {
    pub fn code(&self) -> u8 {
        self.code
    }
    pub fn value(&self) -> u16 {
        self.value
    }
    /// Only for restoring a value the monitor itself reported (see `controller::check_restore`).
    pub(crate) fn with_value(self, value: u16) -> Self {
        ValidatedWrite { value, ..self }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SafetyError {
    #[error("VCP 0x{code:02X} is never written by nubless: {reason}")]
    Denied { code: u8, reason: &'static str },
    #[error("'{id}' is read-only")]
    ReadOnly { id: &'static str },
    #[error("'{id}' could cut the connection to the monitor, so nubless will not change it (use the monitor's own menu)")]
    DeadEnd { id: &'static str },
    #[error(
        "'{id}' is not verified enough to write ({confidence:?}); see `nubless controls`{hint}"
    )]
    NotConfident {
        id: &'static str,
        confidence: crate::profile::Confidence,
        hint: &'static str,
    },
    #[error("the monitor does not declare VCP 0x{code:02X} in its capabilities")]
    NotDeclared { code: u8 },
    #[error("{value} is not a valid value for '{id}' ({allowed})")]
    InvalidValue {
        id: &'static str,
        value: u16,
        allowed: String,
    },
}

/// LG's AI channel. Denied as a whole; only the sub-commands below get through.
pub const AI_CHANNEL: u8 = 0xA6;
/// Low byte that turns a 0xA6 write into "report this sub-command's value".
pub const SUB_QUERY: u8 = 0xFF;
/// 0xA6 sub-commands decoded from LG Switch (`AIMODE_DISPLAYSETTINGS_VALUES`):
/// `(sub-command, may set a value)`. 2 = AI Scene Optimization, 4 = AI Upscaling
/// (value encoding unconfirmed, so query only), 6 = AI Sound. Everything else on
/// 0xA6 (supported-option lists, PC AI mode, scene prediction) stays denied.
pub const AI_SUBCOMMANDS: &[(u8, bool)] = &[(2, true), (4, false), (6, true)];

fn sub_of(control: &Control) -> Option<u8> {
    match control.kind {
        ValueKind::SubCommand { sub, .. } => Some(sub),
        _ => None,
    }
}

fn sub_command_denied(control: &Control, sub: u8, for_set: bool) -> Option<SafetyError> {
    let ok = control.vcp == AI_CHANNEL
        && AI_SUBCOMMANDS
            .iter()
            .any(|&(s, settable)| s == sub && (settable || !for_set));
    (!ok).then_some(SafetyError::Denied {
        code: control.vcp,
        reason: "LG AI channel sub-command that nubless has not mapped",
    })
}

/// The query write LG Switch sends before reading a 0xA6 sub-command. It sets
/// nothing; it asks the monitor to report the current value. Still a write on
/// the wire, so it needs Labs (or a write-confirmed control).
pub fn check_query(
    control: &'static Control,
    policy: WritePolicy,
) -> Result<ValidatedWrite, SafetyError> {
    let Some(sub) = sub_of(control) else {
        return Err(SafetyError::ReadOnly { id: control.id });
    };
    if let Some(e) = sub_command_denied(control, sub, false) {
        return Err(e);
    }
    let allowed = control.confidence == crate::profile::Confidence::WriteConfirmed
        || (policy == WritePolicy::Experimental
            && control.confidence >= crate::profile::Confidence::Candidate);
    if !allowed {
        return Err(SafetyError::NotConfident {
            id: control.id,
            confidence: control.confidence,
            hint: " (reading it needs Labs / --experimental)",
        });
    }
    Ok(ValidatedWrite {
        code: AI_CHANNEL,
        value: (u16::from(sub) << 8) | u16::from(SUB_QUERY),
    })
}

/// Decide whether `value` may be written to `control` on a monitor with `caps`.
/// For a sub-command control, `value` is the argument; the sub-command byte is
/// added here.
pub fn check_write(
    control: &'static Control,
    value: u16,
    caps: &Capabilities,
    policy: WritePolicy,
) -> Result<ValidatedWrite, SafetyError> {
    let sub = sub_of(control);
    match sub {
        Some(sub) => {
            if let Some(e) = sub_command_denied(control, sub, true) {
                return Err(e);
            }
        }
        None => {
            if let Some(reason) = deny_reason(control.vcp) {
                return Err(SafetyError::Denied {
                    code: control.vcp,
                    reason,
                });
            }
        }
    }
    if control.risk == Risk::DeadEnd {
        return Err(SafetyError::DeadEnd { id: control.id });
    }
    if !control.writable || matches!(control.kind, ValueKind::Info) {
        return Err(SafetyError::ReadOnly { id: control.id });
    }
    if !policy.allows(control) {
        let hint = if WritePolicy::Experimental.allows(control) {
            " (allowed with --experimental)"
        } else {
            ""
        };
        return Err(SafetyError::NotConfident {
            id: control.id,
            confidence: control.confidence,
            hint,
        });
    }
    // The 27GM950B answers 0xA6 without declaring it. For sub-commands the
    // controller requires a successful query read before any write instead.
    if sub.is_none() && !caps.declares(control.vcp) {
        return Err(SafetyError::NotDeclared { code: control.vcp });
    }
    if !control.value_is_valid(value) || (sub.is_some() && value > 0xFE) {
        return Err(SafetyError::InvalidValue {
            id: control.id,
            value,
            allowed: describe_allowed(control),
        });
    }
    Ok(ValidatedWrite {
        code: control.vcp,
        value: sub.map_or(value, |s| (u16::from(s) << 8) | value),
    })
}

pub fn describe_allowed(control: &Control) -> String {
    match control.kind {
        ValueKind::Range { min, max } => format!("{min}..={max}"),
        ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => choices
            .iter()
            .map(|c| format!("{} ({})", c.label, c.raw))
            .collect::<Vec<_>>()
            .join(", "),
        ValueKind::Info => "read-only".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Confidence, Control};
    use crate::profiles::gm950b::{CAPABILITIES_SAMPLE, PROFILE};

    fn caps() -> Capabilities {
        Capabilities::parse(CAPABILITIES_SAMPLE).unwrap()
    }

    #[test]
    fn brightness_in_range_is_allowed() {
        let c = PROFILE.control("brightness").unwrap();
        let w = check_write(c, 55, &caps(), WritePolicy::Safe).unwrap();
        assert_eq!((w.code(), w.value()), (0x10, 55));
    }

    #[test]
    fn out_of_range_is_rejected() {
        let c = PROFILE.control("brightness").unwrap();
        assert!(matches!(
            check_write(c, 101, &caps(), WritePolicy::Safe),
            Err(SafetyError::InvalidValue { .. })
        ));
    }

    #[test]
    fn strong_controls_need_experimental() {
        let c = PROFILE.control("adaptive-sync").unwrap();
        assert!(matches!(
            check_write(c, 3, &caps(), WritePolicy::Safe),
            Err(SafetyError::NotConfident { .. })
        ));
        assert!(check_write(c, 3, &caps(), WritePolicy::Experimental).is_ok());
        assert!(
            check_write(c, 1, &caps(), WritePolicy::Experimental).is_err(),
            "1 (On) is not offered on this model"
        );
    }

    #[test]
    fn candidates_and_dead_ends_are_never_written() {
        for id in ["picture-mode", "input", "red-gain", "firmware", "dual-mode"] {
            let c = PROFILE.control(id).unwrap();
            for v in [0u16, 1, 2, 3, 0x0F, 18, 50] {
                assert!(
                    check_write(c, v, &caps(), WritePolicy::Experimental).is_err(),
                    "{id}={v}"
                );
            }
        }
    }

    #[test]
    fn deny_list_beats_any_profile() {
        static EVIL: Control = Control {
            id: "evil",
            label: "Factory reset",
            vcp: 0x04,
            kind: ValueKind::Range { min: 0, max: 255 },
            confidence: Confidence::WriteConfirmed,
            standard: true,
            writable: true,
            risk: crate::profile::Risk::None,
            note: "",
        };
        assert!(matches!(
            check_write(&EVIL, 1, &caps(), WritePolicy::Experimental),
            Err(SafetyError::Denied { .. })
        ));
    }

    #[test]
    fn every_denied_dangerous_code_stays_denied() {
        for code in [
            0x04, 0x05, 0x08, 0xD6, 0xE4, 0xE6, 0xE7, 0xEB, 0xB1, 0xDB, 0x78,
        ] {
            assert!(deny_reason(code).is_some(), "0x{code:02X}");
        }
    }

    #[test]
    fn undeclared_code_is_rejected() {
        let mut caps = caps();
        caps.vcp.remove(&0x10);
        let c = PROFILE.control("brightness").unwrap();
        assert_eq!(
            check_write(c, 50, &caps, WritePolicy::Safe),
            Err(SafetyError::NotDeclared { code: 0x10 })
        );
    }
}
