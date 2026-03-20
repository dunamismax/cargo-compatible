use crate::cli::ExplainCommand;
use crate::compat::analyze_current_workspace;
use crate::metadata::package_id_from_query;
use crate::model::{
    BlockerKind, CompatibilityStatus, ExplainReport, PackageIssue, Selection, WorkspaceData,
};
use crate::resolution::build_candidate_resolution;
use anyhow::Result;

pub fn build_explain_report(
    workspace: &WorkspaceData,
    selection: &Selection,
    command: &ExplainCommand,
) -> Result<ExplainReport> {
    let current = analyze_current_workspace(workspace, selection)?;
    let package_id = package_id_from_query(workspace, &command.query).map(|id| id.repr.clone());
    let package = package_id
        .as_ref()
        .and_then(|id| workspace.packages_by_id.get(id))
        .cloned();
    let package_for_candidate = package.clone();
    let current_issue = package_id.as_ref().and_then(|id| find_issue(&current, id));
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
    let candidate_issue = package_id
        .as_ref()
        .and_then(|id| find_issue(&resolve.candidate, id));
    let blocker = classify_blocker(package.as_ref(), current_issue, candidate_issue, selection);

    Ok(ExplainReport {
        query: command.query.clone(),
        target: selection.target.clone(),
        package,
        current_status: current_issue.map(|issue| issue.status.clone()),
        current_reason: current_issue.map(|issue| issue.reason.clone()),
        current_paths: current_issue
            .map(|issue| issue.paths.clone())
            .unwrap_or_default(),
        current_rust_version: current_issue
            .map(|issue| issue.package.rust_version.clone())
            .unwrap_or(None),
        candidate_version: package_id.as_ref().and_then(|id| {
            resolve
                .candidate
                .incompatible_packages
                .iter()
                .chain(resolve.candidate.unknown_packages.iter())
                .find(|issue| issue.package.id == *id)
                .map(|issue| issue.package.version.to_string())
                .or_else(|| {
                    resolve
                        .version_changes
                        .iter()
                        .find(|change| {
                            package_for_candidate
                                .as_ref()
                                .map(|pkg| change.package_name == pkg.name)
                                .unwrap_or(false)
                        })
                        .and_then(|change| change.after.clone())
                })
        }),
        candidate_status: candidate_issue.map(|issue| issue.status.clone()),
        blocker,
        notes: resolve.notes,
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
