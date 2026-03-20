use crate::cli::OutputFormat;
use crate::model::{
    ExplainReport, ManifestSuggestion, ResolveReport, ScanReport, Selection, WorkspaceData,
};
use anyhow::Result;
use itertools::Itertools;

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
                "  {}@{} ({})",
                issue.package.name, issue.package.version, issue.reason
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
                "  {}@{} ({})",
                issue.package.name, issue.package.version, issue.reason
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
                "- `{}`@`{}`: {}",
                issue.package.name, issue.package.version, issue.reason
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
                "- `{}`@`{}`: {}",
                issue.package.name, issue.package.version, issue.reason
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
                change.package_name,
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
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
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
            "  resolved package: {}@{}",
            package.name, package.version
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
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}
