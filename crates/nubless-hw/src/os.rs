//! Per-OS display enumeration: the same displays `ddc_hi::Display::enumerate`
//! finds, in the same order, each with what the OS knows about the monitor.

use ddc_hi::Display;

use crate::OsInfo;

/// Fill the EDID-derived fields from what ddc-hi already parsed.
#[cfg(not(windows))]
fn from_edid(display: &Display, os: OsInfo) -> OsInfo {
    let info = &display.info;
    OsInfo {
        serial_number: info.serial_number.clone(),
        serial: info.serial,
        name: info.model_name.clone(),
        ..os
    }
}

#[cfg(target_os = "linux")]
pub fn displays() -> Vec<(Display, OsInfo)> {
    use std::collections::HashMap;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    // ddc-hi names each display by the device number of its /dev/i2c-N.
    let buses: HashMap<String, String> = std::fs::read_dir("/dev")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let rdev = e.metadata().ok()?.rdev();
            name.starts_with("i2c-").then(|| (rdev.to_string(), name))
        })
        .collect();
    // The DRM connector (card1-DP-1) that owns an i2c bus: DisplayPort's AUX
    // bus sits under the connector, HDMI/DVI links theirs as `ddc`.
    let connector = |bus: &str| -> Option<String> {
        std::fs::read_dir("/sys/class/drm")
            .ok()?
            .flatten()
            .find(|e| {
                let dir = e.path();
                dir.join(bus).exists()
                    || std::fs::read_link(dir.join("ddc"))
                        .ok()
                        .is_some_and(|t| t.file_name() == Some(Path::new(bus).as_os_str()))
            })
            .and_then(|e| e.file_name().into_string().ok())
    };
    Display::enumerate()
        .into_iter()
        .map(|d| {
            let bus = buses.get(&d.info.id).cloned();
            let conn = bus.as_deref().and_then(connector);
            // Connector names survive reboots; i2c bus numbers may not.
            let port = match (&conn, &bus) {
                (Some(c), _) => format!("i2c-{c}"),
                (None, Some(b)) => format!("i2c-{b}"),
                (None, None) => format!("i2c-dev-{}", d.info.id),
            };
            let port_name = conn
                .as_deref()
                .map(|c| c.split_once('-').map_or(c, |(_, n)| n).to_string());
            let os = from_edid(
                &d,
                OsInfo {
                    port,
                    port_name,
                    ..OsInfo::default()
                },
            );
            (d, os)
        })
        .collect()
}

#[cfg(target_os = "macos")]
pub fn displays() -> Vec<(Display, OsInfo)> {
    Display::enumerate()
        .into_iter()
        .map(|d| {
            #[allow(unreachable_patterns)]
            let id = match &d.handle {
                ddc_hi::Handle::MacOS(m) => Some(m.handle().id),
                _ => None,
            };
            let mut os = from_edid(
                &d,
                OsInfo {
                    port: id.map_or_else(
                        || format!("macos-{}", d.info.id),
                        |id| format!("macos-{id}"),
                    ),
                    display_id: id,
                    ..OsInfo::default()
                },
            );
            // ddc-hi names a macOS display by the OS's own product name.
            os.name = os.name.or_else(|| Some(d.info.id.clone()));
            (d, os)
        })
        .collect()
}

/// dxva2 has no EDID, so the monitor behind each handle is looked up the way
/// Windows itself does: `HMONITOR` → `\\.\DISPLAYn` → the monitor's device
/// path (`\\?\DISPLAY#GSM5BBF#5&…&UID4352#{…}`) → its EDID in the registry.
/// The EDID gives the serial and name; it is *not* used to decide whether the
/// monitor is supported, which stays on the capabilities fingerprint as before.
#[cfg(windows)]
pub fn displays() -> Vec<(Display, OsInfo)> {
    use ddc_hi::{Backend, DisplayInfo, Handle};
    use std::{mem, ptr};
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::winerror::ERROR_SUCCESS;
    use winapi::um::wingdi::{DISPLAY_DEVICEW, DISPLAY_DEVICE_ACTIVE};
    use winapi::um::winreg::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_BINARY};
    use winapi::um::winuser::{EnumDisplayDevicesW, GetMonitorInfoW, MONITORINFOEXW};

    const EDD_GET_DEVICE_INTERFACE_NAME: DWORD = 1;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let text = |buf: &[u16]| {
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..n])
    };

    let Ok(hmonitors) = ddc_winapi::enumerate_monitors() else {
        return vec![];
    };
    let mut out = vec![];
    for hmon in hmonitors {
        let Ok(physical) = ddc_winapi::get_physical_monitors_from_hmonitor(hmon) else {
            continue;
        };
        // `\\.\DISPLAY2`: the output this HMONITOR is on.
        let mut mi: MONITORINFOEXW = unsafe { mem::zeroed() };
        mi.cbSize = mem::size_of::<MONITORINFOEXW>() as DWORD;
        let ok = unsafe { GetMonitorInfoW(hmon, &mut mi as *mut MONITORINFOEXW as *mut _) } != 0;
        let device = ok.then_some(mi.szDevice);
        let port_name = device.map(|d| text(&d).trim_start_matches(r"\\.\").to_string());
        // The monitors on that output, by device path.
        let mut paths = vec![];
        if let Some(dev) = &device {
            for i in 0.. {
                let mut dd: DISPLAY_DEVICEW = unsafe { mem::zeroed() };
                dd.cb = mem::size_of::<DISPLAY_DEVICEW>() as DWORD;
                if unsafe {
                    EnumDisplayDevicesW(dev.as_ptr(), i, &mut dd, EDD_GET_DEVICE_INTERFACE_NAME)
                } == 0
                {
                    break;
                }
                if dd.StateFlags & DISPLAY_DEVICE_ACTIVE != 0 {
                    paths.push(text(&dd.DeviceID));
                }
            }
        }
        // Mirrored monitors share one output, and nothing says which handle is
        // which path: only trust the pairing when there is exactly one of each.
        let path = (physical.len() == 1 && paths.len() == 1).then(|| paths.remove(0));
        // `DISPLAY#GSM5BBF#5&2d3b7b0e&0&UID4352`: model and connector, stable across reboots.
        let instance = path.as_deref().and_then(|p| {
            let mut parts = p.trim_start_matches(r"\\?\").split('#');
            Some([parts.next()?, parts.next()?, parts.next()?])
        });
        let edid = instance.and_then(|[class, model, inst]| {
            let key = wide(&format!(
                r"SYSTEM\CurrentControlSet\Enum\{class}\{model}\{inst}\Device Parameters"
            ));
            let value = wide("EDID");
            let mut buf = vec![0u8; 1024];
            let mut len = buf.len() as DWORD;
            let r = unsafe {
                RegGetValueW(
                    HKEY_LOCAL_MACHINE,
                    key.as_ptr(),
                    value.as_ptr(),
                    RRF_RT_REG_BINARY,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut _,
                    &mut len,
                )
            };
            buf.truncate(len as usize);
            (r == ERROR_SUCCESS as i32 && buf.len() >= 128).then_some(buf)
        });
        for (i, pm) in physical.into_iter().enumerate() {
            let monitor = unsafe { ddc_winapi::Monitor::new(pm) };
            let description = monitor.description();
            let parsed = edid
                .clone()
                .and_then(|e| DisplayInfo::from_edid(Backend::WinApi, description.clone(), e).ok());
            let port = match (&instance, &port_name) {
                (Some(parts), _) => format!("winapi-{}", parts.join("#")),
                (None, Some(d)) => format!("winapi-{d}-{i}"),
                (None, None) => format!("winapi-{description}-{}", out.len()),
            };
            let os = OsInfo {
                port,
                port_name: port_name.clone(),
                display_id: None,
                serial_number: parsed.as_ref().and_then(|p| p.serial_number.clone()),
                serial: parsed.as_ref().and_then(|p| p.serial),
                name: parsed.as_ref().and_then(|p| p.model_name.clone()),
            };
            // Matching keeps working exactly as before: no EDID in the info.
            let info = DisplayInfo::new(Backend::WinApi, description);
            out.push((Display::new(Handle::WinApi(monitor), info), os));
        }
    }
    out
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn displays() -> Vec<(Display, OsInfo)> {
    Display::enumerate()
        .into_iter()
        .map(|d| {
            let os = from_edid(
                &d,
                OsInfo {
                    port: format!("{}-{}", d.info.backend, d.info.id),
                    ..OsInfo::default()
                },
            );
            (d, os)
        })
        .collect()
}
