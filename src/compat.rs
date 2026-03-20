use crate::metadata::display_rust_version;
use crate::model::{
    CompatibilityStatus, DependencyPath, PackageIssue, PackageSummary, ScanReport, SelectedMember,
    Selection, TargetSelectionMode, WorkspaceData, WorkspaceSummary,
};
use anyhow::{anyhow, Result};
use semver::Version;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn analyze_current_workspace(
    workspace: &WorkspaceData,
    selection: &Selection,
) -> Result<ScanReport> {
    let mut issues_by_id: BTreeMap<String, PackageIssue> = BTreeMap::new();
    let mut incompatible_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut unknown_counts: BTreeMap<String, usize> = BTreeMap::new();
    let paths_by_root = selection
        .members
        .iter()
        .map(|member| {
            shortest_paths_from_root(workspace, member)
                .map(|paths| (member.package_id.clone(), paths))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    for member in &selection.members {
        let Some(target) = member_target(selection, member) else {
            continue;
        };
        let paths = paths_by_root
            .get(&member.package_id)
            .ok_or_else(|| anyhow!("missing path map for {}", member.package_name))?;
        for (package_id, path) in paths {
            let package = workspace
                .packages_by_id
                .get(package_id)
                .ok_or_else(|| anyhow!("package `{package_id}` missing from package map"))?;
            let (status, reason) = classify_package(package.rust_version.as_deref(), &target);
            if matches!(status, CompatibilityStatus::Compatible) {
                continue;
            }
            let issue = issues_by_id
                .entry(package_id.clone())
                .or_insert_with(|| PackageIssue {
                    package: package.clone(),
                    status: status.clone(),
                    target_rust_version: Some(display_rust_version(&target)),
                    reason: reason.clone(),
                    affected_members: BTreeSet::new(),
                    paths: Vec::new(),
                });
            issue.status = strongest_status(issue.status.clone(), status.clone());
            issue.target_rust_version = Some(display_rust_version(&target));
            issue.reason = reason.clone();
            issue.affected_members.insert(member.package_name.clone());
            issue.paths.push(DependencyPath {
                member: member.package_name.clone(),
                target_rust_version: Some(display_rust_version(&target)),
                packages: path.clone(),
            });
            match status {
                CompatibilityStatus::Incompatible => {
                    *incompatible_counts
                        .entry(member.package_name.clone())
                        .or_default() += 1;
                }
                CompatibilityStatus::Unknown => {
                    *unknown_counts
                        .entry(member.package_name.clone())
                        .or_default() += 1;
                }
                CompatibilityStatus::Compatible => {}
            }
        }
    }

    if matches!(selection.target.mode, TargetSelectionMode::Missing) {
        for member in selection
            .members
            .iter()
            .filter(|member| member.rust_version.is_none())
        {
            if let Some(paths) = paths_by_root.get(&member.package_id) {
                for (package_id, path) in paths {
                    let package = workspace.packages_by_id.get(package_id).ok_or_else(|| {
                        anyhow!("package `{package_id}` missing from package map")
                    })?;
                    let issue =
                        issues_by_id
                            .entry(package_id.clone())
                            .or_insert_with(|| PackageIssue {
                                package: package.clone(),
                                status: CompatibilityStatus::Unknown,
                                target_rust_version: None,
                                reason: "selected member is missing `rust-version`".to_string(),
                                affected_members: BTreeSet::new(),
                                paths: Vec::new(),
                            });
                    issue.affected_members.insert(member.package_name.clone());
                    issue.paths.push(DependencyPath {
                        member: member.package_name.clone(),
                        target_rust_version: None,
                        packages: path.clone(),
                    });
                }
            }
        }
    }

    let mut incompatible_packages = issues_by_id
        .values()
        .filter(|issue| matches!(issue.status, CompatibilityStatus::Incompatible))
        .cloned()
        .collect::<Vec<_>>();
    incompatible_packages.sort_by(issue_sort_key);
    let mut unknown_packages = issues_by_id
        .values()
        .filter(|issue| matches!(issue.status, CompatibilityStatus::Unknown))
        .cloned()
        .collect::<Vec<_>>();
    unknown_packages.sort_by(issue_sort_key);

    let mut package_summaries = selection
        .members
        .iter()
        .map(|member| PackageSummary {
            package_name: member.package_name.clone(),
            incompatible: incompatible_counts
                .get(&member.package_name)
                .copied()
                .unwrap_or(0),
            unknown: unknown_counts
                .get(&member.package_name)
                .copied()
                .unwrap_or(0),
        })
        .collect::<Vec<_>>();
    package_summaries.sort_by(|left, right| left.package_name.cmp(&right.package_name));

    Ok(ScanReport {
        target: selection.target.clone(),
        workspace: WorkspaceSummary {
            workspace_root: workspace.workspace_root.clone(),
            selected_members: selection
                .members
                .iter()
                .map(|member| member.package_name.clone())
                .collect(),
            is_virtual_workspace: workspace.is_virtual_workspace,
            resolver: workspace.resolver.clone(),
            recommendations: workspace.recommendations.clone(),
        },
        package_summaries,
        incompatible_packages,
        unknown_packages,
        notes: workspace.recommendations.clone(),
    })
}

pub fn classify_package(
    rust_version: Option<&str>,
    target: &Version,
) -> (CompatibilityStatus, String) {
    match rust_version {
        Some(value) => {
            let package_rust = parse_version_display(value);
            if package_rust > *target {
                (
                    CompatibilityStatus::Incompatible,
                    format!(
                        "resolved package declares rust-version {value}, which exceeds target {}",
                        display_rust_version(target)
                    ),
                )
            } else {
                (
                    CompatibilityStatus::Compatible,
                    format!(
                        "resolved package declares rust-version {value}, which is compatible with target {}",
                        display_rust_version(target)
                    ),
                )
            }
        }
        None => (
            CompatibilityStatus::Unknown,
            "resolved package does not declare `rust-version`; compatibility is unknown"
                .to_string(),
        ),
    }
}

fn strongest_status(
    current: CompatibilityStatus,
    next: CompatibilityStatus,
) -> CompatibilityStatus {
    match (current, next) {
        (CompatibilityStatus::Incompatible, _) | (_, CompatibilityStatus::Incompatible) => {
            CompatibilityStatus::Incompatible
        }
        (CompatibilityStatus::Unknown, _) | (_, CompatibilityStatus::Unknown) => {
            CompatibilityStatus::Unknown
        }
        _ => CompatibilityStatus::Compatible,
    }
}

fn parse_version_display(value: &str) -> Version {
    let parts = value.split('.').collect::<Vec<_>>();
    let normalized = match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => value.to_string(),
    };
    Version::parse(&normalized).expect("package rust-version should parse")
}

fn shortest_paths_from_root(
    workspace: &WorkspaceData,
    member: &SelectedMember,
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut queue = VecDeque::from([member.package_id.clone()]);
    let mut predecessors = BTreeMap::<String, Option<String>>::new();
    predecessors.insert(member.package_id.clone(), None);

    while let Some(package_id) = queue.pop_front() {
        let deps = workspace
            .graph
            .get(&package_id)
            .cloned()
            .unwrap_or_default();
        for dep in deps {
            if predecessors.contains_key(&dep) {
                continue;
            }
            predecessors.insert(dep.clone(), Some(package_id.clone()));
            queue.push_back(dep);
        }
    }

    let mut paths = BTreeMap::new();
    for package_id in predecessors.keys() {
        let mut chain = Vec::new();
        let mut current = Some(package_id.clone());
        while let Some(id) = current {
            let package = workspace
                .packages_by_id
                .get(&id)
                .ok_or_else(|| anyhow!("package `{id}` missing from package map"))?;
            chain.push(format!("{}@{}", package.name, package.version));
            current = predecessors.get(&id).cloned().flatten();
        }
        chain.reverse();
        paths.insert(package_id.clone(), chain);
    }

    Ok(paths)
}

fn member_target(selection: &Selection, member: &SelectedMember) -> Option<Version> {
    if let Some(explicit) = selection.target.target_rust_version.as_deref() {
        if matches!(
            selection.target.mode,
            TargetSelectionMode::Explicit
                | TargetSelectionMode::SelectedPackage
                | TargetSelectionMode::WorkspaceUniform
        ) {
            return Some(parse_version_display(explicit));
        }
    }
    member.rust_version.clone()
}

fn issue_sort_key(left: &PackageIssue, right: &PackageIssue) -> std::cmp::Ordering {
    left.package
        .name
        .cmp(&right.package.name)
        .then_with(|| left.package.version.cmp(&right.package.version))
        .then_with(|| left.package.id.cmp(&right.package.id))
}
