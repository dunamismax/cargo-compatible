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
use tracing::{debug, info};

pub fn build_candidate_resolution(
    workspace: &WorkspaceData,
    selection: &Selection,
    command: &ResolveCommand,
) -> Result<ResolveReport> {
    info!(
        workspace_root = %workspace.workspace_root.display(),
        selected_members = selection.members.len(),
        "building candidate resolution"
    );
    let current = analyze_current_workspace(workspace, selection)?;
    let temp = TempWorkspace::copy_from(&workspace.workspace_root)?;
    let temp_manifest = manifest_in_temp(
        &workspace.workspace_root,
        temp.root(),
        &workspace.workspace_manifest,
    );
    debug!(
        temp_root = %temp.root().display(),
        temp_manifest = %temp_manifest.display(),
        "prepared temporary workspace for resolution"
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
    let (version_changes, ambiguous_version_changes) =
        compute_version_changes(workspace, &candidate_workspace);
    let current_problem_ids = current
        .incompatible_packages
        .iter()
        .chain(current.unknown_packages.iter())
        .map(|issue| package_label(&issue.package))
        .collect::<BTreeSet<_>>();
    let candidate_problem_ids = candidate
        .incompatible_packages
        .iter()
        .chain(candidate.unknown_packages.iter())
        .map(|issue| package_label(&issue.package))
        .collect::<BTreeSet<_>>();
    let improved_packages = current_problem_ids
        .difference(&candidate_problem_ids)
        .cloned()
        .collect::<Vec<_>>();
    let remaining_blockers = candidate_problem_ids.into_iter().collect::<Vec<_>>();
    let mut notes = workspace.recommendations.clone();
    if version_changes.is_empty() && ambiguous_version_changes.is_empty() {
        notes.push("candidate lockfile did not change the resolved dependency graph".to_string());
    }
    if !ambiguous_version_changes.is_empty() {
        notes.push(format!(
            "omitted detailed version change reporting for {} because multiple resolved versions shared the same package identity",
            ambiguous_version_changes.join(", ")
        ));
    }

    info!(
        version_changes = version_changes.len(),
        improved_packages = improved_packages.len(),
        remaining_blockers = remaining_blockers.len(),
        "completed candidate resolution"
    );

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
    let subcommand = if lockfile.exists() {
        "update"
    } else {
        "generate-lockfile"
    };
    if lockfile.exists() {
        command.args(["update", "--workspace", "--manifest-path"]);
    } else {
        command.args(["generate-lockfile", "--manifest-path"]);
    }
    command.arg(manifest_path);
    debug!(
        workspace_root = %workspace_root.display(),
        manifest_path = %manifest_path.display(),
        subcommand,
        "invoking cargo resolver"
    );
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
) -> (Vec<CandidateVersionChange>, Vec<String>) {
    compute_version_changes_from_packages(&current.packages_by_id, &candidate.packages_by_id)
}

fn compute_version_changes_from_packages(
    current_packages: &BTreeMap<String, crate::model::ResolvedPackage>,
    candidate_packages: &BTreeMap<String, crate::model::ResolvedPackage>,
) -> (Vec<CandidateVersionChange>, Vec<String>) {
    let current_versions = grouped_versions_by_identity(current_packages);
    let candidate_versions = grouped_versions_by_identity(candidate_packages);

    let keys = current_versions
        .keys()
        .chain(candidate_versions.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    let mut ambiguous = Vec::new();
    for (name, source) in keys {
        let before_versions = current_versions
            .get(&(name.clone(), source.clone()))
            .cloned()
            .unwrap_or_default();
        let after_versions = candidate_versions
            .get(&(name.clone(), source.clone()))
            .cloned()
            .unwrap_or_default();
        if before_versions == after_versions {
            continue;
        }
        if before_versions.len() <= 1 && after_versions.len() <= 1 {
            changes.push(CandidateVersionChange {
                package_name: name,
                source,
                before: before_versions.into_iter().next(),
                after: after_versions.into_iter().next(),
            });
        } else {
            ambiguous.push(package_identity_label(&name, source.as_deref()));
        }
    }
    (changes, ambiguous)
}

fn grouped_versions_by_identity(
    packages: &BTreeMap<String, crate::model::ResolvedPackage>,
) -> BTreeMap<(String, Option<String>), BTreeSet<String>> {
    let mut versions = BTreeMap::new();
    for package in packages.values() {
        versions
            .entry((package.name.clone(), package.source.clone()))
            .or_insert_with(BTreeSet::new)
            .insert(package.version.to_string());
    }
    versions
}

fn package_identity_label(name: &str, source: Option<&str>) -> String {
    match source {
        Some(source) => format!("{name} ({source})"),
        None => name.to_string(),
    }
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

fn package_label(package: &crate::model::ResolvedPackage) -> String {
    match package.source.as_deref() {
        Some(source) => format!("{}@{} ({source})", package.name, package.version),
        None => format!("{}@{}", package.name, package.version),
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_version_changes_from_packages, package_identity_label};
    use crate::model::{PackageSourceKind, ResolvedPackage};
    use proptest::collection::btree_map;
    use proptest::prelude::*;
    use semver::Version;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[derive(Debug, Clone)]
    struct PackageSpec {
        source: Option<&'static str>,
        version: (u64, u64, u64),
    }

    fn spec_strategy() -> impl Strategy<Value = PackageSpec> {
        (
            prop_oneof![
                Just(None),
                Just(Some(
                    "registry+https://github.com/rust-lang/crates.io-index"
                )),
                Just(Some("git+https://example.invalid/repo")),
            ],
            (0u64..4, 0u64..6, 0u64..8),
        )
            .prop_map(|(source, version)| PackageSpec { source, version })
    }

    fn package_map(specs: &BTreeMap<u8, PackageSpec>) -> BTreeMap<String, ResolvedPackage> {
        specs
            .iter()
            .map(|(id, spec)| {
                resolved_package(
                    &format!("pkg-{id}"),
                    &format!("crate_{id}"),
                    spec.version,
                    spec.source,
                )
            })
            .collect()
    }

    fn resolved_package(
        id: &str,
        name: &str,
        version: (u64, u64, u64),
        source: Option<&str>,
    ) -> (String, ResolvedPackage) {
        (
            id.to_string(),
            ResolvedPackage {
                id: id.to_string(),
                name: name.to_string(),
                version: Version::new(version.0, version.1, version.2),
                source: source.map(str::to_string),
                source_kind: match source {
                    Some(source) if source.starts_with("registry+") => PackageSourceKind::Registry,
                    Some(source) if source.starts_with("git+") => PackageSourceKind::Git,
                    Some(_) => PackageSourceKind::Unknown,
                    None => PackageSourceKind::Path,
                },
                manifest_path: PathBuf::from("Cargo.toml"),
                rust_version: Some("1.70".to_string()),
                workspace_member: false,
            },
        )
    }

    proptest! {
        #[test]
        fn version_change_diff_matches_package_versions(
            current in btree_map(0u8..32, spec_strategy(), 0..24),
            candidate in btree_map(0u8..32, spec_strategy(), 0..24),
        ) {
            let current_packages = package_map(&current);
            let candidate_packages = package_map(&candidate);
            let (changes, ambiguous) =
                compute_version_changes_from_packages(&current_packages, &candidate_packages);

            let mut expected = BTreeMap::new();
            for package in current_packages.values() {
                expected.insert(
                    (package.name.clone(), package.source.clone()),
                    (Some(package.version.to_string()), None),
                );
            }
            for package in candidate_packages.values() {
                expected
                    .entry((package.name.clone(), package.source.clone()))
                    .and_modify(|entry| entry.1 = Some(package.version.to_string()))
                    .or_insert((None, Some(package.version.to_string())));
            }
            expected.retain(|_, versions| versions.0 != versions.1);

            let actual = changes
                .into_iter()
                .map(|change| {
                    (
                        (change.package_name, change.source),
                        (change.before, change.after),
                    )
                })
                .collect::<BTreeMap<_, _>>();

            prop_assert_eq!(actual, expected);
            prop_assert!(ambiguous.is_empty());
        }
    }

    #[test]
    fn version_change_diff_omits_ambiguous_same_name_same_source_versions() {
        let registry = Some("registry+https://github.com/rust-lang/crates.io-index");
        let current = BTreeMap::from([
            resolved_package("pkg-a", "shared", (1, 0, 0), registry),
            resolved_package("pkg-b", "shared", (2, 0, 0), registry),
        ]);
        let candidate = BTreeMap::from([resolved_package("pkg-c", "shared", (1, 0, 0), registry)]);

        let (changes, ambiguous) = compute_version_changes_from_packages(&current, &candidate);

        assert!(changes.is_empty());
        assert_eq!(
            ambiguous,
            vec![package_identity_label(
                "shared",
                Some("registry+https://github.com/rust-lang/crates.io-index")
            )]
        );
    }

    #[test]
    fn version_change_diff_keeps_same_name_different_sources_separate() {
        let current = BTreeMap::from([
            resolved_package(
                "pkg-a",
                "shared",
                (1, 0, 0),
                Some("registry+https://github.com/rust-lang/crates.io-index"),
            ),
            resolved_package(
                "pkg-b",
                "shared",
                (1, 0, 0),
                Some("git+https://example.invalid/repo"),
            ),
        ]);
        let candidate = BTreeMap::from([
            resolved_package(
                "pkg-a2",
                "shared",
                (0, 9, 0),
                Some("registry+https://github.com/rust-lang/crates.io-index"),
            ),
            resolved_package(
                "pkg-b2",
                "shared",
                (1, 0, 0),
                Some("git+https://example.invalid/repo"),
            ),
        ]);

        let (changes, ambiguous) = compute_version_changes_from_packages(&current, &candidate);

        assert!(ambiguous.is_empty());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].package_name, "shared");
        assert_eq!(
            changes[0].source.as_deref(),
            Some("registry+https://github.com/rust-lang/crates.io-index")
        );
        assert_eq!(changes[0].before.as_deref(), Some("1.0.0"));
        assert_eq!(changes[0].after.as_deref(), Some("0.9.0"));
    }
}
