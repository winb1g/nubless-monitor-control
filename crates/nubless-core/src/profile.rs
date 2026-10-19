//! Model profiles: which VCP codes a monitor has, what their values mean, and
//! how sure we are. A control is only writable when the evidence is good enough
//! for the active [`WritePolicy`].

use serde::Serialize;

/// How well a control's mapping is established. Ordered from weakest to strongest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// No lead on the code or the encoding.
    Unknown,
    /// Plausible from names or value shape; not matched to LG's UI.
    Candidate,
    /// LG's own opcode/enum name *and* a live read-back both match LG's UI.
    Strong,
    /// The live read-back matched LG's UI for this control; never written.
    ReadConfirmed,
    /// Written and read back on real hardware, every value in the table.
    WriteConfirmed,
}

/// What a write may do to the user's ability to see and use the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    /// Cannot make the screen unusable.
    None,
    /// Can make the picture very dark, washed out or tinted: needs
    /// confirm-or-revert so the user is never stuck looking at a black screen.
    Visibility,
    /// The panel may blank briefly while it resyncs (VRR, response mode):
    /// confirm-or-revert as well.
    Resync,
    /// Could cut the DDC/CI link itself (input switch, power): the app might
    /// not be able to undo it. Never writable until proven recoverable.
    DeadEnd,
}

impl Risk {
    pub fn describe(self) -> &'static str {
        match self {
            Risk::None => "safe",
            Risk::Visibility => "can make the picture hard to see: confirm or it reverts",
            Risk::Resync => "screen may flicker while it resyncs: confirm or it reverts",
            Risk::DeadEnd => "could cut the connection: never changed by nubless",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub raw: u16,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ValueKind {
    /// Inclusive range shown as a slider.
    Range { min: u16, max: u16 },
    /// A fixed list shown as a dropdown or tiles.
    Choice { choices: &'static [Choice] },
    /// Read-only information (firmware level, usage time, bitmasks).
    Info,
    /// One sub-command of LG's multiplexed AI channel (VCP 0xA6). The value sent
    /// is `(sub << 8) | arg`; reading needs a query write of `(sub << 8) | 0xFF`
    /// first, exactly as LG Switch does. `choices` lists the allowed `arg`s.
    SubCommand { sub: u8, choices: &'static [Choice] },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Control {
    /// Stable id used by the CLI, config files and the UI (`brightness`).
    pub id: &'static str,
    /// Label as the user knows it from the monitor/LG's panel.
    pub label: &'static str,
    pub vcp: u8,
    pub kind: ValueKind,
    pub confidence: Confidence,
    /// A standard VESA MCCS code with standard semantics.
    pub standard: bool,
    /// Writing is ever allowed for this control (subject to confidence/policy).
    pub writable: bool,
    pub risk: Risk,
    /// Evidence and open questions, shown in `nubless controls`.
    pub note: &'static str,
}

impl Control {
    pub fn choice_label(&self, raw: u16) -> Option<&'static str> {
        match self.kind {
            ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => {
                choices.iter().find(|c| c.raw == raw).map(|c| c.label)
            }
            _ => None,
        }
    }

    pub fn value_is_valid(&self, raw: u16) -> bool {
        match self.kind {
            ValueKind::Range { min, max } => (min..=max).contains(&raw),
            ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => {
                choices.iter().any(|c| c.raw == raw)
            }
            ValueKind::Info => false,
        }
    }

    /// Whether the change needs the user to confirm it before a timeout, or it reverts.
    pub fn needs_confirm_or_revert(&self) -> bool {
        matches!(self.risk, Risk::Visibility | Risk::Resync)
    }
}

/// Which level of evidence the user accepts for writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritePolicy {
    /// Default. Standard MCCS codes need `ReadConfirmed`; LG-private codes need `WriteConfirmed`.
    #[default]
    Safe,
    /// Opt-in for people helping map the monitor: `Strong` is enough. Still never
    /// `Candidate`/`Unknown`, never the deny-list, and confirm-or-revert still applies.
    Experimental,
}

impl WritePolicy {
    pub fn allows(self, c: &Control) -> bool {
        if !c.writable || c.risk == Risk::DeadEnd {
            return false;
        }
        match c.confidence {
            Confidence::WriteConfirmed => true,
            Confidence::ReadConfirmed => c.standard || self == WritePolicy::Experimental,
            Confidence::Strong => self == WritePolicy::Experimental,
            Confidence::Candidate | Confidence::Unknown => false,
        }
    }
}

/// How the app recognises the monitor.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Identity {
    /// EDID manufacturer id (`GSM` for LG).
    pub edid_manufacturer: &'static str,
    /// EDID product codes seen for this model.
    pub edid_product_codes: &'static [u16],
    /// EDID monitor name descriptor.
    pub edid_names: &'static [&'static str],
    /// The `model(...)` value in the capabilities string, which may be wrong
    /// (the 27GM950B says `WK95U`), so it is used only together with `vcp_fingerprint`.
    pub caps_model: &'static str,
    /// VCP codes that must all be declared for a capabilities-only match.
    pub vcp_fingerprint: &'static [u8],
}

/// How confidently a connected display was recognised as a profile's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Match {
    /// EDID manufacturer + product code or name agree.
    Edid,
    /// No EDID available (Windows `dxva2`), but the capabilities string has the
    /// expected `model(...)` and every fingerprint code.
    Fingerprint,
    No,
}

impl Identity {
    pub fn matches(
        &self,
        edid_manufacturer: Option<&str>,
        edid_product: Option<u16>,
        edid_name: Option<&str>,
        caps: Option<&crate::caps::Capabilities>,
    ) -> Match {
        if edid_manufacturer == Some(self.edid_manufacturer)
            && (edid_product.is_some_and(|p| self.edid_product_codes.contains(&p))
                || edid_name.is_some_and(|n| {
                    self.edid_names
                        .iter()
                        .any(|x| x.eq_ignore_ascii_case(n.trim()))
                }))
        {
            return Match::Edid;
        }
        // An EDID that names another monitor wins over a fingerprint.
        if edid_manufacturer.is_some() || edid_product.is_some() {
            return Match::No;
        }
        match caps {
            Some(c)
                if c.model.as_deref() == Some(self.caps_model)
                    && self.vcp_fingerprint.iter().all(|code| c.declares(*code)) =>
            {
                Match::Fingerprint
            }
            _ => Match::No,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Profile {
    pub model: &'static str,
    pub identity: Identity,
    pub controls: &'static [Control],
    /// Control whose value scopes the others: each picture mode keeps its own
    /// brightness, contrast and so on, so a snapshot only applies in its mode.
    pub mode_control: Option<&'static str>,
}

impl Profile {
    pub fn control(&self, id: &str) -> Option<&'static Control> {
        self.controls.iter().find(|c| c.id.eq_ignore_ascii_case(id))
    }
}
