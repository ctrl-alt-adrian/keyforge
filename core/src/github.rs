//! Native background repository operations using GitHub CLI's credential store.
//! Commands are argument arrays (never shell strings); credentials are never read
//! into the app, serialized or logged. This module is independent of all GUIs.
use crate::{
    BuildStatus, Error, FirmwareArtifact, Project, Result,
    services::BuildEvent,
    zmk::{Document, Origin},
};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub struct Connection {
    pub account: Option<String>,
    pub repositories: Vec<String>,
    pub repository: String,
    pub branches: Vec<String>,
    pub branch: String,
    pub files: Vec<String>,
    pub file: String,
    pub workflows: Vec<Workflow>,
    pub workflow: String,
    pub configurations: Vec<Configuration>,
    pub browsed: bool,
    pub editor_repository: bool,
    pub busy: bool,
    pub message: String,
    pub request: u64,
}
#[derive(Debug, Clone)]
pub struct Configuration {
    pub repository: String,
    pub branch: String,
    pub keymaps: usize,
}
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone)]
pub struct Run {
    pub id: u64,
    pub url: String,
    pub repository: String,
}
#[derive(Debug, Clone)]
pub enum Request {
    BuildEnvironment {
        generation: u64,
        prepare: bool,
    },
    Account,
    FindConfigurations {
        repository: String,
    },
    Browse {
        repository: String,
        branch: String,
    },
    Open {
        repository: String,
        branch: String,
        path: String,
        workflow: Option<u64>,
    },
    Local(PathBuf),
    LocalCopy {
        project: Box<Project>,
        directory: PathBuf,
    },
    LocalBuild {
        path: PathBuf,
        source: String,
        operation: u64,
    },
    Save {
        project: Box<Project>,
    },
    Build {
        origin: Origin,
        dispatch: bool,
        operation: u64,
    },
    Download {
        run: Run,
        directory: PathBuf,
    },
}
pub enum Reply {
    BuildEnvironment {
        generation: u64,
        state: crate::build_environment::State,
        line: String,
    },
    Account {
        login: String,
        repositories: Vec<String>,
    },
    Browser {
        repository: String,
        branches: Vec<String>,
        branch: String,
        files: Vec<String>,
        workflows: Vec<Workflow>,
        editor_repository: bool,
    },
    Configurations {
        configurations: Vec<Configuration>,
        inspected: usize,
        candidates: usize,
        unavailable: usize,
    },
    Open(Document),
    Saved(Box<Project>),
    LocalCopy(Box<Project>),
    LocalBuild {
        event: BuildEvent,
        directory: Option<PathBuf>,
    },
    Build {
        event: BuildEvent,
        run: Option<Run>,
    },
    Downloaded(PathBuf),
    Failed(String),
}
pub struct Event {
    pub request: u64,
    pub reply: Reply,
}
pub struct Service {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    cancel: Arc<AtomicBool>,
}
impl Default for Service {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl Service {
    pub fn submit(&mut self, id: u64, request: Request) {
        let sender = self.sender.clone();
        let cancel = if matches!(request, Request::Build { .. } | Request::LocalBuild { .. }) {
            self.cancel.store(true, Ordering::Relaxed);
            self.cancel = Arc::new(AtomicBool::new(false));
            self.cancel.clone()
        } else {
            self.cancel.clone()
        };
        thread::spawn(move || {
            if let Request::BuildEnvironment {
                generation,
                prepare,
            } = request
            {
                if prepare {
                    crate::build_environment::prepare(generation, id, &sender);
                } else {
                    let _ = sender.send(Event {
                        request: id,
                        reply: Reply::BuildEnvironment {
                            generation,
                            state: crate::build_environment::inspect(),
                            line: String::new(),
                        },
                    });
                }
                return;
            }
            if let Request::LocalBuild {
                path,
                source,
                operation,
            } = request
            {
                if let Err(error) =
                    crate::local_build::build(path, source, operation, id, &sender, &cancel)
                {
                    let _ = sender.send(Event {
                        request: id,
                        reply: Reply::LocalBuild {
                            event: BuildEvent {
                                operation,
                                status: BuildStatus::Failed,
                                progress: 0,
                                line: error.to_string(),
                                artifacts: vec![],
                            },
                            directory: None,
                        },
                    });
                }
                return;
            }
            if let Request::Build {
                origin,
                dispatch,
                operation,
            } = request
            {
                if let Err(error) = watch(origin, dispatch, operation, id, &sender, &cancel) {
                    let event = BuildEvent {
                        operation,
                        status: BuildStatus::Failed,
                        progress: 100,
                        line: error.to_string(),
                        artifacts: vec![],
                    };
                    let _ = sender.send(Event {
                        request: id,
                        reply: Reply::Build { event, run: None },
                    });
                }
                return;
            }
            let reply = perform(request).unwrap_or_else(|e| Reply::Failed(e.to_string()));
            let _ = sender.send(Event { request: id, reply });
        });
    }
    pub fn poll(&self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }
    pub fn cancel_build(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn error(message: impl Into<String>) -> Error {
    Error::InvalidDocument(message.into())
}
fn gh(args: &[&str], input: Option<&Value>) -> Result<Vec<u8>> {
    let mut command = Command::new("gh");
    command
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_HOST", "github.com")
        .env("GH_PAGER", "cat")
        .env("NO_COLOR", "1")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        error(format!(
            "GitHub CLI could not start: {e}. Install gh and sign in with gh auth login --web."
        ))
    })?;
    if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or_else(|| error("Unable to send GitHub request"))?
            .write_all(&serde_json::to_vec(input)?)?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let message = String::from_utf8_lossy(&out.stderr);
        return Err(error(format!("GitHub: {}", message.trim())));
    }
    Ok(out.stdout)
}
pub(crate) fn clone_snapshot(
    repo: &str,
    branch: &str,
    commit: &str,
    destination: &std::path::Path,
) -> Result<()> {
    let repo = repository(repo)?;
    gh(
        &[
            "repo",
            "clone",
            &repo,
            &destination.to_string_lossy(),
            "--",
            "--no-checkout",
        ],
        None,
    )?;
    let result = Command::new("git")
        .arg("-C")
        .arg(destination)
        .args(["checkout", "-B", branch, commit])
        .stdin(Stdio::null())
        .output()?;
    if !result.status.success() {
        return Err(error(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ));
    }
    let result = Command::new("git")
        .arg("-C")
        .arg(destination)
        .args([
            "branch",
            "--set-upstream-to",
            &format!("origin/{branch}"),
            branch,
        ])
        .stdin(Stdio::null())
        .output()?;
    if !result.status.success() {
        return Err(error(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ));
    }
    Ok(())
}
fn api(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&gh(&["api", path], None)?)?)
}
fn api_write(method: &str, path: &str, body: Value) -> Result<Value> {
    let out = gh(
        &["api", "--method", method, path, "--input", "-"],
        Some(&body),
    )?;
    if out.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(serde_json::from_slice(&out)?)
    }
}
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn file_path(value: &str) -> Result<String> {
    if value.starts_with('/') || value.split('/').any(|s| s == ".." || s.is_empty()) {
        return Err(error("Choose a repository-relative keymap path"));
    }
    Ok(value.split('/').map(encode).collect::<Vec<_>>().join("/"))
}
pub fn repository(value: &str) -> Result<String> {
    let value = value
        .trim()
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")
        .unwrap_or(value.trim().trim_end_matches('/'));
    let value = value.strip_suffix(".git").unwrap_or(value);
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err(error("Enter owner/repository or its github.com URL"));
    }
    Ok(value.into())
}
fn string(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| error(format!("GitHub response is missing {key}")))
}
fn head(repo: &str, branch: &str) -> Result<String> {
    let v = api(&format!("repos/{repo}/branches/{}", encode(branch)))?;
    string(&v["commit"], "sha")
}
fn contents(repo: &str, branch: &str, path: &str) -> Result<Value> {
    api(&format!(
        "repos/{repo}/contents/{}?ref={}",
        file_path(path)?,
        encode(branch)
    ))
}
fn text_contents(value: &Value) -> Result<String> {
    if value["type"] != "file" || value["encoding"] != "base64" {
        return Err(error("Select a regular keymap file under 1 MB"));
    }
    let bytes = decode64(&string(value, "content")?)?;
    String::from_utf8(bytes).map_err(|_| error("Keymap must be UTF-8"))
}
fn paginated(path: &str) -> Result<Vec<Value>> {
    let pages: Vec<Vec<Value>> =
        serde_json::from_slice(&gh(&["api", "--paginate", "--slurp", path], None)?)?;
    Ok(pages.into_iter().flatten().collect())
}
fn repository_tree(repo: &str, branch: &str) -> Result<Value> {
    let tree = api(&format!(
        "repos/{repo}/git/trees/{}?recursive=1",
        encode(branch)
    ))?;
    if tree["truncated"].as_bool() == Some(true) {
        return Err(error(
            "Repository tree is too large to enumerate completely; open a local keymap instead",
        ));
    }
    Ok(tree)
}
fn keymap_paths(tree: &Value) -> Vec<String> {
    let mut files = tree["tree"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["type"] == "blob")
        .filter_map(|f| f["path"].as_str())
        .filter(|p| p.to_ascii_lowercase().ends_with(".keymap"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    files.sort();
    files
}
/// Name matching only limits discovery work. Every suggestion is verified
/// against its default branch; users can still open any repository directly.
fn find_configurations(repo: &str) -> Result<Reply> {
    let owner = repository(repo)?.split('/').next().unwrap().to_string();
    let user = api("user")?;
    let repositories = if user["login"]
        .as_str()
        .is_some_and(|login| login.eq_ignore_ascii_case(&owner))
    {
        paginated("user/repos?per_page=100&sort=updated")?
    } else {
        let account = api(&format!("users/{owner}"))?;
        let endpoint = if account["type"] == "Organization" {
            "orgs"
        } else {
            "users"
        };
        paginated(&format!(
            "{endpoint}/{owner}/repos?per_page=100&sort=updated"
        ))?
    };
    let mut candidates = repositories
        .into_iter()
        .filter(|r| {
            r["owner"]["login"]
                .as_str()
                .is_some_and(|login| login.eq_ignore_ascii_case(&owner))
                && r["archived"] != true
                && r["disabled"] != true
                && r["name"].as_str().is_some_and(|name| {
                    let name = name.to_ascii_lowercase();
                    ["zmk", "keymap", "keyboard", "corne"]
                        .iter()
                        .any(|word| name.contains(word))
                })
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|r| {
        let name = r["name"].as_str().unwrap_or_default().to_ascii_lowercase();
        (!name.contains("zmk-config"), !name.contains("zmk"), name)
    });
    let total = candidates.len();
    let inspected = total.min(24);
    let mut configurations = vec![];
    let mut unavailable = 0;
    for candidate in candidates.into_iter().take(inspected) {
        let repo = string(&candidate, "full_name")?;
        let branch = string(&candidate, "default_branch")?;
        match repository_tree(&repo, &branch) {
            Ok(tree) => {
                let keymaps = keymap_paths(&tree).len();
                if keymaps > 0 {
                    configurations.push(Configuration {
                        repository: repo,
                        branch,
                        keymaps,
                    });
                }
            }
            Err(_) => unavailable += 1,
        }
    }
    Ok(Reply::Configurations {
        configurations,
        inspected,
        candidates: total,
        unavailable,
    })
}
fn perform(request: Request) -> Result<Reply> {
    match request {
        Request::FindConfigurations { repository } => find_configurations(&repository),
        Request::Account => {
            let user = api("user")?;
            let login = string(&user, "login")?;
            let mut repositories = paginated("user/repos?per_page=100&sort=updated")?
                .into_iter()
                .filter_map(|r| r["full_name"].as_str().map(str::to_string))
                .collect::<Vec<_>>();
            repositories.sort();
            repositories.dedup();
            Ok(Reply::Account {
                login,
                repositories,
            })
        }
        Request::Browse {
            repository: repo,
            branch,
        } => {
            let repo = repository(&repo)?;
            let meta = api(&format!("repos/{repo}"))?;
            let branch = if branch.is_empty() {
                string(&meta, "default_branch")?
            } else {
                branch
            };
            let pages: Value = serde_json::from_slice(&gh(
                &[
                    "api",
                    "--paginate",
                    "--slurp",
                    &format!("repos/{repo}/branches?per_page=100"),
                ],
                None,
            )?)?;
            let branches = pages
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|p| p.as_array().into_iter().flatten())
                .filter_map(|b| b["name"].as_str().map(str::to_string))
                .collect();
            let tree = repository_tree(&repo, &branch)?;
            let files = keymap_paths(&tree);
            let editor_repository = tree["tree"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f["path"].as_str())
                .any(|p| p == "api/services/zmk/keymap.js")
                || meta["parent"]["full_name"] == "nickcoutsos/keymap-editor"
                || meta["full_name"] == "nickcoutsos/keymap-editor";
            let workflows =
                api(&format!("repos/{repo}/actions/workflows?per_page=100"))?["workflows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|w| {
                        Some(Workflow {
                            id: w["id"].as_u64()?,
                            name: w["name"].as_str()?.into(),
                            path: w["path"].as_str()?.into(),
                        })
                    })
                    .collect();
            Ok(Reply::Browser {
                repository: repo,
                branches,
                branch,
                files,
                workflows,
                editor_repository,
            })
        }
        Request::Open {
            repository: repo,
            branch,
            path,
            workflow,
        } => {
            let repo = repository(&repo)?;
            let v = contents(&repo, &branch, &path)?;
            let source = text_contents(&v)?;
            let sha = string(&v, "sha")?;
            let commit = head(&repo, &branch)?;
            Ok(Reply::Open(Document {
                source,
                origin: Origin::GitHub {
                    repository: repo,
                    branch,
                    path,
                    sha,
                    commit,
                    workflow,
                },
            }))
        }
        Request::Local(path) => {
            let path = crate::local_build::open_path(&path)?;
            let source = std::fs::read_to_string(&path)?;
            Ok(Reply::Open(Document {
                origin: Origin::Local {
                    path,
                    saved_source: source.clone(),
                },
                source,
            }))
        }
        Request::LocalCopy { project, directory } => Ok(Reply::LocalCopy(
            crate::local_build::save_copy(project, &directory)?,
        )),
        Request::Save { mut project } => {
            let doc = project
                .zmk
                .as_mut()
                .ok_or_else(|| error("Open a real keymap before saving"))?;
            doc.tree()?;
            match &mut doc.origin {
                Origin::GitHub {
                    repository: repo,
                    branch,
                    path,
                    sha,
                    commit,
                    ..
                } => {
                    let repo = repository(repo)?;
                    let current = contents(&repo, branch, path)?;
                    if string(&current, "sha")? != *sha {
                        return Err(error(
                            "This keymap changed on GitHub. Your edits are retained; reopen the latest revision before saving.",
                        ));
                    }
                    let result = api_write(
                        "PUT",
                        &format!("repos/{repo}/contents/{}", file_path(path)?),
                        json!({"message":format!("Update {path} with Keyforge"),"content":base64(doc.source.as_bytes()),"sha":sha,"branch":branch}),
                    )?;
                    *sha = string(&result["content"], "sha")?;
                    *commit = string(&result["commit"], "sha")?;
                }
                Origin::Local { path, saved_source } => {
                    if std::fs::read_to_string(&*path)? != *saved_source {
                        return Err(error(
                            "The local file changed outside Keyforge. Your edits are retained; reopen it before saving.",
                        ));
                    }
                    let temporary =
                        path.with_extension(format!("keymap.keyforge-{}.tmp", std::process::id()));
                    let permissions = std::fs::metadata(&*path)?.permissions();
                    let write = (|| -> std::io::Result<()> {
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&temporary)?;
                        file.write_all(doc.source.as_bytes())?;
                        file.set_permissions(permissions)?;
                        file.sync_all()?;
                        std::fs::rename(&temporary, &*path)
                    })();
                    if write.is_err() {
                        let _ = std::fs::remove_file(&temporary);
                    }
                    write?;
                    *saved_source = doc.source.clone();
                }
            }
            Ok(Reply::Saved(project))
        }
        Request::Download { run, directory } => {
            std::fs::create_dir_all(&directory)?;
            gh(
                &[
                    "run",
                    "download",
                    &run.id.to_string(),
                    "--repo",
                    &run.repository,
                    "--dir",
                    &directory.to_string_lossy(),
                ],
                None,
            )?;
            Ok(Reply::Downloaded(directory))
        }
        Request::Build { .. } | Request::LocalBuild { .. } | Request::BuildEnvironment { .. } => {
            unreachable!()
        }
    }
}
fn watch(
    origin: Origin,
    dispatch: bool,
    operation: u64,
    id: u64,
    sender: &Sender<Event>,
    cancel: &AtomicBool,
) -> Result<()> {
    let Origin::GitHub {
        repository: repo,
        branch,
        path,
        sha,
        mut commit,
        workflow,
    } = origin
    else {
        return Err(error(
            "Local keymaps save to disk. Connect a GitHub repository to build firmware.",
        ));
    };
    let repo = repository(&repo)?;
    let mut minimum_run = 0;
    if dispatch {
        if string(&contents(&repo, &branch, &path)?, "sha")? != sha {
            return Err(error(
                "GitHub has a newer keymap. Reopen it before starting a build.",
            ));
        }
        let workflow = workflow.ok_or_else(|| {
            error("Select an existing build workflow when opening the repository")
        })?;
        minimum_run = api(&format!(
            "repos/{repo}/actions/workflows/{workflow}/runs?per_page=1"
        ))?["workflow_runs"][0]["id"]
            .as_u64()
            .unwrap_or(0);
        commit = head(&repo, &branch)?;
        if cancel.load(Ordering::Relaxed) {
            let _ = sender.send(Event {
                request: id,
                reply: Reply::Build {
                    event: BuildEvent {
                        operation,
                        status: BuildStatus::Cancelled,
                        progress: 0,
                        line: "Build cancelled before dispatch".into(),
                        artifacts: vec![],
                    },
                    run: None,
                },
            });
            return Ok(());
        }
        api_write(
            "POST",
            &format!("repos/{repo}/actions/workflows/{workflow}/dispatches"),
            json!({"ref":branch}),
        )?;
    }
    let deadline = Instant::now() + Duration::from_secs(7200);
    let discover_deadline = Instant::now() + Duration::from_secs(90);
    let mut run: Option<Run> = None;
    let mut previous = String::new();
    let mut cancellation_sent = false;
    let emit = |status, progress, line: String, artifacts, run: Option<Run>| {
        let _ = sender.send(Event {
            request: id,
            reply: Reply::Build {
                event: BuildEvent {
                    operation,
                    status,
                    progress,
                    line,
                    artifacts,
                },
                run,
            },
        });
    };
    emit(
        BuildStatus::Running,
        0,
        "Waiting for the repository's firmware workflow…".into(),
        vec![],
        None,
    );
    while Instant::now() < deadline {
        if cancel.load(Ordering::Relaxed)
            && !cancellation_sent
            && let Some(active) = &run
        {
            gh(
                &["run", "cancel", &active.id.to_string(), "--repo", &repo],
                None,
            )?;
            cancellation_sent = true;
            emit(
                BuildStatus::Running,
                1,
                "GitHub accepted the cancellation request; waiting for the workflow to stop…"
                    .into(),
                vec![],
                run.clone(),
            );
        }
        let value = if let Some(run) = &run {
            api(&format!("repos/{repo}/actions/runs/{}", run.id))?
        } else {
            let runs = api(&format!(
                "repos/{repo}/actions/runs?head_sha={}&branch={}&per_page=100",
                encode(&commit),
                encode(&branch)
            ))?;
            let found = runs["workflow_runs"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| {
                    r["head_sha"].as_str() == Some(commit.as_str())
                        && r["head_branch"].as_str() == Some(branch.as_str())
                        && r["id"].as_u64().is_some_and(|v| v > minimum_run)
                        && workflow.is_none_or(|w| r["workflow_id"].as_u64() == Some(w))
                })
                .cloned();
            if let Some(value) = found {
                run = Some(Run {
                    id: value["id"]
                        .as_u64()
                        .ok_or_else(|| error("Missing workflow run ID"))?,
                    url: string(&value, "html_url")?,
                    repository: repo.clone(),
                });
                value
            } else {
                if Instant::now() > discover_deadline {
                    return Err(error(
                        "No firmware workflow started for this commit. Check the repository's push triggers or select a workflow with workflow_dispatch.",
                    ));
                }
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        let status = value["status"].as_str().unwrap_or("unknown");
        let conclusion = value["conclusion"].as_str().unwrap_or("");
        if previous != status {
            emit(
                BuildStatus::Running,
                if status == "queued" { 0 } else { 1 },
                format!(
                    "GitHub Actions: {status} · run {}",
                    run.as_ref().unwrap().id
                ),
                vec![],
                run.clone(),
            );
            previous = status.into();
        }
        if status == "completed" {
            let active = run.as_ref().unwrap();
            if let Ok(log) = gh(
                &[
                    "run",
                    "view",
                    &active.id.to_string(),
                    "--repo",
                    &repo,
                    "--log",
                ],
                None,
            ) {
                let raw = String::from_utf8_lossy(&log);
                let mut lines = raw.lines().rev().take(300).collect::<Vec<_>>();
                lines.reverse();
                emit(
                    BuildStatus::Running,
                    99,
                    lines.join("\n"),
                    vec![],
                    run.clone(),
                );
            }
            let mut artifacts = vec![];
            if conclusion == "success" {
                let data = api(&format!(
                    "repos/{repo}/actions/runs/{}/artifacts?per_page=100",
                    active.id
                ))?;
                for a in data["artifacts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|a| a["expired"] != true)
                {
                    artifacts.push(FirmwareArtifact {
                        name: string(a, "name")?,
                        bytes: a["size_in_bytes"].as_u64().unwrap_or(0),
                        simulated: false,
                    });
                }
            }
            emit(
                match conclusion {
                    "success" => BuildStatus::Successful,
                    "cancelled" => BuildStatus::Cancelled,
                    _ => BuildStatus::Failed,
                },
                100,
                format!("Firmware workflow {conclusion}"),
                artifacts,
                run.clone(),
            );
            return Ok(());
        }
        thread::sleep(Duration::from_secs(3));
    }
    Err(error(
        "Stopped watching after two hours. The workflow can still be viewed on GitHub.",
    ))
}
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let v = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        out.push(ALPHABET[((v >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((v >> 12) & 63) as usize] as char);
        out.push(if c.len() > 1 {
            ALPHABET[((v >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            ALPHABET[(v & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
fn decode64(value: &str) -> Result<Vec<u8>> {
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = vec![];
    for b in value.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if b == b'=' {
            break;
        }
        let n = ALPHABET
            .iter()
            .position(|v| *v == b)
            .ok_or_else(|| error("Invalid GitHub file encoding"))?;
        bits = (bits << 6) | n as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Ok(out)
}
