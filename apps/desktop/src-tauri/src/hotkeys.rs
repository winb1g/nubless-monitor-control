//! Global keyboard shortcuts, so the monitor can be adjusted without opening
//! nubless. Bindings live in their own file next to the other state; the
//! monitor thread only ever sees an [`Action`].

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use nubless_core::paths;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    BrightnessUp,
    BrightnessDown,
    NextPictureMode,
}

impl Action {
    pub const ALL: [Action; 3] = [
        Action::BrightnessUp,
        Action::BrightnessDown,
        Action::NextPictureMode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::BrightnessUp => "Brightness up",
            Action::BrightnessDown => "Brightness down",
            Action::NextPictureMode => "Next picture mode",
        }
    }

    /// Ctrl+Alt with keys that type nothing under AltGr layouts and that
    /// graphics drivers don't claim (Intel uses Ctrl+Alt+arrows to rotate).
    pub fn default_keys(self) -> &'static str {
        match self {
            Action::BrightnessUp => "Ctrl+Alt+PageUp",
            Action::BrightnessDown => "Ctrl+Alt+PageDown",
            Action::NextPictureMode => "Ctrl+Alt+End",
        }
    }
}

/// Brightness change per key press.
pub const BRIGHTNESS_STEP: i32 = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub enabled: bool,
    pub bindings: BTreeMap<Action, String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            enabled: true,
            bindings: Action::ALL
                .iter()
                .map(|&a| (a, a.default_keys().to_string()))
                .collect(),
        }
    }
}

fn prefs_path() -> PathBuf {
    paths::state_dir().join("hotkeys.json")
}

impl Prefs {
    pub fn load() -> Prefs {
        let mut p: Prefs = std::fs::read_to_string(prefs_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        p.fill_defaults();
        p
    }

    /// An action added in a later version gets its default binding.
    fn fill_defaults(&mut self) {
        for a in Action::ALL {
            self.bindings
                .entry(a)
                .or_insert_with(|| a.default_keys().to_string());
        }
    }

    fn save(&self) -> std::io::Result<()> {
        let path = prefs_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BindingView {
    pub action: Action,
    pub label: &'static str,
    pub keys: String,
    pub default_keys: &'static str,
    /// Why this binding isn't active, if it isn't (taken by another app, not a valid key).
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HotkeysView {
    pub enabled: bool,
    pub bindings: Vec<BindingView>,
}

/// The bindings in effect, by shortcut id, for the plugin's handler.
#[derive(Default)]
pub struct Registry {
    prefs: Mutex<Prefs>,
    active: Mutex<HashMap<u32, Action>>,
    errors: Mutex<BTreeMap<Action, String>>,
}

impl Registry {
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<Action> {
        self.active.lock().ok()?.get(&shortcut.id()).copied()
    }

    pub fn view(&self) -> HotkeysView {
        let prefs = self.prefs.lock().expect("hotkey prefs").clone();
        let errors = self.errors.lock().expect("hotkey errors").clone();
        HotkeysView {
            enabled: prefs.enabled,
            bindings: Action::ALL
                .iter()
                .map(|&a| BindingView {
                    action: a,
                    label: a.label(),
                    keys: prefs.bindings.get(&a).cloned().unwrap_or_default(),
                    default_keys: a.default_keys(),
                    error: errors.get(&a).cloned(),
                })
                .collect(),
        }
    }
}

/// Register `prefs` with the OS, replacing whatever was registered before, and
/// remember them. A binding that can't be registered is reported, not fatal.
pub fn apply<R: Runtime>(app: &AppHandle<R>, prefs: Prefs, persist: bool) -> HotkeysView {
    let registry = app.state::<Registry>();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut active = HashMap::new();
    let mut errors = BTreeMap::new();
    let mut seen = HashMap::new();
    if prefs.enabled {
        for (&action, keys) in &prefs.bindings {
            let shortcut = match Shortcut::from_str(keys) {
                Ok(s) => s,
                Err(_) => {
                    errors.insert(
                        action,
                        format!("{keys} isn't a key combination nubless understands."),
                    );
                    continue;
                }
            };
            if let Some(other) = seen.insert(shortcut.id(), action) {
                errors.insert(
                    action,
                    format!("Already used for {}.", Action::label(other)),
                );
                continue;
            }
            match gs.register(shortcut) {
                Ok(()) => {
                    active.insert(shortcut.id(), action);
                }
                Err(_) => {
                    errors.insert(action, format!("Another app is already using {keys}."));
                }
            }
        }
    }
    if persist {
        let _ = prefs.save();
    }
    *registry.prefs.lock().expect("hotkey prefs") = prefs;
    *registry.active.lock().expect("hotkey map") = active;
    *registry.errors.lock().expect("hotkey errors") = errors;
    registry.view()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_differ() {
        let ids: std::collections::HashSet<_> = Action::ALL
            .iter()
            .map(|a| Shortcut::from_str(a.default_keys()).unwrap().id())
            .collect();
        assert_eq!(ids.len(), Action::ALL.len());
    }

    #[test]
    fn browser_key_codes_parse() {
        for keys in [
            "Ctrl+Alt+KeyM",
            "Ctrl+Shift+Digit1",
            "Alt+ArrowUp",
            "Ctrl+Alt+F9",
        ] {
            assert!(Shortcut::from_str(keys).is_ok(), "{keys}");
        }
    }

    #[test]
    fn older_prefs_files_get_new_actions() {
        let mut p: Prefs = serde_json::from_str(
            r#"{"enabled":false,"bindings":{"brightness-up":"Ctrl+Alt+KeyU"}}"#,
        )
        .unwrap();
        p.fill_defaults();
        assert!(!p.enabled);
        assert_eq!(p.bindings[&Action::BrightnessUp], "Ctrl+Alt+KeyU");
        assert_eq!(p.bindings[&Action::NextPictureMode], "Ctrl+Alt+End");
    }
}
