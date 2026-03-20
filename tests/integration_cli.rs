use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn bin() -> Command {
    Command::cargo_bin("cargo-compatible").expect("binary should build")
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
        stderr.contains("query `definitely-not-a-package` did not match any resolved package"),
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
        stderr.contains("package spec `helper` did not match any workspace member"),
        "unexpected stderr: {stderr}"
    );
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
