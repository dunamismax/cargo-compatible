use crate::index::RegistryLookup;
use crate::metadata::display_rust_version;
use crate::model::{
    DependencyConstraint, ManifestSuggestion, ResolveReport, Selection, WorkspaceData,
};
use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::DependencyKind;
use semver::VersionReq;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item};

const DEFAULT_CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";

pub fn suggest_manifest_changes(
    workspace: &WorkspaceData,
    selection: &Selection,
    resolution: &ResolveReport,
    registry: &dyn RegistryLookup,
    allow_major: bool,
) -> Result<Vec<ManifestSuggestion>> {
    let candidate_problem_identities = resolution
        .candidate
        .incompatible_packages
        .iter()
        .chain(resolution.candidate.unknown_packages.iter())
        .map(|issue| package_identity(&issue.package.name, issue.package.source.as_deref()))
        .collect::<BTreeSet<_>>();
    if candidate_problem_identities.is_empty() {
        return Ok(Vec::new());
    }

    let mut suggestions = Vec::new();
    for member in &selection.members {
        let member_target = member.rust_version.clone();
        let Some(target_rust_version) = member_target else {
            continue;
        };
        let constraints =
            direct_dependency_constraints(workspace, &member.package_id, &member.manifest_path)?;
        for constraint in constraints {
            if !candidate_problem_identities.contains(&package_identity(
                &constraint.package_name,
                constraint.source.as_deref(),
            )) {
                continue;
            }
            if !constraint
                .source
                .as_deref()
                .map(|source| source.starts_with("registry+"))
                .unwrap_or(true)
            {
                continue;
            }
            let Some(candidate) = registry.highest_compatible(&constraint, allow_major)? else {
                continue;
            };
            let current_req = VersionReq::parse(&constraint.requirement).with_context(|| {
                format!("invalid version requirement `{}`", constraint.requirement)
            })?;
            if current_req.matches(&candidate.version)
                && resolution
                    .candidate
                    .incompatible_packages
                    .iter()
                    .all(|issue| {
                        package_identity(&issue.package.name, issue.package.source.as_deref())
                            != package_identity(
                                &constraint.package_name,
                                constraint.source.as_deref(),
                            )
                    })
            {
                continue;
            }
            suggestions.push(ManifestSuggestion {
                package_name: member.package_name.clone(),
                dependency_key: constraint.dependency_key.clone(),
                dependency_name: constraint.package_name.clone(),
                manifest_path: constraint.manifest_path.clone(),
                current_requirement: constraint.requirement.clone(),
                suggested_requirement: candidate.version.to_string(),
                reason: format!(
                    "highest compatible non-yanked release with requested features for target {}",
                    display_rust_version(&target_rust_version)
                ),
                target_rust_version: display_rust_version(&target_rust_version),
                section: constraint.section.clone(),
            });
        }
    }
    suggestions.sort_by(|left, right| {
        left.package_name
            .cmp(&right.package_name)
            .then_with(|| left.dependency_key.cmp(&right.dependency_key))
    });
    suggestions.dedup_by(|left, right| {
        left.manifest_path == right.manifest_path && left.dependency_key == right.dependency_key
    });
    Ok(suggestions)
}

pub fn apply_manifest_suggestions(suggestions: &[ManifestSuggestion]) -> Result<()> {
    let by_manifest = suggestions.iter().fold(
        BTreeMap::<PathBuf, Vec<&ManifestSuggestion>>::new(),
        |mut acc, suggestion| {
            acc.entry(suggestion.manifest_path.clone())
                .or_default()
                .push(suggestion);
            acc
        },
    );
    let mut staged_writes = Vec::new();
    for (manifest_path, manifest_suggestions) in by_manifest {
        let contents = fs::read_to_string(&manifest_path)?;
        let mut document = contents.parse::<DocumentMut>()?;
        for suggestion in manifest_suggestions {
            let updated = update_dependency_requirement(&mut document, suggestion)?;
            if !updated {
                bail!(
                    "failed to locate dependency `{}` in `{}`",
                    suggestion.dependency_key,
                    manifest_path.display()
                );
            }
        }
        staged_writes.push((manifest_path, document.to_string()));
    }
    for (manifest_path, contents) in staged_writes {
        atomic_write(&manifest_path, contents.as_bytes())?;
    }
    Ok(())
}

fn direct_dependency_constraints(
    workspace: &WorkspaceData,
    package_id: &str,
    manifest_path: &Path,
) -> Result<Vec<DependencyConstraint>> {
    let package = workspace
        .metadata
        .packages
        .iter()
        .find(|package| package.id.repr == package_id)
        .ok_or_else(|| anyhow!("selected package `{package_id}` missing from metadata"))?;
    let constraints = package
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Normal)
        .map(|dependency| DependencyConstraint {
            package_name: dependency.name.clone(),
            dependency_key: dependency
                .rename
                .clone()
                .unwrap_or_else(|| dependency.name.clone()),
            manifest_path: manifest_path.to_path_buf(),
            requirement: dependency.req.to_string(),
            source: dependency_source(dependency),
            features: dependency.features.iter().cloned().collect::<BTreeSet<_>>(),
            uses_default_features: dependency.uses_default_features,
            optional: dependency.optional,
            section: dependency_section_label(dependency.kind),
            target_rust_version: package.rust_version.clone(),
        })
        .collect::<Vec<_>>();
    Ok(constraints)
}

fn dependency_section_label(kind: DependencyKind) -> String {
    match kind {
        DependencyKind::Normal => "dependencies",
        DependencyKind::Development => "dev-dependencies",
        DependencyKind::Build => "build-dependencies",
        _ => "dependencies",
    }
    .to_string()
}

fn dependency_source(dependency: &cargo_metadata::Dependency) -> Option<String> {
    if dependency.path.is_some() {
        return None;
    }
    dependency
        .source
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| {
            dependency
                .registry
                .as_ref()
                .map(|registry| format!("registry+{registry}"))
        })
        .or_else(|| Some(DEFAULT_CRATES_IO_SOURCE.to_string()))
}

fn package_identity(name: &str, source: Option<&str>) -> (String, Option<String>) {
    (name.to_string(), source.map(ToOwned::to_owned))
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

fn update_dependency_requirement(
    document: &mut DocumentMut,
    suggestion: &ManifestSuggestion,
) -> Result<bool> {
    if update_dependency_in_root_table(document, &suggestion.section, suggestion)? {
        return Ok(true);
    }
    let Some(target_item) = document.get_mut("target") else {
        return Ok(false);
    };
    let Some(target_table) = target_item.as_table_like_mut() else {
        return Ok(false);
    };
    for (_, item) in target_table.iter_mut() {
        let Some(target_cfg) = item.as_table_like_mut() else {
            continue;
        };
        if let Some(dep_table) = target_cfg.get_mut(&suggestion.section) {
            if update_dep_item(
                dep_table,
                &suggestion.dependency_key,
                &suggestion.suggested_requirement,
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn update_dependency_in_root_table(
    document: &mut DocumentMut,
    section: &str,
    suggestion: &ManifestSuggestion,
) -> Result<bool> {
    let Some(item) = document.get_mut(section) else {
        return Ok(false);
    };
    update_dep_item(
        item,
        &suggestion.dependency_key,
        &suggestion.suggested_requirement,
    )
}

fn update_dep_item(item: &mut Item, key: &str, suggested_requirement: &str) -> Result<bool> {
    let Some(table) = item.as_table_like_mut() else {
        return Ok(false);
    };
    let Some(dep_item) = table.get_mut(key) else {
        return Ok(false);
    };
    if dep_item.is_str() {
        *dep_item = value(suggested_requirement);
        return Ok(true);
    }
    if let Some(inline) = dep_item.as_inline_table_mut() {
        inline.insert("version", toml_edit::Value::from(suggested_requirement));
        return Ok(true);
    }
    if let Some(dep_table) = dep_item.as_table_like_mut() {
        dep_table.insert("version", value(suggested_requirement));
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::apply_manifest_suggestions;
    use crate::model::ManifestSuggestion;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn apply_manifest_suggestions_updates_multiple_manifests() {
        let temp = tempdir().unwrap();
        let first = temp.path().join("first").join("Cargo.toml");
        let second = temp.path().join("second").join("Cargo.toml");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(
            &first,
            "[package]\nname = \"first\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .unwrap();
        fs::write(
            &second,
            "[package]\nname = \"second\"\nversion = \"0.1.0\"\n\n[dependencies]\nregex = { version = \"1\" }\n",
        )
        .unwrap();

        let suggestions = vec![
            suggestion(&first, "first", "serde", "0.9.0"),
            suggestion(&second, "second", "regex", "1.10.0"),
        ];

        apply_manifest_suggestions(&suggestions).unwrap();

        let first_contents = fs::read_to_string(&first).unwrap();
        let second_contents = fs::read_to_string(&second).unwrap();
        assert!(first_contents.contains("serde = \"0.9.0\""));
        assert!(second_contents.contains("version = \"1.10.0\""));
    }

    #[test]
    fn apply_manifest_suggestions_avoids_partial_writes_on_failure() {
        let temp = tempdir().unwrap();
        let first = temp.path().join("first").join("Cargo.toml");
        let second = temp.path().join("second").join("Cargo.toml");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(
            &first,
            "[package]\nname = \"first\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .unwrap();
        fs::write(
            &second,
            "[package]\nname = \"second\"\nversion = \"0.1.0\"\n\n[dependencies]\nregex = \"1\"\n",
        )
        .unwrap();
        let first_before = fs::read_to_string(&first).unwrap();
        let second_before = fs::read_to_string(&second).unwrap();

        let suggestions = vec![
            suggestion(&first, "first", "serde", "0.9.0"),
            suggestion(&second, "second", "missing", "1.10.0"),
        ];

        let error = apply_manifest_suggestions(&suggestions).unwrap_err();

        assert!(error
            .to_string()
            .contains("failed to locate dependency `missing`"));
        assert_eq!(fs::read_to_string(&first).unwrap(), first_before);
        assert_eq!(fs::read_to_string(&second).unwrap(), second_before);
    }

    fn suggestion(
        manifest_path: &std::path::Path,
        package_name: &str,
        dependency_key: &str,
        suggested_requirement: &str,
    ) -> ManifestSuggestion {
        ManifestSuggestion {
            package_name: package_name.to_string(),
            dependency_key: dependency_key.to_string(),
            dependency_name: dependency_key.to_string(),
            manifest_path: manifest_path.to_path_buf(),
            current_requirement: "1".to_string(),
            suggested_requirement: suggested_requirement.to_string(),
            reason: "test suggestion".to_string(),
            target_rust_version: "1.70".to_string(),
            section: "dependencies".to_string(),
        }
    }
}
