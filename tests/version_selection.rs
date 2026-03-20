use cargo_compatible::index::select_best_candidate;
use cargo_compatible::model::{DependencyConstraint, RegistryCandidate};
use semver::Version;
use std::path::PathBuf;

fn constraint(requirement: &str, target: &str, features: &[&str]) -> DependencyConstraint {
    DependencyConstraint {
        package_name: "demo".to_string(),
        dependency_key: "demo".to_string(),
        manifest_path: PathBuf::from("Cargo.toml"),
        requirement: requirement.to_string(),
        source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
        features: features.iter().map(|feature| feature.to_string()).collect(),
        uses_default_features: true,
        optional: false,
        section: "dependencies".to_string(),
        target_rust_version: Some(Version::parse(target).unwrap()),
    }
}

fn candidate(version: &str, rust_version: Option<&str>, features: &[&str]) -> RegistryCandidate {
    RegistryCandidate {
        version: Version::parse(version).unwrap(),
        rust_version: rust_version.map(|value| Version::parse(value).unwrap()),
        yanked: false,
        features: features.iter().map(|feature| feature.to_string()).collect(),
    }
}

#[test]
fn stays_within_current_major_by_default() {
    let candidates = vec![
        candidate("1.8.0", Some("1.60.0"), &[]),
        candidate("2.0.0", Some("1.60.0"), &[]),
    ];
    let selected = select_best_candidate(&candidates, &constraint("^1.2", "1.60.0", &[]), false)
        .unwrap()
        .unwrap();
    assert_eq!(selected.version, Version::parse("1.8.0").unwrap());
}

#[test]
fn allows_major_with_flag() {
    let candidates = vec![
        candidate("1.8.0", Some("1.60.0"), &[]),
        candidate("2.0.0", Some("1.60.0"), &[]),
    ];
    let selected = select_best_candidate(&candidates, &constraint("^1.2", "1.60.0", &[]), true)
        .unwrap()
        .unwrap();
    assert_eq!(selected.version, Version::parse("2.0.0").unwrap());
}

#[test]
fn respects_feature_requirements() {
    let candidates = vec![
        candidate("1.5.0", Some("1.60.0"), &[]),
        candidate("1.4.0", Some("1.60.0"), &["serde"]),
    ];
    let selected = select_best_candidate(
        &candidates,
        &constraint("^1.0", "1.60.0", &["serde"]),
        false,
    )
    .unwrap()
    .unwrap();
    assert_eq!(selected.version, Version::parse("1.4.0").unwrap());
}

#[test]
fn rejects_candidates_without_rust_version_metadata() {
    let candidates = vec![
        candidate("1.5.0", None, &[]),
        candidate("1.4.0", Some("1.60.0"), &[]),
    ];
    let selected = select_best_candidate(&candidates, &constraint("^1.0", "1.60.0", &[]), false)
        .unwrap()
        .unwrap();
    assert_eq!(selected.version, Version::parse("1.4.0").unwrap());
}
