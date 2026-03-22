use crate::model::{DependencyConstraint, RegistryCandidate};
use anyhow::{Context, Result};
use crates_index::{Crate, SparseIndex};
use semver::Version;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;
use tracing::debug;

pub trait RegistryLookup {
    fn highest_compatible(
        &self,
        dependency: &DependencyConstraint,
        allow_major: bool,
    ) -> Result<Option<RegistryCandidate>>;
}

pub fn registry_lookup_for_workspace(workspace_root: &Path) -> Result<Box<dyn RegistryLookup>> {
    if let Some(path) = local_registry_path(workspace_root)? {
        return Ok(Box::new(LocalRegistryIndex { root: path }));
    }
    Ok(Box::new(CratesIoIndex::new()?))
}

pub struct CratesIoIndex {
    index: SparseIndex,
}

struct LocalRegistryIndex {
    root: PathBuf,
}

impl CratesIoIndex {
    pub fn new() -> Result<Self> {
        let index = SparseIndex::new_cargo_default()
            .context("failed to open Cargo sparse registry index")?;
        Ok(Self { index })
    }
}

impl RegistryLookup for CratesIoIndex {
    fn highest_compatible(
        &self,
        dependency: &DependencyConstraint,
        allow_major: bool,
    ) -> Result<Option<RegistryCandidate>> {
        let Ok(krate) = self.index.crate_from_cache(&dependency.package_name) else {
            return Ok(None);
        };
        select_best_candidate(&collect_candidates(&krate), dependency, allow_major)
    }
}

impl RegistryLookup for LocalRegistryIndex {
    fn highest_compatible(
        &self,
        dependency: &DependencyConstraint,
        allow_major: bool,
    ) -> Result<Option<RegistryCandidate>> {
        let candidates = self.load_candidates(&dependency.package_name)?;
        select_best_candidate(&candidates, dependency, allow_major)
    }
}

impl LocalRegistryIndex {
    fn load_candidates(&self, crate_name: &str) -> Result<Vec<RegistryCandidate>> {
        let index_path = self
            .root
            .join("index")
            .join(local_registry_index_path(crate_name)?);
        let Ok(contents) = fs::read_to_string(&index_path) else {
            return Ok(Vec::new());
        };
        let mut candidates = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_local_registry_candidate(line, crate_name))
            .collect::<Result<Vec<_>>>()?;
        candidates.sort_by(|left, right| left.version.cmp(&right.version));
        Ok(candidates)
    }
}

pub fn select_best_candidate(
    candidates: &[RegistryCandidate],
    dependency: &DependencyConstraint,
    allow_major: bool,
) -> Result<Option<RegistryCandidate>> {
    debug!(
        package_name = %dependency.package_name,
        requirement = %dependency.requirement,
        candidates = candidates.len(),
        allow_major,
        "selecting registry candidate"
    );
    let current_req = semver::VersionReq::parse(&dependency.requirement).with_context(|| {
        format!(
            "invalid dependency requirement `{}`",
            dependency.requirement
        )
    })?;
    let current_major = parse_requirement_anchor(&dependency.requirement);
    let selected = if allow_major {
        candidates
            .iter()
            .rev()
            .find(|candidate| candidate_matches(candidate, dependency, true, current_major))
            .cloned()
    } else {
        let preferred = candidates.iter().rev().find(|candidate| {
            candidate_matches(candidate, dependency, allow_major, current_major)
                && current_req.matches(&candidate.version)
        });
        let fallback = candidates
            .iter()
            .rev()
            .find(|candidate| candidate_matches(candidate, dependency, allow_major, current_major));
        preferred.or(fallback).cloned()
    };
    debug!(
        package_name = %dependency.package_name,
        selected_version = ?selected.as_ref().map(|candidate| candidate.version.to_string()),
        "selected registry candidate"
    );
    Ok(selected)
}

fn collect_candidates(krate: &Crate) -> Vec<RegistryCandidate> {
    let mut candidates = krate
        .versions()
        .iter()
        .filter_map(|version| {
            let parsed = Version::parse(version.version()).ok()?;
            let rust_version = version
                .rust_version()
                .and_then(|value| Version::parse(value).ok());
            let mut feature_names = version
                .features()
                .keys()
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>();
            for dependency in version.dependencies() {
                if dependency.is_optional() {
                    feature_names.insert(dependency.name().to_string());
                }
            }
            Some(RegistryCandidate {
                version: parsed,
                rust_version,
                yanked: version.is_yanked(),
                features: feature_names,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.version.cmp(&right.version));
    candidates
}

fn candidate_matches(
    candidate: &RegistryCandidate,
    dependency: &DependencyConstraint,
    allow_major: bool,
    current_major: Option<u64>,
) -> bool {
    !candidate.yanked
        && dependency
            .target_rust_version
            .as_ref()
            .zip(candidate.rust_version.as_ref())
            .map(|(target, package)| package <= target)
            .unwrap_or(candidate.rust_version.is_some())
        && dependency
            .features
            .iter()
            .all(|feature| candidate.features.contains(feature))
        && (allow_major
            || current_major
                .map(|major| major == candidate.version.major)
                .unwrap_or(true))
}

fn parse_requirement_anchor(requirement: &str) -> Option<u64> {
    requirement
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '.')
        .collect::<String>()
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
}

fn local_registry_path(workspace_root: &Path) -> Result<Option<PathBuf>> {
    let config_path = workspace_root.join(".cargo").join("config.toml");
    let contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let document = contents.parse::<DocumentMut>()?;
    let source = document.get("source").and_then(|item| item.as_table_like());
    let Some(source) = source else {
        return Ok(None);
    };
    let replace_with = source
        .get("crates-io")
        .and_then(|item| item.as_table_like())
        .and_then(|table| table.get("replace-with"))
        .and_then(|item| item.as_value())
        .and_then(|value| value.as_str());
    let Some(replace_with) = replace_with else {
        return Ok(None);
    };
    let local_registry = source
        .get(replace_with)
        .and_then(|item| item.as_table_like())
        .and_then(|table| table.get("local-registry"))
        .and_then(|item| item.as_value())
        .and_then(|value| value.as_str());
    let Some(local_registry) = local_registry else {
        return Ok(None);
    };
    let path = Path::new(local_registry);
    let base = config_path.parent().unwrap_or(workspace_root);
    Ok(Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }))
}

fn local_registry_index_path(crate_name: &str) -> Result<PathBuf> {
    let name = crate_name.to_lowercase();
    let path = match name.len() {
        1 => PathBuf::from("1").join(name),
        2 => PathBuf::from("2").join(name),
        3 => PathBuf::from("3").join(&name[0..1]).join(name),
        len if len >= 4 => PathBuf::from(&name[0..2]).join(&name[2..4]).join(name),
        _ => anyhow::bail!("crate name `{crate_name}` is not a valid registry index key"),
    };
    Ok(path)
}

fn parse_local_registry_candidate(line: &str, crate_name: &str) -> Result<RegistryCandidate> {
    let version: LocalRegistryVersion = serde_json::from_str(line)
        .with_context(|| format!("failed to parse local registry entry for `{crate_name}`"))?;
    let mut features = version.features.into_keys().collect::<BTreeSet<_>>();
    for dependency in version.deps {
        if dependency.optional {
            features.insert(dependency.name);
        }
    }
    Ok(RegistryCandidate {
        version: Version::parse(&version.vers)
            .with_context(|| format!("invalid version `{}` for `{crate_name}`", version.vers))?,
        rust_version: version
            .rust_version
            .as_deref()
            .map(normalize_rust_version)
            .transpose()
            .with_context(|| {
                format!(
                    "invalid rust-version `{}` for `{crate_name}`",
                    version.rust_version.unwrap_or_default()
                )
            })?,
        yanked: version.yanked,
        features,
    })
}

fn normalize_rust_version(value: &str) -> Result<Version, semver::Error> {
    let parts = value.split('.').collect::<Vec<_>>();
    let normalized = match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => value.to_string(),
    };
    Version::parse(&normalized)
}

#[derive(Debug, Deserialize)]
struct LocalRegistryVersion {
    vers: String,
    #[serde(default)]
    deps: Vec<LocalRegistryDependency>,
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    yanked: bool,
    rust_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalRegistryDependency {
    name: String,
    #[serde(default)]
    optional: bool,
}

#[cfg(test)]
mod tests {
    use super::{candidate_matches, parse_requirement_anchor, select_best_candidate};
    use crate::model::{DependencyConstraint, RegistryCandidate};
    use proptest::collection::vec;
    use proptest::prelude::*;
    use semver::{Version, VersionReq};
    use std::path::PathBuf;

    #[derive(Debug, Clone)]
    struct CandidateSpec {
        version: (u64, u64, u64),
        rust_version: Option<(u64, u64, u64)>,
        yanked: bool,
        features: Vec<&'static str>,
    }

    fn candidate_spec_strategy() -> impl Strategy<Value = CandidateSpec> {
        (
            (0u64..4, 0u64..8, 0u64..8),
            prop_oneof![Just(None), (0u64..4, 0u64..8, 0u64..8).prop_map(Some),],
            any::<bool>(),
            prop::sample::subsequence(&["serde", "std", "derive", "alloc"], 0..4),
        )
            .prop_map(|(version, rust_version, yanked, features)| CandidateSpec {
                version,
                rust_version,
                yanked,
                features,
            })
    }

    fn dependency_constraint(
        requirement: &str,
        target_rust_version: Option<Version>,
        required_features: &[&str],
    ) -> DependencyConstraint {
        DependencyConstraint {
            package_name: "demo".to_string(),
            dependency_key: "demo".to_string(),
            manifest_path: PathBuf::from("Cargo.toml"),
            requirement: requirement.to_string(),
            source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
            features: required_features
                .iter()
                .map(|feature| feature.to_string())
                .collect(),
            uses_default_features: true,
            optional: false,
            section: "dependencies".to_string(),
            target_rust_version,
        }
    }

    fn to_candidate(spec: &CandidateSpec) -> RegistryCandidate {
        RegistryCandidate {
            version: Version::new(spec.version.0, spec.version.1, spec.version.2),
            rust_version: spec
                .rust_version
                .map(|version| Version::new(version.0, version.1, version.2)),
            yanked: spec.yanked,
            features: spec
                .features
                .iter()
                .map(|feature| feature.to_string())
                .collect(),
        }
    }

    proptest! {
        #[test]
        fn selected_candidate_respects_semver_and_feature_invariants(
            anchor_major in 0u64..4,
            anchor_minor in 0u64..6,
            anchor_patch in 0u64..4,
            allow_major in any::<bool>(),
            target_rust_version in prop_oneof![
                Just(None),
                (0u64..4, 0u64..8, 0u64..8)
                    .prop_map(|(major, minor, patch)| Some(Version::new(major, minor, patch))),
            ],
            required_features in prop::sample::subsequence(&["serde", "std", "derive", "alloc"], 0..4),
            candidate_specs in vec(candidate_spec_strategy(), 0..32),
        ) {
            let requirement = format!("^{anchor_major}.{anchor_minor}.{anchor_patch}");
            let dependency =
                dependency_constraint(&requirement, target_rust_version.clone(), &required_features);
            let mut candidates = candidate_specs.iter().map(to_candidate).collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.version.cmp(&right.version));

            let selected = select_best_candidate(&candidates, &dependency, allow_major).unwrap();

            if let Some(selected) = selected {
                prop_assert!(!selected.yanked);
                if let Some(target) = target_rust_version {
                    prop_assert!(selected
                        .rust_version
                        .as_ref()
                        .is_some_and(|package| package <= &target));
                } else {
                    prop_assert!(selected.rust_version.is_some());
                }
                prop_assert!(required_features
                    .iter()
                    .all(|feature| selected.features.contains(*feature)));
                if !allow_major {
                    if let Some(current_major) = parse_requirement_anchor(&requirement) {
                        prop_assert_eq!(selected.version.major, current_major);
                    }
                }
            }
        }

        #[test]
        fn selected_candidate_matches_preferred_resolution_tier(
            anchor_major in 0u64..4,
            anchor_minor in 0u64..6,
            anchor_patch in 0u64..4,
            allow_major in any::<bool>(),
            target_rust_version in prop_oneof![
                Just(None),
                (0u64..4, 0u64..8, 0u64..8)
                    .prop_map(|(major, minor, patch)| Some(Version::new(major, minor, patch))),
            ],
            required_features in prop::sample::subsequence(&["serde", "std", "derive", "alloc"], 0..4),
            candidate_specs in vec(candidate_spec_strategy(), 0..32),
        ) {
            let requirement = format!("^{anchor_major}.{anchor_minor}.{anchor_patch}");
            let current_major = parse_requirement_anchor(&requirement);
            let current_req = VersionReq::parse(&requirement).unwrap();
            let dependency =
                dependency_constraint(&requirement, target_rust_version, &required_features);
            let mut candidates = candidate_specs.iter().map(to_candidate).collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.version.cmp(&right.version));

            let expected = if allow_major {
                candidates
                    .iter()
                    .rev()
                    .find(|candidate| candidate_matches(candidate, &dependency, true, current_major))
                    .cloned()
            } else {
                let preferred = candidates
                    .iter()
                    .rev()
                    .find(|candidate| {
                        candidate_matches(candidate, &dependency, false, current_major)
                            && current_req.matches(&candidate.version)
                    })
                    .cloned();
                preferred.or_else(|| {
                    candidates
                        .iter()
                        .rev()
                        .find(|candidate| candidate_matches(candidate, &dependency, false, current_major))
                        .cloned()
                })
            };

            let selected = select_best_candidate(&candidates, &dependency, allow_major).unwrap();
            prop_assert_eq!(
                selected.as_ref().map(|candidate| candidate.version.clone()),
                expected.as_ref().map(|candidate| candidate.version.clone())
            );
        }
    }
}
