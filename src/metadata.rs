use crate::cli::SelectionArgs;
use crate::model::{
    MemberTarget, PackageSourceKind, ResolvedPackage, SelectedMember, Selection, TargetSelection,
    TargetSelectionMode, WorkspaceData,
};
use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;
use tracing::{debug, info};

pub fn load_workspace(manifest_path: Option<&Path>) -> Result<WorkspaceData> {
    debug!(manifest_path = ?manifest_path, "loading cargo metadata");
    let mut command = MetadataCommand::new();
    if let Some(path) = manifest_path {
        command.manifest_path(path);
    }
    let metadata = command.exec().context("failed to read cargo metadata")?;
    let workspace_root = PathBuf::from(metadata.workspace_root.as_std_path());
    let workspace_manifest = workspace_root.join("Cargo.toml");
    let is_virtual_workspace = metadata.root_package().is_none();
    let resolver = workspace_resolver(&workspace_manifest)?;
    let mut recommendations = Vec::new();
    if is_virtual_workspace && resolver.as_deref() != Some("3") {
        recommendations.push(
            "virtual workspace is missing `workspace.resolver = \"3\"`; Cargo's rust-version-aware fallback is clearer with resolver 3"
                .to_string(),
        );
    }
    let packages_by_id = metadata
        .packages
        .iter()
        .map(|package| {
            let id = package.id.repr.clone();
            package_to_resolved(package, &metadata).map(|resolved| (id, resolved))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let graph = resolve_graph(&metadata)?;

    info!(
        workspace_root = %workspace_root.display(),
        packages = metadata.packages.len(),
        workspace_members = metadata.workspace_members.len(),
        is_virtual_workspace,
        resolver = ?resolver,
        "loaded workspace metadata"
    );

    Ok(WorkspaceData {
        workspace_root,
        workspace_manifest,
        is_virtual_workspace,
        resolver,
        recommendations,
        metadata,
        packages_by_id,
        graph,
    })
}

pub fn select_packages(workspace: &WorkspaceData, args: &SelectionArgs) -> Result<Selection> {
    debug!(
        manifest_path = ?args.manifest_path,
        workspace = args.workspace,
        packages = ?args.package,
        rust_version = ?args.rust_version,
        "selecting workspace packages"
    );
    let selected_ids = if !args.package.is_empty() {
        match_selected_packages(&workspace.metadata, &args.package)?
    } else if args.workspace || workspace.is_virtual_workspace {
        workspace
            .metadata
            .workspace_members
            .iter()
            .map(|id| id.repr.clone())
            .collect()
    } else {
        let root = workspace.metadata.root_package().ok_or_else(|| {
            anyhow!("this workspace has no root package; pass --workspace or --package")
        })?;
        vec![root.id.repr.clone()]
    };

    let members = selected_ids
        .into_iter()
        .map(|id| {
            let package = workspace
                .metadata
                .packages
                .iter()
                .find(|package| package.id.repr == id)
                .ok_or_else(|| anyhow!("selected package `{id}` not found in metadata"))?;
            Ok(SelectedMember {
                package_id: package.id.repr.clone(),
                package_name: package.name.to_string(),
                manifest_path: package.manifest_path.clone().into_std_path_buf(),
                rust_version: package.rust_version.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let target = select_target(&members, args.rust_version.as_deref())?;

    info!(
        selected_members = members.len(),
        target_mode = ?target.mode,
        target_rust_version = ?target.target_rust_version,
        "selected workspace packages"
    );

    Ok(Selection { members, target })
}

pub fn normalize_rust_version(input: &str) -> Result<Version> {
    let parts = input.split('.').collect::<Vec<_>>();
    let normalized = match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        3 => input.to_string(),
        _ => bail!("invalid Rust version `{input}`"),
    };
    Version::parse(&normalized).map_err(Into::into)
}

pub fn display_rust_version(version: &Version) -> String {
    if version.patch == 0 {
        format!("{}.{}", version.major, version.minor)
    } else {
        version.to_string()
    }
}

fn select_target(members: &[SelectedMember], explicit: Option<&str>) -> Result<TargetSelection> {
    if let Some(value) = explicit {
        let version = normalize_rust_version(value)?;
        return Ok(TargetSelection {
            mode: TargetSelectionMode::Explicit,
            target_rust_version: Some(display_rust_version(&version)),
            members: members_to_targets(members),
            notes: Vec::new(),
        });
    }

    let known = members
        .iter()
        .filter_map(|member| {
            member
                .rust_version
                .as_ref()
                .map(|version| (member, version))
        })
        .collect::<Vec<_>>();
    let unique_versions = known
        .iter()
        .map(|(_, version)| display_rust_version(version))
        .collect::<BTreeSet<_>>();

    let mode = if members.len() == 1 && known.len() == 1 {
        TargetSelectionMode::SelectedPackage
    } else if !members.is_empty() && unique_versions.len() == 1 && known.len() == members.len() {
        TargetSelectionMode::WorkspaceUniform
    } else if unique_versions.len() > 1 {
        TargetSelectionMode::WorkspaceMixed
    } else {
        TargetSelectionMode::Missing
    };

    let target_rust_version = if unique_versions.len() == 1 && known.len() == members.len() {
        unique_versions.into_iter().next()
    } else {
        None
    };
    let mut notes = Vec::new();
    if matches!(mode, TargetSelectionMode::WorkspaceMixed) {
        notes.push(
            "selected packages use different `rust-version` values; results are grouped by affected member".to_string(),
        );
    }
    if matches!(mode, TargetSelectionMode::Missing) {
        notes.push(
            "at least one selected package is missing `rust-version`; compatibility cannot be asserted for that member".to_string(),
        );
    }

    Ok(TargetSelection {
        mode,
        target_rust_version,
        members: members_to_targets(members),
        notes,
    })
}

fn members_to_targets(members: &[SelectedMember]) -> Vec<MemberTarget> {
    let mut targets = members
        .iter()
        .map(|member| MemberTarget {
            package_id: member.package_id.clone(),
            package_name: member.package_name.clone(),
            rust_version: member.rust_version.as_ref().map(display_rust_version),
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    targets
}

fn match_selected_packages(metadata: &Metadata, specs: &[String]) -> Result<Vec<String>> {
    let workspace_member_ids = metadata
        .workspace_members
        .iter()
        .map(|id| id.repr.clone())
        .collect::<BTreeSet<_>>();
    let mut matched = BTreeSet::new();
    for spec in specs {
        let candidates = metadata
            .packages
            .iter()
            .filter(|package| {
                workspace_member_ids.contains(&package.id.repr)
                    && (package.name.to_string() == *spec
                        || package.id.repr == *spec
                        || package.manifest_path.as_str().contains(spec))
            })
            .map(|package| package.id.repr.clone())
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            bail!("package spec `{spec}` did not match any workspace member");
        }
        matched.extend(candidates);
    }
    Ok(matched.into_iter().collect())
}

fn package_to_resolved(package: &Package, metadata: &Metadata) -> Result<ResolvedPackage> {
    let workspace_member = metadata
        .workspace_members
        .iter()
        .any(|id| id == &package.id);
    let source = package.source.as_ref().map(ToString::to_string);
    let source_kind = match source.as_deref() {
        None if workspace_member => PackageSourceKind::Workspace,
        Some(value) if value.starts_with("registry+") => PackageSourceKind::Registry,
        Some(value) if value.starts_with("git+") => PackageSourceKind::Git,
        None => PackageSourceKind::Path,
        _ => PackageSourceKind::Unknown,
    };

    Ok(ResolvedPackage {
        id: package.id.repr.clone(),
        name: package.name.to_string(),
        version: package.version.clone(),
        source,
        source_kind,
        manifest_path: package.manifest_path.clone().into_std_path_buf(),
        rust_version: package.rust_version.as_ref().map(display_rust_version),
        workspace_member,
    })
}

fn resolve_graph(metadata: &Metadata) -> Result<BTreeMap<String, Vec<String>>> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| anyhow!("cargo metadata returned no resolve graph"))?;
    let mut graph = BTreeMap::new();
    for node in &resolve.nodes {
        let mut deps = node
            .deps
            .iter()
            .map(|dep| dep.pkg.repr.clone())
            .collect::<Vec<_>>();
        deps.sort();
        graph.insert(node.id.repr.clone(), deps);
    }
    Ok(graph)
}

fn workspace_resolver(path: &Path) -> Result<Option<String>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let document = contents.parse::<DocumentMut>()?;
    let package = document
        .get("package")
        .and_then(|item| item.as_table_like())
        .and_then(|table| table.get("resolver"))
        .and_then(|item| item.as_value())
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let workspace = document
        .get("workspace")
        .and_then(|item| item.as_table_like())
        .and_then(|table| table.get("resolver"))
        .and_then(|item| item.as_value())
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    Ok(workspace.or(package))
}

pub fn package_id_from_query<'a>(
    workspace: &'a WorkspaceData,
    query: &str,
) -> Option<&'a PackageId> {
    workspace
        .metadata
        .packages
        .iter()
        .find(|package| {
            package.id.repr == query
                || package.name.to_string() == query
                || format!("{}@{}", package.name, package.version) == query
        })
        .map(|package| &package.id)
}
