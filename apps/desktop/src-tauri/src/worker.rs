//! One thread owns the monitor connection. Every request from the UI is a
//! message to it, so DDC/CI traffic is serialised and paced in one place, and
//! slider drags collapse to the latest value instead of flooding the monitor.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use nubless_core::baseline;
use nubless_core::controller::Snapshot;
use nubless_core::journal::Journal;
use nubless_core::mock::MockMonitor;
use nubless_core::paths;
use nubless_core::profile::{Match, WritePolicy};
use nubless_core::profiles::gm950b::{AI_CAPABLE_PICTURE_MODES, PROFILE};
use nubless_core::rules::{self, Decision, Feature, FeatureState};
use nubless_core::transport::Transport;
use nubless_core::{Controller, Timing};
use serde::Serialize;

use crate::view::{self, AppState, BaselineView, FeaturesView, MonitorCard};

type Reply<T> = Sender<T>;

pub enum Req {
    State(Reply<AppState>),
    Rescan(Reply<AppState>),
    UseDemo(Reply<AppState>),
    SetLabs(bool, Reply<AppState>),
    Set {
        id: String,
        value: u16,
        reply: Reply<Result<Outcome, String>>,
    },
    Feature {
        feature: Feature,
        on: bool,
        confirmed: bool,
        reply: Reply<FeatureOutcome>,
    },
    SimulatePbp(bool, Reply<FeatureOutcome>),
    PictureMode {
        label: String,
        confirmed: bool,
        reply: Reply<FeatureOutcome>,
    },
    Undo(Reply<Result<Outcome, String>>),
    ResetOriginal(Reply<Result<Outcome, String>>),
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
    demo: bool,
    baseline: Option<Snapshot>,
    baseline_error: Option<String>,
    just_saved: bool,
    /// Last known value per control id; `None` = unknown (no answer, or reading
    /// needs Labs). Keyed by id because the AI controls share VCP 0xA6.
    values: BTreeMap<&'static str, Option<u16>>,
}

impl Conn {
    fn refresh(&mut self) {
        for c in PROFILE.controls {
            let v = self.ctl.read(c.id).ok().map(|r| r.current);
            self.values.insert(c.id, v);
        }
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
    conn: Option<Conn>,
    monitors: Vec<MonitorCard>,
    /// Demo only: simulated smart-feature state. On hardware it's read from the monitor.
    features: FeatureState,
    /// Demo only: simulated picture mode.
    demo_picture: Option<String>,
    labs: bool,
    error: Option<String>,
    settings_path: PathBuf,
    /// Values need re-reading (after a picture-mode switch). Done after the
    /// reply is sent, so the UI isn't kept waiting on ~25 DDC/CI reads.
    refresh_pending: bool,
    /// Pushes a fresh state to every window (set up by lib.rs).
    notify: Box<dyn Fn(&AppState) + Send>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Settings {
    labs: bool,
}

pub fn spawn(force_demo: bool, notify: Box<dyn Fn(&AppState) + Send>) -> Sender<Req> {
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
                conn: None,
                monitors: vec![],
                features: FeatureState::default(),
                demo_picture: None,
                labs: settings.labs,
                error: None,
                settings_path,
                refresh_pending: false,
                notify,
            };
            if force_demo {
                w.connect_demo();
            } else {
                w.scan();
            }
            w.run(rx);
        })
        .expect("spawn monitor thread");
    tx
}

impl Worker {
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
            for (i, r) in batch.iter().enumerate() {
                if let Req::Set { id, .. } = r {
                    latest.insert(id.clone(), i);
                }
            }
            for (i, req) in batch.into_iter().enumerate() {
                match req {
                    Req::Set { id, value, reply } if latest.get(&id) == Some(&i) => {
                        let _ = reply.send(self.set(&id, value));
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
            if self.refresh_pending {
                self.refresh_pending = false;
                if let Some(c) = &mut self.conn {
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
                if let Some(c) = &mut self.conn {
                    c.refresh();
                }
                let _ = r.send(self.state());
            }
            Req::Rescan(r) => {
                self.scan();
                let _ = r.send(self.state());
            }
            Req::UseDemo(r) => {
                self.connect_demo();
                let _ = r.send(self.state());
            }
            Req::SetLabs(on, r) => {
                self.labs = on;
                let policy = self.policy();
                if let Some(c) = &mut self.conn {
                    c.ctl.set_policy(policy);
                    // Labs changes what can be read (the AI channel needs it).
                    c.refresh();
                }
                if let Ok(text) = serde_json::to_string_pretty(&Settings { labs: on }) {
                    if let Some(dir) = self.settings_path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    let _ = std::fs::write(&self.settings_path, text);
                }
                let _ = r.send(self.state());
            }
            Req::Feature {
                feature,
                on,
                confirmed,
                reply,
            } => {
                let _ = reply.send(self.feature(feature, on, confirmed));
            }
            Req::SimulatePbp(on, reply) => {
                let out = if self.conn.as_ref().is_some_and(|c| c.demo) {
                    let (next, notice) = rules::on_pbp_changed(self.features, on);
                    self.features = next;
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
                label,
                confirmed,
                reply,
            } => {
                let _ = reply.send(self.picture_mode(&label, confirmed));
            }
            Req::Undo(r) => {
                let out = match &mut self.conn {
                    Some(c) => {
                        let r = c.ctl.undo_last().map_err(|e| e.to_string());
                        c.refresh();
                        r
                    }
                    None => Err("No monitor connected.".into()),
                };
                let _ = r.send(out.map(|_| Outcome {
                    state: self.state(),
                    toast: Some("Undone".into()),
                    undoable: false,
                }));
            }
            Req::ResetOriginal(r) => {
                let _ = r.send(self.reset_original());
            }
            Req::Set { .. } => unreachable!("handled in run"),
        }
    }

    fn scan(&mut self) {
        self.error = None;
        let mut displays = nubless_hw::enumerate(&PROFILE);
        self.monitors = displays
            .iter()
            .map(|d| MonitorCard {
                id: d.index,
                name: if d.matched == Match::No {
                    "Other display".into()
                } else {
                    PROFILE.model.into()
                },
                detail: d.description.clone(),
                supported: d.matched != Match::No,
                selected: false,
                demo: false,
            })
            .collect();
        let Some(pos) = displays.iter().position(|d| d.matched != Match::No) else {
            self.conn = None;
            return;
        };
        let display = displays.swap_remove(pos);
        let id = display.index;
        let journal = Journal::open(paths::journal_file()).unwrap_or_default();
        match Controller::new(
            Box::new(display) as Box<dyn Transport>,
            &PROFILE,
            self.policy(),
            Timing::HARDWARE,
            journal,
        ) {
            Ok(mut ctl) => {
                // Capture the user's original settings before anything can be written.
                let (baseline, baseline_error, just_saved) =
                    match baseline::ensure(&paths::baseline_file(), &mut ctl) {
                        Ok((b, saved)) => (Some(b), None, saved),
                        Err(e) => (None, Some(e.to_string()), false),
                    };
                for m in &mut self.monitors {
                    m.selected = m.id == id;
                }
                self.features = FeatureState::default();
                let mut conn = Conn {
                    ctl,
                    demo: false,
                    baseline,
                    baseline_error,
                    just_saved,
                    values: Default::default(),
                };
                conn.refresh();
                self.conn = Some(conn);
            }
            Err(e) => {
                self.conn = None;
                self.error = Some(format!(
                    "Found your {} but couldn't talk to it: {e}",
                    PROFILE.model
                ));
            }
        }
    }

    fn connect_demo(&mut self) {
        self.error = None;
        let mut ctl = Controller::new(
            Box::new(MockMonitor::gm950b()) as Box<dyn Transport>,
            &PROFILE,
            self.policy(),
            Timing::instant(),
            Journal::in_memory(),
        )
        .expect("mock monitor always answers");
        let baseline = Some(ctl.snapshot());
        self.monitors.retain(|m| !m.demo);
        for m in &mut self.monitors {
            m.selected = false;
        }
        self.monitors.push(MonitorCard {
            id: usize::MAX,
            name: format!("{} (demo)", PROFILE.model),
            detail: "Simulated monitor. Nothing is sent to real hardware.".into(),
            supported: true,
            selected: true,
            demo: true,
        });
        self.features = FeatureState::default();
        self.demo_picture = Some("Personalized Picture".into());
        let mut conn = Conn {
            ctl,
            demo: true,
            baseline,
            baseline_error: None,
            // The simulated monitor's settings are captured fresh on every connect.
            just_saved: true,
            values: Default::default(),
        };
        conn.refresh();
        self.conn = Some(conn);
    }

    fn writes_allowed(&self) -> Result<(), String> {
        match &self.conn {
            None => Err("No monitor connected.".into()),
            Some(c) if c.baseline.is_none() => Err(format!(
                "Your original settings couldn't be saved ({}), so nubless won't change anything yet.",
                c.baseline_error.as_deref().unwrap_or("unknown error")
            )),
            Some(_) => Ok(()),
        }
    }

    fn set(&mut self, id: &str, value: u16) -> Result<Outcome, String> {
        self.writes_allowed()?;
        let conn = self.conn.as_mut().expect("checked");
        let control = conn.ctl.control(id).map_err(|e| e.to_string())?;
        let applied = match conn.ctl.write(id, value) {
            Ok(a) => a,
            Err(e) => {
                // The write was refused or rolled back: show what the monitor really has.
                let v = conn.ctl.read(control.id).ok().map(|r| r.current);
                conn.values.insert(control.id, v);
                return Err(e.to_string());
            }
        };
        conn.values.insert(control.id, Some(applied.new));
        let label = control.label;
        let changed = applied.journal_index.is_some();
        Ok(Outcome {
            state: self.state(),
            toast: changed.then(|| format!("{label} updated")),
            undoable: changed,
        })
    }

    /// Smart-feature state: simulated in the demo, read from the monitor otherwise.
    fn effective_features(&self) -> FeatureState {
        let Some(conn) = &self.conn else {
            return self.features;
        };
        if conn.demo {
            let blocks = self
                .demo_picture
                .as_deref()
                .is_some_and(|p| !AI_CAPABLE_PICTURE_MODES.contains(&p));
            return FeatureState {
                picture_mode_blocks_ai: blocks,
                ..self.features
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
        let conn = self.conn.as_ref()?;
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

    fn feature(&mut self, feature: Feature, on: bool, confirmed: bool) -> FeatureOutcome {
        if let Some(reason) = self.feature_lock(feature) {
            return FeatureOutcome::Blocked {
                reason,
                state: self.state(),
            };
        }
        let demo = self.conn.as_ref().is_some_and(|c| c.demo);
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
            self.features = FeatureState {
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
    fn picture_mode(&mut self, label: &str, confirmed: bool) -> FeatureOutcome {
        let demo = self.conn.as_ref().is_some_and(|c| c.demo);
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
            self.features = FeatureState {
                picture_mode_blocks_ai: false,
                ..next
            };
            self.demo_picture = Some(label.to_string());
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
            self.refresh_pending = true;
        }
        FeatureOutcome::Applied {
            state: self.state(),
            notice: None,
        }
    }

    fn reset_original(&mut self) -> Result<Outcome, String> {
        self.writes_allowed()?;
        let conn = self.conn.as_mut().expect("checked");
        let base = conn.baseline.clone().expect("checked");
        let failed: Vec<String> = conn
            .ctl
            .restore(&base)
            .into_iter()
            .filter_map(|(id, r)| r.err().map(|e| format!("{id}: {e}")))
            .collect();
        conn.refresh();
        if self.conn.as_ref().is_some_and(|c| c.demo) {
            self.features = FeatureState::default();
            self.demo_picture = Some("Personalized Picture".into());
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
        let picture_mode = match &self.conn {
            Some(c) if c.demo => self.demo_picture.clone(),
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
        let Some(conn) = &mut self.conn else {
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
                lg_switch_running,
                error: self.error.clone(),
                version,
                refreshing: false,
            };
        };
        let controls = PROFILE
            .controls
            .iter()
            .map(|c| view::view_control(c, conn.value(c.id), policy))
            .collect();
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
            lg_switch_running,
            error: self.error.clone(),
            version,
            refreshing: self.refresh_pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_worker(name: &str) -> Worker {
        let mut w = Worker {
            conn: None,
            monitors: vec![],
            features: FeatureState::default(),
            demo_picture: None,
            labs: false,
            error: None,
            settings_path: std::env::temp_dir()
                .join(format!("nubless-worker-{}-{name}", std::process::id()))
                .join("settings.json"),
            refresh_pending: false,
            notify: Box::new(|_| {}),
        };
        w.connect_demo();
        w
    }

    /// A simulated monitor treated like real hardware (no demo shortcuts).
    fn hardware_worker(name: &str, labs: bool) -> Worker {
        let mut w = demo_worker(name);
        w.labs = labs;
        let policy = w.policy();
        let c = w.conn.as_mut().unwrap();
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

    #[test]
    fn values_outside_the_safe_range_are_refused() {
        let mut w = demo_worker("floor");
        assert!(w.set("contrast", 5).is_err());
        assert_eq!(value(&mut w, "contrast"), Some(70));
    }

    #[test]
    fn labs_gates_strong_controls() {
        let mut w = demo_worker("labs");
        assert!(w.set("response-time", 1).is_err());
        let (tx, rx) = channel();
        w.handle(Req::SetLabs(true, tx));
        assert!(rx.recv().unwrap().labs);
        assert!(w.set("response-time", 1).is_ok());
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
            w.features.ai_scene_optimization,
            "nothing changes before the answer"
        );
        assert!(matches!(
            w.feature(Feature::AiPictureMode, true, true),
            FeatureOutcome::Applied { .. }
        ));
        assert!(w.features.ai_picture_mode && !w.features.ai_scene_optimization);
    }

    #[test]
    fn calibration_modes_and_ai_dont_mix() {
        let mut w = demo_worker("picmode");
        w.feature(Feature::AiSceneOptimization, true, false);
        assert!(matches!(
            w.picture_mode("Calibration1", false),
            FeatureOutcome::Confirm { .. }
        ));
        assert_eq!(w.demo_picture.as_deref(), Some("Personalized Picture"));
        w.picture_mode("Calibration1", true);
        assert!(!w.features.ai_scene_optimization);
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
        w.conn
            .as_mut()
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
        w.conn.as_mut().unwrap().baseline = None;
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
        let journal = &w.conn.as_ref().unwrap().ctl.journal().entries;
        assert_eq!(journal.len(), 1, "only one write reached the monitor");
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
            w.set("response-time", 1).is_err(),
            "unconfirmed controls still need Labs"
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
        assert!(!w.refresh_pending);
    }
}
