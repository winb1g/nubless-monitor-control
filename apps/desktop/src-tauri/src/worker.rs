//! One thread owns every monitor connection. Every request from the UI is a
//! message to it, so DDC/CI traffic is serialised and paced in one place, and
//! slider drags collapse to the latest value instead of flooding the monitor.
//!
//! With two or more supported monitors, each has its own connection, original
//! settings and undo history. Requests act on the selected one; when the user
//! links them ("Change all monitors together"), each change that works on the
//! selected monitor is then repeated on the others.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use nubless_core::baseline;
use nubless_core::controller::{Applied, ControlError, Snapshot};
use nubless_core::journal::Journal;
use nubless_core::mock::MockMonitor;
use nubless_core::monitors::{self, Folders, Identity};
use nubless_core::paths;
use nubless_core::profile::{Match, WritePolicy};
use nubless_core::profiles::gm950b::{AI_CAPABLE_PICTURE_MODES, PROFILE};
use nubless_core::rules::{self, Decision, Feature, FeatureState};
use nubless_core::transport::Transport;
use nubless_core::{Controller, Timing};
use serde::Serialize;

use crate::hotkeys::{Action, BRIGHTNESS_STEP};
use crate::view::{self, AppState, BaselineView, FeaturesView, MonitorCard};

type Reply<T> = Sender<T>;

/// Requests that change a monitor name the one the window was showing
/// (`monitor`, a `MonitorCard::id`), so a change made in one window never lands
/// on a monitor another window has just selected. `None` means the selected one.
pub enum Req {
    State(Reply<AppState>),
    Rescan(Reply<AppState>),
    UseDemo(Reply<AppState>),
    SetLabs(bool, Reply<AppState>),
    /// Make a monitor (by `MonitorCard::id`) the one the UI controls.
    Select(String, Reply<Result<AppState, String>>),
    /// Repeat every change on all connected monitors.
    SetLinked(bool, Reply<AppState>),
    Set {
        monitor: Option<String>,
        id: String,
        value: u16,
        reply: Reply<Result<Outcome, String>>,
    },
    Feature {
        monitor: Option<String>,
        feature: Feature,
        on: bool,
        confirmed: bool,
        reply: Reply<FeatureOutcome>,
    },
    SimulatePbp(bool, Reply<FeatureOutcome>),
    PictureMode {
        monitor: Option<String>,
        label: String,
        confirmed: bool,
        reply: Reply<FeatureOutcome>,
    },
    Undo {
        monitor: Option<String>,
        reply: Reply<Result<Outcome, String>>,
    },
    /// Put one control back to a value it had, using the restore rules (any value
    /// the monitor itself reported is allowed). The UI's answer to a change nobody
    /// kept: see `ControlView::confirm`.
    Revert {
        monitor: Option<String>,
        id: String,
        value: u16,
        reply: Reply<Result<Outcome, String>>,
    },
    ResetOriginal {
        monitor: Option<String>,
        reply: Reply<Result<Outcome, String>>,
    },
    /// A global shortcut fired. No reply: every window gets the new state.
    Hotkey(Action),
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub state: AppState,
    pub toast: Option<String>,
    /// The toast offers "Undo".
    pub undoable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FeatureOutcome {
    Applied {
        state: AppState,
        notice: Option<String>,
    },
    Confirm {
        question: String,
    },
    Blocked {
        reason: String,
        state: AppState,
    },
}

struct Conn {
    ctl: Controller<Box<dyn Transport>>,
    /// `MonitorCard::id` of this monitor: the name of its settings folder, so
    /// it stays the same across scans and reconnects (and is what's remembered
    /// as the user's selection).
    card: String,
    /// What the UI calls it: the OS's name for it ("LG ULTRAGEAR+ (2)").
    name: String,
    /// Which physical monitor this is, to find it again after it drops off.
    /// `None` for simulated monitors.
    identity: Option<Identity>,
    /// Its own folder of original settings and undo history; `None` when it
    /// couldn't be given one (then changes are paused, see `baseline_error`).
    folder: Option<PathBuf>,
    /// macOS display id, for the name System Settings shows.
    display_id: Option<u32>,
    /// DDC/CI pacing it was opened with, to open it again the same way.
    timing: Timing,
    demo: bool,
    /// Demo only: simulated smart-feature state. On hardware it's read from the monitor.
    features: FeatureState,
    /// Demo only: simulated picture mode.
    demo_picture: Option<String>,
    /// Values need re-reading (after a picture-mode switch). Done after the
    /// reply is sent, so the UI isn't kept waiting on ~25 DDC/CI reads.
    stale: bool,
    baseline: Option<Snapshot>,
    baseline_error: Option<String>,
    just_saved: bool,
    /// Last known value per control id; `None` = unknown (no answer, or reading
    /// needs Labs). Keyed by id because the AI controls share VCP 0xA6.
    values: BTreeMap<&'static str, Option<u16>>,
}

impl Conn {
    /// Re-read some controls after something else changed them behind our back.
    /// One the monitor doesn't answer keeps its last value.
    fn reread(&mut self, ids: &[&'static str]) {
        for &id in ids {
            if let Ok(r) = self.ctl.read(id) {
                self.values.insert(id, Some(r.current));
            }
        }
    }

    /// Re-read every control. A control the monitor doesn't answer keeps the value
    /// it last had: after a write that resyncs the panel the monitor goes quiet for
    /// a few seconds, and forgetting every value then would grey out the whole app.
    /// Reading stops after a run of misses so a quiet monitor can't hold up the
    /// thread (and every request queued behind it) for tens of seconds.
    /// Returns how many controls answered.
    fn refresh(&mut self) -> usize {
        const MISSES_BEFORE_GIVING_UP: u32 = 4;
        let mut misses = 0;
        let mut answered = 0;
        for c in PROFILE.controls {
            if misses >= MISSES_BEFORE_GIVING_UP {
                break;
            }
            match self.ctl.read(c.id) {
                Ok(r) => {
                    misses = 0;
                    answered += 1;
                    self.values.insert(c.id, Some(r.current));
                }
                // Not readable by policy (needs Labs): genuinely unknown.
                Err(ControlError::Safety(_)) => {
                    self.values.insert(c.id, None);
                }
                Err(_) => {
                    misses += 1;
                    self.values.entry(c.id).or_insert(None);
                }
            }
        }
        answered
    }

    fn value(&self, id: &str) -> Option<u16> {
        self.values.get(id).copied().flatten()
    }

    /// Label of the current picture mode, if its raw value is mapped.
    fn picture_mode(&self) -> Option<&'static str> {
        PROFILE
            .control("picture-mode")?
            .choice_label(self.value("picture-mode")?)
    }
}

struct Worker {
    /// One per connected supported monitor, in the order the system lists them.
    conns: Vec<Conn>,
    /// Index into `conns` of the monitor the UI controls.
    active: usize,
    monitors: Vec<MonitorCard>,
    labs: bool,
    /// Changes go to every connected monitor, not just the selected one.
    linked: bool,
    /// Card id of the monitor the user last selected, so it is selected again next time.
    preferred: Option<String>,
    /// Opens a monitor again by identity (after it went quiet, or its handle
    /// went stale). Real hardware in the app; a stand-in in tests.
    reopen: Reopen,
    /// How long to wait between attempts to find a monitor that went quiet.
    silence_wait: Duration,
    /// Asks the OS for its display names (macOS). Empty until the app is up.
    os_names: OsNames,
    /// Some monitor still lacks the OS's name for it.
    names_pending: bool,
    error: Option<String>,
    settings_path: PathBuf,
    /// Pushes a fresh state to every window (set up by lib.rs).
    notify: Box<dyn Fn(&AppState) + Send>,
}

pub type Reopen = Box<dyn Fn(&Identity) -> Option<Box<dyn Transport>> + Send>;
pub type OsNames = Box<dyn Fn() -> Vec<(u32, String)> + Send>;

/// Real hardware: find the monitor with this identity and open it.
fn reopen_hardware() -> Reopen {
    Box::new(|id| nubless_hw::reopen(&PROFILE, id).map(|d| Box::new(d) as Box<dyn Transport>))
}

/// Everything about a monitor except its connection, for [`Worker::connect`].
struct Meta {
    card: String,
    name: String,
    identity: Option<Identity>,
    folder: Result<PathBuf, String>,
    display_id: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
struct Settings {
    labs: bool,
    linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected: Option<String>,
}

/// `demo_monitors` > 0 starts with that many simulated monitors instead of scanning.
pub fn spawn(
    demo_monitors: usize,
    notify: Box<dyn Fn(&AppState) + Send>,
    os_names: OsNames,
) -> Sender<Req> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("nubless-monitor".into())
        .spawn(move || {
            let settings_path = paths::state_dir().join("settings.json");
            let settings: Settings = std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            let mut w = Worker {
                conns: vec![],
                active: 0,
                monitors: vec![],
                labs: settings.labs,
                linked: settings.linked,
                preferred: settings.selected,
                reopen: reopen_hardware(),
                silence_wait: Duration::from_secs(2),
                os_names,
                names_pending: false,
                error: None,
                settings_path,
                notify,
            };
            if demo_monitors > 0 {
                w.connect_demo(demo_monitors);
            } else {
                w.scan();
            }
            w.run(rx);
        })
        .expect("spawn monitor thread");
    tx
}

impl Worker {
    /// The selected monitor's connection.
    fn conn(&self) -> Option<&Conn> {
        self.conns.get(self.active)
    }

    fn conn_mut(&mut self) -> Option<&mut Conn> {
        self.conns.get_mut(self.active)
    }

    /// Some monitor's values are waiting to be re-read.
    fn refresh_pending(&self) -> bool {
        self.conns.iter().any(|c| c.stale)
    }

    fn save_settings(&self) {
        let settings = Settings {
            labs: self.labs,
            linked: self.linked,
            selected: self.preferred.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&settings) {
            if let Some(dir) = self.settings_path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&self.settings_path, text);
        }
    }

    /// Run `op` on the monitor a request was made for, then select again the
    /// one that was selected. Returns whether that was a different monitor (and
    /// so whether the state `op` built shows the wrong one).
    fn aimed<R>(
        &mut self,
        monitor: Option<&str>,
        op: impl FnOnce(&mut Self) -> R,
    ) -> Result<(R, bool), String> {
        let home = self.active;
        if let Some(m) = monitor {
            self.active = self.conns.iter().position(|c| c.card == m).ok_or(
                "That monitor isn't connected any more, so nothing was changed. Scan again from Displays.",
            )?;
        }
        let r = op(self);
        let moved = self.active != home;
        self.active = home;
        Ok((r, moved))
    }

    fn aimed_outcome(
        &mut self,
        monitor: Option<&str>,
        op: impl FnOnce(&mut Self) -> Result<Outcome, String>,
    ) -> Result<Outcome, String> {
        let (r, moved) = self.aimed(monitor, op)?;
        let mut out = r?;
        if moved {
            out.state = self.state();
        }
        Ok(out)
    }

    fn aimed_feature(
        &mut self,
        monitor: Option<&str>,
        op: impl FnOnce(&mut Self) -> FeatureOutcome,
    ) -> FeatureOutcome {
        match self.aimed(monitor, op) {
            Err(reason) => FeatureOutcome::Blocked {
                reason,
                state: self.state(),
            },
            Ok((mut out, moved)) => {
                if moved {
                    if let FeatureOutcome::Applied { state, .. }
                    | FeatureOutcome::Blocked { state, .. } = &mut out
                    {
                        *state = self.state();
                    }
                }
                out
            }
        }
    }

    /// When linked, repeat `op` on every connected monitor other than the
    /// selected one, after it worked there. Returns what went wrong where.
    fn mirror(&mut self, mut op: impl FnMut(&mut Self) -> Result<(), String>) -> Vec<String> {
        if !self.linked || self.conns.len() < 2 {
            return vec![];
        }
        let home = self.active;
        let mut failed = vec![];
        for i in (0..self.conns.len()).filter(|&i| i != home) {
            self.active = i;
            if let Err(e) = op(self) {
                failed.push(format!("{}: {e}", self.conns[i].name));
            }
        }
        self.active = home;
        failed
    }

    /// Add what didn't work on the other monitors to a toast.
    fn with_failures(toast: Option<String>, failed: &[String]) -> Option<String> {
        if failed.is_empty() {
            return toast;
        }
        let note = format!("Not changed on {}", failed.join("; "));
        Some(match toast {
            Some(t) => format!("{t}. {note}"),
            None => note,
        })
    }

    fn policy(&self) -> WritePolicy {
        if self.labs {
            WritePolicy::Experimental
        } else {
            WritePolicy::Safe
        }
    }

    /// The current policy lets nubless write this control.
    fn allows(&self, id: &str) -> bool {
        PROFILE.control(id).is_some_and(|c| self.policy().allows(c))
    }

    fn run(&mut self, rx: Receiver<Req>) {
        while let Ok(first) = rx.recv() {
            let mut batch = vec![first];
            batch.extend(rx.try_iter());
            // A newer value for the same control replaces older ones (slider drags).
            let mut latest = std::collections::HashMap::new();
            // A held brightness key adds up to one write, not a queue of them.
            let mut brightness_steps = 0;
            for (i, r) in batch.iter().enumerate() {
                match r {
                    Req::Set { monitor, id, .. } => {
                        latest.insert((monitor.clone(), id.clone()), i);
                    }
                    Req::Hotkey(Action::BrightnessUp) => brightness_steps += 1,
                    Req::Hotkey(Action::BrightnessDown) => brightness_steps -= 1,
                    _ => {}
                }
            }
            if brightness_steps != 0 {
                self.step_all("brightness", brightness_steps * BRIGHTNESS_STEP);
                let s = self.state();
                (self.notify)(&s);
            }
            for (i, req) in batch.into_iter().enumerate() {
                match req {
                    Req::Hotkey(Action::BrightnessUp | Action::BrightnessDown) => {}
                    Req::Hotkey(Action::NextPictureMode) => {
                        if let Some(label) = self.next_picture_mode() {
                            self.mirror(|w| match w.picture_mode(label, true) {
                                FeatureOutcome::Blocked { reason, .. } => Err(reason),
                                _ => Ok(()),
                            });
                        }
                        let s = self.state();
                        (self.notify)(&s);
                    }
                    Req::Set {
                        monitor,
                        id,
                        value,
                        reply,
                    } if latest.get(&(monitor.clone(), id.clone())) == Some(&i) => {
                        let out = self.aimed_outcome(monitor.as_deref(), |w| w.set_all(&id, value));
                        let _ = reply.send(out);
                    }
                    Req::Set { reply, .. } => {
                        let _ = reply.send(Ok(Outcome {
                            state: self.state(),
                            toast: None,
                            undoable: false,
                        }));
                    }
                    other => self.handle(other),
                }
            }
            if self.refresh_pending() {
                for c in self.conns.iter_mut().filter(|c| c.stale) {
                    c.stale = false;
                    c.refresh();
                }
                let s = self.state();
                (self.notify)(&s);
            }
        }
    }

    fn handle(&mut self, req: Req) {
        match req {
            Req::State(r) => {
                if self.names_pending {
                    self.apply_os_names();
                }
                if let Some(c) = self.conn_mut() {
                    c.refresh();
                }
                let _ = r.send(self.state());
            }
            Req::Rescan(r) => {
                self.scan();
                let _ = r.send(self.state());
            }
            Req::UseDemo(r) => {
                self.connect_demo(1);
                let _ = r.send(self.state());
            }
            Req::SetLabs(on, r) => {
                self.labs = on;
                let policy = self.policy();
                for c in &mut self.conns {
                    c.ctl.set_policy(policy);
                    // Labs changes what can be read (the AI channel needs it).
                    c.refresh();
                }
                self.save_settings();
                let _ = r.send(self.state());
            }
            Req::Select(card, r) => {
                let out = match self.conns.iter().position(|c| c.card == card) {
                    Some(i) => {
                        self.active = i;
                        self.mark_selected();
                        self.preferred = Some(card);
                        self.save_settings();
                        // It may have been changed from its own menu since it was last
                        // read. If it doesn't answer at all, its handle may have gone
                        // stale (the OS re-detected monitors): open it again.
                        if self.conns[i].refresh() == 0 {
                            self.reconnect_active();
                        }
                        Ok(self.state())
                    }
                    None => Err("That display isn't connected. Try Scan again.".into()),
                };
                let _ = r.send(out);
            }
            Req::SetLinked(on, r) => {
                self.linked = on;
                self.save_settings();
                let _ = r.send(self.state());
            }
            Req::Feature {
                monitor,
                feature,
                on,
                confirmed,
                reply,
            } => {
                let out = self.aimed_feature(monitor.as_deref(), |w| {
                    w.feature_all(feature, on, confirmed)
                });
                let _ = reply.send(out);
            }
            Req::SimulatePbp(on, reply) => {
                let out = if let Some(c) = self.conn_mut().filter(|c| c.demo) {
                    let (next, notice) = rules::on_pbp_changed(c.features, on);
                    c.features = next;
                    FeatureOutcome::Applied {
                        state: self.state(),
                        notice: notice.map(|n| n.text()),
                    }
                } else {
                    FeatureOutcome::Blocked {
                        reason: "Only available with the demo monitor.".into(),
                        state: self.state(),
                    }
                };
                let _ = reply.send(out);
            }
            Req::PictureMode {
                monitor,
                label,
                confirmed,
                reply,
            } => {
                let out = self.aimed_feature(monitor.as_deref(), |w| {
                    w.picture_mode_all(&label, confirmed)
                });
                let _ = reply.send(out);
            }
            Req::Undo { monitor, reply } => {
                let out = self.aimed_outcome(monitor.as_deref(), |w| {
                    w.undo().map(|mut out| {
                        let failed = w.mirror(|w| w.undo().map(drop));
                        out.toast = Self::with_failures(out.toast, &failed);
                        out
                    })
                });
                let _ = reply.send(out);
            }
            Req::Revert {
                monitor,
                id,
                value,
                reply,
            } => {
                // Linked monitors were set to the same value, so they go back to the same one.
                let out = self.aimed_outcome(monitor.as_deref(), |w| {
                    w.revert(&id, value).map(|mut out| {
                        let failed = w.mirror(|w| w.revert(&id, value).map(drop));
                        out.toast = Self::with_failures(out.toast, &failed);
                        out
                    })
                });
                let _ = reply.send(out);
            }
            Req::ResetOriginal { monitor, reply } => {
                // Each monitor goes back to its own originals.
                let out = self.aimed_outcome(monitor.as_deref(), |w| {
                    w.reset_original().map(|mut out| {
                        let failed = w.mirror(|w| w.reset_original().map(drop));
                        out.toast = Self::with_failures(out.toast, &failed);
                        out
                    })
                });
                let _ = reply.send(out);
            }
            Req::Set { .. } | Req::Hotkey(_) => unreachable!("handled in run"),
        }
    }

    /// Find every display, connect to each supported one, and select the one
    /// the user picked last time (else the first).
    fn scan(&mut self) {
        self.error = None;
        let displays = nubless_hw::enumerate(&PROFILE);
        let names = card_names(&displays);
        let mut folders = Folders::new();
        let mut legacy_adopted = false;
        self.monitors.clear();
        self.conns.clear();
        self.active = 0;
        let mut errors = vec![];
        for (display, name) in displays.into_iter().zip(names) {
            let port = display.os.port_name.clone();
            if display.matched == Match::No {
                self.monitors.push(MonitorCard {
                    id: format!("other-{}", display.index),
                    name: display
                        .os
                        .name
                        .clone()
                        .unwrap_or_else(|| "Other display".into()),
                    detail: display.description.clone(),
                    port,
                    supported: false,
                    selected: false,
                    demo: false,
                });
                continue;
            }
            let identity = display.identity();
            let folder = folders.claim(&identity).map_err(|e| e.to_string());
            // Versions before multi-monitor support kept one set of files, for
            // the first supported monitor: that one inherits them.
            if let (Ok(dir), false) = (&folder, legacy_adopted) {
                let _ = monitors::adopt_single_monitor_files(dir);
                legacy_adopted = true;
            }
            let card = match &folder {
                Ok(dir) => dir.file_name().map_or_else(
                    || identity.key().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                ),
                Err(_) => format!("unclaimed-{}", display.index),
            };
            self.monitors.push(MonitorCard {
                id: card.clone(),
                name: name.clone(),
                detail: display.description.clone(),
                port,
                supported: true,
                selected: false,
                demo: false,
            });
            let meta = Meta {
                card,
                name: name.clone(),
                identity: Some(identity),
                folder,
                display_id: display.os.display_id,
            };
            match Self::connect(Box::new(display), meta, self.policy(), Timing::HARDWARE) {
                Ok(conn) => self.conns.push(conn),
                Err(e) => errors.push(format!("Found your {name} but couldn't talk to it: {e}")),
            }
        }
        if !errors.is_empty() {
            self.error = Some(errors.join(" "));
        }
        if let Some(i) = self
            .preferred
            .as_ref()
            .and_then(|k| self.conns.iter().position(|c| &c.card == k))
        {
            self.active = i;
        }
        self.names_pending = self.conns.iter().any(|c| c.display_id.is_some());
        self.apply_os_names();
        self.mark_selected();
    }

    /// Use the names the OS shows (macOS: "LG ULTRAGEAR+ (2)"), once it can say.
    fn apply_os_names(&mut self) {
        if !self.names_pending {
            return;
        }
        let names = (self.os_names)();
        if names.is_empty() {
            return;
        }
        self.names_pending = false;
        for c in &mut self.conns {
            let Some(name) = c
                .display_id
                .and_then(|id| names.iter().find(|(d, _)| *d == id))
                .map(|(_, n)| n.clone())
            else {
                continue;
            };
            if let Some(m) = self.monitors.iter_mut().find(|m| m.id == c.card) {
                m.name = name.clone();
            }
            c.name = name;
        }
    }

    /// Open one supported display and capture its original settings, in its
    /// own folder, before anything can be written to it.
    fn connect(
        transport: Box<dyn Transport>,
        meta: Meta,
        policy: WritePolicy,
        timing: Timing,
    ) -> Result<Conn, String> {
        let journal = match &meta.folder {
            Ok(dir) => Journal::open(monitors::journal_file(dir)).unwrap_or_default(),
            Err(_) => Journal::in_memory(),
        };
        let mut ctl = Controller::new(transport, &PROFILE, policy, timing, journal)
            .map_err(|e| e.to_string())?;
        let (baseline, baseline_error, just_saved) = match &meta.folder {
            Ok(dir) => match baseline::ensure(&monitors::baseline_file(dir), &mut ctl) {
                Ok((b, saved)) => (Some(b), None, saved),
                Err(e) => (None, Some(e.to_string()), false),
            },
            Err(e) => (None, Some(e.clone()), false),
        };
        let mut conn = Conn {
            ctl,
            card: meta.card,
            name: meta.name,
            identity: meta.identity,
            folder: meta.folder.ok(),
            display_id: meta.display_id,
            timing,
            demo: false,
            features: FeatureState::default(),
            demo_picture: None,
            stale: false,
            baseline,
            baseline_error,
            just_saved,
            values: Default::default(),
        };
        conn.refresh();
        Ok(conn)
    }

    /// Open the selected monitor again (its panel resynced, or the OS
    /// re-detected it and its old handle went stale), found by identity, and
    /// leave every other monitor alone. It keeps its card, name and folder.
    /// Returns whether it was found.
    fn reconnect_active(&mut self) -> bool {
        let Some(old) = self.conn() else { return false };
        let Some(identity) = old.identity.clone() else {
            return false;
        };
        let Some(transport) = (self.reopen)(&identity) else {
            return false;
        };
        let meta = Meta {
            card: old.card.clone(),
            name: old.name.clone(),
            identity: Some(identity),
            folder: old.folder.clone().ok_or_else(|| {
                old.baseline_error
                    .clone()
                    .unwrap_or_else(|| "no settings folder".into())
            }),
            display_id: old.display_id,
        };
        let timing = old.timing;
        match Self::connect(transport, meta, self.policy(), timing) {
            Ok(mut conn) => {
                let old = &self.conns[self.active];
                // Keep what was on screen for anything it doesn't answer yet.
                for (id, v) in &old.values {
                    if conn.value(id).is_none() {
                        conn.values.insert(id, *v);
                    }
                }
                conn.just_saved = old.just_saved;
                self.conns[self.active] = conn;
                true
            }
            Err(_) => false,
        }
    }

    fn mark_selected(&mut self) {
        let card = self.conn().map(|c| c.card.clone());
        for m in &mut self.monitors {
            m.selected = Some(&m.id) == card.as_ref();
        }
    }

    /// Simulated monitors instead of real ones (`count` of them).
    fn connect_demo(&mut self, count: usize) {
        self.error = None;
        self.monitors.retain(|m| !m.demo);
        self.conns.clear();
        self.active = 0;
        for n in 1..=count {
            let mut ctl = Controller::new(
                Box::new(MockMonitor::gm950b()) as Box<dyn Transport>,
                &PROFILE,
                self.policy(),
                Timing::instant(),
                Journal::in_memory(),
            )
            .expect("mock monitor always answers");
            let baseline = Some(ctl.snapshot());
            let name = format!("{} (demo)", numbered(n, count));
            let card = format!("demo-{n}");
            self.monitors.push(MonitorCard {
                id: card.clone(),
                name: name.clone(),
                detail: "Simulated monitor. Nothing is sent to real hardware.".into(),
                port: None,
                supported: true,
                selected: false,
                demo: true,
            });
            let mut conn = Conn {
                ctl,
                card,
                name,
                identity: None,
                folder: None,
                display_id: None,
                timing: Timing::instant(),
                demo: true,
                features: FeatureState::default(),
                demo_picture: Some("Personalized Picture".into()),
                stale: false,
                baseline,
                baseline_error: None,
                // The simulated monitor's settings are captured fresh on every connect.
                just_saved: true,
                values: Default::default(),
            };
            conn.refresh();
            self.conns.push(conn);
        }
        self.mark_selected();
    }

    fn writes_allowed(&self) -> Result<(), String> {
        match self.conn() {
            None => Err("No monitor connected.".into()),
            Some(c) if c.baseline.is_none() => Err(format!(
                "Your original settings couldn't be saved ({}), so nubless won't change anything yet.",
                c.baseline_error.as_deref().unwrap_or("unknown error")
            )),
            Some(_) => Ok(()),
        }
    }

    /// [`Self::set`] on the selected monitor, then on the others when linked.
    fn set_all(&mut self, id: &str, value: u16) -> Result<Outcome, String> {
        let mut out = self.set(id, value)?;
        let failed = self.mirror(|w| w.set(id, value).map(drop));
        out.toast = Self::with_failures(out.toast, &failed);
        Ok(out)
    }

    fn set(&mut self, id: &str, value: u16) -> Result<Outcome, String> {
        self.writes_allowed()?;
        let conn = self.conn_mut().expect("checked");
        let control = conn.ctl.control(id).map_err(|e| e.to_string())?;
        let blur_before = conn.value("motion-blur");
        let mut first = conn.ctl.write(id, value);
        // Nothing was written: the monitor didn't answer the read before the
        // write. Its handle may have gone stale when the OS re-detected monitors
        // (switching VRR on *another* monitor can do that). Open it again and
        // try once more.
        if matches!(first, Err(ControlError::CannotReadCurrent(_))) && self.reconnect_active() {
            first = self.conn_mut().expect("reconnected").ctl.write(id, value);
        }
        let conn = self.conn_mut().expect("checked");
        let applied = match first {
            Ok(a) => a,
            // The panel is resyncing and the monitor went quiet (or its link was
            // reopened by Windows): reconnect and ask again instead of calling it failed.
            Err(ControlError::Silent { .. }) => self.after_silence(id, value)?,
            Err(e) => {
                // The write was refused or rolled back: show what the monitor really has.
                conn.reread(&[control.id]);
                if matches!(
                    e,
                    ControlError::Interlocked(_) | ControlError::CannotCheck { .. }
                ) {
                    // What it depends on may have changed in the monitor's own menu.
                    conn.reread(&["adaptive-sync"]);
                }
                return Err(e.to_string());
            }
        };
        let Some(conn) = self.conn_mut() else {
            return Err("The monitor is not connected".into());
        };
        conn.values.insert(control.id, Some(applied.new));
        // The monitor clears Motion Blur Reduction when Adaptive-Sync comes on.
        // Say so: Undo puts Adaptive-Sync back, not what it cleared.
        let mut cleared_blur = false;
        if id == "adaptive-sync" {
            conn.reread(&["motion-blur"]);
            cleared_blur = blur_before == Some(1) && conn.value("motion-blur") != Some(1);
        }
        let label = control.label;
        let changed = applied.journal_index.is_some();
        let toast = match (changed, cleared_blur) {
            (true, true) => Some(format!(
                "{label} updated. Motion Blur Reduction turned off with it."
            )),
            (true, false) => Some(format!("{label} updated")),
            (false, _) => None,
        };
        Ok(Outcome {
            state: self.state(),
            toast,
            undoable: changed,
        })
    }

    /// Reopen the display until it answers, then settle the pending write.
    fn after_silence(&mut self, id: &str, value: u16) -> Result<Applied, String> {
        for _ in 0..6 {
            thread::sleep(self.silence_wait);
            // Until it's found again, keep asking the old handle: on some systems
            // the monitor comes back on the same one.
            self.reconnect_active();
            let Some(conn) = self.conn_mut() else {
                continue;
            };
            match conn.ctl.settle_silent(id, value) {
                Ok(a) => return Ok(a),
                Err(ControlError::Silent { .. }) => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("The monitor stopped answering after the change and has not come back. Look at the screen: the change may have applied. Reconnect from Displays."
            .into())
    }

    fn undo(&mut self) -> Result<Outcome, String> {
        let Some(c) = self.conn_mut() else {
            return Err("No monitor connected.".into());
        };
        let r = c.ctl.undo_last().map_err(|e| e.to_string());
        c.refresh();
        r.map(|_| Outcome {
            state: self.state(),
            toast: Some("Undone".into()),
            undoable: false,
        })
    }

    fn revert(&mut self, id: &str, value: u16) -> Result<Outcome, String> {
        self.writes_allowed()?;
        let conn = self.conn_mut().expect("checked");
        let control = conn.ctl.control(id).map_err(|e| e.to_string())?;
        let snapshot = Snapshot {
            model: PROFILE.model.to_string(),
            values: BTreeMap::from([(control.id.to_string(), value)]),
            mode: None,
        };
        let results = conn.ctl.restore(&snapshot);
        conn.reread(&[control.id]);
        if control.id == "adaptive-sync" {
            conn.reread(&["motion-blur"]);
        }
        if let Some((_, Err(e))) = results.into_iter().next() {
            return Err(format!("{} couldn't be put back: {e}", control.label));
        }
        Ok(Outcome {
            state: self.state(),
            toast: Some(format!("{} put back", control.label)),
            undoable: false,
        })
    }

    /// Move a range control by `delta`, clamped to its safe range. Silent when
    /// it can't (no monitor, locked): a hotkey has nowhere to show an error.
    /// Linked monitors are set to the selected one's new value, so they line up.
    fn step_all(&mut self, id: &str, delta: i32) {
        let Some(conn) = self.conn() else { return };
        let Some(control) = PROFILE.control(id) else {
            return;
        };
        let nubless_core::profile::ValueKind::Range { min, max } = control.kind else {
            return;
        };
        let Some(current) = conn.value(id) else {
            return;
        };
        let next = (i32::from(current) + delta).clamp(i32::from(min), i32::from(max)) as u16;
        if next != current {
            let _ = self.set_all(id, next);
        }
    }

    /// Switch to the next picture mode in the monitor's order. Modes that would
    /// need a question first (turning AI off) are skipped: a hotkey can't ask.
    /// Returns the mode switched to.
    fn next_picture_mode(&mut self) -> Option<&'static str> {
        let Some(nubless_core::profile::ValueKind::Choice { choices }) =
            PROFILE.control("picture-mode").map(|c| c.kind)
        else {
            return None;
        };
        let current = match self.conn() {
            Some(c) if c.demo => c.demo_picture.clone(),
            Some(c) => c.picture_mode().map(str::to_string),
            None => return None,
        };
        let start = current
            .as_deref()
            .and_then(|l| choices.iter().position(|ch| ch.label == l))
            .map_or(0, |i| i + 1);
        for k in 0..choices.len() {
            let label = choices[(start + k) % choices.len()].label;
            if current.as_deref() == Some(label) {
                continue;
            }
            match self.picture_mode(label, false) {
                FeatureOutcome::Confirm { .. } => continue,
                FeatureOutcome::Applied { .. } => return Some(label),
                FeatureOutcome::Blocked { .. } => return None,
            }
        }
        None
    }

    /// Smart-feature state: simulated in the demo, read from the monitor otherwise.
    fn effective_features(&self) -> FeatureState {
        let Some(conn) = self.conn() else {
            return FeatureState::default();
        };
        if conn.demo {
            let blocks = conn
                .demo_picture
                .as_deref()
                .is_some_and(|p| !AI_CAPABLE_PICTURE_MODES.contains(&p));
            return FeatureState {
                picture_mode_blocks_ai: blocks,
                ..conn.features
            };
        }
        FeatureState {
            ai_scene_optimization: conn.value("ai-scene-optimization") == Some(2),
            picture_mode_blocks_ai: conn
                .picture_mode()
                .is_some_and(|p| !AI_CAPABLE_PICTURE_MODES.contains(&p)),
            ..FeatureState::default()
        }
    }

    /// Why a smart feature can't be switched on this connection, if it can't.
    fn feature_lock(&self, feature: Feature) -> Option<String> {
        let conn = self.conn()?;
        if conn.demo {
            return None;
        }
        match feature {
            Feature::AiSceneOptimization if !self.allows("ai-scene-optimization") => Some(
                "Turn on Labs to switch AI Scene Optimization from nubless. It's decoded from LG Switch but not yet tested on hardware."
                    .into(),
            ),
            Feature::AiSceneOptimization if conn.value("ai-scene-optimization").is_none() => {
                Some("Your monitor didn't report AI Scene Optimization. Try Scan again.".into())
            }
            Feature::AiSceneOptimization => None,
            Feature::AiPictureMode => Some(
                "AI Picture Mode runs on LG's own AI model inside LG Switch, so it isn't part of nubless."
                    .into(),
            ),
            Feature::MyDisplayPreset => Some(
                "Per-app picture modes are coming once picture modes are fully mapped.".into(),
            ),
        }
    }

    /// [`Self::feature`] on the selected monitor, then on the others when linked.
    /// The question is asked about the selected monitor; the answer counts for all.
    fn feature_all(&mut self, feature: Feature, on: bool, confirmed: bool) -> FeatureOutcome {
        let out = self.feature(feature, on, confirmed);
        let FeatureOutcome::Applied { notice, .. } = out else {
            return out;
        };
        let failed = self.mirror(|w| match w.feature(feature, on, true) {
            FeatureOutcome::Blocked { reason, .. } => Err(reason),
            _ => Ok(()),
        });
        FeatureOutcome::Applied {
            state: self.state(),
            notice: Self::with_failures(notice, &failed),
        }
    }

    fn feature(&mut self, feature: Feature, on: bool, confirmed: bool) -> FeatureOutcome {
        if let Some(reason) = self.feature_lock(feature) {
            return FeatureOutcome::Blocked {
                reason,
                state: self.state(),
            };
        }
        let demo = self.conn().is_some_and(|c| c.demo);
        let current = self.effective_features();
        let (next, notice) = match rules::decide(current, feature, on) {
            Decision::Apply { next, notice } => (next, notice),
            Decision::Confirm { next, .. } if confirmed => (next, None),
            Decision::Confirm { question, .. } => {
                return FeatureOutcome::Confirm {
                    question: question.text(),
                }
            }
            Decision::Blocked { reason } => {
                return FeatureOutcome::Blocked {
                    reason: reason.text(),
                    state: self.state(),
                }
            }
        };
        if demo {
            let c = self.conn_mut().expect("demo");
            c.features = FeatureState {
                picture_mode_blocks_ai: false,
                ..next
            };
        } else if next.ai_scene_optimization != current.ai_scene_optimization {
            // Hardware: AI Scene Optimization is the only smart feature on the monitor.
            let value = if next.ai_scene_optimization { 2 } else { 1 };
            if let Err(reason) = self.set("ai-scene-optimization", value) {
                return FeatureOutcome::Blocked {
                    reason,
                    state: self.state(),
                };
            }
        }
        FeatureOutcome::Applied {
            state: self.state(),
            notice: notice.map(|n| n.text()),
        }
    }

    /// Switch picture mode through the AI interlock: if AI Scene Optimization is
    /// on and the new mode can't use it, it is turned off first (after asking),
    /// and turned back on if the mode switch then fails.
    /// [`Self::picture_mode`] on the selected monitor, then on the others when
    /// linked. The question is asked about the selected monitor; the answer
    /// counts for all.
    fn picture_mode_all(&mut self, label: &str, confirmed: bool) -> FeatureOutcome {
        let out = self.picture_mode(label, confirmed);
        let FeatureOutcome::Applied { notice, .. } = out else {
            return out;
        };
        let failed = self.mirror(|w| match w.picture_mode(label, true) {
            FeatureOutcome::Blocked { reason, .. } => Err(reason),
            _ => Ok(()),
        });
        FeatureOutcome::Applied {
            state: self.state(),
            notice: Self::with_failures(notice, &failed),
        }
    }

    fn picture_mode(&mut self, label: &str, confirmed: bool) -> FeatureOutcome {
        let demo = self.conn().is_some_and(|c| c.demo);
        let Some(raw) = PROFILE.control("picture-mode").and_then(|c| match c.kind {
            nubless_core::profile::ValueKind::Choice { choices } => {
                choices.iter().find(|ch| ch.label == label).map(|ch| ch.raw)
            }
            _ => None,
        }) else {
            return FeatureOutcome::Blocked {
                reason: format!("{label} isn't a picture mode nubless knows."),
                state: self.state(),
            };
        };
        if !demo && !self.allows("picture-mode") {
            return FeatureOutcome::Blocked {
                reason: "Turn on Labs to switch picture modes from nubless. Every mode's value is known, but nubless hasn't switched one on real hardware yet.".into(),
                state: self.state(),
            };
        }
        let current = self.effective_features();
        let allows_ai = AI_CAPABLE_PICTURE_MODES.contains(&label);
        let next = match rules::decide_picture_mode(current, label, allows_ai) {
            Decision::Apply { next, .. } => next,
            Decision::Confirm { next, .. } if confirmed => next,
            Decision::Confirm { question, .. } => {
                return FeatureOutcome::Confirm {
                    question: question.text(),
                }
            }
            Decision::Blocked { reason } => {
                return FeatureOutcome::Blocked {
                    reason: reason.text(),
                    state: self.state(),
                }
            }
        };
        if demo {
            let c = self.conn_mut().expect("demo");
            c.features = FeatureState {
                picture_mode_blocks_ai: false,
                ..next
            };
            c.demo_picture = Some(label.to_string());
        } else {
            let ai_off_first = current.ai_scene_optimization && !next.ai_scene_optimization;
            if ai_off_first {
                if let Err(reason) = self.set("ai-scene-optimization", 1) {
                    return FeatureOutcome::Blocked {
                        reason,
                        state: self.state(),
                    };
                }
            }
            if let Err(e) = self.set("picture-mode", raw) {
                if ai_off_first {
                    // Put things back the way they were.
                    let _ = self.set("ai-scene-optimization", 2);
                }
                return FeatureOutcome::Blocked {
                    reason: format!("{label} couldn't be applied: {e}"),
                    state: self.state(),
                };
            }
            // Each picture mode has its own brightness, contrast and so on:
            // re-read them right after replying.
            if let Some(c) = self.conn_mut() {
                c.stale = true;
            }
        }
        FeatureOutcome::Applied {
            state: self.state(),
            notice: None,
        }
    }

    fn reset_original(&mut self) -> Result<Outcome, String> {
        self.writes_allowed()?;
        let conn = self.conn_mut().expect("checked");
        let base = conn.baseline.clone().expect("checked");
        let failed: Vec<String> = conn
            .ctl
            .restore(&base)
            .into_iter()
            .filter_map(|(id, r)| r.err().map(|e| format!("{id}: {e}")))
            .collect();
        conn.refresh();
        if conn.demo {
            conn.features = FeatureState::default();
            conn.demo_picture = Some("Personalized Picture".into());
        }
        if failed.is_empty() {
            Ok(Outcome {
                state: self.state(),
                toast: Some("Your original settings are back".into()),
                undoable: false,
            })
        } else {
            Err(format!(
                "Some settings couldn't be restored: {}",
                failed.join("; ")
            ))
        }
    }

    fn state(&mut self) -> AppState {
        let policy = self.policy();
        let lg_switch_running = crate::platform::lg_switch_running();
        let version = env!("CARGO_PKG_VERSION");
        let effective = self.effective_features();
        let locks: BTreeMap<&'static str, String> = [
            ("ai-scene-optimization", Feature::AiSceneOptimization),
            ("ai-picture-mode", Feature::AiPictureMode),
            ("my-display-preset", Feature::MyDisplayPreset),
        ]
        .into_iter()
        .filter_map(|(k, f)| self.feature_lock(f).map(|r| (k, r)))
        .collect();
        let picture_mode = match self.conn() {
            Some(c) if c.demo => c.demo_picture.clone(),
            Some(c) => c.picture_mode().map(str::to_string),
            None => None,
        };
        let features = FeaturesView {
            state: effective,
            availability: rules::availability(effective),
            locks,
            picture_mode,
            ai_capable_modes: AI_CAPABLE_PICTURE_MODES.to_vec(),
        };
        let linked = self.linked;
        let refreshing = self.conn().is_some_and(|c| c.stale);
        let Some(conn) = self.conns.get(self.active) else {
            return AppState {
                phase: "empty",
                demo: false,
                monitors: self.monitors.clone(),
                model: PROFILE.model,
                controls: vec![],
                features,
                baseline: BaselineView {
                    saved: false,
                    values: 0,
                    just_saved: false,
                    error: None,
                    saved_in: None,
                },
                labs: self.labs,
                linked,
                lg_switch_running,
                error: self.error.clone(),
                version,
                refreshing: false,
            };
        };
        let mut controls: Vec<_> = PROFILE
            .controls
            .iter()
            .map(|c| view::view_control(c, conn.value(c.id), policy))
            .collect();
        view::apply_interlocks(&mut controls, &features.availability);
        let baseline = BaselineView {
            saved: conn.baseline.is_some(),
            values: conn.baseline.as_ref().map_or(0, |b| b.values.len()),
            // Kept for the whole connection: taking it on first read let the
            // hidden tray window swallow it before the main window asked.
            just_saved: conn.just_saved,
            error: conn.baseline_error.clone(),
            saved_in: conn
                .baseline
                .as_ref()
                .and_then(|b| b.mode)
                .map(|m| conn.ctl.mode_name(Some(m))),
        };
        AppState {
            phase: "connected",
            demo: conn.demo,
            monitors: self.monitors.clone(),
            model: PROFILE.model,
            controls,
            features,
            baseline,
            labs: self.labs,
            linked,
            lg_switch_running,
            error: self.error.clone(),
            version,
            refreshing,
        }
    }
}

/// What to call each display: the name the OS shows for it ("LG ULTRAGEAR+"),
/// else the model. When several supported monitors share a name, each gets the
/// OS's name for its port ("LG ULTRAGEAR+ (DP-1)"), else a number in the order
/// the system lists them. On macOS the exact System Settings name replaces this
/// once it is known (see `Worker::apply_os_names`).
fn card_names(displays: &[nubless_hw::HwDisplay]) -> Vec<String> {
    let base = |d: &nubless_hw::HwDisplay| {
        d.os.name
            .clone()
            .unwrap_or_else(|| PROFILE.model.to_string())
    };
    let supported: Vec<String> = displays
        .iter()
        .filter(|d| d.matched != Match::No)
        .map(base)
        .collect();
    let mut seen = BTreeMap::<String, usize>::new();
    displays
        .iter()
        .map(|d| {
            let name = base(d);
            if d.matched == Match::No || supported.iter().filter(|n| **n == name).count() < 2 {
                return name;
            }
            let n = seen.entry(name.clone()).or_default();
            *n += 1;
            match &d.os.port_name {
                Some(p) => format!("{name} ({p})"),
                None => format!("{name} ({n})"),
            }
        })
        .collect()
}

/// "LG 27GM950B", or "LG 27GM950B (2)" when there are several (demo monitors).
fn numbered(n: usize, of: usize) -> String {
    if of > 1 {
        format!("{} ({n})", PROFILE.model)
    } else {
        PROFILE.model.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_worker(name: &str) -> Worker {
        demo_workers(name, 1)
    }

    fn demo_workers(name: &str, count: usize) -> Worker {
        let mut w = Worker {
            conns: vec![],
            active: 0,
            monitors: vec![],
            labs: false,
            linked: false,
            preferred: None,
            reopen: Box::new(|_| None),
            silence_wait: Duration::ZERO,
            os_names: Box::new(Vec::new),
            names_pending: false,
            error: None,
            settings_path: std::env::temp_dir()
                .join(format!("nubless-worker-{}-{name}", std::process::id()))
                .join("settings.json"),
            notify: Box::new(|_| {}),
        };
        w.connect_demo(count);
        w
    }

    /// A simulated monitor treated like real hardware (no demo shortcuts).
    fn hardware_worker(name: &str, labs: bool) -> Worker {
        let mut w = demo_worker(name);
        w.labs = labs;
        let policy = w.policy();
        let c = w.conn_mut().unwrap();
        c.demo = false;
        c.ctl.set_policy(policy);
        c.refresh();
        w
    }

    fn value(w: &mut Worker, id: &str) -> Option<u16> {
        w.state()
            .controls
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.value)
    }

    #[test]
    fn writes_update_the_cached_state() {
        let mut w = demo_worker("set");
        let out = w.set("brightness", 42).unwrap();
        assert!(out.undoable);
        assert_eq!(value(&mut w, "brightness"), Some(42));
    }

    /// A simulated monitor the test can keep poking after the worker owns it.
    #[derive(Clone)]
    struct Shared(
        std::sync::Arc<std::sync::Mutex<MockMonitor>>,
        std::sync::Arc<std::sync::atomic::AtomicU32>,
    );
    impl Transport for Shared {
        fn capabilities(&mut self) -> Result<String, nubless_core::transport::TransportError> {
            self.0.lock().unwrap().capabilities()
        }
        fn get_vcp(
            &mut self,
            code: u8,
        ) -> Result<nubless_core::transport::VcpReading, nubless_core::transport::TransportError>
        {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.0.lock().unwrap().get_vcp(code)
        }
        fn set_vcp(
            &mut self,
            w: &nubless_core::safety::ValidatedWrite,
        ) -> Result<(), nubless_core::transport::TransportError> {
            self.0.lock().unwrap().set_vcp(w)
        }
    }

    fn worker_on(shared: Shared, name: &str) -> Worker {
        let mut w = demo_worker(name);
        let ctl = Controller::new(
            Box::new(shared) as Box<dyn Transport>,
            &PROFILE,
            WritePolicy::Experimental,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        let c = w.conn_mut().unwrap();
        c.demo = false;
        c.ctl = ctl;
        w.labs = true;
        w.conn_mut().unwrap().refresh();
        w
    }

    fn shared() -> Shared {
        Shared(Default::default(), Default::default())
    }

    #[test]
    fn a_quiet_monitor_does_not_grey_out_the_app() {
        let s = shared();
        let mut w = worker_on(s.clone(), "quiet");
        assert_eq!(value(&mut w, "brightness"), Some(80));
        // The panel is resyncing: nothing answers for a while.
        s.0.lock().unwrap().faults.offline = true;
        w.conn_mut().unwrap().refresh();
        let state = w.state();
        assert_eq!(value(&mut w, "brightness"), Some(80), "known values stay");
        assert!(
            state
                .controls
                .iter()
                .filter(|c| c.value.is_some())
                .all(|c| c.enabled || c.reason.is_some()),
            "nothing is disabled just because the monitor was quiet"
        );
        let brightness = state
            .controls
            .iter()
            .find(|c| c.id == "brightness")
            .unwrap();
        assert!(brightness.enabled);
        // And when it answers again, values update.
        {
            let mut m = s.0.lock().unwrap();
            m.faults.offline = false;
            m.values.get_mut(&0x10).unwrap().current = 33;
        }
        w.conn_mut().unwrap().refresh();
        assert_eq!(value(&mut w, "brightness"), Some(33));
    }

    #[test]
    fn a_quiet_monitor_does_not_hold_up_the_thread() {
        let s = shared();
        let mut w = worker_on(s.clone(), "quiet-fast");
        s.0.lock().unwrap().faults.offline = true;
        s.1.store(0, std::sync::atomic::Ordering::SeqCst);
        w.conn_mut().unwrap().refresh();
        let reads = s.1.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            reads <= 4 * 3,
            "gave up after a few controls, not all of them: {reads} reads"
        );
    }

    #[test]
    fn only_risky_controls_ask_to_be_kept() {
        let mut w = hardware_worker("keep", true);
        let asks: Vec<_> = w
            .state()
            .controls
            .iter()
            .filter(|c| c.confirm)
            .map(|c| c.id)
            .collect();
        for id in [
            "contrast",
            "black-stabilizer",
            "response-time",
            "adaptive-sync",
        ] {
            assert!(asks.contains(&id), "{id} must ask");
        }
        for id in [
            "brightness",
            "volume",
            "mute",
            "ai-sound",
            "ai-scene-optimization",
        ] {
            assert!(!asks.contains(&id), "{id} must not ask");
        }
        // Picture mode has its own question (the AI interlock) and is never set through `set`.
        assert!(!asks.contains(&"picture-mode"));
    }

    #[test]
    fn an_unkept_change_is_put_back() {
        let mut w = hardware_worker("revert", true);
        let before = value(&mut w, "response-time").unwrap();
        w.set("response-time", 3).unwrap();
        let out = w.revert("response-time", before).unwrap();
        assert!(!out.undoable);
        assert_eq!(value(&mut w, "response-time"), Some(before));
    }

    #[test]
    fn revert_can_restore_a_value_below_the_safe_range() {
        // The user's own contrast was 10 (below the 20 floor sliders allow).
        // Putting it back must still work, or "Revert" could leave the screen worse.
        let mut w = hardware_worker("revert-floor", false);
        w.conn_mut().unwrap().ctl.snapshot();
        assert!(w.set("contrast", 5).is_err());
        w.set("contrast", 40).unwrap();
        w.revert("contrast", 10).unwrap();
        assert_eq!(value(&mut w, "contrast"), Some(10));
    }

    #[test]
    fn revert_is_refused_without_saved_original_settings() {
        let mut w = hardware_worker("revert-none", true);
        w.conn_mut().unwrap().baseline = None;
        assert!(w.revert("response-time", 2).is_err());
    }

    #[test]
    fn values_outside_the_safe_range_are_refused() {
        let mut w = demo_worker("floor");
        assert!(w.set("contrast", 5).is_err());
        assert_eq!(value(&mut w, "contrast"), Some(70));
    }

    #[test]
    fn labs_never_unlocks_unconfirmed_controls() {
        // Nothing is in Labs now: confirmed controls need no switch, and an
        // unconfirmed one stays read-only with Labs on.
        let mut w = demo_worker("labs");
        assert!(w.set("response-time", 1).is_ok());
        assert!(w.set("ai-upscaling", 2).is_ok());
        assert!(w.set("color-preset", 3).is_ok());
        assert!(w.set("red-gain", 60).is_err());
        let (tx, rx) = channel();
        w.handle(Req::SetLabs(true, tx));
        assert!(rx.recv().unwrap().labs);
        assert!(w.set("red-gain", 60).is_err());
    }

    #[test]
    fn motion_blur_is_locked_until_adaptive_sync_is_off() {
        let mut w = demo_worker("motion-blur");
        let view = |w: &mut Worker| {
            w.state()
                .controls
                .into_iter()
                .find(|c| c.id == "motion-blur")
                .unwrap()
        };
        // The demo monitor starts on Adaptive-Sync Extended.
        let locked = view(&mut w);
        assert!(!locked.enabled);
        assert_eq!(
            locked.reason,
            Some("Motion Blur Reduction only works while Adaptive-Sync is off. Turn Adaptive-Sync off first.")
        );
        assert!(w.set("motion-blur", 1).is_err());
        w.set("adaptive-sync", 0).unwrap();
        assert!(view(&mut w).enabled);
        w.set("motion-blur", 1).unwrap();
        assert_eq!(view(&mut w).value, Some(1));
        let out = w.set("adaptive-sync", 3).unwrap();
        assert_eq!(
            out.toast.as_deref(),
            Some("Adaptive-Sync/VRR updated. Motion Blur Reduction turned off with it.")
        );
        let blur = view(&mut w);
        assert_eq!((blur.value, blur.enabled), (Some(2), false));
    }

    #[test]
    fn motion_blur_does_not_ask_to_be_kept() {
        let mut w = demo_worker("motion-blur-keep");
        let blur = w
            .state()
            .controls
            .into_iter()
            .find(|c| c.id == "motion-blur")
            .unwrap();
        assert!(!blur.confirm, "it can't hide the picture");
    }

    #[test]
    fn a_refused_motion_blur_write_rereads_adaptive_sync() {
        let mut w = demo_worker("motion-blur-stale");
        w.set("adaptive-sync", 0).unwrap();
        // Adaptive-Sync is turned on in the monitor's own menu: the cache is stale.
        w.conn_mut().unwrap().ctl.write("adaptive-sync", 2).unwrap();
        let err = w.set("motion-blur", 1).unwrap_err();
        assert!(err.contains("Turn Adaptive-Sync off first"), "{err}");
        let locked = w
            .state()
            .controls
            .into_iter()
            .find(|c| c.id == "motion-blur")
            .unwrap();
        assert!(!locked.enabled, "the lock follows what the monitor has");
    }

    #[test]
    fn interlocks_ask_then_apply() {
        let mut w = demo_worker("interlock");
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, true, false),
            FeatureOutcome::Applied { .. }
        ));
        match w.feature(Feature::AiPictureMode, true, false) {
            FeatureOutcome::Confirm { question } => {
                assert!(question.contains("AI Scene Optimization"))
            }
            other => panic!("expected a question, got {other:?}"),
        }
        assert!(
            w.conn().unwrap().features.ai_scene_optimization,
            "nothing changes before the answer"
        );
        assert!(matches!(
            w.feature(Feature::AiPictureMode, true, true),
            FeatureOutcome::Applied { .. }
        ));
        assert!(
            w.conn().unwrap().features.ai_picture_mode
                && !w.conn().unwrap().features.ai_scene_optimization
        );
    }

    #[test]
    fn calibration_modes_and_ai_dont_mix() {
        let mut w = demo_worker("picmode");
        w.feature(Feature::AiSceneOptimization, true, false);
        assert!(matches!(
            w.picture_mode("Calibration1", false),
            FeatureOutcome::Confirm { .. }
        ));
        assert_eq!(
            w.conn().unwrap().demo_picture.as_deref(),
            Some("Personalized Picture")
        );
        w.picture_mode("Calibration1", true);
        assert!(!w.conn().unwrap().features.ai_scene_optimization);
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, true, false),
            FeatureOutcome::Blocked { .. }
        ));
        assert!(matches!(
            w.picture_mode("Gamer 1", false),
            FeatureOutcome::Applied { .. }
        ));
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, true, false),
            FeatureOutcome::Applied { .. }
        ));
    }

    #[test]
    fn pbp_blocks_smart_features() {
        let mut w = demo_worker("pbp");
        let (tx, rx) = channel();
        w.handle(Req::SimulatePbp(true, tx));
        rx.recv().unwrap();
        assert!(matches!(
            w.feature(Feature::MyDisplayPreset, true, false),
            FeatureOutcome::Blocked { .. }
        ));
        let upscaling = w
            .state()
            .controls
            .into_iter()
            .find(|c| c.id == "ai-upscaling")
            .unwrap();
        assert!(!upscaling.enabled);
        assert_eq!(
            upscaling.reason,
            Some("AI Upscaling is not available while PBP/PIP is on.")
        );
    }

    #[test]
    fn hardware_ai_scene_works_without_labs() {
        let mut w = hardware_worker("hw-safe", false);
        assert!(
            w.state().features.state.ai_scene_optimization,
            "read without Labs"
        );
        assert!(!w
            .state()
            .features
            .locks
            .contains_key("ai-scene-optimization"));
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, false, false),
            FeatureOutcome::Applied { .. }
        ));
    }

    #[test]
    fn hardware_ai_scene_is_read_and_switched_over_the_ai_channel() {
        let mut w = hardware_worker("hw-labs", true);
        assert!(
            w.state().features.state.ai_scene_optimization,
            "mock reports on"
        );
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, false, false),
            FeatureOutcome::Applied { .. }
        ));
        assert!(!w.state().features.state.ai_scene_optimization);
        assert!(matches!(
            w.feature(Feature::AiPictureMode, true, false),
            FeatureOutcome::Blocked { .. }
        ));
    }

    #[test]
    fn hardware_ai_scene_respects_the_picture_mode() {
        let mut w = hardware_worker("hw-cal", true);
        w.conn_mut()
            .unwrap()
            .values
            .insert("picture-mode", Some(18)); // Calibration1
        w.feature(Feature::AiSceneOptimization, false, false);
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, true, false),
            FeatureOutcome::Blocked { .. }
        ));
    }

    #[test]
    fn no_writes_without_saved_original_settings() {
        let mut w = demo_worker("nobase");
        w.conn_mut().unwrap().baseline = None;
        let err = w.set("brightness", 10).unwrap_err();
        assert!(err.contains("original settings"), "{err}");
        assert!(w.reset_original().is_err());
    }

    #[test]
    fn reset_brings_back_the_original_settings() {
        let mut w = demo_worker("reset");
        w.set("brightness", 10).unwrap();
        w.set("contrast", 90).unwrap();
        w.reset_original().unwrap();
        assert_eq!(value(&mut w, "brightness"), Some(80));
        assert_eq!(value(&mut w, "contrast"), Some(70));
    }

    #[test]
    fn slider_drags_collapse_to_the_last_value() {
        let mut w = demo_worker("drag");
        let (tx, rx) = channel();
        let mut replies = vec![];
        for v in [70, 60, 50] {
            let (rtx, rrx) = channel();
            tx.send(Req::Set {
                monitor: None,
                id: "brightness".into(),
                value: v,
                reply: rtx,
            })
            .unwrap();
            replies.push(rrx);
        }
        drop(tx);
        w.run(rx);
        let outs: Vec<_> = replies
            .into_iter()
            .map(|r| r.recv().unwrap().unwrap())
            .collect();
        assert!(
            !outs[0].undoable && !outs[1].undoable,
            "superseded values are skipped"
        );
        assert!(outs[2].undoable);
        assert_eq!(value(&mut w, "brightness"), Some(50));
        let journal = &w.conn().unwrap().ctl.journal().entries;
        assert_eq!(journal.len(), 1, "only one write reached the monitor");
    }

    /// `n` presses of the same hotkey, as a held key delivers them.
    fn presses(action: Action, n: usize) -> Vec<Req> {
        (0..n).map(|_| Req::Hotkey(action)).collect()
    }

    /// Feed requests to the worker the way the app does, then stop.
    fn run_batch(w: &mut Worker, reqs: Vec<Req>) {
        let (tx, rx) = channel();
        for r in reqs {
            tx.send(r).unwrap();
        }
        drop(tx);
        w.run(rx);
    }

    #[test]
    fn held_brightness_keys_become_one_write() {
        let mut w = hardware_worker("hk-bright", false);
        let before = value(&mut w, "brightness").unwrap();
        run_batch(&mut w, presses(Action::BrightnessDown, 3));
        assert_eq!(
            value(&mut w, "brightness"),
            Some(before - 3 * BRIGHTNESS_STEP as u16)
        );
        let journal = &w.conn().unwrap().ctl.journal().entries;
        assert_eq!(journal.len(), 1, "three presses, one write");
    }

    #[test]
    fn brightness_keys_stop_at_the_safe_range() {
        let mut w = hardware_worker("hk-clamp", false);
        run_batch(&mut w, presses(Action::BrightnessUp, 40));
        assert_eq!(value(&mut w, "brightness"), Some(100));
    }

    #[test]
    fn next_picture_mode_skips_modes_that_would_need_a_question() {
        let mut w = hardware_worker("hk-mode", false);
        assert!(
            w.state().features.state.ai_scene_optimization,
            "mock reports AI on"
        );
        let order: Vec<_> = (0..4)
            .map(|_| {
                run_batch(&mut w, vec![Req::Hotkey(Action::NextPictureMode)]);
                w.state().features.picture_mode.unwrap()
            })
            .collect();
        // From Personalized Picture, with AI on: only the AI-capable modes.
        assert_eq!(order, ["Gamer 1", "Gamer 2", "FPS", "Personalized Picture"]);
        assert!(
            w.state().features.state.ai_scene_optimization,
            "AI was never switched off"
        );
    }

    #[test]
    fn hardware_picture_modes_work_without_labs() {
        let mut w = hardware_worker("pm-safe", false);
        assert!(matches!(
            w.picture_mode("Gamer 1", false),
            FeatureOutcome::Applied { .. }
        ));
        assert_eq!(value(&mut w, "picture-mode"), Some(45));
        assert!(
            w.set("red-gain", 60).is_err(),
            "unconfirmed controls are never writable"
        );
    }

    #[test]
    fn hardware_switch_to_calibration_turns_ai_off_first() {
        let mut w = hardware_worker("pm-cal", true);
        assert!(w.state().features.state.ai_scene_optimization);
        assert!(matches!(
            w.picture_mode("Calibration1", false),
            FeatureOutcome::Confirm { .. }
        ));
        assert_eq!(
            value(&mut w, "picture-mode"),
            Some(65),
            "nothing sent before the answer"
        );
        assert!(matches!(
            w.picture_mode("Calibration1", true),
            FeatureOutcome::Applied { .. }
        ));
        let s = w.state();
        assert_eq!(s.features.picture_mode.as_deref(), Some("Calibration1"));
        assert!(!s.features.state.ai_scene_optimization);
        assert!(s.features.state.picture_mode_blocks_ai);
        // Back to an AI mode: AI Scene Optimization can be turned on again.
        assert!(matches!(
            w.picture_mode("Gamer 1", false),
            FeatureOutcome::Applied { .. }
        ));
        assert!(matches!(
            w.feature(Feature::AiSceneOptimization, true, false),
            FeatureOutcome::Applied { .. }
        ));
    }

    #[test]
    fn picture_switch_replies_first_then_refreshes_and_notifies() {
        let mut w = hardware_worker("pm-bg", true);
        let notified = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let n = notified.clone();
        w.notify = Box::new(move |s| {
            assert!(!s.refreshing);
            n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let (tx, rx) = channel();
        let (rtx, rrx) = channel();
        tx.send(Req::PictureMode {
            monitor: None,
            label: "Gamer 1".into(),
            confirmed: false,
            reply: rtx,
        })
        .unwrap();
        drop(tx);
        w.run(rx);
        match rrx.recv().unwrap() {
            FeatureOutcome::Applied { state, .. } => {
                assert!(state.refreshing, "reply comes before the re-read")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(notified.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(!w.refresh_pending());
    }

    fn card(w: &Worker, n: usize) -> String {
        w.conns[n].card.clone()
    }

    fn select(w: &mut Worker, n: usize) {
        let (tx, rx) = channel();
        w.handle(Req::Select(card(w, n), tx));
        rx.recv().unwrap().unwrap();
    }

    #[test]
    fn identical_monitors_get_their_own_names_and_the_first_is_selected() {
        let mut w = demo_workers("multi-names", 3);
        let s = w.state();
        let names: Vec<_> = s.monitors.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert!(
            names[1].contains("(2)") && names[2].contains("(3)"),
            "{names:?}"
        );
        assert_eq!(s.monitors.iter().filter(|m| m.selected).count(), 1);
        assert!(s.monitors[0].selected);
    }

    #[test]
    fn changes_go_to_the_selected_monitor_only() {
        let mut w = demo_workers("multi-select", 3);
        select(&mut w, 1);
        assert!(w.state().monitors[1].selected);
        w.set_all("brightness", 30).unwrap();
        assert_eq!(value(&mut w, "brightness"), Some(30));
        select(&mut w, 0);
        assert_eq!(
            value(&mut w, "brightness"),
            Some(80),
            "monitor 1 is untouched"
        );
        select(&mut w, 2);
        assert_eq!(
            value(&mut w, "brightness"),
            Some(80),
            "monitor 3 is untouched"
        );
        // Undo works on the selected monitor's own history.
        assert!(w.undo().is_err(), "nothing was changed on monitor 3");
        select(&mut w, 1);
        w.undo().unwrap();
        assert_eq!(value(&mut w, "brightness"), Some(80));
    }

    #[test]
    fn selecting_an_unknown_monitor_is_refused() {
        let mut w = demo_workers("multi-unknown", 2);
        let (tx, rx) = channel();
        w.handle(Req::Select("no-such-monitor".into(), tx));
        assert!(rx.recv().unwrap().is_err());
        assert!(w.state().monitors[0].selected, "selection unchanged");
    }

    #[test]
    fn linked_monitors_change_together() {
        let mut w = demo_workers("multi-linked", 3);
        let (tx, rx) = channel();
        w.handle(Req::SetLinked(true, tx));
        assert!(rx.recv().unwrap().linked);
        select(&mut w, 1);
        w.set_all("brightness", 55).unwrap();
        assert!(matches!(
            w.picture_mode_all("Gamer 1", false),
            FeatureOutcome::Applied { .. }
        ));
        for n in 0..3 {
            select(&mut w, n);
            assert_eq!(value(&mut w, "brightness"), Some(55), "monitor {n}");
            assert_eq!(
                w.state().features.picture_mode.as_deref(),
                Some("Gamer 1"),
                "monitor {n}"
            );
        }
        assert!(
            w.state().monitors[2].selected,
            "mirroring leaves the selection alone"
        );
        // Undo is linked too: each monitor undoes its own last change.
        let (tx, rx) = channel();
        w.handle(Req::Undo {
            monitor: None,
            reply: tx,
        });
        rx.recv().unwrap().unwrap();
        for n in 0..3 {
            select(&mut w, n);
            assert_eq!(value(&mut w, "brightness"), Some(80), "monitor {n}");
        }
    }

    #[test]
    fn a_linked_monitor_that_refuses_is_named_and_does_not_stop_the_rest() {
        let mut w = demo_workers("multi-refuse", 3);
        w.linked = true;
        w.conns[1].baseline = None;
        w.conns[1].baseline_error = Some("disk full".into());
        let out = w.set_all("brightness", 40).unwrap();
        let toast = out.toast.unwrap();
        assert!(
            toast.contains(&w.conns[1].name) && toast.contains("disk full"),
            "{toast}"
        );
        select(&mut w, 2);
        assert_eq!(value(&mut w, "brightness"), Some(40));
        select(&mut w, 1);
        assert_eq!(value(&mut w, "brightness"), Some(80));
    }

    #[test]
    fn linked_hotkeys_line_the_monitors_up() {
        let mut w = demo_workers("multi-hotkeys", 2);
        w.linked = true;
        select(&mut w, 1);
        w.set("brightness", 50).unwrap(); // only monitor 2, before linking mattered
        select(&mut w, 0);
        run_batch(&mut w, presses(Action::BrightnessUp, 1));
        let up = 80 + BRIGHTNESS_STEP as u16;
        assert_eq!(value(&mut w, "brightness"), Some(up));
        select(&mut w, 1);
        assert_eq!(
            value(&mut w, "brightness"),
            Some(up),
            "set to the same value"
        );
    }

    #[test]
    fn older_settings_files_still_load() {
        let old: Settings = serde_json::from_str(r#"{"labs":true}"#).unwrap();
        assert!(old.labs && !old.linked && old.selected.is_none());
    }

    // ---- A desk of real-looking monitors: handles that go stale, monitors the
    // OS loses and finds again (what a VRR switch does).

    use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
    use std::sync::{Arc, Mutex};

    /// One open connection to a monitor. `dead` is this handle only: after the
    /// OS re-detects monitors the old handle stops working, a new one works.
    struct Link {
        mon: Arc<Mutex<MockMonitor>>,
        dead: Arc<AtomicBool>,
    }

    impl Transport for Link {
        fn capabilities(&mut self) -> Result<String, nubless_core::transport::TransportError> {
            if self.dead.load(SeqCst) {
                return Err(nubless_core::transport::TransportError::Io(
                    "stale handle".into(),
                ));
            }
            self.mon.lock().unwrap().capabilities()
        }
        fn get_vcp(
            &mut self,
            code: u8,
        ) -> Result<nubless_core::transport::VcpReading, nubless_core::transport::TransportError>
        {
            if self.dead.load(SeqCst) {
                return Err(nubless_core::transport::TransportError::NoReply(code));
            }
            self.mon.lock().unwrap().get_vcp(code)
        }
        fn set_vcp(
            &mut self,
            w: &nubless_core::safety::ValidatedWrite,
        ) -> Result<(), nubless_core::transport::TransportError> {
            if self.dead.load(SeqCst) {
                return Err(nubless_core::transport::TransportError::Io(
                    "stale handle".into(),
                ));
            }
            self.mon.lock().unwrap().set_vcp(w)
        }
    }

    impl Worker {
        /// One request through the same loop the app uses.
        fn handle_for_test(&mut self, req: Req) {
            let (tx, rx) = channel();
            tx.send(req).unwrap();
            drop(tx);
            self.run(rx);
        }
    }

    struct Desk {
        mons: Vec<Arc<Mutex<MockMonitor>>>,
        /// The handle each monitor's connection is using right now.
        handles: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
        /// Whether the OS can see each monitor (a reopen finds it).
        present: Arc<Mutex<Vec<bool>>>,
        /// Reopens asked for, by monitor.
        reopened: Arc<Mutex<Vec<u32>>>,
    }

    impl Desk {
        /// Changes sent to a monitor. Reading LG's AI channel is itself a write
        /// (a query, low byte 0xFF), so those don't count.
        fn writes(&self, n: usize) -> usize {
            let m = self.mons[n].lock().unwrap();
            m.writes
                .iter()
                .filter(|(code, v)| *code != 0xA6 || v & 0xFF != 0xFF)
                .count()
        }
        fn kill_handle(&self, n: usize) {
            self.handles.lock().unwrap()[n].store(true, SeqCst);
        }
    }

    /// `n` identical monitors, each with its own serial, port and folder, on
    /// a worker that treats them as real hardware.
    fn desk(name: &str, n: usize) -> (Worker, Desk) {
        let mut w = demo_workers(name, n);
        let root = std::env::temp_dir().join(format!("nubless-desk-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut folders = Folders::in_dir(&root);
        let d = Desk {
            mons: (0..n)
                .map(|_| Arc::new(Mutex::new(MockMonitor::gm950b())))
                .collect(),
            handles: Arc::new(Mutex::new(vec![])),
            present: Arc::new(Mutex::new(vec![true; n])),
            reopened: Arc::new(Mutex::new(vec![0; n])),
        };
        let identity = |i: usize| Identity::new(Some(&format!("SN{i}")), None, format!("DP-{i}"));
        for i in 0..n {
            let dead = Arc::new(AtomicBool::new(false));
            d.handles.lock().unwrap().push(dead.clone());
            let folder = folders.claim(&identity(i)).unwrap();
            let meta = Meta {
                card: folder.file_name().unwrap().to_string_lossy().into_owned(),
                name: format!("LG ULTRAGEAR+ (DP-{i})"),
                identity: Some(identity(i)),
                folder: Ok(folder),
                display_id: None,
            };
            let link = Link {
                mon: d.mons[i].clone(),
                dead,
            };
            w.conns[i] =
                Worker::connect(Box::new(link), meta, WritePolicy::Safe, Timing::instant())
                    .unwrap();
        }
        let (mons, handles, present, reopened) = (
            d.mons.clone(),
            d.handles.clone(),
            d.present.clone(),
            d.reopened.clone(),
        );
        w.reopen = Box::new(move |id| {
            let i = (0..mons.len()).find(|&i| identity(i) == *id)?;
            reopened.lock().unwrap()[i] += 1;
            if !present.lock().unwrap()[i] {
                return None;
            }
            let dead = Arc::new(AtomicBool::new(false));
            handles.lock().unwrap()[i] = dead.clone();
            Some(Box::new(Link {
                mon: mons[i].clone(),
                dead,
            }) as Box<dyn Transport>)
        });
        w.monitors = w
            .conns
            .iter()
            .map(|c| MonitorCard {
                id: c.card.clone(),
                name: c.name.clone(),
                detail: String::new(),
                port: None,
                supported: true,
                selected: false,
                demo: false,
            })
            .collect();
        w.mark_selected();
        (w, d)
    }

    fn set_on(w: &mut Worker, n: Option<usize>, id: &str, value: u16) -> Result<Outcome, String> {
        let monitor = n.map(|n| card(w, n));
        let (tx, rx) = channel();
        w.handle_for_test(Req::Set {
            monitor,
            id: id.into(),
            value,
            reply: tx,
        });
        rx.recv().unwrap()
    }

    #[test]
    fn a_stale_handle_is_reopened_and_the_change_still_goes_through() {
        let (mut w, d) = desk("stale", 3);
        // Another monitor's VRR switch made the OS re-detect this one.
        d.kill_handle(0);
        let out = set_on(&mut w, None, "brightness", 40).unwrap();
        assert!(out.undoable);
        assert_eq!(d.mons[0].lock().unwrap().values[&0x10].current, 40);
        assert_eq!(
            *d.reopened.lock().unwrap(),
            vec![1, 0, 0],
            "only that monitor"
        );
        assert_eq!(d.writes(1) + d.writes(2), 0, "the others are untouched");
    }

    #[test]
    fn selecting_a_monitor_whose_handle_went_stale_reopens_it() {
        let (mut w, d) = desk("stale-select", 2);
        d.mons[1]
            .lock()
            .unwrap()
            .values
            .get_mut(&0x10)
            .unwrap()
            .current = 33;
        d.kill_handle(1);
        select(&mut w, 1);
        assert_eq!(
            value(&mut w, "brightness"),
            Some(33),
            "fresh values, not stale ones"
        );
        assert_eq!(*d.reopened.lock().unwrap(), vec![0, 1]);
    }

    #[test]
    fn a_vrr_switch_that_drops_the_monitor_finds_the_same_one_again() {
        let (mut w, d) = desk("vrr", 3);
        select(&mut w, 1);
        let card_before = card(&w, 1);
        let folder_before = w.conns[1].folder.clone();
        // The panel resyncs: nothing answers for longer than the read-back waits,
        // and the OS hands out a new handle for the monitor.
        d.mons[1]
            .lock()
            .unwrap()
            .faults
            .quiet_after_write
            .insert(0xF8, 40);
        let out = set_on(&mut w, None, "adaptive-sync", 0).unwrap();
        assert!(out.undoable, "{:?}", out.toast);
        assert_eq!(d.mons[1].lock().unwrap().values[&0xF8].current, 0);
        let reopened = d.reopened.lock().unwrap().clone();
        assert!(reopened[1] >= 1, "{reopened:?}");
        assert_eq!(reopened[0] + reopened[2], 0, "the others are left alone");
        // Same monitor, same card, same folder, still selected.
        assert_eq!(card(&w, 1), card_before);
        assert_eq!(w.conns[1].folder, folder_before);
        assert!(w.state().monitors[1].selected);
        // Its journal survived the reconnect: Undo puts VRR back.
        d.mons[1].lock().unwrap().faults.quiet_after_write.clear();
        w.undo().unwrap();
        assert_eq!(d.mons[1].lock().unwrap().values[&0xF8].current, 3);
    }

    #[test]
    fn original_settings_survive_a_reconnect() {
        let (mut w, d) = desk("vrr-baseline", 2);
        set_on(&mut w, None, "brightness", 30).unwrap();
        d.kill_handle(0);
        set_on(&mut w, None, "brightness", 35).unwrap();
        let base = w.conns[0].baseline.as_ref().unwrap();
        assert_eq!(
            base.values["brightness"], 80,
            "still the value from before nubless"
        );
    }

    #[test]
    fn a_monitor_that_never_comes_back_is_reported_and_the_others_keep_working() {
        let (mut w, d) = desk("gone", 2);
        d.mons[0]
            .lock()
            .unwrap()
            .faults
            .quiet_after_write
            .insert(0xF8, 10_000);
        d.present.lock().unwrap()[0] = false;
        let err = set_on(&mut w, None, "adaptive-sync", 0).unwrap_err();
        assert!(err.contains("has not come back"), "{err}");
        assert_eq!(
            w.state().monitors.len(),
            2,
            "nothing disappears from the list"
        );
        set_on(&mut w, Some(1), "brightness", 50).unwrap();
        assert_eq!(d.mons[1].lock().unwrap().values[&0x10].current, 50);
    }

    #[test]
    fn a_change_lands_on_the_monitor_the_window_showed_not_the_selected_one() {
        // The tray selected monitor 2 while the main window still showed monitor 1.
        let (mut w, d) = desk("aimed", 3);
        select(&mut w, 1);
        let out = set_on(&mut w, Some(0), "brightness", 25).unwrap();
        assert_eq!(d.mons[0].lock().unwrap().values[&0x10].current, 25);
        assert_eq!(d.writes(1), 0);
        assert!(
            out.state.monitors[1].selected,
            "the selection is left alone"
        );
        let shown = out
            .state
            .controls
            .iter()
            .find(|c| c.id == "brightness")
            .unwrap();
        assert_eq!(
            shown.value,
            Some(80),
            "the state shows the selected monitor"
        );
        // "Keep this change?" timing out reverts the right monitor too.
        let (tx, rx) = channel();
        w.handle_for_test(Req::Revert {
            monitor: Some(card(&w, 0)),
            id: "brightness".into(),
            value: 80,
            reply: tx,
        });
        rx.recv().unwrap().unwrap();
        assert_eq!(d.mons[0].lock().unwrap().values[&0x10].current, 80);
        assert_eq!(d.writes(1), 0);
    }

    #[test]
    fn a_change_for_a_monitor_that_is_gone_is_refused() {
        let (mut w, d) = desk("aimed-gone", 2);
        let (tx, rx) = channel();
        w.handle_for_test(Req::Set {
            monitor: Some("unplugged".into()),
            id: "brightness".into(),
            value: 10,
            reply: tx,
        });
        assert!(rx.recv().unwrap().is_err());
        assert_eq!(d.writes(0) + d.writes(1), 0, "nothing was sent anywhere");
    }

    #[test]
    fn a_monitor_without_its_own_folder_is_never_changed() {
        let (mut w, d) = desk("ambiguous", 2);
        w.conns[1].folder = None;
        w.conns[1].baseline = None;
        w.conns[1].baseline_error =
            Some("this monitor can't be told apart from another connected one".into());
        assert!(set_on(&mut w, Some(1), "brightness", 10).is_err());
        assert_eq!(d.writes(1), 0);
    }
}
