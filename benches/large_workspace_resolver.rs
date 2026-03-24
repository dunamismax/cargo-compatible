use cargo_compatible::cli::{OutputFormat, ResolveCommand, SelectionArgs};
use cargo_compatible::compat::analyze_current_workspace;
use cargo_compatible::metadata::{load_workspace, select_packages};
use cargo_compatible::resolution::build_candidate_resolution;
use cargo_compatible::temp_workspace::TempWorkspace;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Workspace generators
// ---------------------------------------------------------------------------

struct GeneratedWorkspace {
    _tempdir: TempDir,
    manifest_path: PathBuf,
}

impl GeneratedWorkspace {
    fn new(member_count: usize) -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let root = tempdir.path().to_path_buf();
        create_workspace(&root, member_count);
        Self {
            _tempdir: tempdir,
            manifest_path: root.join("Cargo.toml"),
        }
    }

    fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }
}

fn create_workspace(root: &Path, member_count: usize) {
    let members = (0..member_count)
        .map(|index| format!("\"members/member-{index:03}\""))
        .collect::<Vec<_>>()
        .join(",\n    ");
    write_file(
        &root.join("Cargo.toml"),
        &format!("[workspace]\nresolver = \"3\"\nmembers = [\n    \"shared\",\n    {members}\n]\n"),
    );

    write_file(
        &root.join("shared/Cargo.toml"),
        "[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.70\"\n",
    );
    write_file(
        &root.join("shared/src/lib.rs"),
        "pub fn value() -> usize {\n    1\n}\n",
    );

    for index in 0..member_count {
        let package_name = format!("member-{index:03}");
        let previous_dependency = index.checked_sub(1).map(|previous| {
            format!(
                "member_{previous:03} = {{ package = \"member-{previous:03}\", path = \"../member-{previous:03}\" }}\n"
            )
        });
        let manifest = format!(
            "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.70\"\n\n[dependencies]\nshared = {{ path = \"../../shared\" }}\n{}",
            previous_dependency.unwrap_or_default()
        );
        let source = if index == 0 {
            format!("pub fn value() -> usize {{\n    shared::value() + {index}\n}}\n")
        } else {
            format!(
                "pub fn value() -> usize {{\n    shared::value() + member_{:03}::value() + {index}\n}}\n",
                index - 1
            )
        };
        write_file(
            &root.join(format!("members/{package_name}/Cargo.toml")),
            &manifest,
        );
        write_file(
            &root.join(format!("members/{package_name}/src/lib.rs")),
            &source,
        );
    }
}

/// Create a workspace with deeper dependency chains: each member depends on
/// all lower-numbered members, producing a much denser graph than the linear
/// chain used in `create_workspace`.
fn create_dense_workspace(root: &Path, member_count: usize) {
    let members = (0..member_count)
        .map(|index| format!("\"members/member-{index:03}\""))
        .collect::<Vec<_>>()
        .join(",\n    ");
    write_file(
        &root.join("Cargo.toml"),
        &format!("[workspace]\nresolver = \"3\"\nmembers = [\n    \"shared\",\n    {members}\n]\n"),
    );

    write_file(
        &root.join("shared/Cargo.toml"),
        "[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.70\"\n",
    );
    write_file(
        &root.join("shared/src/lib.rs"),
        "pub fn value() -> usize {\n    1\n}\n",
    );

    for index in 0..member_count {
        let package_name = format!("member-{index:03}");
        // Depend on up to 8 preceding members (capped to avoid quadratic manifests)
        let max_deps = 8.min(index);
        let mut deps = String::from("shared = { path = \"../../shared\" }\n");
        for dep_index in (index.saturating_sub(max_deps))..index {
            deps.push_str(&format!(
                "member_{dep_index:03} = {{ package = \"member-{dep_index:03}\", path = \"../member-{dep_index:03}\" }}\n"
            ));
        }
        let manifest = format!(
            "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.70\"\n\n[dependencies]\n{deps}"
        );
        write_file(
            &root.join(format!("members/{package_name}/Cargo.toml")),
            &manifest,
        );
        write_file(
            &root.join(format!("members/{package_name}/src/lib.rs")),
            &format!("pub fn value() -> usize {{ {index} }}\n"),
        );
    }
}

/// Create a workspace where some members have incompatible rust-version
/// declarations and some are missing rust-version entirely. This exercises
/// the mixed-workspace analysis and unknown-classification paths.
fn create_mixed_version_workspace(root: &Path, member_count: usize) {
    let members = (0..member_count)
        .map(|index| format!("\"members/member-{index:03}\""))
        .collect::<Vec<_>>()
        .join(",\n    ");
    write_file(
        &root.join("Cargo.toml"),
        &format!("[workspace]\nresolver = \"3\"\nmembers = [\n    {members}\n]\n"),
    );

    for index in 0..member_count {
        let package_name = format!("member-{index:03}");
        let rust_version_line = match index % 3 {
            0 => "rust-version = \"1.70\"\n".to_string(),
            1 => "rust-version = \"1.80\"\n".to_string(),
            _ => String::new(), // missing rust-version
        };
        // Each member depends on the next (wrapping), producing a cycle-free
        // dependency by only depending on higher-numbered members.
        let deps = if index + 1 < member_count {
            format!(
                "[dependencies]\nmember_{:03} = {{ package = \"member-{:03}\", path = \"../member-{:03}\" }}\n",
                index + 1, index + 1, index + 1,
            )
        } else {
            String::new()
        };
        let manifest = format!(
            "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{rust_version_line}\n{deps}"
        );
        write_file(
            &root.join(format!("members/{package_name}/Cargo.toml")),
            &manifest,
        );
        write_file(
            &root.join(format!("members/{package_name}/src/lib.rs")),
            &format!("pub fn value() -> usize {{ {index} }}\n"),
        );
    }
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should be created");
    }
    fs::write(path, contents).expect("file should be written");
}

// ---------------------------------------------------------------------------
// Helper: build a SelectionArgs for --workspace on a given manifest
// ---------------------------------------------------------------------------

fn workspace_selection(manifest_path: &Path) -> SelectionArgs {
    SelectionArgs {
        manifest_path: Some(manifest_path.to_path_buf()),
        rust_version: None,
        workspace: true,
        package: Vec::new(),
    }
}

fn resolve_command(manifest_path: &Path) -> ResolveCommand {
    ResolveCommand {
        selection: workspace_selection(manifest_path),
        format: OutputFormat::Json,
        write_candidate: None,
        write_report: None,
    }
}

// ---------------------------------------------------------------------------
// Benchmark group: end-to-end resolve (original)
// ---------------------------------------------------------------------------

fn bench_resolver(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_end_to_end");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    for member_count in [32usize, 96usize] {
        let workspace_fixture = GeneratedWorkspace::new(member_count);
        let selection_args = workspace_selection(workspace_fixture.manifest_path());
        let workspace = load_workspace(Some(workspace_fixture.manifest_path()))
            .expect("workspace metadata should load");
        let selection =
            select_packages(&workspace, &selection_args).expect("workspace selection should work");
        let command = resolve_command(workspace_fixture.manifest_path());

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let report = build_candidate_resolution(&workspace, &selection, &command)
                        .expect("candidate resolution should succeed");
                    black_box(report.version_changes.len());
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: scan-only (compatibility analysis without resolution)
// ---------------------------------------------------------------------------

fn bench_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_analysis");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    for member_count in [32usize, 96usize] {
        let workspace_fixture = GeneratedWorkspace::new(member_count);
        let selection_args = workspace_selection(workspace_fixture.manifest_path());
        let workspace = load_workspace(Some(workspace_fixture.manifest_path()))
            .expect("workspace metadata should load");
        let selection =
            select_packages(&workspace, &selection_args).expect("workspace selection should work");

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let report = analyze_current_workspace(&workspace, &selection)
                        .expect("scan analysis should succeed");
                    black_box(report.incompatible_packages.len());
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: metadata loading
// ---------------------------------------------------------------------------

fn bench_metadata_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_load");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    for member_count in [32usize, 96usize] {
        let workspace_fixture = GeneratedWorkspace::new(member_count);

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let workspace = load_workspace(Some(workspace_fixture.manifest_path()))
                        .expect("workspace metadata should load");
                    black_box(workspace.packages_by_id.len());
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: temp-workspace copy cost (isolated)
// ---------------------------------------------------------------------------

fn bench_temp_workspace_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("temp_workspace_copy");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    for member_count in [32usize, 96usize] {
        let workspace_fixture = GeneratedWorkspace::new(member_count);
        let workspace = load_workspace(Some(workspace_fixture.manifest_path()))
            .expect("workspace metadata should load");

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let temp = TempWorkspace::copy_from(&workspace.workspace_root)
                        .expect("temp workspace copy should succeed");
                    black_box(temp.root());
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: dense graph analysis
// ---------------------------------------------------------------------------

fn bench_dense_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("dense_graph_scan");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    for member_count in [32usize, 64usize] {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        create_dense_workspace(tempdir.path(), member_count);
        let manifest_path = tempdir.path().join("Cargo.toml");
        let selection_args = workspace_selection(&manifest_path);
        let workspace =
            load_workspace(Some(&manifest_path)).expect("workspace metadata should load");
        let selection =
            select_packages(&workspace, &selection_args).expect("workspace selection should work");

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let report = analyze_current_workspace(&workspace, &selection)
                        .expect("scan analysis should succeed");
                    black_box(report.incompatible_packages.len());
                });
            },
        );

        // Keep tempdir alive through the benchmark
        drop(workspace);
        drop(tempdir);
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: mixed-version workspace analysis
// ---------------------------------------------------------------------------

fn bench_mixed_version(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_version_scan");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    for member_count in [32usize, 96usize] {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        create_mixed_version_workspace(tempdir.path(), member_count);
        let manifest_path = tempdir.path().join("Cargo.toml");
        let selection_args = workspace_selection(&manifest_path);
        let workspace =
            load_workspace(Some(&manifest_path)).expect("workspace metadata should load");
        let selection =
            select_packages(&workspace, &selection_args).expect("workspace selection should work");

        group.throughput(Throughput::Elements(member_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(member_count),
            &member_count,
            |b, _| {
                b.iter(|| {
                    let report = analyze_current_workspace(&workspace, &selection)
                        .expect("scan analysis should succeed");
                    black_box(report.incompatible_packages.len() + report.unknown_packages.len());
                });
            },
        );

        drop(workspace);
        drop(tempdir);
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group: fixture-derived workspaces from test fixtures
// ---------------------------------------------------------------------------

fn bench_fixture_workspaces(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_scan");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    let fixtures = [
        ("path-too-new", "tests/fixtures/path-too-new/Cargo.toml"),
        (
            "mixed-workspace",
            "tests/fixtures/mixed-workspace/Cargo.toml",
        ),
        (
            "missing-rust-version",
            "tests/fixtures/missing-rust-version/Cargo.toml",
        ),
        (
            "virtual-workspace",
            "tests/fixtures/virtual-workspace/Cargo.toml",
        ),
    ];

    for (label, manifest) in &fixtures {
        let manifest_path = PathBuf::from(manifest);
        if !manifest_path.exists() {
            continue;
        }
        let selection_args = SelectionArgs {
            manifest_path: Some(manifest_path.clone()),
            rust_version: None,
            workspace: true,
            package: Vec::new(),
        };
        let workspace = match load_workspace(Some(&manifest_path)) {
            Ok(ws) => ws,
            Err(_) => continue,
        };
        let selection = match select_packages(&workspace, &selection_args) {
            Ok(sel) => sel,
            Err(_) => continue,
        };

        group.bench_with_input(BenchmarkId::from_parameter(label), label, |b, _| {
            b.iter(|| {
                let report = analyze_current_workspace(&workspace, &selection)
                    .expect("scan analysis should succeed");
                black_box(report.incompatible_packages.len());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_resolver,
    bench_scan,
    bench_metadata_load,
    bench_temp_workspace_copy,
    bench_dense_graph,
    bench_mixed_version,
    bench_fixture_workspaces,
);
criterion_main!(benches);
