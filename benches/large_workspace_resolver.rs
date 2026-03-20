use cargo_compatible::cli::{OutputFormat, ResolveCommand, SelectionArgs};
use cargo_compatible::metadata::{load_workspace, select_packages};
use cargo_compatible::resolution::build_candidate_resolution;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

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

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should be created");
    }
    fs::write(path, contents).expect("file should be written");
}

fn bench_resolver(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_workspace_resolver");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    for member_count in [32usize, 96usize] {
        let workspace_fixture = GeneratedWorkspace::new(member_count);
        let selection_args = SelectionArgs {
            manifest_path: Some(workspace_fixture.manifest_path().to_path_buf()),
            rust_version: None,
            workspace: true,
            package: Vec::new(),
        };
        let workspace = load_workspace(Some(workspace_fixture.manifest_path()))
            .expect("workspace metadata should load");
        let selection =
            select_packages(&workspace, &selection_args).expect("workspace selection should work");
        let command = ResolveCommand {
            selection: selection_args,
            format: OutputFormat::Json,
            write_candidate: None,
            write_report: None,
        };

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

criterion_group!(benches, bench_resolver);
criterion_main!(benches);
