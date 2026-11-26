use crate::{
    BuildStatus, FirmwareArtifact, Keymap, Project, Result,
    persistence::{LocalStorage, decode_project},
    workspace::Preferences,
};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

pub fn sample_project() -> Result<Project> {
    decode_project(include_str!("../../fixtures/keymaps/corne.json"))
}

#[derive(Debug, Clone)]
pub struct BuildEvent {
    pub operation: u64,
    pub status: BuildStatus,
    pub progress: u8,
    pub line: String,
    pub artifacts: Vec<FirmwareArtifact>,
}
/// Pure deterministic plan; time delivery is separate from the domain transitions.
pub fn build_step(operation: u64, step: usize, fail: bool) -> BuildEvent {
    let (status, progress, line) = match step {
        0 => (
            BuildStatus::Running,
            5,
            "[simulated] Preparing Corne / nice_nano_v2",
        ),
        1 => (
            BuildStatus::Running,
            20,
            "[simulated] Reading prototype bindings",
        ),
        2 => (
            BuildStatus::Running,
            40,
            "[simulated] Configuring left and right shields",
        ),
        3 => (BuildStatus::Running, 65, "[simulated] Compiling keymap"),
        4 => (BuildStatus::Running, 85, "[simulated] Linking firmware"),
        _ if fail => (
            BuildStatus::Failed,
            100,
            "[simulated] Failure requested: linker error (deterministic fixture)",
        ),
        _ => (
            BuildStatus::Successful,
            100,
            "[simulated] Complete: 2 demonstration artifacts; no firmware generated",
        ),
    };
    let artifacts = if status == BuildStatus::Successful {
        vec![
            FirmwareArtifact {
                name: "corne_left-simulated.uf2".into(),
                bytes: 245760,
                simulated: true,
            },
            FirmwareArtifact {
                name: "corne_right-simulated.uf2".into(),
                bytes: 241664,
                simulated: true,
            },
        ]
    } else {
        vec![]
    };
    BuildEvent {
        operation,
        status,
        progress,
        line: line.into(),
        artifacts,
    }
}
pub struct BuildClock {
    pub operation: u64,
    step: usize,
    fail: bool,
    next: Instant,
}
impl BuildClock {
    pub fn new(operation: u64, fail: bool) -> Self {
        Self {
            operation,
            step: 0,
            fail,
            next: Instant::now() + Duration::from_millis(500),
        }
    }
    pub fn poll(&mut self) -> Option<BuildEvent> {
        if Instant::now() < self.next || self.step > 5 {
            return None;
        }
        let event = build_step(self.operation, self.step, self.fail);
        self.step += 1;
        self.next = Instant::now() + Duration::from_millis(500);
        Some(event)
    }
}

pub enum StorageRequest {
    Save {
        epoch: u64,
        project: Box<Project>,
        build_after_save: bool,
    },
    Preferences(Preferences),
    CacheProject(Box<Project>),
    Flush(Sender<()>),
}
pub enum StorageEvent {
    Saved {
        epoch: u64,
        keymap: Keymap,
        project: Box<Project>,
        build_after_save: bool,
    },
    PreferencesSaved,
    CacheSaved,
    CacheFailed(String),
    Failed(String),
}
pub struct StorageWorker {
    sender: Sender<StorageRequest>,
    receiver: Receiver<StorageEvent>,
    pub pending: usize,
}
impl StorageWorker {
    pub fn new(storage: LocalStorage) -> Self {
        let (tx, requests) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        thread::spawn(move || {
            for request in requests {
                let event = match request {
                    StorageRequest::Save {
                        epoch,
                        project,
                        build_after_save,
                    } => match storage.save_project(&project) {
                        Ok(()) => StorageEvent::Saved {
                            epoch,
                            keymap: project.keymap.clone(),
                            project,
                            build_after_save,
                        },
                        Err(e) => StorageEvent::Failed(e.to_string()),
                    },
                    StorageRequest::CacheProject(p) => match storage.save_project(&p) {
                        Ok(()) => StorageEvent::CacheSaved,
                        Err(e) => StorageEvent::CacheFailed(e.to_string()),
                    },
                    StorageRequest::Flush(finished) => {
                        let _ = finished.send(());
                        continue;
                    }
                    StorageRequest::Preferences(p) => match storage.save_preferences(&p) {
                        Ok(()) => StorageEvent::PreferencesSaved,
                        Err(e) => StorageEvent::Failed(e.to_string()),
                    },
                };
                if events.send(event).is_err() {
                    break;
                }
            }
        });
        Self {
            sender: tx,
            receiver: rx,
            pending: 0,
        }
    }
    pub fn submit(&mut self, request: StorageRequest) -> Result<()> {
        self.sender
            .send(request)
            .map_err(|_| crate::Error::ServiceUnavailable)?;
        self.pending += 1;
        Ok(())
    }
    pub fn poll(&mut self) -> Option<StorageEvent> {
        let event = self.receiver.try_recv().ok()?;
        self.pending = self.pending.saturating_sub(1);
        Some(event)
    }
    /// Completes after all previously submitted writes have emitted their result.
    /// Wait for this on a background thread, then poll to consume those results.
    pub fn flush(&self) -> Result<Receiver<()>> {
        let (finished, completion) = mpsc::channel();
        self.sender
            .send(StorageRequest::Flush(finished))
            .map_err(|_| crate::Error::ServiceUnavailable)?;
        Ok(completion)
    }
}
