//! Local configuration copies and container builds. No GitHub workflow is run
//! locally and no host toolchain, credentials or Docker socket enter the image.
use crate::{
    BuildStatus, Error, FirmwareArtifact, Project, Result, github, services::BuildEvent,
    zmk::Origin,
};
use std::{
    fs,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// Manifest digest of the official stable image used by ZMK's v0.3 workflow.
pub const IMAGE: &str = "docker.io/zmkfirmware/zmk-build-arm@sha256:edb1c953438c6f720ddb79c3762f3972013b7fbbaf4fff3592fc869983e7afc5";
fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidDocument(message.into())
}
fn unique() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn configuration_root(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    let start = if path.is_dir() {
        path.as_path()
    } else {
        path.parent()
            .ok_or_else(|| invalid("Keymap has no parent folder"))?
    };
    start.ancestors().find(|root| {
        root.join("build.yaml").is_file()
            && (root.join("config/west.yml").is_file() || root.join("west.yml").is_file())
    }).map(Path::to_path_buf).ok_or_else(|| invalid("Local builds need the complete configuration folder containing build.yaml and config/west.yml (or west.yml). Use Save local copy on a GitHub configuration, or open a keymap inside your local configuration repository."))
}
pub fn open_path(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    if path.is_file() {
        return Ok(path);
    }
    let mut files = vec![];
    collect_keymaps(&path, &mut files)?;
    files.sort();
    match files.len() {
        1 => Ok(files.remove(0)),
        0 => Err(invalid(
            "This folder contains no .keymap file. Open your ZMK configuration folder rather than the website editor's source folder.",
        )),
        _ => Err(invalid(format!(
            "This folder contains {} keymaps. Choose the specific .keymap file you want to edit.",
            files.len()
        ))),
    }
}
fn collect_keymaps(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if ![
                ".git",
                ".keyforge",
                ".west",
                "target",
                "node_modules",
                "build",
            ]
            .contains(&entry.file_name().to_string_lossy().as_ref())
            {
                collect_keymaps(&entry.path(), files)?;
            }
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("keymap"))
        {
            files.push(entry.path());
        }
    }
    Ok(())
}
fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if [
            ".git",
            ".keyforge",
            ".west",
            "target",
            "node_modules",
            "build",
        ]
        .contains(&entry.file_name().to_string_lossy().as_ref())
        {
            continue;
        }
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(invalid(format!(
                "Local copy encountered a symbolic link at {}. Copy the repository with Git or resolve the link before exporting.",
                entry.path().display()
            )));
        } else if kind.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
/// Create a new child folder; never merge into or overwrite an existing folder.
pub fn save_copy(mut project: Box<Project>, parent: &Path) -> Result<Box<Project>> {
    project.check_structure()?;
    let doc = project
        .zmk
        .as_mut()
        .ok_or_else(|| invalid("Open a keymap first"))?;
    let parent = parent.canonicalize()?;
    let (name, relative, local_root) = match &doc.origin {
        Origin::GitHub {
            repository, path, ..
        } => (
            github::repository(repository)?
                .split('/')
                .next_back()
                .unwrap()
                .to_string(),
            PathBuf::from(path),
            None,
        ),
        Origin::Local { path, .. } => match configuration_root(path) {
            Ok(root) => {
                if parent.starts_with(&root) {
                    return Err(invalid(
                        "Choose a destination outside the source configuration folder",
                    ));
                }
                let relative = path
                    .strip_prefix(&root)
                    .map_err(|_| invalid("Keymap is outside the configuration folder"))?
                    .to_path_buf();
                (
                    root.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    relative,
                    Some(root),
                )
            }
            Err(_) => (
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                PathBuf::from(path.file_name().unwrap_or_default()),
                None,
            ),
        },
    };
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(invalid(
            "The keymap must have a relative path inside its configuration",
        ));
    }
    let target = parent.join(format!("{name}-keyforge"));
    if target.exists() {
        return Err(invalid(format!(
            "{} already exists. Choose another destination; existing configurations are never overwritten.",
            target.display()
        )));
    }
    let staging = parent.join(format!(
        ".keyforge-copy-{}-{}",
        std::process::id(),
        unique()
    ));
    let result = (|| -> Result<()> {
        match &doc.origin {
            Origin::GitHub {
                repository,
                branch,
                commit,
                ..
            } => {
                github::clone_snapshot(repository, branch, commit, &staging)?;
            }
            Origin::Local { .. } => {
                if let Some(root) = local_root {
                    copy_directory(&root, &staging)?;
                } else {
                    fs::create_dir(&staging)?;
                }
            }
        }
        let file = staging.join(&relative);
        if file.exists() && !file.canonicalize()?.starts_with(staging.canonicalize()?) {
            return Err(invalid(
                "The keymap path resolves outside the copied configuration",
            ));
        }
        fs::write(&file, &doc.source)?;
        fs::rename(&staging, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;
    doc.origin = Origin::Local {
        path: target.join(relative),
        saved_source: doc.source.clone(),
    };
    Ok(project)
}

pub fn build(
    path: PathBuf,
    source: String,
    operation: u64,
    request: u64,
    sender: &Sender<github::Event>,
    cancel: &AtomicBool,
) -> Result<()> {
    let root = configuration_root(&path)?;
    let engine = match crate::build_environment::inspect() {
        crate::build_environment::State::Ready { engine } => engine,
        crate::build_environment::State::Unavailable { reason, .. } => {
            return Err(Error::BuildEnvironment(reason));
        }
        _ => {
            return Err(Error::BuildEnvironment(
                "Set up the local build environment before compiling firmware".into(),
            ));
        }
    };
    if cancel.load(Ordering::Relaxed) {
        emit(
            sender,
            request,
            operation,
            BuildStatus::Cancelled,
            "Local build cancelled before container launch".into(),
            None,
            vec![],
        );
        return Ok(());
    }
    let mut hash = std::hash::DefaultHasher::new();
    root.hash(&mut hash);
    IMAGE.hash(&mut hash);
    let base = if let Some(base) = std::env::var_os("KEYFORGE_DATA_DIR") {
        PathBuf::from(base)
    } else {
        directories::ProjectDirs::from("dev", "Keyforge", "Keyforge")
            .ok_or_else(|| invalid("No application data directory"))?
            .data_local_dir()
            .to_path_buf()
    };
    let cache = base
        .join("firmware-builds")
        .join(format!("{:016x}", hash.finish()));
    fs::create_dir_all(&cache)?;
    let cache = cache.canonicalize()?;
    if cache.starts_with(&root) {
        return Err(invalid(
            "The build cache must be outside the configuration folder. Set KEYFORGE_DATA_DIR to a separate folder.",
        ));
    }
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join("build.lock"))?;
    lock.try_lock().map_err(|_| {
        invalid("Another Keyforge process is building this configuration. Wait for it to finish.")
    })?;
    let snapshot = cache.join("source");
    if snapshot.exists() {
        fs::remove_dir_all(&snapshot)?;
    }
    copy_directory(&root, &snapshot)?;
    let relative = path
        .canonicalize()?
        .strip_prefix(&root)
        .map_err(|_| invalid("The keymap is outside its configuration folder"))?
        .to_path_buf();
    fs::write(snapshot.join(relative), source)?;
    let run = format!("{}-{operation}", unique());
    // Leave host capacity for the editor/desktop and avoid Ninja's unbounded
    // default fan-out on machines with many logical CPUs.
    let cpus = thread::available_parallelism().map_or(1, usize::from);
    let jobs = cpus.saturating_sub(2).clamp(1, 4);
    let output = cache.join("artifacts").join(&run);
    let name = format!("keyforge-{}-{run}", std::process::id());
    let mut command = Command::new(&engine);
    command
        .args([
            "run",
            "--rm",
            "--pull=never",
            "--name",
            &name,
            "--security-opt",
            "label=disable",
        ])
        .args(["--cpus", &jobs.to_string()])
        .arg("--volume")
        .arg(format!("{}:/source:ro", snapshot.display()))
        .arg("--volume")
        .arg(format!("{}:/work", cache.display()))
        .args([
            "--workdir",
            "/work",
            "--env",
            "HOME=/work/home",
            "--env",
            "PYTHONUNBUFFERED=1",
            "--env",
            "GIT_TERMINAL_PROMPT=0",
        ])
        .args(["--env", &format!("KEYFORGE_BUILD_JOBS={jobs}")]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(&cache)?;
        if engine == "podman" {
            command.arg("--userns=keep-id");
        }
        command
            .arg("--user")
            .arg(format!("{}:{}", metadata.uid(), metadata.gid()));
    }
    command
        .args([
            IMAGE,
            "python3",
            "-u",
            "-c",
            include_str!("local_build.py"),
            &run,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    emit(
        sender,
        request,
        operation,
        BuildStatus::Running,
        format!(
            "Local build using {engine}\nImage: {IMAGE}\nCompiler jobs: {jobs} (container CPU limit: {jobs})\nConfiguration: {}\nFirmware output: {}",
            root.display(),
            output.display()
        ),
        None,
        vec![],
    );
    let mut child = command.spawn()?;
    let (lines, received) = mpsc::channel();
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
        let lines = lines.clone();
        readers.push(thread::spawn(move || {
            for line in BufReader::new(stream)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if lines.send(line).is_err() {
                    break;
                }
            }
        }));
    }
    drop(lines);
    let mut cancelling = false;
    let status = loop {
        let batch = received.try_iter().take(100).collect::<Vec<_>>();
        if !batch.is_empty() {
            emit(
                sender,
                request,
                operation,
                BuildStatus::Running,
                batch.join("\n"),
                None,
                vec![],
            );
        }
        if cancel.load(Ordering::Relaxed) && !cancelling {
            cancelling = true;
            let stopped = Command::new(&engine)
                .args(["stop", "--time", "1", &name])
                .stdin(Stdio::null())
                .output();
            if !stopped.as_ref().is_ok_and(|o| o.status.success()) {
                // Pull may still be in progress, before a container exists.
                let _ = child.kill();
            }
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(50));
    };
    for reader in readers {
        let _ = reader.join();
    }
    if cancelling {
        let _ = Command::new(&engine)
            .args(["rm", "--force", &name])
            .stdin(Stdio::null())
            .output();
    }
    loop {
        let remaining = received.try_iter().take(100).collect::<Vec<_>>();
        if remaining.is_empty() {
            break;
        }
        emit(
            sender,
            request,
            operation,
            BuildStatus::Running,
            remaining.join("\n"),
            None,
            vec![],
        );
    }
    if cancelling {
        emit(
            sender,
            request,
            operation,
            BuildStatus::Cancelled,
            "Local build cancelled".into(),
            None,
            vec![],
        );
    } else if status.success() {
        let mut artifacts = vec![];
        for entry in fs::read_dir(&output)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                artifacts.push(FirmwareArtifact {
                    name: entry.file_name().to_string_lossy().to_string(),
                    bytes: entry.metadata()?.len(),
                    simulated: false,
                });
            }
        }
        artifacts.sort_by(|a, b| a.name.cmp(&b.name));
        if artifacts.is_empty() {
            return Err(invalid(
                "Build exited successfully but produced no firmware files",
            ));
        }
        emit(
            sender,
            request,
            operation,
            BuildStatus::Successful,
            format!(
                "Local firmware build completed. Files saved in {}",
                output.display()
            ),
            Some(output),
            artifacts,
        );
    } else {
        return Err(invalid(format!(
            "Local firmware build failed ({status}). See the compiler output above."
        )));
    }
    Ok(())
}
fn emit(
    sender: &Sender<github::Event>,
    request: u64,
    operation: u64,
    status: BuildStatus,
    line: String,
    directory: Option<PathBuf>,
    artifacts: Vec<FirmwareArtifact>,
) {
    let _ = sender.send(github::Event {
        request,
        reply: github::Reply::LocalBuild {
            event: BuildEvent {
                operation,
                status,
                progress: 0,
                line,
                artifacts,
            },
            directory,
        },
    });
}
