use crate::model::{PackageSourceKind, ResolvedPackage};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn base_package_label(package: &ResolvedPackage, workspace_root: &Path) -> String {
    let mut label = format!("{}@{}", package.name, package.version);
    if let Some(detail) = package_identity_detail(package, workspace_root) {
        label.push_str(&format!(" [{detail}]"));
    }
    label
}

pub fn package_identity_label(package: &ResolvedPackage, workspace_root: &Path) -> String {
    let mut label = package.name.clone();
    if let Some(detail) = package_identity_detail(package, workspace_root) {
        label.push_str(&format!(" [{detail}]"));
    }
    label
}

pub fn colliding_base_labels<'a>(
    packages: impl IntoIterator<Item = (&'a ResolvedPackage, &'a Path)>,
) -> BTreeSet<String> {
    let mut counts = BTreeMap::new();
    for (package, workspace_root) in packages {
        *counts
            .entry(base_package_label(package, workspace_root))
            .or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(label, count)| (count > 1).then_some(label))
        .collect()
}

pub fn unique_package_label(
    package: &ResolvedPackage,
    workspace_root: &Path,
    collisions: &BTreeSet<String>,
) -> String {
    let base = base_package_label(package, workspace_root);
    if collisions.contains(&base) {
        format!("{base} [package-id: {}]", package.id)
    } else {
        base
    }
}

pub fn stable_package_identity(package: &ResolvedPackage, workspace_root: &Path) -> String {
    format!(
        "{}@{}|{}",
        package.name,
        package.version,
        stable_origin(package, workspace_root)
    )
}

pub fn stable_package_origin(package: &ResolvedPackage, workspace_root: &Path) -> String {
    stable_origin(package, workspace_root)
}

pub fn source_detail(source: Option<&str>) -> Option<String> {
    let source = source?;
    if let Some(registry) = source.strip_prefix("registry+") {
        let registry = match registry {
            "https://github.com/rust-lang/crates.io-index" | "sparse+https://index.crates.io/" => {
                "crates.io"
            }
            other => other,
        };
        return Some(format!("registry: {registry}"));
    }
    if let Some(git) = source.strip_prefix("git+") {
        if let Some((repo, rev)) = git.split_once('#') {
            let short_rev = rev.chars().take(12).collect::<String>();
            return Some(format!("git: {repo}#{short_rev}"));
        }
        return Some(format!("git: {git}"));
    }
    Some(source.to_string())
}

fn package_identity_detail(package: &ResolvedPackage, workspace_root: &Path) -> Option<String> {
    match package.source_kind {
        PackageSourceKind::Workspace => Some("workspace".to_string()),
        PackageSourceKind::Path => Some(format!(
            "path: {}",
            display_path_detail(&package.manifest_path, workspace_root)
        )),
        PackageSourceKind::Registry | PackageSourceKind::Git | PackageSourceKind::Unknown => {
            source_detail(package.source.as_deref())
        }
    }
}

fn stable_origin(package: &ResolvedPackage, workspace_root: &Path) -> String {
    match package.source_kind {
        PackageSourceKind::Workspace => "workspace".to_string(),
        PackageSourceKind::Path => format!(
            "path:{}",
            stable_path_detail(&package.manifest_path, workspace_root)
        ),
        PackageSourceKind::Registry | PackageSourceKind::Git | PackageSourceKind::Unknown => {
            package
                .source
                .clone()
                .unwrap_or_else(|| format!("package-id:{}", package.id))
        }
    }
}

fn display_path_detail(manifest_path: &Path, workspace_root: &Path) -> String {
    let directory = manifest_path.parent().unwrap_or(manifest_path);
    if let Ok(relative) = directory.strip_prefix(workspace_root) {
        if !relative.as_os_str().is_empty() {
            return relative.display().to_string();
        }
    }
    directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| directory.display().to_string())
}

fn stable_path_detail(manifest_path: &Path, workspace_root: &Path) -> String {
    let directory = manifest_path.parent().unwrap_or(manifest_path);
    if let Ok(relative) = directory.strip_prefix(workspace_root) {
        if !relative.as_os_str().is_empty() {
            return relative.display().to_string();
        }
    }
    format!("package-id:{}", manifest_path.display())
}

#[cfg(test)]
mod tests {
    use super::{
        base_package_label, colliding_base_labels, package_identity_label, stable_package_identity,
        unique_package_label,
    };
    use crate::model::{PackageSourceKind, ResolvedPackage};
    use semver::Version;
    use std::path::{Path, PathBuf};

    fn path_package(id: &str, manifest_path: &str) -> ResolvedPackage {
        ResolvedPackage {
            id: id.to_string(),
            name: "shared".to_string(),
            version: Version::new(0, 1, 0),
            source: None,
            source_kind: PackageSourceKind::Path,
            manifest_path: PathBuf::from(manifest_path),
            rust_version: Some("1.70".to_string()),
            workspace_member: false,
        }
    }

    #[test]
    fn unique_package_label_falls_back_to_package_id_for_colliding_labels() {
        let workspace_root = Path::new("/workspace");
        let left = path_package(
            "path+file:///tmp/one/shared#shared@0.1.0",
            "/tmp/one/shared/Cargo.toml",
        );
        let right = path_package(
            "path+file:///var/two/shared#shared@0.1.0",
            "/var/two/shared/Cargo.toml",
        );
        let collisions = colliding_base_labels([(&left, workspace_root), (&right, workspace_root)]);

        assert_eq!(
            base_package_label(&left, workspace_root),
            "shared@0.1.0 [path: shared]"
        );
        assert!(unique_package_label(&left, workspace_root, &collisions)
            .contains("package-id: path+file:///tmp/one/shared#shared@0.1.0"));
    }

    #[test]
    fn stable_package_identity_uses_workspace_relative_path_for_path_packages() {
        let package = path_package(
            "path+file:///tmp/workspace/deps/shared#shared@0.1.0",
            "/tmp/workspace/deps/shared/Cargo.toml",
        );

        assert_eq!(
            stable_package_identity(&package, Path::new("/tmp/workspace")),
            "shared@0.1.0|path:deps/shared"
        );
        assert_eq!(
            package_identity_label(&package, Path::new("/tmp/workspace")),
            "shared [path: deps/shared]"
        );
    }
}
