//! Machine prerequisites are distinct from a firmware build's lifecycle.
use crate::{
    github::{Event, Reply},
    local_build::IMAGE,
};
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::mpsc::Sender,
    thread,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Unchecked,
    Checking,
    Unavailable {
        installed: bool,
        reason: String,
    },
    NeedsImage {
        engine: String,
    },
    Installing {
        engine: String,
    },
    Ready {
        engine: String,
    },
}
impl State {
    pub fn busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Installing { .. })
    }
    pub fn ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unchecked | Self::Checking => "Checking local builds…",
            Self::Unavailable {
                installed: false, ..
            } => "Build runtime required",
            Self::Unavailable { .. } => "Build runtime unavailable",
            Self::NeedsImage { .. } => "Build setup required",
            Self::Installing { .. } => "Setting up local builds…",
            Self::Ready { .. } => "Ready",
        }
    }
}
pub fn inspect() -> State {
    let mut installed = false;
    let mut errors = vec![];
    for engine in ["docker", "podman"] {
        let info = Command::new(engine)
            .arg("info")
            .stdin(Stdio::null())
            .output();
        let output = match info {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                errors.push(format!("{engine}: {e}"));
                continue;
            }
            Ok(out) => {
                installed = true;
                out
            }
        };
        if !output.status.success() {
            errors.push(format!(
                "{engine}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            continue;
        }
        let image = Command::new(engine)
            .args(["image", "inspect", IMAGE])
            .stdin(Stdio::null())
            .output();
        match image {
            Ok(out) if out.status.success() => {
                // A cached image alone does not prove the runtime can start it.
                let probe = Command::new(engine).args(["run","--rm","--pull=never","--network=none",IMAGE,"python3","-c","import yaml, west; from pathlib import Path; raise SystemExit(0 if any(Path('/opt').glob('zephyr-sdk-*')) else 'ZMK SDK is missing')"]).stdin(Stdio::null()).output();
                match probe {
                    Ok(out) if out.status.success() => {
                        return State::Ready {
                            engine: engine.into(),
                        };
                    }
                    Ok(out) => errors.push(format!(
                        "{engine}: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    )),
                    Err(e) => errors.push(format!("{engine}: {e}")),
                }
            }
            Ok(out) => {
                let reason = String::from_utf8_lossy(&out.stderr).to_string();
                let lower = reason.to_ascii_lowercase();
                if ["no such image", "not found", "not known"]
                    .iter()
                    .any(|s| lower.contains(s))
                {
                    return State::NeedsImage {
                        engine: engine.into(),
                    };
                }
                errors.push(format!("{engine}: {}", reason.trim()));
            }
            Err(e) => errors.push(format!("{engine}: {e}")),
        }
    }
    State::Unavailable {
        installed,
        reason: if installed {
            format!(
                "Docker or Podman is installed, but Keyforge cannot use it. Start its engine and ensure your user has access.\n\n{}",
                errors.join("\n")
            )
        } else {
            "Local firmware builds need Docker or Podman. Install a runtime and start its engine, then retry. Editing and local saving remain available.".into()
        },
    }
}
pub fn prepare(generation: u64, request: u64, sender: &Sender<Event>) {
    let send = |state, line: String| {
        let _ = sender.send(Event {
            request,
            reply: Reply::BuildEnvironment {
                generation,
                state,
                line,
            },
        });
    };
    let state = inspect();
    let State::NeedsImage { engine } = state else {
        send(state, String::new());
        return;
    };
    send(
        State::Installing {
            engine: engine.clone(),
        },
        "Downloading the ZMK build environment…".into(),
    );
    let result = (|| -> std::io::Result<bool> {
        let mut child = Command::new(&engine)
            .args(["pull", IMAGE])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut readers = vec![];
        for stream in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let sender = sender.clone();
            let engine = engine.clone();
            readers.push(thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let _ = sender.send(Event {
                        request,
                        reply: Reply::BuildEnvironment {
                            generation,
                            state: State::Installing {
                                engine: engine.clone(),
                            },
                            line,
                        },
                    });
                }
            }));
        }
        let status = child.wait()?;
        for reader in readers {
            let _ = reader.join();
        }
        Ok(status.success())
    })();
    match result {
        Ok(true) => {
            let state = inspect();
            let line = if state.ready() {
                "Local build environment is ready."
            } else {
                "The downloaded build environment could not start. Check the runtime details."
            };
            send(state, line.into());
        }
        Ok(false) => send(
            State::Unavailable {
                installed: true,
                reason: "The build-image download failed. See the output above and retry setup."
                    .into(),
            },
            "Build environment setup failed.".into(),
        ),
        Err(error) => send(
            State::Unavailable {
                installed: true,
                reason: error.to_string(),
            },
            format!("Build environment setup failed: {error}"),
        ),
    }
}
