use crate::cli::ResolveCommand;
use crate::compat::analyze_current_workspace;
use crate::metadata::load_workspace;
use crate::model::{CandidateVersionChange, ResolveReport, Selection, WorkspaceData};
use crate::temp_workspace::TempWorkspace;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_candidate_resolution(
    workspace: &WorkspaceData,
    selection: &Selection,
    command: &ResolveCommand,
) -> Result<ResolveReport> {
    let current = analyze_current_workspace(workspace, selection)?;
    let temp = TempWorkspace::copy_from(&workspace.workspace_root)?;
    let temp_manifest = manifest_in_temp(
        &workspace.workspace_root,
        temp.root(),
        &workspace.workspace_manifest,
    );
    run_resolution_command(temp.root(), &temp_manifest)?;
    let candidate_workspace = load_workspace(Some(&temp_manifest))?;
    let candidate_selection =
        crate::metadata::select_packages(&candidate_workspace, &command.selection)?;
    let candidate = analyze_current_workspace(&candidate_workspace, &candidate_selection)?;
    let candidate_lockfile_path = temp.root().join("Cargo.lock");
    let candidate_lockfile = if candidate_lockfile_path.exists() {
        Some(fs::read_to_string(&candidate_lockfile_path)?)
    } else {
        None
    };
    let version_changes = compute_version_changes(workspace, &candidate_workspace);
    let current_problem_ids = current
        .incompatible_packages
        .iter()
        .chain(current.unknown_packages.iter())
        .map(|issue| issue.package.name.clone())
        .collect::<BTreeSet<_>>();
    let candidate_problem_ids = candidate
        .incompatible_packages
        .iter()
        .chain(candidate.unknown_packages.iter())
        .map(|issue| issue.package.name.clone())
        .collect::<BTreeSet<_>>();
    let improved_packages = current_problem_ids
        .difference(&candidate_problem_ids)
        .cloned()
        .collect::<Vec<_>>();
    let remaining_blockers = candidate_problem_ids.into_iter().collect::<Vec<_>>();
    let mut notes = workspace.recommendations.clone();
    if version_changes.is_empty() {
        notes.push("candidate lockfile did not change the resolved dependency graph".to_string());
    }

    Ok(ResolveReport {
        current,
        candidate,
        version_changes,
        improved_packages,
        remaining_blockers,
        candidate_lockfile,
        notes,
    })
}

pub fn apply_candidate_lockfile(
    workspace_root: &Path,
    candidate_lockfile: PathBuf,
) -> Result<String> {
    if !candidate_lockfile.exists() {
        bail!(
            "candidate lockfile `{}` does not exist; run `cargo compatible resolve --write-candidate {}` first",
            candidate_lockfile.display(),
            candidate_lockfile.display()
        );
    }
    let destination = workspace_root.join("Cargo.lock");
    let before = fs::read_to_string(&destination).unwrap_or_default();
    let after = fs::read_to_string(&candidate_lockfile)?;
    if before == after {
        return Ok(
            "candidate lockfile matches the current Cargo.lock; nothing to apply".to_string(),
        );
    }
    atomic_write(&destination, after.as_bytes())?;
    let summary = diff_summary(&before, &after);
    Ok(format!(
        "applied candidate lockfile to {} ({summary})",
        destination.display()
    ))
}

fn run_resolution_command(workspace_root: &Path, manifest_path: &Path) -> Result<()> {
    let lockfile = workspace_root.join("Cargo.lock");
    let mut command = Command::new("cargo");
    command.current_dir(workspace_root);
    if lockfile.exists() {
        command.args(["update", "--workspace", "--manifest-path"]);
    } else {
        command.args(["generate-lockfile", "--manifest-path"]);
    }
    command.arg(manifest_path);
    let output = command
        .output()
        .context("failed to execute cargo resolver")?;
    if !output.status.success() {
        return Err(anyhow!(
            "cargo resolution failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn manifest_in_temp(real_root: &Path, temp_root: &Path, real_manifest: &Path) -> PathBuf {
    let relative = real_manifest
        .strip_prefix(real_root)
        .unwrap_or(Path::new("Cargo.toml"));
    temp_root.join(relative)
}

fn compute_version_changes(
    current: &WorkspaceData,
    candidate: &WorkspaceData,
) -> Vec<CandidateVersionChange> {
    let mut current_versions = BTreeMap::new();
    for package in current.packages_by_id.values() {
        current_versions.insert(
            (package.name.clone(), package.source.clone()),
            package.version.to_string(),
        );
    }
    let mut candidate_versions = BTreeMap::new();
    for package in candidate.packages_by_id.values() {
        candidate_versions.insert(
            (package.name.clone(), package.source.clone()),
            package.version.to_string(),
        );
    }

    let keys = current_versions
        .keys()
        .chain(candidate_versions.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|(name, source)| {
            let before = current_versions
                .get(&(name.clone(), source.clone()))
                .cloned();
            let after = candidate_versions
                .get(&(name.clone(), source.clone()))
                .cloned();
            if before == after {
                return None;
            }
            Some(CandidateVersionChange {
                package_name: name,
                source,
                before,
                after,
            })
        })
        .collect()
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path `{}` has no parent", path.display()))?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    use std::io::Write;
    temp.write_all(contents)?;
    temp.flush()?;
    temp.persist(path)
        .map_err(|error| anyhow!("failed to persist `{}`: {}", path.display(), error.error))?;
    Ok(())
}

fn diff_summary(before: &str, after: &str) -> String {
    let before_count = before
        .lines()
        .filter(|line| line.trim_start().starts_with("name ="))
        .count();
    let after_count = after
        .lines()
        .filter(|line| line.trim_start().starts_with("name ="))
        .count();
    format!("package entries: {before_count} -> {after_count}")
}
