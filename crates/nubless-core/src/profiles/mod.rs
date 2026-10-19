//! Built-in model profiles. Scope is deliberately one model for now.

pub mod gm950b;

use crate::profile::Profile;

pub fn all() -> &'static [&'static Profile] {
    &[&gm950b::PROFILE]
}
