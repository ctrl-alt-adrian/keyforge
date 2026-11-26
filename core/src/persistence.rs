use crate::{Error, Project, Result, workspace::Preferences};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct LocalStorage {
    pub directory: PathBuf,
}
impl LocalStorage {
    pub fn for_frontend(frontend: &str) -> Result<Self> {
        if !["iced", "gpui", "egui"].contains(&frontend) {
            return Err(Error::InvalidDocument(
                "Unknown frontend storage namespace".into(),
            ));
        }
        let base = if let Some(path) = std::env::var_os("KEYFORGE_DATA_DIR") {
            PathBuf::from(path)
        } else {
            directories::ProjectDirs::from("dev", "Keyforge", "Keyforge")
                .ok_or_else(|| Error::InvalidDocument("No application data directory".into()))?
                .data_local_dir()
                .to_owned()
        };
        Ok(Self {
            directory: base.join(frontend),
        })
    }
    pub fn document_path(&self) -> PathBuf {
        self.directory.join("prototype-project.json")
    }
    pub fn preferences_path(&self) -> PathBuf {
        self.directory.join("workspace.json")
    }
    pub fn load_project(&self) -> Result<Project> {
        decode_project(&fs::read_to_string(self.document_path())?)
    }
    pub fn load_preferences(&self) -> Result<Preferences> {
        let mut p: Preferences = serde_json::from_slice(&fs::read(self.preferences_path())?)?;
        if p.schema_version != 1 {
            return Err(Error::InvalidDocument(
                "Unsupported preferences schema".into(),
            ));
        }
        p.sanitize();
        Ok(p)
    }
    pub fn save_project(&self, project: &Project) -> Result<()> {
        project.check_structure()?;
        write_json(&self.document_path(), project)
    }
    pub fn save_preferences(&self, p: &Preferences) -> Result<()> {
        write_json(&self.preferences_path(), p)
    }
}
pub fn decode_project(source: &str) -> Result<Project> {
    let p: Project = serde_json::from_str(source)?;
    p.check_structure()?;
    Ok(p)
}
pub fn encode_project(project: &Project) -> Result<String> {
    project.check_structure()?;
    Ok(serde_json::to_string_pretty(project)?)
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidDocument("Missing storage parent".into()))?;
    fs::create_dir_all(parent)?;
    // Same-directory temporary file; serialize writes through the worker.
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    let result = fs::rename(&temporary, path);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}
