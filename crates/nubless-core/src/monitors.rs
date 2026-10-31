//! Which folder holds one monitor's original settings and undo history.
//!
//! The folder must follow the *physical* monitor: three identical monitors must
//! never share one, and a monitor must find its own again after a reboot, a
//! cable swap, or a VRR switch that makes the OS re-detect it. A monitor is
//! known by two things:
//!
//! - its **serial** from the EDID, which follows the monitor anywhere, but which
//!   some monitors don't report and some OS paths can't read every time; and
//! - its **port**, where it is plugged in, as the OS names it. Always present,
//!   but it changes if the cable moves.
//!
//! The folder is named after the serial when there is one, else after the port,
//! and each folder records both (`identity.json`), so a monitor whose serial
//! could not be read this time still finds the folder it used on that port, and
//! a folder made without a serial is taken over once the serial shows up.
//! When none of that can tell two connected monitors apart, [`Folders::claim`]
//! refuses, and the caller pauses changes rather than guess.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What the OS told us about one monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// `sn-405NTAB1C234` (the EDID serial string) or `serial-123456` (the
    /// EDID serial number), when the monitor reports a real one.
    pub serial: Option<String>,
    /// Where it is plugged in, e.g. `i2c-card1-DP-1` or `winapi-DISPLAY#GSM5BBF#…`.
    pub port: String,
}

impl Identity {
    pub fn new(serial_number: Option<&str>, serial: Option<u32>, port: impl Into<String>) -> Self {
        // Some monitors fill the serial with zeros or spaces instead of leaving it out.
        let real = |s: &&str| !s.trim().is_empty() && s.trim().chars().any(|c| c != '0');
        let serial = match (serial_number.filter(real), serial) {
            (Some(s), _) => Some(format!("sn-{}", s.trim())),
            (None, Some(n)) if n != 0 => Some(format!("serial-{n}")),
            _ => None,
        };
        Identity {
            serial,
            port: port.into(),
        }
    }

    /// A short name for messages and `nubless list`.
    pub fn key(&self) -> &str {
        self.serial.as_deref().unwrap_or(&self.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FolderError {
    #[error("this monitor can't be told apart from another connected one ({0}), so nubless won't change it in case it mixes up their original settings")]
    Ambiguous(String),
    #[error("couldn't open this monitor's settings folder: {0}")]
    Io(#[from] io::Error),
}

/// Hands out folders to the monitors found in one scan, never the same one twice.
pub struct Folders {
    root: PathBuf,
    taken: BTreeSet<PathBuf>,
}

const IDENTITY_FILE: &str = "identity.json";

impl Folders {
    /// Folders under [`crate::paths::monitors_dir`].
    pub fn new() -> Self {
        Self::in_dir(crate::paths::monitors_dir())
    }

    pub fn in_dir(root: impl Into<PathBuf>) -> Self {
        Folders {
            root: root.into(),
            taken: BTreeSet::new(),
        }
    }

    /// The folder for `id`, created if needed, with its `identity.json` updated.
    pub fn claim(&mut self, id: &Identity) -> Result<PathBuf, FolderError> {
        let dir = self.find(id)?;
        if !self.taken.insert(dir.clone()) {
            return Err(FolderError::Ambiguous(id.key().to_string()));
        }
        fs::create_dir_all(&dir)?;
        let stored = read_identity(&dir);
        let record = Identity {
            // Never forget a serial that was seen before.
            serial: id.serial.clone().or(stored.and_then(|s| s.serial)),
            port: id.port.clone(),
        };
        let tmp = dir.join("identity.tmp");
        fs::write(
            &tmp,
            serde_json::to_string_pretty(&record).map_err(io::Error::from)?,
        )?;
        fs::rename(&tmp, dir.join(IDENTITY_FILE))?;
        Ok(dir)
    }

    fn find(&self, id: &Identity) -> io::Result<PathBuf> {
        let by_port = self.root.join(safe(&id.port));
        if let Some(serial) = &id.serial {
            let by_serial = self.root.join(safe(serial));
            if by_serial.is_dir() {
                return Ok(by_serial);
            }
            // Made on this port before the serial could be read: it's this monitor's.
            let unclaimed = read_identity(&by_port).is_some_and(|s| s.serial.is_none());
            if unclaimed && !self.taken.contains(&by_port) {
                fs::rename(&by_port, &by_serial)?;
            }
            return Ok(by_serial);
        }
        if by_port.is_dir() {
            return Ok(by_port);
        }
        // No serial this time: the folder of whichever monitor was last seen on
        // this port, if exactly one was.
        let mut seen_here = self.folders().filter(|(dir, stored)| {
            stored.port == id.port && stored.serial.is_some() && !self.taken.contains(dir)
        });
        match (seen_here.next(), seen_here.next()) {
            (Some((dir, _)), None) => Ok(dir),
            _ => Ok(by_port),
        }
    }

    fn folders(&self) -> impl Iterator<Item = (PathBuf, Identity)> {
        fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let dir = e.path();
                read_identity(&dir).map(|id| (dir, id))
            })
    }
}

impl Default for Folders {
    fn default() -> Self {
        Self::new()
    }
}

fn read_identity(dir: &Path) -> Option<Identity> {
    let text = fs::read_to_string(dir.join(IDENTITY_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// A folder name made only of characters every OS accepts.
fn safe(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn baseline_file(folder: &Path) -> PathBuf {
    folder.join("baseline.json")
}

pub fn journal_file(folder: &Path) -> PathBuf {
    folder.join("journal.jsonl")
}

/// Versions before multi-monitor support kept a single baseline and journal,
/// for the first supported monitor they found. Hand those files to that
/// monitor's `folder`, unless it already has its own. Moving rather than
/// copying means a second monitor can never inherit the first one's originals.
pub fn adopt_single_monitor_files(folder: &Path) -> io::Result<()> {
    for (old, new) in [
        (crate::paths::baseline_file(), baseline_file(folder)),
        (crate::paths::journal_file(), journal_file(folder)),
    ] {
        if old.exists() && !new.exists() {
            fs::create_dir_all(folder)?;
            fs::rename(&old, &new)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nubless-folders-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn with(serial: Option<&str>, port: &str) -> Identity {
        Identity::new(serial, None, port)
    }

    /// Put a marker in a folder, as if the monitor had saved its originals there.
    fn mark(dir: &Path, text: &str) {
        fs::write(baseline_file(dir), text).unwrap();
    }

    fn marker(dir: &Path) -> String {
        fs::read_to_string(baseline_file(dir)).unwrap_or_default()
    }

    #[test]
    fn serials_that_are_zeros_or_blank_dont_count() {
        assert_eq!(Identity::new(Some("000000"), Some(0), "p").serial, None);
        assert_eq!(
            Identity::new(Some("  "), Some(42), "p").serial.as_deref(),
            Some("serial-42")
        );
        assert_eq!(
            Identity::new(Some(" 405NT "), Some(42), "p")
                .serial
                .as_deref(),
            Some("sn-405NT")
        );
    }

    #[test]
    fn a_monitor_keeps_its_folder_when_it_moves_ports() {
        let r = root("moves");
        let a = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "DP-1"))
            .unwrap();
        mark(&a, "originals of A");
        let again = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "HDMI-1"))
            .unwrap();
        assert_eq!(a, again);
        assert_eq!(marker(&again), "originals of A");
    }

    #[test]
    fn identical_monitors_get_different_folders() {
        let r = root("identical");
        let mut f = Folders::in_dir(&r);
        let a = f.claim(&with(Some("A1"), "DP-1")).unwrap();
        let b = f.claim(&with(Some("B2"), "DP-2")).unwrap();
        let c = f.claim(&with(None, "DP-3")).unwrap();
        assert!(a != b && b != c && a != c);
    }

    #[test]
    fn a_serial_read_that_fails_once_still_finds_the_folder_on_that_port() {
        let r = root("flaky-serial");
        let a = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "DP-1"))
            .unwrap();
        mark(&a, "originals of A");
        let degraded = Folders::in_dir(&r).claim(&with(None, "DP-1")).unwrap();
        assert_eq!(degraded, a, "same port, same monitor");
        // The serial is not forgotten by the degraded visit.
        let back = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "DP-1"))
            .unwrap();
        assert_eq!(back, a);
        assert_eq!(read_identity(&a).unwrap().serial.as_deref(), Some("sn-A1"));
    }

    #[test]
    fn a_folder_made_without_a_serial_is_taken_over_when_the_serial_appears() {
        let r = root("adopt");
        let p = Folders::in_dir(&r).claim(&with(None, "DP-1")).unwrap();
        mark(&p, "originals");
        let s = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "DP-1"))
            .unwrap();
        assert_ne!(p, s);
        assert_eq!(marker(&s), "originals", "moved, not started over");
        assert!(!p.exists());
    }

    #[test]
    fn a_serial_folder_on_another_port_is_never_taken_over() {
        let r = root("no-steal");
        let a = Folders::in_dir(&r)
            .claim(&with(Some("A1"), "DP-1"))
            .unwrap();
        mark(&a, "originals of A");
        // Monitor B, no serial, on another port: it must not get A's originals.
        let b = Folders::in_dir(&r).claim(&with(None, "DP-2")).unwrap();
        assert_ne!(a, b);
        assert_eq!(marker(&b), "");
    }

    #[test]
    fn two_monitors_that_look_the_same_are_refused_not_merged() {
        let r = root("ambiguous");
        let mut f = Folders::in_dir(&r);
        f.claim(&with(Some("SAME"), "DP-1")).unwrap();
        let err = f.claim(&with(Some("SAME"), "DP-2")).unwrap_err();
        assert!(matches!(err, FolderError::Ambiguous(_)), "{err}");
    }

    #[test]
    fn a_port_folder_already_claimed_is_not_moved_under_another_monitor() {
        let r = root("claimed-port");
        Folders::in_dir(&r).claim(&with(None, "DP-1")).unwrap();
        let mut f = Folders::in_dir(&r);
        let first = f.claim(&with(None, "DP-1")).unwrap();
        // Same port reported again in this scan, now with a serial: no rename under the first.
        let second = f.claim(&with(Some("X"), "DP-1")).unwrap();
        assert!(first.exists() && first != second);
    }

    #[test]
    fn folder_names_are_safe_on_every_os() {
        let r = root("safe");
        let dir = Folders::in_dir(&r)
            .claim(&with(
                None,
                "winapi-\\\\?\\DISPLAY#GSM5BBF#5&1&0&UID4352#{e6f0}",
            ))
            .unwrap();
        let name = dir.file_name().unwrap().to_str().unwrap();
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)),
            "{name}"
        );
    }
}
