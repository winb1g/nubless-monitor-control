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

/// Place the popover next to the tray icon, inside the work area, above a
/// bottom taskbar or below a top menu bar.
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
    let (cx, cy) = (pos.x + size.width / 2.0, pos.y + size.height / 2.0);
    let Ok(outer) = win.outer_size() else { return };
    let (w, h) = (outer.width as f64, outer.height as f64);
    let gap = 12.0 * scale;
    let (mut x, mut y) = (cx - w / 2.0, pos.y + size.height + gap);
    if let Ok(Some(m)) = app.monitor_from_point(cx, cy) {
        let area = m.work_area();
        let (ax, ay) = (area.position.x as f64, area.position.y as f64);
        let (aw, ah) = (area.size.width as f64, area.size.height as f64);
        y = if cy > ay + ah / 2.0 {
            ay + ah - h - gap
        } else {
            ay + gap
        };
        x = x.clamp(ax + gap, ax + aw - w - gap);
    }
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
            TrayIconBuilder::with_id("nubless")
                .icon(app.default_window_icon().cloned().expect("bundle icon"))
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
