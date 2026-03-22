pub mod cli;
pub mod compat;
pub mod explain;
pub mod index;
pub mod manifest_edit;
pub mod metadata;
pub mod model;
pub mod report;
pub mod resolution;
pub mod temp_workspace;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, OutputFormat, ResolveCommand};
use compat::analyze_current_workspace;
use explain::build_explain_report;
use index::registry_lookup_for_workspace;
use manifest_edit::{apply_manifest_suggestions, suggest_manifest_changes};
use metadata::{load_workspace, select_packages};
use report::{
    render_explain_report, render_manifest_suggestions_report, render_resolve_report,
    render_scan_report,
};
use resolution::{apply_candidate_lockfile, build_candidate_resolution};
use std::fs;
use std::path::PathBuf;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    dispatch(cli)
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Scan(command) => {
            let workspace = load_workspace(command.selection.manifest_path.as_deref())?;
            let selection = select_packages(&workspace, &command.selection)?;
            let report = analyze_current_workspace(&workspace, &selection)?;
            print_output(command.format, render_scan_report(&report, command.format)?)?;
        }
        Commands::Resolve(command) => {
            let workspace = load_workspace(command.selection.manifest_path.as_deref())?;
            let selection = select_packages(&workspace, &command.selection)?;
            let report = build_candidate_resolution(&workspace, &selection, &command)?;
            let rendered = render_resolve_report(&report, command.format)?;
            if let Some(path) = command.write_report.as_ref() {
                persist_text(path, rendered.as_bytes())?;
            }
            if let Some(path) = command.write_candidate.as_ref() {
                if let Some(candidate) = report.candidate_lockfile.as_ref() {
                    persist_text(path, candidate.as_bytes())?;
                }
            }
            print_output(command.format, rendered)?;
        }
        Commands::ApplyLock(command) => {
            let workspace = load_workspace(command.manifest_path.as_deref())?;
            let applied = apply_candidate_lockfile(
                &workspace.workspace_root,
                command
                    .candidate_lockfile
                    .unwrap_or_else(default_candidate_lockfile_path),
            )?;
            println!("{applied}");
        }
        Commands::SuggestManifest(command) => {
            let workspace = load_workspace(command.selection.manifest_path.as_deref())?;
            let selection = select_packages(&workspace, &command.selection)?;
            let resolution = build_candidate_resolution(
                &workspace,
                &selection,
                &ResolveCommand {
                    selection: command.selection.clone(),
                    format: command.format,
                    write_candidate: None,
                    write_report: None,
                },
            )?;
            let registry = registry_lookup_for_workspace(&workspace.workspace_root)?;
            let suggestions = suggest_manifest_changes(
                &workspace,
                &selection,
                &resolution,
                registry.as_ref(),
                command.allow_major,
            )?;
            if command.write_manifests {
                apply_manifest_suggestions(&suggestions)?;
            }
            print_output(
                command.format,
                render_manifest_suggestions_report(
                    &workspace,
                    &selection,
                    &resolution,
                    &suggestions,
                    command.format,
                    command.write_manifests,
                )?,
            )?;
        }
        Commands::Explain(command) => {
            let workspace = load_workspace(command.selection.manifest_path.as_deref())?;
            let selection = select_packages(&workspace, &command.selection)?;
            let report = build_explain_report(&workspace, &selection, &command)?;
            print_output(
                command.format,
                render_explain_report(&report, command.format)?,
            )?;
        }
    }

    Ok(())
}

fn persist_text(path: &PathBuf, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn print_output(format: OutputFormat, contents: String) -> Result<()> {
    match format {
        OutputFormat::Human | OutputFormat::Markdown | OutputFormat::Json => {
            println!("{contents}");
        }
    }

    Ok(())
}

fn default_candidate_lockfile_path() -> PathBuf {
    PathBuf::from(".cargo-compatible/candidate/Cargo.lock")
}
