use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tempfile::{tempdir, TempDir};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn bin() -> Command {
    Command::cargo_bin("cargo-compatible").expect("binary should build")
}

#[test]
fn top_level_help_mentions_core_workflow() {
    let output = bin()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("Audit a workspace's resolved dependency graph"));
    assert!(stdout.contains("cargo compatible resolve --write-candidate"));
}

#[test]
fn resolve_help_mentions_temp_workspace_safety() {
    let output = bin()
        .args(["resolve", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("temporary workspace copy"));
    assert!(stdout.contains("Write the candidate Cargo.lock"));
}

struct LocalRegistryFixture {
    _temp: TempDir,
    workspace_root: PathBuf,
    cargo_home: PathBuf,
}

struct GitDependencyFixture {
    _temp: TempDir,
    workspace_root: PathBuf,
}

#[derive(Clone, Copy)]
struct LocalRegistryPackage {
    name: &'static str,
    version: &'static str,
    rust_version: &'static str,
}

fn stage_local_registry_fixture(workspace_fixture: &str) -> LocalRegistryFixture {
    stage_local_registry_fixture_with_packages(
        workspace_fixture,
        &[
            LocalRegistryPackage {
                name: "compat-demo",
                version: "1.1.0",
                rust_version: "1.60",
            },
            LocalRegistryPackage {
                name: "compat-demo",
                version: "1.2.0",
                rust_version: "1.70",
            },
        ],
    )
}

fn stage_local_registry_fixture_with_packages(
    workspace_fixture: &str,
    packages: &[LocalRegistryPackage],
) -> LocalRegistryFixture {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    copy_dir_all(&fixture(workspace_fixture), &workspace_root);

    let registry_root = temp.path().join("local-registry");
    build_local_registry(&registry_root, packages);
    write_local_registry_config(&workspace_root, &registry_root);

    let cargo_home = temp.path().join("cargo-home");
    fs::create_dir_all(&cargo_home).unwrap();

    LocalRegistryFixture {
        _temp: temp,
        workspace_root,
        cargo_home,
    }
}

fn stage_git_dependency_fixture() -> GitDependencyFixture {
    let temp = tempdir().unwrap();
    let repo_a = create_git_package_repo(temp.path(), "shared-a", "shared", "0.1.0", "1.70");
    let repo_b = create_git_package_repo(temp.path(), "shared-b", "shared", "0.1.0", "1.69");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("src")).unwrap();
    fs::write(
        workspace_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"git-identity-chains\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.60\"\n\n[dependencies]\nshared_a = {{ package = \"shared\", git = \"{}\" }}\nshared_b = {{ package = \"shared\", git = \"{}\" }}\n",
            file_uri(&repo_a),
            file_uri(&repo_b),
        ),
    )
    .unwrap();
    fs::write(workspace_root.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    let status = ProcessCommand::new("cargo")
        .args([
            "generate-lockfile",
            "--manifest-path",
            workspace_root.join("Cargo.toml").to_str().unwrap(),
        ])
        .current_dir(&workspace_root)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "cargo generate-lockfile should succeed for git dependency fixture"
    );

    GitDependencyFixture {
        _temp: temp,
        workspace_root,
    }
}

fn create_git_package_repo(
    root: &Path,
    repo_name: &str,
    package_name: &str,
    version: &str,
    rust_version: &str,
) -> PathBuf {
    let repo_root = root.join(repo_name);
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::write(
        repo_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{package_name}\"\nversion = \"{version}\"\nedition = \"2021\"\nrust-version = \"{rust_version}\"\n"
        ),
    )
    .unwrap();
    fs::write(
        repo_root.join("src").join("lib.rs"),
        "pub fn version() {}\n",
    )
    .unwrap();

    run_git(["init"], &repo_root);
    run_git(
        ["config", "user.name", "Cargo Compatible Tests"],
        &repo_root,
    );
    run_git(
        ["config", "user.email", "tests@example.invalid"],
        &repo_root,
    );
    run_git(["add", "."], &repo_root);
    run_git(["commit", "-m", "initial"], &repo_root);
    repo_root
}

fn run_git<const N: usize>(args: [&str; N], cwd: &Path) {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "git command {:?} should succeed in {}",
        args,
        cwd.display()
    );
}

fn build_local_registry(registry_root: &Path, packages: &[LocalRegistryPackage]) {
    fs::create_dir_all(registry_root.join("index").join("co").join("mp")).unwrap();
    fs::write(
        registry_root.join("index").join("config.json"),
        format!(r#"{{"dl":"{}"}}"#, file_uri(registry_root)),
    )
    .unwrap();

    let mut entries = Vec::new();
    for package in packages {
        let crate_path = package_crate_archive(registry_root, package);
        let bytes = fs::read(&crate_path).unwrap();
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        entries.push(
            serde_json::json!({
                "name": package.name,
                "vers": package.version,
                "deps": [],
                "cksum": checksum,
                "features": {},
                "yanked": false,
                "rust_version": package.rust_version,
            })
            .to_string(),
        );
    }
    let sparse_entry = entries.join("\n");
    fs::write(
        registry_root
            .join("index")
            .join("co")
            .join("mp")
            .join("compat-demo"),
        format!("{sparse_entry}\n"),
    )
    .unwrap();
}

fn package_crate_archive(registry_root: &Path, package: &LocalRegistryPackage) -> PathBuf {
    let package_root = registry_root
        .parent()
        .unwrap()
        .join("package-sources")
        .join(format!("{}-{}", package.name, package.version));
    fs::create_dir_all(package_root.join("src")).unwrap();
    fs::write(
        package_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2021\"\nrust-version = \"{}\"\ndescription = \"local registry fixture\"\nlicense = \"MIT\"\n",
            package.name, package.version, package.rust_version
        ),
    )
    .unwrap();
    fs::write(
        package_root.join("src").join("lib.rs"),
        format!(
            "pub fn version() -> &'static str {{ \"{}\" }}\n",
            package.version
        ),
    )
    .unwrap();

    let manifest_path = package_root.join("Cargo.toml");
    let status = ProcessCommand::new("cargo")
        .args([
            "package",
            "--manifest-path",
            manifest_path.to_str().unwrap(),
            "--allow-dirty",
            "--no-verify",
            "--quiet",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "cargo package should succeed for local registry fixture"
    );

    let packaged = package_root
        .join("target")
        .join("package")
        .join(format!("{}-{}.crate", package.name, package.version));
    let archive_path = registry_root.join(format!("{}-{}.crate", package.name, package.version));
    fs::copy(&packaged, &archive_path).unwrap();
    archive_path
}

fn write_local_registry_config(workspace_root: &Path, registry_root: &Path) {
    let cargo_dir = workspace_root.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    fs::write(
        cargo_dir.join("config.toml"),
        format!(
            "[source.crates-io]\nreplace-with = \"local\"\n\n[source.local]\nlocal-registry = \"{}\"\n\n[net]\noffline = true\n",
            path_string(registry_root)
        ),
    )
    .unwrap();
}

fn copy_dir_all(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap();
        }
    }
}

fn write_basic_workspace(root: &Path, lockfile_contents: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"apply-lock-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("Cargo.lock"), lockfile_contents).unwrap();
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn file_uri(path: &Path) -> String {
    let normalized = path_string(path);
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

fn package_id_prefix(path: &Path) -> String {
    let normalized = path_string(path);
    if normalized.starts_with('/') {
        format!("path+file://{normalized}")
    } else {
        format!("path+file:///{normalized}")
    }
}

fn replace_path_variants(text: &str, path: &Path, placeholder: &str) -> String {
    normalize_placeholder_paths(
        text.replace("\r\n", "\n")
            .replace(
                &package_id_prefix(path),
                &format!("path+file://{placeholder}"),
            )
            .replace(&file_uri(path), &format!("file://{placeholder}"))
            .replace(&path.display().to_string(), placeholder)
            .replace(&path_string(path), placeholder),
    )
}

fn normalize_placeholder_paths(text: String) -> String {
    if text.contains("$FIXTURE") || text.contains("$REPO") {
        text.replace('\\', "/")
    } else {
        text
    }
}

fn sanitize_text(text: &str, fixture_root: &Path) -> String {
    let text = replace_path_variants(text, fixture_root, "$FIXTURE");
    replace_path_variants(
        text.as_str(),
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "$REPO",
    )
}

fn sanitize_json(value: &mut Value, fixture_root: &Path) {
    match value {
        Value::String(string) => {
            *string = replace_path_variants(string, fixture_root, "$FIXTURE");
            *string = replace_path_variants(string, Path::new(env!("CARGO_MANIFEST_DIR")), "$REPO");
        }
        Value::Array(items) => {
            for item in items {
                sanitize_json(item, fixture_root);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                sanitize_json(value, fixture_root);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn replace_json_path_variants(value: &mut Value, path: &Path, placeholder: &str) {
    match value {
        Value::String(string) => {
            *string = replace_path_variants(string, path, placeholder);
        }
        Value::Array(items) => {
            for item in items {
                replace_json_path_variants(item, path, placeholder);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                replace_json_path_variants(value, path, placeholder);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[test]
fn scan_missing_rust_version_json_snapshot() {
    let fixture_root = fixture("missing-rust-version");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mut value: Value = serde_json::from_slice(&output).unwrap();
    sanitize_json(&mut value, &fixture_root);
    assert_json_snapshot!("scan_missing_rust_version_json", value);
}

#[test]
fn replace_path_variants_normalizes_placeholder_backslashes() {
    let fixture_root = fixture("missing-rust-version");
    let input = format!("{}\\helper\\Cargo.toml", fixture_root.display());
    let output = replace_path_variants(&input, &fixture_root, "$FIXTURE");

    assert_eq!(output, "$FIXTURE/helper/Cargo.toml");
}

#[test]
fn scan_mixed_workspace_human_snapshot() {
    let fixture_root = fixture("mixed-workspace");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert_snapshot!("scan_mixed_workspace_human", output);
}

#[test]
fn explain_path_dep_human_snapshot() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "explain",
            "too_new",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert_snapshot!("explain_path_dep_human", output);
}

#[test]
fn explain_path_dep_markdown_snapshot() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "explain",
            "too_new",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert_snapshot!("explain_path_dep_markdown", output);
}

#[test]
fn scan_path_too_new_reports_incompatible() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(
        stdout.contains("Incompatible Packages"),
        "should have incompatible section"
    );
    assert!(
        stdout.contains("too_new"),
        "should mention the too_new package as incompatible"
    );
    assert!(
        stdout.contains("exceeds target"),
        "should explain why it's incompatible"
    );
}

#[test]
fn scan_path_too_new_json_has_incompatible() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    let incompatible = value
        .get("incompatible_packages")
        .and_then(Value::as_array)
        .expect("should have incompatible_packages array");
    assert!(
        !incompatible.is_empty(),
        "should have at least one incompatible package"
    );
    let first = &incompatible[0];
    assert_eq!(
        first.pointer("/package/name").and_then(Value::as_str),
        Some("too_new")
    );
    assert_eq!(
        first.pointer("/status").and_then(Value::as_str),
        Some("incompatible")
    );
}

#[test]
fn scan_disambiguates_dependency_paths_for_same_name_git_packages() {
    let fixture = stage_git_dependency_fixture();
    let output = bin()
        .current_dir(&fixture.workspace_root)
        .args([
            "scan",
            "--manifest-path",
            fixture.workspace_root.join("Cargo.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    let path_lines = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("via git-identity-chains:"))
        .collect::<Vec<_>>();
    assert_eq!(path_lines.len(), 2, "unexpected output:\n{stdout}");
    assert_ne!(
        path_lines[0], path_lines[1],
        "dependency paths should be disambiguated"
    );
    assert!(path_lines
        .iter()
        .all(|line| line.contains("shared@0.1.0 [git: file://")));
    assert!(path_lines.iter().all(|line| line.contains('#')));
}

#[test]
fn explain_path_dep_json_has_blocker_classification() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "explain",
            "too_new",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        value.pointer("/query").and_then(Value::as_str),
        Some("too_new")
    );
    assert!(
        value.get("blocker").is_some(),
        "explain JSON should include blocker classification"
    );
    assert_eq!(
        value.pointer("/blocker").and_then(Value::as_str),
        Some("path_or_git_constraint"),
        "path dep should be classified as path_or_git_constraint"
    );
    assert!(
        value
            .get("current_paths")
            .and_then(Value::as_array)
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "should include dependency paths"
    );
}

#[test]
fn explain_rejects_unknown_query() {
    let fixture_root = fixture("path-too-new");
    let output = bin()
        .args([
            "explain",
            "definitely-not-a-package",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert!(
        stderr.contains(
            "query `definitely-not-a-package` did not match any package in the selected dependency graph"
        ),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn explain_rejects_query_outside_selected_graph() {
    let fixture_root = fixture("mixed-workspace");
    let output = bin()
        .args([
            "explain",
            "low",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--package",
            "high",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert!(
        stderr.contains("query `low` did not match any package in the selected dependency graph"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn scan_rejects_non_workspace_package_selection() {
    let fixture_root = fixture("missing-rust-version");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--package",
            "helper",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert!(
        stderr.contains(
            "package spec `helper` did not match any workspace member by exact package name, package ID, or manifest path"
        ),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn scan_rejects_manifest_path_substring_package_selection() {
    let fixture_root = fixture("mixed-workspace");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--package",
            "members",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert!(
        stderr.contains(
            "package spec `members` did not match any workspace member by exact package name, package ID, or manifest path"
        ),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn scan_accepts_exact_manifest_path_package_selection() {
    let fixture_root = fixture("mixed-workspace");
    let output = bin()
        .args([
            "scan",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--package",
            fixture_root
                .join("members")
                .join("high")
                .join("Cargo.toml")
                .to_str()
                .unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    let selected_members = value
        .pointer("/workspace/selected_members")
        .and_then(Value::as_array)
        .expect("selected members array should exist");
    assert_eq!(selected_members.len(), 1);
    assert_eq!(selected_members[0].as_str(), Some("high"));
}

#[test]
fn resolve_virtual_workspace_json_snapshot() {
    let fixture_root = fixture("virtual-workspace");
    let output = bin()
        .args([
            "resolve",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mut value: Value = serde_json::from_slice(&output).unwrap();
    sanitize_json(&mut value, &fixture_root);
    if let Some(temp_root) = value
        .pointer("/candidate/workspace/workspace_root")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        replace_json_path_variants(&mut value, Path::new(&temp_root), "$TEMP_WORKSPACE");
    }
    assert_json_snapshot!("resolve_virtual_workspace_json", value);
}

#[test]
fn resolve_virtual_workspace_markdown_snapshot() {
    let fixture_root = fixture("virtual-workspace");
    let output = bin()
        .args([
            "resolve",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = sanitize_text(&String::from_utf8(output).unwrap(), &fixture_root);
    assert_snapshot!("resolve_virtual_workspace_markdown", output);
}

#[test]
fn resolve_write_report_honors_selected_format() {
    let fixture_root = fixture("virtual-workspace");
    let temp = tempdir().unwrap();
    let report_path = temp.path().join("nested").join("report.md");
    let output = bin()
        .args([
            "resolve",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
            "--format",
            "markdown",
            "--write-report",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let report = fs::read_to_string(&report_path).unwrap();
    assert_eq!(report, stdout.trim_end_matches('\n'));
    assert!(report.starts_with("# Candidate Resolution\n"));
}

#[test]
fn resolve_write_candidate_writes_lockfile() {
    let fixture_root = fixture("virtual-workspace");
    let temp = tempdir().unwrap();
    let candidate_path = temp.path().join("candidate").join("Cargo.lock");
    bin()
        .args([
            "resolve",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
            "--write-candidate",
            candidate_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let candidate = fs::read_to_string(&candidate_path).unwrap();
    assert!(candidate.contains("[[package]]"));
    assert!(candidate.contains("name = \"member\""));
}

#[test]
fn suggest_manifest_write_manifests_uses_local_registry_fixture_end_to_end() {
    let fixture = stage_local_registry_fixture("local-registry-manifest-blocker");
    let manifest_path = fixture.workspace_root.join("Cargo.toml");
    let output = bin()
        .current_dir(&fixture.workspace_root)
        .env("CARGO_HOME", &fixture.cargo_home)
        .args([
            "suggest-manifest",
            "--manifest-path",
            manifest_path.to_str().unwrap(),
            "--write-manifests",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).unwrap();
    let suggestions = value
        .get("manifest_suggestions")
        .and_then(Value::as_array)
        .expect("manifest suggestions array should exist");
    assert_eq!(
        value.get("write_manifests").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(suggestions.len(), 1);
    assert_eq!(
        suggestions[0]
            .get("dependency_name")
            .and_then(Value::as_str),
        Some("compat-demo")
    );
    assert_eq!(
        suggestions[0]
            .get("suggested_requirement")
            .and_then(Value::as_str),
        Some("1.1.0")
    );

    let manifest = fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest.contains("compat-demo = \"1.1.0\""));
    assert!(!manifest.contains("compat-demo = \"=1.2.0\""));
}

#[test]
fn apply_lock_writes_candidate_lockfile_to_workspace() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    let current_lockfile = "# current\nversion = 4\n\n[[package]]\nname = \"apply-lock-fixture\"\nversion = \"0.1.0\"\n";
    let candidate_lockfile = "# candidate\nversion = 4\n\n[[package]]\nname = \"apply-lock-fixture\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"dep\"\nversion = \"0.2.0\"\n";
    write_basic_workspace(&workspace_root, current_lockfile);
    let candidate_path = temp.path().join("candidate").join("Cargo.lock");
    fs::create_dir_all(candidate_path.parent().unwrap()).unwrap();
    fs::write(&candidate_path, candidate_lockfile).unwrap();

    let output = bin()
        .args([
            "apply-lock",
            "--manifest-path",
            workspace_root.join("Cargo.toml").to_str().unwrap(),
            "--candidate-lockfile",
            candidate_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("applied candidate lockfile"));
    assert_eq!(
        fs::read_to_string(workspace_root.join("Cargo.lock")).unwrap(),
        candidate_lockfile
    );
}

#[test]
fn apply_lock_reports_noop_when_candidate_matches_current() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    let lockfile_contents =
        "version = 4\n\n[[package]]\nname = \"apply-lock-fixture\"\nversion = \"0.1.0\"\n";
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(workspace_root.join("Cargo.lock"), lockfile_contents).unwrap();
    let candidate_path = temp.path().join("candidate").join("Cargo.lock");
    fs::create_dir_all(candidate_path.parent().unwrap()).unwrap();
    fs::write(&candidate_path, lockfile_contents).unwrap();

    let result =
        cargo_compatible::resolution::apply_candidate_lockfile(&workspace_root, candidate_path)
            .unwrap();

    assert!(
        result.contains("nothing to apply"),
        "expected no-op message, got: {result}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("Cargo.lock")).unwrap(),
        lockfile_contents
    );
}

#[test]
fn resolve_write_report_honors_json_format() {
    let fixture_root = fixture("virtual-workspace");
    let temp = tempdir().unwrap();
    let report_path = temp.path().join("nested").join("report.json");
    let output = bin()
        .args([
            "resolve",
            "--manifest-path",
            fixture_root.join("Cargo.toml").to_str().unwrap(),
            "--workspace",
            "--format",
            "json",
            "--write-report",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let report = fs::read_to_string(&report_path).unwrap();
    assert_eq!(report, stdout.trim_end_matches('\n'));
    let parsed: Value = serde_json::from_str(&report).expect("report should be valid JSON");
    assert!(
        parsed.get("current").is_some(),
        "JSON report should have 'current' key"
    );
    assert!(
        parsed.get("candidate").is_some(),
        "JSON report should have 'candidate' key"
    );
}

#[test]
fn apply_lock_rejects_missing_candidate_lockfile() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    write_basic_workspace(
        &workspace_root,
        "# current\nversion = 4\n\n[[package]]\nname = \"apply-lock-fixture\"\nversion = \"0.1.0\"\n",
    );
    let missing_candidate = temp.path().join("missing").join("Cargo.lock");

    let output = bin()
        .args([
            "apply-lock",
            "--manifest-path",
            workspace_root.join("Cargo.toml").to_str().unwrap(),
            "--candidate-lockfile",
            missing_candidate.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).unwrap();
    assert!(stderr.contains("candidate lockfile"));
    assert!(stderr.contains("does not exist"));
}

#[test]
fn lockfile_improvement_scan_reports_incompatible() {
    let fixture = stage_lockfile_improvement_fixture();
    let output = bin()
        .current_dir(&fixture.workspace_root)
        .env("CARGO_HOME", &fixture.cargo_home)
        .args([
            "scan",
            "--manifest-path",
            fixture.workspace_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();

    let incompatible = value
        .get("incompatible_packages")
        .and_then(Value::as_array)
        .expect("incompatible_packages should exist");
    assert!(
        !incompatible.is_empty(),
        "scan should report compat-demo 1.2.0 as incompatible"
    );
    let compat_demo = incompatible
        .iter()
        .find(|p| {
            p.pointer("/package/name").and_then(Value::as_str) == Some("compat-demo")
                && p.pointer("/package/version").and_then(Value::as_str) == Some("1.2.0")
        })
        .expect("compat-demo 1.2.0 should be in incompatible packages");
    assert_eq!(
        compat_demo.pointer("/status").and_then(Value::as_str),
        Some("incompatible")
    );
}

#[test]
fn lockfile_improvement_resolve_upgrades_to_compatible_version() {
    let fixture = stage_lockfile_improvement_fixture();
    let output = bin()
        .current_dir(&fixture.workspace_root)
        .env("CARGO_HOME", &fixture.cargo_home)
        .args([
            "resolve",
            "--manifest-path",
            fixture.workspace_root.join("Cargo.toml").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();

    // Verify version_changes shows 1.2.0 → 1.3.0
    let changes = value
        .get("version_changes")
        .and_then(Value::as_array)
        .expect("version_changes should exist");
    let upgrade = changes
        .iter()
        .find(|c| {
            c.get("package_name").and_then(Value::as_str) == Some("compat-demo")
                && c.get("before").and_then(Value::as_str) == Some("1.2.0")
                && c.get("after").and_then(Value::as_str) == Some("1.3.0")
        })
        .expect("should show compat-demo upgrade from 1.2.0 to 1.3.0");
    assert_eq!(
        upgrade.get("package_name").and_then(Value::as_str),
        Some("compat-demo")
    );

    // Verify improved_packages is non-empty
    let improved = value
        .get("improved_packages")
        .and_then(Value::as_array)
        .expect("improved_packages should exist");
    assert!(
        !improved.is_empty(),
        "improved_packages should contain compat-demo"
    );

    // Verify remaining_blockers is empty (the upgrade fully resolves the issue)
    let blockers = value
        .get("remaining_blockers")
        .and_then(Value::as_array)
        .expect("remaining_blockers should exist");
    assert!(
        blockers.is_empty(),
        "remaining_blockers should be empty after lockfile improvement, got: {blockers:?}"
    );
}

fn stage_lockfile_improvement_fixture() -> LocalRegistryFixture {
    let packages = [
        LocalRegistryPackage {
            name: "compat-demo",
            version: "1.1.0",
            rust_version: "1.60",
        },
        LocalRegistryPackage {
            name: "compat-demo",
            version: "1.2.0",
            rust_version: "1.70",
        },
        LocalRegistryPackage {
            name: "compat-demo",
            version: "1.3.0",
            rust_version: "1.60",
        },
    ];
    let fixture = stage_local_registry_fixture_with_packages("lockfile-improvement", &packages);

    // Generate a lockfile pinned to 1.2.0 by temporarily using an exact version requirement
    let manifest_path = fixture.workspace_root.join("Cargo.toml");
    let original_manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        original_manifest.replace("compat-demo = \">=1.0\"", "compat-demo = \"=1.2.0\""),
    )
    .unwrap();

    let status = ProcessCommand::new("cargo")
        .args([
            "generate-lockfile",
            "--manifest-path",
            manifest_path.to_str().unwrap(),
        ])
        .env("CARGO_HOME", &fixture.cargo_home)
        .current_dir(&fixture.workspace_root)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "cargo generate-lockfile should succeed for lockfile-improvement fixture"
    );

    // Verify lockfile pins 1.2.0
    let lockfile = fs::read_to_string(fixture.workspace_root.join("Cargo.lock")).unwrap();
    assert!(
        lockfile.contains("name = \"compat-demo\"") && lockfile.contains("version = \"1.2.0\""),
        "generated lockfile should pin compat-demo at 1.2.0"
    );

    // Restore the original manifest with the loose requirement
    fs::write(&manifest_path, original_manifest).unwrap();

    fixture
}
