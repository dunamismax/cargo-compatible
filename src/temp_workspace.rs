use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::{DirEntry, WalkDir};

pub struct TempWorkspace {
    _tempdir: TempDir,
    root: PathBuf,
}

impl TempWorkspace {
    pub fn copy_from(workspace_root: &Path) -> Result<Self> {
        let tempdir = tempfile::tempdir().context("failed to create temp directory")?;
        let destination_root = tempdir.path().join("workspace");
        fs::create_dir_all(&destination_root)?;
        for entry in WalkDir::new(workspace_root)
            .into_iter()
            .filter_entry(should_copy)
        {
            let entry = entry?;
            let source = entry.path();
            let relative = source.strip_prefix(workspace_root)?;
            if relative.as_os_str().is_empty() {
                continue;
            }
            let destination = destination_root.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&destination)?;
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, &destination).with_context(|| {
                    format!(
                        "failed to copy `{}` to `{}`",
                        source.display(),
                        destination.display()
                    )
                })?;
            }
        }

        Ok(Self {
            _tempdir: tempdir,
            root: destination_root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn should_copy(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name != ".git" && name != "target" && name != ".cargo-compatible"
}
