//! The names the OS shows for displays, where only the OS knows them.
//!
//! macOS names two identical monitors "LG ULTRAGEAR+ (1)" and "(2)" in System
//! Settings; that numbering lives in AppKit (`NSScreen.localizedName`), which
//! may only be asked on the main thread. Elsewhere the EDID name read with the
//! display is already what the OS shows, so there is nothing to add.

/// `CGDirectDisplayID` → the name macOS shows. Main thread only.
#[cfg(target_os = "macos")]
pub fn names() -> Vec<(u32, String)> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2_foundation::{NSNumber, NSString};

    let Some(mtm) = MainThreadMarker::new() else {
        return vec![];
    };
    let key = NSString::from_str("NSScreenNumber");
    NSScreen::screens(mtm)
        .iter()
        .filter_map(|screen| {
            let number = screen.deviceDescription().objectForKey(&key)?;
            let id = number.downcast::<NSNumber>().ok()?.unsignedIntValue();
            Some((id, screen.localizedName().to_string()))
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn names() -> Vec<(u32, String)> {
    vec![]
}
