use semver::Version;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelectionMode {
    Explicit,
    SelectedPackage,
    WorkspaceUniform,
    WorkspaceMixed,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberTarget {
    pub package_id: String,
    pub package_name: String,
    pub rust_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetSelection {
    pub mode: TargetSelectionMode,
    pub target_rust_version: Option<String>,
    pub members: Vec<MemberTarget>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSourceKind {
    Workspace,
    Registry,
    Git,
    Path,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedPackage {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub source: Option<String>,
    pub source_kind: PackageSourceKind,
    pub manifest_path: PathBuf,
    pub rust_version: Option<String>,
    pub workspace_member: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyPath {
    pub member: String,
    pub target_rust_version: Option<String>,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageIssue {
    pub package: ResolvedPackage,
    pub status: CompatibilityStatus,
    pub target_rust_version: Option<String>,
    pub reason: String,
    pub affected_members: BTreeSet<String>,
    pub paths: Vec<DependencyPath>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageSummary {
    pub package_name: String,
    pub incompatible: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub workspace_root: PathBuf,
    pub selected_members: Vec<String>,
    pub is_virtual_workspace: bool,
    pub resolver: Option<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub target: TargetSelection,
    pub workspace: WorkspaceSummary,
    pub package_summaries: Vec<PackageSummary>,
    pub incompatible_packages: Vec<PackageIssue>,
    pub unknown_packages: Vec<PackageIssue>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateVersionChange {
    pub package_name: String,
    pub source: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolveReport {
    pub current: ScanReport,
    pub candidate: ScanReport,
    pub version_changes: Vec<CandidateVersionChange>,
    pub improved_packages: Vec<String>,
    pub remaining_blockers: Vec<String>,
    pub candidate_lockfile: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    Compatible,
    UnknownRustVersion,
    LockfileDrift,
    DirectDependencyTooNew,
    FeatureRequirementTooRestrictive,
    MixedWorkspaceRustVersionUnification,
    PathOrGitConstraint,
    NonRegistryConstraint,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplainReport {
    pub query: String,
    pub target: TargetSelection,
    pub package: Option<ResolvedPackage>,
    pub current_status: Option<CompatibilityStatus>,
    pub current_reason: Option<String>,
    pub current_paths: Vec<DependencyPath>,
    pub current_rust_version: Option<String>,
    pub candidate_version: Option<String>,
    pub candidate_status: Option<CompatibilityStatus>,
    pub blocker: Option<BlockerKind>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestSuggestion {
    pub package_name: String,
    pub dependency_key: String,
    pub dependency_name: String,
    pub manifest_path: PathBuf,
    pub current_requirement: String,
    pub suggested_requirement: String,
    pub reason: String,
    pub target_rust_version: String,
    pub section: String,
}

#[derive(Debug, Clone)]
pub struct RegistryCandidate {
    pub version: Version,
    pub rust_version: Option<Version>,
    pub yanked: bool,
    pub features: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyConstraint {
    pub package_name: String,
    pub dependency_key: String,
    pub manifest_path: PathBuf,
    pub requirement: String,
    pub source: Option<String>,
    pub features: BTreeSet<String>,
    pub uses_default_features: bool,
    pub optional: bool,
    pub section: String,
    pub target_rust_version: Option<Version>,
}

#[derive(Debug, Clone)]
pub struct SelectedMember {
    pub package_id: String,
    pub package_name: String,
    pub manifest_path: PathBuf,
    pub rust_version: Option<Version>,
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub members: Vec<SelectedMember>,
    pub target: TargetSelection,
}

#[derive(Debug, Clone)]
pub struct WorkspaceData {
    pub workspace_root: PathBuf,
    pub workspace_manifest: PathBuf,
    pub is_virtual_workspace: bool,
    pub resolver: Option<String>,
    pub recommendations: Vec<String>,
    pub metadata: cargo_metadata::Metadata,
    pub packages_by_id: BTreeMap<String, ResolvedPackage>,
    pub graph: BTreeMap<String, Vec<String>>,
}
