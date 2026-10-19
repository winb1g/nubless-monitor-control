//! Every write is recorded with the value it replaced, *before* it is sent, so a
//! crash mid-write still leaves enough to undo it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryState {
    /// Recorded, about to be sent.
    Pending,
    /// Sent and read back with the new value.
    Applied,
    /// The monitor did not take it; the old value was restored.
    RolledBack,
    /// Undone by the user or by confirm-or-revert.
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub unix_ms: u128,
    pub control: String,
    pub vcp: u8,
    pub old: u16,
    pub new: u16,
    pub state: EntryState,
}

#[derive(Debug, Default)]
pub struct Journal {
    pub entries: Vec<Entry>,
    path: Option<PathBuf>,
}

impl Journal {
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Load (or start) a journal persisted as JSON lines at `path`.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let entries = match fs::read_to_string(&path) {
            Ok(text) => text
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Journal {
            entries,
            path: Some(path),
        })
    }

    pub fn record(&mut self, control: &str, vcp: u8, old: u16, new: u16) -> usize {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        self.entries.push(Entry {
            unix_ms,
            control: control.into(),
            vcp,
            old,
            new,
            state: EntryState::Pending,
        });
        self.persist();
        self.entries.len() - 1
    }

    pub fn set_state(&mut self, index: usize, state: EntryState) {
        if let Some(e) = self.entries.get_mut(index) {
            e.state = state;
            self.persist();
        }
    }

    /// The most recent change that is still in effect (or may be, if pending).
    pub fn last_undoable(&self) -> Option<(usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| matches!(e.state, EntryState::Applied | EntryState::Pending))
    }

    fn persist(&self) {
        let Some(path) = &self.path else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let body: String = self
            .entries
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .map(|l| l + "\n")
            .collect();
        // Write-then-rename so a crash never leaves a half-written journal.
        let tmp = path.with_extension("tmp");
        if fs::write(&tmp, body).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }
}
