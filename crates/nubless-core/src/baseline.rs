//! The user's original settings, captured the first time nubless connects to a
//! monitor and **before any write is possible**. "Reset to my original
//! settings" restores it.
//!
//! The file is only ever added to, never overwritten: if a control could not be
//! read the first time, it is filled in on a later connection, but a value that
//! was already captured is never replaced by one nubless itself may have set.

use std::fs;
use std::io;
use std::path::Path;

use crate::controller::{Controller, Snapshot};
use crate::transport::Transport;

/// Load the baseline at `path`, creating or completing it from the monitor.
/// Returns the baseline and whether anything new was saved.
pub fn ensure<T: Transport>(path: &Path, ctl: &mut Controller<T>) -> io::Result<(Snapshot, bool)> {
    let model = ctl.profile().model;
    let existing = load(path)?.filter(|s| s.model == model);
    let current = ctl.snapshot();
    let (merged, changed) = match existing {
        None => {
            let changed = !current.values.is_empty();
            (current, changed)
        }
        Some(mut base) => {
            let mut changed = false;
            for (id, v) in current.values {
                if let std::collections::btree_map::Entry::Vacant(slot) = base.values.entry(id) {
                    slot.insert(v);
                    changed = true;
                }
            }
            (base, changed)
        }
    };
    if changed {
        save(path, &merged)?;
    }
    Ok((merged, changed))
}

pub fn load(path: &Path) -> io::Result<Option<Snapshot>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text).ok()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Write-then-rename so a crash never leaves a half-written baseline.
fn save(path: &Path, snap: &Snapshot) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_string_pretty(snap)?)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::mock::MockMonitor;
    use crate::profiles::gm950b::PROFILE;
    use crate::{Timing, WritePolicy};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nubless-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("baseline.json")
    }

    #[test]
    fn first_connection_saves_and_later_ones_never_overwrite() {
        let path = temp_path("ensure");
        let mut ctl = Controller::new(
            MockMonitor::gm950b(),
            &PROFILE,
            WritePolicy::Safe,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        let (base, saved) = ensure(&path, &mut ctl).unwrap();
        assert!(saved);
        assert_eq!(base.values["brightness"], 80);

        ctl.write("brightness", 30).unwrap();
        let (again, saved) = ensure(&path, &mut ctl).unwrap();
        assert!(!saved);
        assert_eq!(
            again.values["brightness"], 80,
            "the original must survive later changes"
        );
    }

    #[test]
    fn missing_values_are_filled_in_later() {
        let path = temp_path("fill");
        let mut m = MockMonitor::gm950b();
        m.values.remove(&0x62);
        let mut ctl = Controller::new(
            m,
            &PROFILE,
            WritePolicy::Safe,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        let (base, _) = ensure(&path, &mut ctl).unwrap();
        assert!(!base.values.contains_key("volume"));

        let mut ctl = Controller::new(
            MockMonitor::gm950b(),
            &PROFILE,
            WritePolicy::Safe,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        let (base, saved) = ensure(&path, &mut ctl).unwrap();
        assert!(saved);
        assert_eq!(base.values["volume"], 20);
    }

    #[test]
    fn reset_restores_values_below_the_safe_range() {
        let path = temp_path("reset");
        let mut m = MockMonitor::gm950b();
        m.values.get_mut(&0x12).unwrap().current = 5; // set with the OSD, below our 20 floor
        let mut ctl = Controller::new(
            m,
            &PROFILE,
            WritePolicy::Safe,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        let (base, _) = ensure(&path, &mut ctl).unwrap();
        ctl.write("contrast", 60).unwrap();
        assert!(ctl.restore(&base).iter().all(|(_, r)| r.is_ok()));
        assert_eq!(ctl.read("contrast").unwrap().current, 5);
    }
}
