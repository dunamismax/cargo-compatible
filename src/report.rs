use crate::cli::OutputFormat;
use crate::identity::{
    base_package_label, colliding_base_labels, source_detail, unique_package_label,
};
use crate::model::{
    BlockerKind, CandidateVersionChange, CompatibilityStatus, ExplainReport, ManifestSuggestion,
    ResolveReport, ScanReport, Selection, WorkspaceData,
};
use anyhow::Result;
use itertools::Itertools;
use std::path::Path;

pub fn render_scan_report(report: &ScanReport, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Markdown => Ok(render_scan_markdown(report)),
        OutputFormat::Human => Ok(render_scan_human(report)),
    }
}

pub fn render_resolve_report(report: &ResolveReport, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Markdown => Ok(render_resolve_markdown(report)),
        OutputFormat::Human => Ok(render_resolve_human(report)),
    }
}

pub fn render_manifest_suggestions_report(
    _workspace: &WorkspaceData,
    _selection: &Selection,
    resolution: &ResolveReport,
    suggestions: &[ManifestSuggestion],
    format: OutputFormat,
    wrote_manifests: bool,
) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&serde_json::json!({
            "candidate_resolution": resolution,
            "manifest_suggestions": suggestions,
            "write_manifests": wrote_manifests
        }))?),
        OutputFormat::Markdown => Ok(render_manifest_markdown(
            resolution,
            suggestions,
            wrote_manifests,
        )),
        OutputFormat::Human => Ok(render_manifest_human(
            resolution,
            suggestions,
            wrote_manifests,
        )),
    }
}

pub fn render_explain_report(report: &ExplainReport, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Markdown => Ok(render_explain_markdown(report)),
        OutputFormat::Human => Ok(render_explain_human(report)),
    }
}

fn render_scan_human(report: &ScanReport) -> String {
    let issue_package_collisions = colliding_base_labels(
        report
            .incompatible_packages
            .iter()
            .chain(report.unknown_packages.iter())
            .map(|issue| (&issue.package, report.workspace.workspace_root.as_path())),
    );
    let mut lines = Vec::new();
    lines.push("Current State".to_string());
    lines.push(format!(
        "  target selection: {}",
        report
            .target
            .target_rust_version
            .clone()
            .unwrap_or_else(|| "mixed-or-missing".to_string())
    ));
    lines.push(format!(
        "  selected members: {}",
        report.workspace.selected_members.iter().join(", ")
    ));
    if !report.workspace.recommendations.is_empty() {
        lines.push("  recommendations:".to_string());
        for recommendation in &report.workspace.recommendations {
            lines.push(format!("    - {recommendation}"));
        }
    }
    lines.push("".to_string());
    lines.push("Package Summaries".to_string());
    for summary in &report.package_summaries {
        lines.push(format!(
            "  {}: {} incompatible, {} unknown",
            summary.package_name, summary.incompatible, summary.unknown
        ));
    }
    lines.push("".to_string());
    lines.push("Incompatible Packages".to_string());
    if report.incompatible_packages.is_empty() {
        lines.push("  none".to_string());
    } else {
        for issue in &report.incompatible_packages {
            lines.push(format!(
                "  {} ({})",
                unique_package_label(
                    &issue.package,
                    &report.workspace.workspace_root,
                    &issue_package_collisions,
                ),
                issue.reason
            ));
            for path in &issue.paths {
                lines.push(format!(
                    "    via {}: {}",
                    path.member,
                    path.packages.join(" -> ")
                ));
            }
        }
    }
    lines.push("".to_string());
    lines.push("Unknown Packages".to_string());
    if report.unknown_packages.is_empty() {
        lines.push("  none".to_string());
    } else {
        for issue in &report.unknown_packages {
            lines.push(format!(
                "  {} ({})",
                unique_package_label(
                    &issue.package,
                    &report.workspace.workspace_root,
                    &issue_package_collisions,
                ),
                issue.reason
            ));
            for path in &issue.paths {
                lines.push(format!(
                    "    via {}: {}",
                    path.member,
                    path.packages.join(" -> ")
                ));
            }
        }
    }
    lines.join("\n")
}

fn render_scan_markdown(report: &ScanReport) -> String {
    let issue_package_collisions = colliding_base_labels(
        report
            .incompatible_packages
            .iter()
            .chain(report.unknown_packages.iter())
            .map(|issue| (&issue.package, report.workspace.workspace_root.as_path())),
    );
    let mut output = vec![
        "# Current State".to_string(),
        format!(
            "- Target selection: {}",
            report
                .target
                .target_rust_version
                .clone()
                .unwrap_or_else(|| "mixed-or-missing".to_string())
        ),
        format!(
            "- Selected members: {}",
            report.workspace.selected_members.iter().join(", ")
        ),
        "".to_string(),
        "## Incompatible Packages".to_string(),
    ];
    if report.incompatible_packages.is_empty() {
        output.push("- None".to_string());
    } else {
        for issue in &report.incompatible_packages {
            output.push(format!(
                "- {}: {}",
                backtick(&unique_package_label(
                    &issue.package,
                    &report.workspace.workspace_root,
                    &issue_package_collisions,
                )),
                issue.reason
            ));
        }
    }
    output.push("".to_string());
    output.push("## Unknown Packages".to_string());
    if report.unknown_packages.is_empty() {
        output.push("- None".to_string());
    } else {
        for issue in &report.unknown_packages {
            output.push(format!(
                "- {}: {}",
                backtick(&unique_package_label(
                    &issue.package,
                    &report.workspace.workspace_root,
                    &issue_package_collisions,
                )),
                issue.reason
            ));
        }
    }
    output.join("\n")
}

fn render_resolve_human(report: &ResolveReport) -> String {
    let mut lines = vec![
        "Current State".to_string(),
        format!(
            "  incompatible: {}, unknown: {}",
            report.current.incompatible_packages.len(),
            report.current.unknown_packages.len()
        ),
        "".to_string(),
        "Candidate Lockfile Improvements".to_string(),
    ];
    if report.version_changes.is_empty() {
        lines.push("  no version changes".to_string());
    } else {
        for change in &report.version_changes {
            lines.push(format!(
                "  {}: {} -> {}",
                format_version_change_identity(change),
                change
                    .before
                    .clone()
                    .unwrap_or_else(|| "<none>".to_string()),
                change.after.clone().unwrap_or_else(|| "<none>".to_string())
            ));
        }
    }
    lines.push("".to_string());
    lines.push("Remaining Blockers".to_string());
    if report.remaining_blockers.is_empty() {
        lines.push("  none".to_string());
    } else {
        for blocker in &report.remaining_blockers {
            lines.push(format!("  {blocker}"));
        }
    }
    lines.join("\n")
}

fn render_resolve_markdown(report: &ResolveReport) -> String {
    let mut lines = vec![
        "# Candidate Resolution".to_string(),
        format!(
            "- Current blockers: {} incompatible, {} unknown",
            report.current.incompatible_packages.len(),
            report.current.unknown_packages.len()
        ),
        format!(
            "- Candidate blockers: {} incompatible, {} unknown",
            report.candidate.incompatible_packages.len(),
            report.candidate.unknown_packages.len()
        ),
        format!(
            "- Candidate lockfile captured: {}",
            yes_no(report.candidate_lockfile.is_some())
        ),
        "".to_string(),
        "## Version Changes".to_string(),
    ];
    if report.version_changes.is_empty() {
        lines.push("- None".to_string());
    } else {
        for change in &report.version_changes {
            lines.push(format!(
                "- {}: `{}` -> `{}`",
                backtick(&format_version_change_identity(change)),
                change.before.as_deref().unwrap_or("<none>"),
                change.after.as_deref().unwrap_or("<none>"),
            ));
        }
    }
    lines.push("".to_string());
    lines.push("## Improved Packages".to_string());
    if report.improved_packages.is_empty() {
        lines.push("- None".to_string());
    } else {
        for package in &report.improved_packages {
            lines.push(format!("- {}", backtick(package)));
        }
    }
    lines.push("".to_string());
    lines.push("## Remaining Blockers".to_string());
    if report.remaining_blockers.is_empty() {
        lines.push("- None".to_string());
    } else {
        for blocker in &report.remaining_blockers {
            lines.push(format!("- {}", backtick(blocker)));
        }
    }
    if !report.notes.is_empty() {
        lines.push("".to_string());
        lines.push("## Notes".to_string());
        for note in &report.notes {
            lines.push(format!("- {note}"));
        }
    }
    lines.join("\n")
}

fn render_manifest_human(
    resolution: &ResolveReport,
    suggestions: &[ManifestSuggestion],
    wrote_manifests: bool,
) -> String {
    let mut lines = vec![
        "Candidate Lockfile Improvements".to_string(),
        format!(
            "  improved packages: {}",
            resolution.improved_packages.iter().join(", ")
        ),
        "".to_string(),
        "Suggested Direct Manifest Changes".to_string(),
    ];
    if suggestions.is_empty() {
        lines.push("  none".to_string());
    } else {
        for suggestion in suggestions {
            lines.push(format!(
                "  {} in {}: {} -> {} ({})",
                suggestion.dependency_key,
                suggestion.package_name,
                suggestion.current_requirement,
                suggestion.suggested_requirement,
                suggestion.reason
            ));
        }
    }
    lines.push("".to_string());
    lines.push(if wrote_manifests {
        "Manifest write mode: applied".to_string()
    } else {
        "Manifest write mode: dry-run".to_string()
    });
    lines.join("\n")
}

fn render_manifest_markdown(
    resolution: &ResolveReport,
    suggestions: &[ManifestSuggestion],
    wrote_manifests: bool,
) -> String {
    let mut lines = vec![
        "# Suggested Direct Manifest Changes".to_string(),
        format!("- Dry run: {}", !wrote_manifests),
        format!(
            "- Remaining blockers: {}",
            resolution.remaining_blockers.iter().join(", ")
        ),
    ];
    for suggestion in suggestions {
        lines.push(format!(
            "- `{}` in `{}`: `{}` -> `{}`",
            suggestion.dependency_key,
            suggestion.package_name,
            suggestion.current_requirement,
            suggestion.suggested_requirement
        ));
    }
    lines.join("\n")
}

fn render_explain_human(report: &ExplainReport) -> String {
    let mut lines = vec![
        "Explanation".to_string(),
        format!("  query: {}", report.query),
    ];
    if let Some(package) = &report.package {
        lines.push(format!(
            "  resolved package: {}",
            base_package_label(package, Path::new("."))
        ));
    }
    if let Some(reason) = &report.current_reason {
        lines.push(format!("  current result: {reason}"));
    }
    if let Some(blocker) = &report.blocker {
        lines.push(format!("  blocker: {:?}", blocker));
    }
    if !report.current_paths.is_empty() {
        lines.push("".to_string());
        lines.push("Dependency Paths".to_string());
        for path in &report.current_paths {
            lines.push(format!("  {}: {}", path.member, path.packages.join(" -> ")));
        }
    }
    lines.join("\n")
}

fn render_explain_markdown(report: &ExplainReport) -> String {
    let mut lines = vec![
        "# Explanation".to_string(),
        format!("- Query: {}", backtick(&report.query)),
        format!(
            "- Target selection: {}",
            report
                .target
                .target_rust_version
                .clone()
                .unwrap_or_else(|| "mixed-or-missing".to_string())
        ),
    ];
    if let Some(package) = &report.package {
        lines.push(format!(
            "- Resolved package: {}",
            backtick(&base_package_label(package, Path::new(".")))
        ));
    }
    if let Some(status) = &report.current_status {
        lines.push(format!(
            "- Current status: {}",
            backtick(compatibility_status(status))
        ));
    }
    if let Some(reason) = &report.current_reason {
        lines.push(format!("- Current result: {reason}"));
    }
    if let Some(rust_version) = &report.current_rust_version {
        lines.push(format!(
            "- Current rust-version: {}",
            backtick(rust_version)
        ));
    }
    if let Some(version) = &report.candidate_version {
        lines.push(format!("- Candidate version: {}", backtick(version)));
    }
    if let Some(status) = &report.candidate_status {
        lines.push(format!(
            "- Candidate status: {}",
            backtick(compatibility_status(status))
        ));
    }
    if let Some(blocker) = &report.blocker {
        lines.push(format!("- Blocker: {}", backtick(blocker_kind(blocker))));
    }
    if !report.current_paths.is_empty() {
        lines.push("".to_string());
        lines.push("## Dependency Paths".to_string());
        for path in &report.current_paths {
            lines.push(format!(
                "- {}: {}",
                backtick(&path.member),
                backtick(&path.packages.join(" -> "))
            ));
        }
    }
    if !report.notes.is_empty() {
        lines.push("".to_string());
        lines.push("## Notes".to_string());
        for note in &report.notes {
            lines.push(format!("- {note}"));
        }
    }
    lines.join("\n")
}

fn format_version_change_identity(change: &CandidateVersionChange) -> String {
    if let Some(label) = &change.package_label {
        return label.clone();
    }
    let mut label = change.package_name.clone();
    if let Some(detail) = source_detail(change.source.as_deref()) {
        label.push_str(&format!(" [{detail}]"));
    }
    label
}

fn backtick(value: &str) -> String {
    format!("`{value}`")
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn compatibility_status(status: &CompatibilityStatus) -> &'static str {
    match status {
        CompatibilityStatus::Compatible => "compatible",
        CompatibilityStatus::Incompatible => "incompatible",
        CompatibilityStatus::Unknown => "unknown",
    }
}

fn blocker_kind(blocker: &BlockerKind) -> &'static str {
    match blocker {
        BlockerKind::Compatible => "compatible",
        BlockerKind::UnknownRustVersion => "unknown_rust_version",
        BlockerKind::LockfileDrift => "lockfile_drift",
        BlockerKind::DirectDependencyTooNew => "direct_dependency_too_new",
        BlockerKind::FeatureRequirementTooRestrictive => "feature_requirement_too_restrictive",
        BlockerKind::MixedWorkspaceRustVersionUnification => {
            "mixed_workspace_rust_version_unification"
        }
        BlockerKind::PathOrGitConstraint => "path_or_git_constraint",
        BlockerKind::NonRegistryConstraint => "non_registry_constraint",
    }
}

#[cfg(test)]
mod tests {
    use super::{render_resolve_human, render_scan_human};
    use crate::model::{
        CandidateVersionChange, CompatibilityStatus, DependencyPath, PackageIssue,
        PackageSourceKind, PackageSummary, ResolveReport, ResolvedPackage, ScanReport,
        TargetSelection, TargetSelectionMode, WorkspaceSummary,
    };
    use semver::Version;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn resolve_human_disambiguates_registry_version_changes() {
        let report = ResolveReport {
            current: empty_scan_report(),
            candidate: empty_scan_report(),
            version_changes: vec![CandidateVersionChange {
                package_name: "compat-demo".to_string(),
                source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
                package_label: None,
                before: Some("1.2.0".to_string()),
                after: Some("1.1.0".to_string()),
            }],
            improved_packages: Vec::new(),
            remaining_blockers: Vec::new(),
            candidate_lockfile: None,
            notes: Vec::new(),
        };

        let rendered = render_resolve_human(&report);

        assert!(rendered.contains("compat-demo [registry: crates.io]: 1.2.0 -> 1.1.0"));
    }

    #[test]
    fn scan_human_labels_path_dependencies_with_relative_location() {
        let mut report = empty_scan_report();
        report.incompatible_packages.push(PackageIssue {
            package: ResolvedPackage {
                id: "path+file:///workspace/too_new#0.1.0".to_string(),
                name: "too_new".to_string(),
                version: Version::new(0, 1, 0),
                source: None,
                source_kind: PackageSourceKind::Path,
                manifest_path: PathBuf::from("/workspace/too_new/Cargo.toml"),
                rust_version: Some("1.70".to_string()),
                workspace_member: false,
            },
            status: CompatibilityStatus::Incompatible,
            target_rust_version: Some("1.60".to_string()),
            reason: "resolved package declares rust-version 1.70, which exceeds target 1.60"
                .to_string(),
            affected_members: BTreeSet::from(["app".to_string()]),
            paths: vec![DependencyPath {
                member: "app".to_string(),
                target_rust_version: Some("1.60".to_string()),
                packages: vec!["app@0.1.0".to_string(), "too_new@0.1.0".to_string()],
            }],
        });

        let rendered = render_scan_human(&report);

        assert!(rendered.contains("too_new@0.1.0 [path: too_new]"));
    }

    fn empty_scan_report() -> ScanReport {
        ScanReport {
            target: TargetSelection {
                mode: TargetSelectionMode::SelectedPackage,
                target_rust_version: Some("1.60".to_string()),
                members: Vec::new(),
                notes: Vec::new(),
            },
            workspace: WorkspaceSummary {
                workspace_root: PathBuf::from("/workspace"),
                selected_members: vec!["app".to_string()],
                is_virtual_workspace: false,
                resolver: Some("3".to_string()),
                recommendations: Vec::new(),
            },
            package_summaries: vec![PackageSummary {
                package_name: "app".to_string(),
                incompatible: 0,
                unknown: 0,
            }],
            incompatible_packages: Vec::new(),
            unknown_packages: Vec::new(),
            notes: Vec::new(),
        }
    }
}
