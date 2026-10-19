//! Where nubless keeps its files. Shared by the CLI and the desktop app so both
//! see the same original-settings baseline.

use std::path::PathBuf;

/// `NUBLESS_HOME`, else `%APPDATA%\nubless` (Windows),
/// `~/Library/Application Support/nubless` (macOS) or
/// `$XDG_STATE_HOME/nubless` / `~/.local/state/nubless` (Linux).
pub fn state_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("NUBLESS_HOME") {
        return PathBuf::from(p);
    }
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join("nubless")
}

pub fn baseline_file() -> PathBuf {
    state_dir().join("baseline.json")
}

pub fn journal_file() -> PathBuf {
    state_dir().join("journal.jsonl")
}
