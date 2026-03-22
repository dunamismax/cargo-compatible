use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn bin() -> Command {
    Command::cargo_bin("cargo-compatible").expect("binary should build")
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

fn sanitize_text(text: &str, fixture_root: &Path) -> String {
    text.replace(&fixture_root.display().to_string(), "$FIXTURE")
        .replace(env!("CARGO_MANIFEST_DIR"), "$REPO")
}

fn sanitize_json(value: &mut Value, fixture_root: &Path) {
    match value {
        Value::String(string) => {
            *string = string
                .replace(&fixture_root.display().to_string(), "$FIXTURE")
                .replace(env!("CARGO_MANIFEST_DIR"), "$REPO");
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

fn replace_json_strings(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(string) => {
            *string = string.replace(from, to);
        }
        Value::Array(items) => {
            for item in items {
                replace_json_strings(item, from, to);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                replace_json_strings(value, from, to);
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
        replace_json_strings(&mut value, &temp_root, "$TEMP_WORKSPACE");
        replace_json_strings(
            &mut value,
            &format!("path+file://{temp_root}"),
            "path+file://$TEMP_WORKSPACE",
        );
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
