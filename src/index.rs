use crate::model::{DependencyConstraint, RegistryCandidate};
use anyhow::{Context, Result};
use crates_index::{Crate, SparseIndex};
use semver::Version;
use std::collections::BTreeSet;

pub trait RegistryLookup {
    fn highest_compatible(
        &self,
        dependency: &DependencyConstraint,
        allow_major: bool,
    ) -> Result<Option<RegistryCandidate>>;
}

pub struct CratesIoIndex {
    index: SparseIndex,
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

pub fn select_best_candidate(
    candidates: &[RegistryCandidate],
    dependency: &DependencyConstraint,
    allow_major: bool,
) -> Result<Option<RegistryCandidate>> {
    let current_req = semver::VersionReq::parse(&dependency.requirement).with_context(|| {
        format!(
            "invalid dependency requirement `{}`",
            dependency.requirement
        )
    })?;
    let current_major = parse_requirement_anchor(&dependency.requirement);
    if allow_major {
        return Ok(candidates
            .iter()
            .rev()
            .find(|candidate| candidate_matches(candidate, dependency, true, current_major))
            .cloned());
    }
    let preferred = candidates.iter().rev().find(|candidate| {
        candidate_matches(candidate, dependency, allow_major, current_major)
            && current_req.matches(&candidate.version)
    });
    let fallback = candidates
        .iter()
        .rev()
        .find(|candidate| candidate_matches(candidate, dependency, allow_major, current_major));
    Ok(preferred.or(fallback).cloned())
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
