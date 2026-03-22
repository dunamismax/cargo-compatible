use crate::cli::ExplainCommand;
use crate::compat::analyze_current_workspace;
use crate::metadata::resolve_package_query;
use crate::model::{
    BlockerKind, CompatibilityStatus, ExplainReport, PackageIssue, Selection, WorkspaceData,
};
use crate::resolution::build_candidate_resolution;
use anyhow::{anyhow, Result};

pub fn build_explain_report(
    workspace: &WorkspaceData,
    selection: &Selection,
    command: &ExplainCommand,
) -> Result<ExplainReport> {
    let selected_graph = selected_graph_package_ids(workspace, selection);
    let package_id = resolve_package_query(workspace, &selected_graph, &command.query)?;
    let current = analyze_current_workspace(workspace, selection)?;
    let package = workspace
        .packages_by_id
        .get(&package_id)
        .cloned()
        .ok_or_else(|| anyhow!("resolved package `{package_id}` missing from package map"))?;
    let current_issue = find_issue(&current, &package_id);

    let mut candidate_version = None;
    let mut candidate_status = None;
    let mut notes = workspace.recommendations.clone();
    let blocker;

    if current_issue.is_some() {
        let resolve = build_candidate_resolution(
            workspace,
            selection,
            &crate::cli::ResolveCommand {
                selection: command.selection.clone(),
                format: command.format,
                write_candidate: None,
                write_report: None,
            },
        )?;
        let candidate_issue = find_issue(&resolve.candidate, &package_id);
        candidate_version = candidate_issue
            .map(|issue| issue.package.version.to_string())
            .or_else(|| candidate_version_from_changes(&package, &resolve.version_changes));
        candidate_status = candidate_issue.map(|issue| issue.status.clone());
        notes = resolve.notes;
        blocker = classify_blocker(Some(&package), current_issue, candidate_issue, selection);
    } else {
        blocker = classify_blocker(Some(&package), current_issue, None, selection);
    }

    Ok(ExplainReport {
        query: command.query.clone(),
        target: selection.target.clone(),
        package: Some(package),
        current_status: current_issue.map(|issue| issue.status.clone()),
        current_reason: current_issue.map(|issue| issue.reason.clone()),
        current_paths: current_issue
            .map(|issue| issue.paths.clone())
            .unwrap_or_default(),
        current_rust_version: current_issue
            .map(|issue| issue.package.rust_version.clone())
            .unwrap_or(None),
        candidate_version,
        candidate_status,
        blocker,
        notes,
        workspace_root: workspace.workspace_root.clone(),
    })
}

fn find_issue<'a>(
    report: &'a crate::model::ScanReport,
    package_id: &str,
) -> Option<&'a PackageIssue> {
    report
        .incompatible_packages
        .iter()
        .chain(report.unknown_packages.iter())
        .find(|issue| issue.package.id == package_id)
}

fn selected_graph_package_ids(
    workspace: &WorkspaceData,
    selection: &Selection,
) -> std::collections::BTreeSet<String> {
    let mut reachable = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::from_iter(
        selection
            .members
            .iter()
            .map(|member| member.package_id.clone()),
    );

    while let Some(package_id) = queue.pop_front() {
        if !reachable.insert(package_id.clone()) {
            continue;
        }
        for dependency_id in workspace.graph.get(&package_id).into_iter().flatten() {
            queue.push_back(dependency_id.clone());
        }
    }

    reachable
}

fn candidate_version_from_changes(
    package: &crate::model::ResolvedPackage,
    changes: &[crate::model::CandidateVersionChange],
) -> Option<String> {
    let mut matching_changes = changes
        .iter()
        .filter(|change| change.package_name == package.name && change.source == package.source);
    let change = matching_changes.next()?;
    if matching_changes.next().is_some() {
        return None;
    }
    change.after.clone()
}

fn classify_blocker(
    package: Option<&crate::model::ResolvedPackage>,
    current_issue: Option<&PackageIssue>,
    candidate_issue: Option<&PackageIssue>,
    selection: &Selection,
) -> Option<BlockerKind> {
    let package = package?;
    if current_issue.is_none() {
        return Some(BlockerKind::Compatible);
    }
    if package
        .source
        .as_deref()
        .map(|source| source.starts_with("git+"))
        .unwrap_or(false)
        || matches!(package.source_kind, crate::model::PackageSourceKind::Path)
    {
        return Some(BlockerKind::PathOrGitConstraint);
    }
    if package.rust_version.is_none() {
        return Some(BlockerKind::UnknownRustVersion);
    }
    if current_issue.is_some() && candidate_issue.is_none() {
        return Some(BlockerKind::LockfileDrift);
    }
    if matches!(
        selection.target.mode,
        crate::model::TargetSelectionMode::WorkspaceMixed
    ) {
        return Some(BlockerKind::MixedWorkspaceRustVersionUnification);
    }
    match current_issue.map(|issue| &issue.status) {
        Some(CompatibilityStatus::Unknown) => Some(BlockerKind::NonRegistryConstraint),
        Some(CompatibilityStatus::Incompatible) => Some(BlockerKind::DirectDependencyTooNew),
        _ => Some(BlockerKind::Compatible),
    }
}
