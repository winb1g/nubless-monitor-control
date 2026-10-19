//! Small OS-specific helpers.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Whether LG's own monitor service is running (it talks DDC/CI too and can
/// collide with us). Checked at most every 15 s.
pub fn lg_switch_running() -> bool {
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, v)) = *cache {
        if at.elapsed() < Duration::from_secs(15) {
            return v;
        }
    }
    let v = detect();
    *cache = Some((Instant::now(), v));
    v
}

#[cfg(windows)]
fn detect() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq SmartMonitorControlService.exe", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("SmartMonitorControlService"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn detect() -> bool {
    false
}
