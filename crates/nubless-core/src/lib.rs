//! Core of **nubless**: everything that decides *whether* and *how* a value may
//! be written to the monitor, with no hardware dependency so it can be tested
//! exhaustively.
//!
//! The two promises this crate exists to keep:
//!
//! 1. **Never brick the display.** Every write goes through [`safety::check_write`],
//!    which is the only way to construct a [`safety::ValidatedWrite`], which is the
//!    only thing a [`transport::Transport`] accepts. Factory resets, calibration/LUT
//!    channels, power and unknown codes cannot be written at all.
//! 2. **Never let the user get the settings into a mess.** Values stay inside safe
//!    ranges, feature interlocks ([`rules`]) keep the monitor in combinations its
//!    firmware expects, writes the monitor refuses are rolled back, and the user's
//!    original settings are captured on first connection ([`baseline`]) so "Reset
//!    to my original settings" always works.

pub mod baseline;
pub mod caps;
pub mod controller;
pub mod journal;
pub mod mock;
pub mod paths;
pub mod profile;
pub mod profiles;
pub mod rules;
pub mod safety;
pub mod transport;

pub use controller::{Controller, Timing};
pub use profile::{Confidence, Control, Profile, Risk, WritePolicy};
