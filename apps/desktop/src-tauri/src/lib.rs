mod hotkeys;
mod platform;
mod view;
mod worker;

use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Sender};

use nubless_core::rules::Feature;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::ShortcutState;

use hotkeys::{Action, HotkeysView, Registry};
use view::AppState;
use worker::{FeatureOutcome, Outcome, Req};

/// Passed by the login item, so nubless starts quietly in the tray.
const HIDDEN_ARG: &str = "--hidden";

type Backend = Sender<Req>;

/// Send a request to the monitor thread and wait for the answer off the main thread.
async fn ask<T: Send + 'static>(
    tx: &Backend,
    make: impl FnOnce(Sender<T>) -> Req,
) -> Result<T, String> {
    let (rtx, rrx) = channel();
    tx.send(make(rtx))
        .map_err(|_| "The monitor thread stopped.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || rrx.recv())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|_| "The monitor thread stopped.".to_string())
}

/// Keep the main window and the tray popover in sync.
fn broadcast(app: &AppHandle, state: &AppState) {
    let _ = app.emit("state", state);
}

#[tauri::command]
async fn get_state(b: State<'_, Backend>) -> Result<AppState, String> {
    ask(&b, Req::State).await
}

#[tauri::command]
async fn rescan(app: AppHandle, b: State<'_, Backend>) -> Result<AppState, String> {
    let s = ask(&b, Req::Rescan).await?;
    broadcast(&app, &s);
    Ok(s)
}

#[tauri::command]
async fn use_demo(app: AppHandle, b: State<'_, Backend>) -> Result<AppState, String> {
    let s = ask(&b, Req::UseDemo).await?;
    broadcast(&app, &s);
    Ok(s)
}

#[tauri::command]
async fn set_labs(app: AppHandle, b: State<'_, Backend>, on: bool) -> Result<AppState, String> {
    let s = ask(&b, |r| Req::SetLabs(on, r)).await?;
    broadcast(&app, &s);
    Ok(s)
}

#[tauri::command]
async fn set_control(
    app: AppHandle,
    b: State<'_, Backend>,
    id: String,
    value: u16,
) -> Result<Outcome, String> {
    let out = ask(&b, |reply| Req::Set { id, value, reply }).await??;
    broadcast(&app, &out.state);
    Ok(out)
}

#[tauri::command]
async fn toggle_feature(
    app: AppHandle,
    b: State<'_, Backend>,
    feature: Feature,
    on: bool,
    confirmed: bool,
) -> Result<FeatureOutcome, String> {
    let out = ask(&b, |reply| Req::Feature {
        feature,
        on,
        confirmed,
        reply,
    })
    .await?;
    if let FeatureOutcome::Applied { state, .. } | FeatureOutcome::Blocked { state, .. } = &out {
        broadcast(&app, state);
    }
    Ok(out)
}

#[tauri::command]
async fn simulate_pbp(
    app: AppHandle,
    b: State<'_, Backend>,
    on: bool,
) -> Result<FeatureOutcome, String> {
    let out = ask(&b, |r| Req::SimulatePbp(on, r)).await?;
    if let FeatureOutcome::Applied { state, .. } = &out {
        broadcast(&app, state);
    }
    Ok(out)
}

#[tauri::command]
async fn pick_picture_mode(
    app: AppHandle,
    b: State<'_, Backend>,
    label: String,
    confirmed: bool,
) -> Result<FeatureOutcome, String> {
    let out = ask(&b, |reply| Req::PictureMode {
        label,
        confirmed,
        reply,
    })
    .await?;
    if let FeatureOutcome::Applied { state, .. } | FeatureOutcome::Blocked { state, .. } = &out {
        broadcast(&app, state);
    }
    Ok(out)
}

#[tauri::command]
async fn undo(app: AppHandle, b: State<'_, Backend>) -> Result<Outcome, String> {
    let out = ask(&b, Req::Undo).await??;
    broadcast(&app, &out.state);
    Ok(out)
}

#[tauri::command]
async fn revert_control(
    app: AppHandle,
    b: State<'_, Backend>,
    id: String,
    value: u16,
) -> Result<Outcome, String> {
    let out = ask(&b, |reply| Req::Revert { id, value, reply }).await??;
    broadcast(&app, &out.state);
    Ok(out)
}

#[tauri::command]
async fn reset_original(app: AppHandle, b: State<'_, Backend>) -> Result<Outcome, String> {
    let out = ask(&b, Req::ResetOriginal).await??;
    broadcast(&app, &out.state);
    Ok(out)
}

#[tauri::command]
fn get_hotkeys(registry: State<'_, Registry>) -> HotkeysView {
    registry.view()
}

#[tauri::command]
fn set_hotkeys(app: AppHandle, enabled: bool, bindings: BTreeMap<Action, String>) -> HotkeysView {
    hotkeys::apply(&app, hotkeys::Prefs { enabled, bindings }, true)
}

/// The OS login item is the record of whether this is on, so it stays right
/// even if the user removes it in their system settings.
#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    if on {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|e| format!("Couldn't change start at login: {e}"))?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct UpdatePrefs {
    auto_check: bool,
}

impl Default for UpdatePrefs {
    fn default() -> Self {
        UpdatePrefs { auto_check: true }
    }
}

fn update_prefs_path() -> std::path::PathBuf {
    nubless_core::paths::state_dir().join("updates.json")
}

/// Checking for updates automatically is on unless the user turned it off.
#[tauri::command]
fn get_auto_update() -> bool {
    std::fs::read_to_string(update_prefs_path())
        .ok()
        .and_then(|t| serde_json::from_str::<UpdatePrefs>(&t).ok())
        .unwrap_or_default()
        .auto_check
}

#[tauri::command]
fn set_auto_update(on: bool) -> Result<bool, String> {
    let path = update_prefs_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json =
        serde_json::to_string_pretty(&UpdatePrefs { auto_check: on }).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("Couldn't save this setting: {e}"))?;
    Ok(on)
}

/// The tray's menu entry and plain icon, kept so an available update can swap them.
struct TrayUpdate {
    item: MenuItem<tauri::Wry>,
    icon: tauri::image::Image<'static>,
}

/// `icon` with a green pip in its bottom-right corner (RGBA in, RGBA out).
fn with_pip(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let size = w.min(h) as f32;
    let r = size * 0.24;
    let (cx, cy) = (w as f32 - r - size * 0.03, h as f32 - r - size * 0.03);
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            // Dark ring so the pip reads on light and dark taskbars; green inside.
            let (ring, fill) = (
                (r + 0.5 - d).clamp(0.0, 1.0),
                (r * 0.7 + 0.5 - d).clamp(0.0, 1.0),
            );
            if ring == 0.0 {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            let mix = |base: u8, c: u8, a: f32| (base as f32 * (1.0 - a) + c as f32 * a) as u8;
            for (k, (dark, green)) in [(20u8, 46u8), (20, 196), (20, 79)].into_iter().enumerate() {
                let c = mix(dark, green, fill);
                out[i + k] = mix(out[i + k], c, ring);
            }
            out[i + 3] = out[i + 3].max((ring * 255.0) as u8);
        }
    }
    out
}

/// Tell the tray whether an update is waiting: a green pip on the icon and a menu
/// entry that names the version. `None` puts both back.
#[tauri::command]
fn set_update_available(app: AppHandle, version: Option<String>) {
    let (Some(tray), Some(parts)) = (app.tray_by_id("nubless"), app.try_state::<TrayUpdate>())
    else {
        return;
    };
    let _ = parts.item.set_text(match &version {
        Some(v) => format!("Update {v} available"),
        None => "Check for updates".to_string(),
    });
    let icon = if version.is_some() {
        let img = &parts.icon;
        tauri::image::Image::new_owned(
            with_pip(img.rgba(), img.width(), img.height()),
            img.width(),
            img.height(),
        )
    } else {
        parts.icon.clone()
    };
    let _ = tray.set_icon(Some(icon));
    let _ = tray.set_tooltip(Some(match &version {
        Some(v) => format!("nubless: update {v} available"),
        None => "nubless".to_string(),
    }));
}

#[tauri::command]
fn open_main(app: AppHandle, page: Option<String>) {
    show_main(&app);
    if let Some(p) = page {
        let _ = app.emit("navigate", p);
    }
    if let Some(t) = app.get_webview_window("tray") {
        let _ = t.hide();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Where the popover goes: centred on `anchor`, above it when the anchor is in the
/// lower half of the work area (a bottom taskbar) and below it otherwise, and always
/// inside the work area. All values are physical pixels; `area` is (x, y, w, h).
fn popover_position(
    anchor: (f64, f64),
    (w, h): (f64, f64),
    (ax, ay, aw, ah): (f64, f64, f64, f64),
    gap: f64,
) -> (f64, f64) {
    let y = if anchor.1 > ay + ah / 2.0 {
        ay + ah - h - gap
    } else {
        ay + gap
    };
    let x = (anchor.0 - w / 2.0).clamp(ax + gap, (ax + aw - w - gap).max(ax + gap));
    (x, y)
}

/// Show or hide the popover next to the tray icon.
///
/// Windows can report the icon's rectangle outside every monitor (it did on a 5K
/// screen with the icon in the overflow flyout), so a monitor is looked up by the
/// icon first, then by the cursor, which is on the icon because it was just
/// clicked, and last the primary monitor. The popover is never placed off-screen.
fn toggle_popover(app: &AppHandle, rect: Rect) {
    let Some(win) = app.get_webview_window("tray") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }
    let scale = win.scale_factor().unwrap_or(1.0);
    let pos = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);
    let icon = (pos.x + size.width / 2.0, pos.y + size.height / 2.0);
    let cursor = app.cursor_position().ok().map(|c| (c.x, c.y));
    let found = [Some(icon), cursor].into_iter().flatten().find_map(|pt| {
        let m = app.monitor_from_point(pt.0, pt.1).ok().flatten()?;
        Some((pt, m))
    });
    let (anchor, monitor) = match found {
        Some(f) => f,
        None => match app.primary_monitor() {
            Ok(Some(m)) => (
                (m.position().x as f64 + m.size().width as f64 / 2.0, icon.1),
                m,
            ),
            _ => return,
        },
    };
    let Ok(outer) = win.outer_size() else { return };
    let area = monitor.work_area();
    let (x, y) = popover_position(
        anchor,
        (outer.width as f64, outer.height as f64),
        (
            area.position.x as f64,
            area.position.y as f64,
            area.size.width as f64,
            area.size.height as f64,
        ),
        12.0 * scale,
    );
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit("popover-shown", ());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let demo =
        std::env::args().any(|a| a == "--demo") || std::env::var_os("NUBLESS_DEMO").is_some();
    let hidden = std::env::args().any(|a| a == HIDDEN_ARG);
    // The monitor thread starts before Tauri, so it reaches the app through this cell.
    static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();
    let backend = worker::spawn(
        demo,
        Box::new(|state| {
            if let Some(app) = APP.get() {
                broadcast(app, state);
            }
        }),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // Held keys repeat Pressed; the worker merges the repeats.
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(action) = app.state::<Registry>().action_for(shortcut) {
                        let _ = app.state::<Backend>().send(Req::Hotkey(action));
                    }
                })
                .build(),
        )
        .manage(backend)
        .manage(Registry::default())
        .invoke_handler(tauri::generate_handler![
            get_state,
            rescan,
            use_demo,
            set_labs,
            set_control,
            toggle_feature,
            simulate_pbp,
            pick_picture_mode,
            undo,
            revert_control,
            reset_original,
            get_hotkeys,
            set_hotkeys,
            get_autostart,
            set_autostart,
            get_auto_update,
            set_auto_update,
            set_update_available,
            open_main,
            quit
        ])
        .setup(|app| {
            let _ = APP.set(app.handle().clone());
            hotkeys::apply(app.handle(), hotkeys::Prefs::load(), false);
            let open = MenuItem::with_id(app, "open", "Open nubless", true, None::<&str>)?;
            let updates =
                MenuItem::with_id(app, "updates", "Check for updates", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit nubless", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&open, &updates, &sep, &quit])?;
            let bundled = app.default_window_icon().expect("bundle icon");
            let icon = tauri::image::Image::new_owned(
                bundled.rgba().to_vec(),
                bundled.width(),
                bundled.height(),
            );
            app.manage(TrayUpdate {
                item: updates.clone(),
                icon: icon.clone(),
            });
            TrayIconBuilder::with_id("nubless")
                .icon(icon)
                .tooltip("nubless")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "updates" => {
                        show_main(app);
                        let _ = app.emit("navigate", "updates");
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle(), rect);
                    }
                })
                .build(app)?;
            Ok(())
        })
        // Show the main window from here once its page has loaded (it starts
        // hidden to avoid a blank flash). Showing it from the page's own script
        // left the window ignoring input on first launch.
        // Started at login: stay in the tray until the user opens the window.
        .on_page_load(move |webview, payload| {
            if webview.label() == "main"
                && !hidden
                && payload.event() == tauri::webview::PageLoadEvent::Finished
            {
                let w = webview.window();
                let _ = w.show();
                let _ = w.set_focus();
            }
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the window keeps nubless in the tray.
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("tray", WindowEvent::Focused(false)) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running nubless");
}

#[cfg(test)]
mod tests {
    use super::{popover_position, with_pip};

    #[test]
    fn the_pip_is_green_in_the_corner_and_leaves_the_rest_alone() {
        let (w, h) = (32u32, 32u32);
        let plain = vec![200u8; (w * h * 4) as usize];
        let out = with_pip(&plain, w, h);
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            (out[i], out[i + 1], out[i + 2])
        };
        assert_eq!(px(0, 0), (200, 200, 200));
        let (r, g, b) = px(26, 26);
        assert!(g > r && g > b, "expected green, got {:?}", (r, g, b));
    }

    // The 5K screen from the bug report: work area starts below a 64 px bar.
    const AREA: (f64, f64, f64, f64) = (0.0, 64.0, 5120.0, 2816.0);
    const WIN: (f64, f64) = (680.0, 900.0);

    #[test]
    fn a_bottom_taskbar_icon_opens_the_popover_above_it() {
        let (x, y) = popover_position((4540.0, 2800.0), WIN, AREA, 24.0);
        assert_eq!((x, y), (4200.0, 2880.0 - 900.0 - 24.0));
    }

    #[test]
    fn a_top_bar_icon_opens_it_below() {
        let (_, y) = popover_position((4540.0, 20.0), WIN, AREA, 24.0);
        assert_eq!(y, 64.0 + 24.0);
    }

    #[test]
    fn it_stays_inside_the_work_area_at_the_edges() {
        let (x, _) = popover_position((5100.0, 2800.0), WIN, AREA, 24.0);
        assert_eq!(x, 5120.0 - 680.0 - 24.0);
        let (x, _) = popover_position((10.0, 2800.0), WIN, AREA, 24.0);
        assert_eq!(x, 24.0);
    }
}
