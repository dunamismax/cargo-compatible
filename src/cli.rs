use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const AFTER_HELP: &str = "\
Examples:
  cargo compatible scan --workspace
  cargo compatible scan --package my-crate --format json
  cargo compatible resolve --rust-version 1.70
  cargo compatible resolve --write-candidate .cargo-compatible/candidate/Cargo.lock
  cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
  cargo compatible suggest-manifest --package my-crate
  cargo compatible explain serde";

#[derive(Parser, Debug)]
#[command(name = "cargo-compatible", bin_name = "cargo compatible", version, after_help = AFTER_HELP)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Scan(ScanCommand),
    Resolve(ResolveCommand),
    ApplyLock(ApplyLockCommand),
    SuggestManifest(SuggestManifestCommand),
    Explain(ExplainCommand),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Markdown,
}

#[derive(Args, Debug, Clone)]
pub struct SelectionArgs {
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
    #[arg(long)]
    pub rust_version: Option<String>,
    #[arg(long)]
    pub workspace: bool,
    #[arg(long = "package", short = 'p')]
    pub package: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ScanCommand {
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

#[derive(Args, Debug, Clone)]
pub struct ResolveCommand {
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
    #[arg(long = "write-candidate")]
    pub write_candidate: Option<PathBuf>,
    #[arg(long)]
    pub write_report: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ApplyLockCommand {
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
    #[arg(long)]
    pub candidate_lockfile: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SuggestManifestCommand {
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[arg(long)]
    pub allow_major: bool,
    #[arg(long)]
    pub write_manifests: bool,
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ExplainCommand {
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[arg(value_name = "CRATE-OR-PKGID")]
    pub query: String,
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,
}
