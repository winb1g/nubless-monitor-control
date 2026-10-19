//! One thread owns the monitor connection. Every request from the UI is a
//! message to it, so DDC/CI traffic is serialised and paced in one place, and
//! slider drags collapse to the latest value instead of flooding the monitor.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use nubless_core::baseline;
use nubless_core::controller::Snapshot;
use nubless_core::journal::Journal;
use nubless_core::mock::MockMonitor;
use nubless_core::paths;
use nubless_core::profile::{Match, WritePolicy};
use nubless_core::profiles::gm950b::PROFILE;
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
    /// Last known value per VCP code; `None` = the monitor did not answer.
    values: std::collections::BTreeMap<u8, Option<u16>>,
}

impl Conn {
    fn refresh(&mut self) {
        for c in PROFILE.controls {
            let v = self.ctl.read_code(c.vcp).ok().map(|r| r.current);
            self.values.insert(c.vcp, v);
        }
    }
}

struct Worker {
    conn: Option<Conn>,
    monitors: Vec<MonitorCard>,
    features: FeatureState,
    labs: bool,
    error: Option<String>,
    settings_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Settings {
    labs: bool,
}

pub fn spawn(force_demo: bool) -> Sender<Req> {
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
                labs: settings.labs,
                error: None,
                settings_path,
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
                }
                if let Ok(text) = serde_json::to_string_pretty(&Settings { labs: on }) {
                    let _ = std::fs::create_dir_all(paths::state_dir());
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
        let mut conn = Conn {
            ctl,
            demo: true,
            baseline,
            baseline_error: None,
            just_saved: false,
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
                let v = conn.ctl.read_code(control.vcp).ok().map(|r| r.current);
                conn.values.insert(control.vcp, v);
                return Err(e.to_string());
            }
        };
        conn.values.insert(control.vcp, Some(applied.new));
        let label = control.label;
        let changed = applied.journal_index.is_some();
        Ok(Outcome {
            state: self.state(),
            toast: changed.then(|| format!("{label} updated")),
            undoable: changed,
        })
    }

    fn feature(&mut self, feature: Feature, on: bool, confirmed: bool) -> FeatureOutcome {
        let demo = self.conn.as_ref().is_some_and(|c| c.demo);
        if !demo {
            return FeatureOutcome::Blocked {
                reason: format!("{} isn't mapped yet, so nubless can't switch it. Use the demo monitor to try how it will work.", feature.label()),
                state: self.state(),
            };
        }
        match rules::decide(self.features, feature, on) {
            Decision::Apply { next, notice } => {
                self.features = next;
                FeatureOutcome::Applied {
                    state: self.state(),
                    notice: notice.map(|n| n.text()),
                }
            }
            Decision::Confirm { next, .. } if confirmed => {
                self.features = next;
                FeatureOutcome::Applied {
                    state: self.state(),
                    notice: None,
                }
            }
            Decision::Confirm { question, .. } => FeatureOutcome::Confirm {
                question: question.text(),
            },
            Decision::Blocked { reason } => FeatureOutcome::Blocked {
                reason: reason.text(),
                state: self.state(),
            },
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
        if !self.conn.as_ref().is_some_and(|c| c.demo) {
            // Smart features are not mapped on hardware; nothing to reset there.
        } else {
            self.features = FeatureState::default();
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
        let features = |mapped| FeaturesView {
            state: self.features,
            availability: rules::availability(self.features),
            mapped,
        };
        let Some(conn) = &mut self.conn else {
            return AppState {
                phase: "empty",
                demo: false,
                monitors: self.monitors.clone(),
                model: PROFILE.model,
                controls: vec![],
                features: features(false),
                baseline: BaselineView {
                    saved: false,
                    values: 0,
                    just_saved: false,
                    error: None,
                },
                labs: self.labs,
                lg_switch_running,
                error: self.error.clone(),
                version,
            };
        };
        let mut controls = Vec::new();
        for c in PROFILE.controls {
            let value = conn.values.get(&c.vcp).copied().flatten();
            controls.push(view::view_control(c, value, policy));
        }
        let baseline = BaselineView {
            saved: conn.baseline.is_some(),
            values: conn.baseline.as_ref().map_or(0, |b| b.values.len()),
            just_saved: std::mem::take(&mut conn.just_saved),
            error: conn.baseline_error.clone(),
        };
        let demo = conn.demo;
        AppState {
            phase: "connected",
            demo,
            monitors: self.monitors.clone(),
            model: PROFILE.model,
            controls,
            features: FeaturesView {
                state: self.features,
                availability: rules::availability(self.features),
                mapped: demo,
            },
            baseline,
            labs: self.labs,
            lg_switch_running,
            error: self.error.clone(),
            version,
        }
    }
}
