#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

const ALLOWED_CLASSES: [&str; 2] = ["internal", "public-preview"];
const ALLOWED_ROOTS: [&str; 12] = [
    ".github",
    "apps",
    "packages",
    "fixtures",
    "docs",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "README.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "bootstrap-surfaces.tsv",
];
const FORBIDDEN_CONTENT_MARKERS: [&str; 6] = [
    "lg-workstreams",
    "Build/bin/",
    "Build/src/",
    "git@github.com:luckgrid/",
    "ssh://",
    "../",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Surface {
    class: String,
    path: PathBuf,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    if args.len() != 3 || args[0] != "check" {
        return Err(
            "usage: ds-check check <bootstrap-surfaces.tsv> <plain-fixture-dir>".to_owned(),
        );
    }

    let root = env::current_dir().map_err(|error| format!("resolve repository root: {error}"))?;
    let manifest = Path::new(&args[1]);
    let fixture = Path::new(&args[2]);

    let count = validate_manifest(&root, manifest)?;
    validate_plain_fixture(&root, fixture)?;

    Ok(format!(
        "validated {count} bootstrap surfaces and plain fixture {}",
        fixture.display()
    ))
}

fn validate_manifest(root: &Path, manifest: &Path) -> Result<usize, String> {
    validate_relative_path(manifest)?;
    let root = canonical(root, "repository root")?;
    let manifest_path = root.join(manifest);
    let text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("read manifest {}: {error}", manifest.display()))?;
    let surfaces = parse_manifest(&text)?;

    if surfaces.is_empty() {
        return Err("manifest contains no surfaces".to_owned());
    }

    for surface in &surfaces {
        validate_relative_path(&surface.path)?;
        validate_allowed_root(&surface.path)?;

        let candidate = root.join(&surface.path);
        let resolved = canonical(&candidate, &format!("surface {}", surface.path.display()))?;
        if !resolved.starts_with(&root) {
            return Err(format!(
                "surface escapes repository root: {}",
                surface.path.display()
            ));
        }

        if resolved.is_file() {
            let content = fs::read_to_string(&resolved).map_err(|error| {
                format!("read surface {}: {error}", surface.path.display())
            })?;
            scan_supported_content(&surface.path, &content)?;
        }
    }

    Ok(surfaces.len())
}

fn parse_manifest(text: &str) -> Result<Vec<Surface>, String> {
    let mut surfaces = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (class, path) = line.split_once('\t').ok_or_else(|| {
            format!("manifest line {line_number} must be <class><tab><path>")
        })?;

        if !ALLOWED_CLASSES.contains(&class) {
            return Err(format!(
                "manifest line {line_number} uses unsupported class '{class}'"
            ));
        }
        if path.trim().is_empty() {
            return Err(format!("manifest line {line_number} has an empty path"));
        }

        surfaces.push(Surface {
            class: class.to_owned(),
            path: PathBuf::from(path),
        });
    }

    Ok(surfaces)
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(format!("path must be non-empty and relative: {}", path.display()));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "path contains a forbidden component: {}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

fn validate_allowed_root(path: &Path) -> Result<(), String> {
    let first = path
        .components()
        .find_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .ok_or_else(|| format!("path has no normal component: {}", path.display()))?;

    if ALLOWED_ROOTS.contains(&first) {
        Ok(())
    } else {
        Err(format!(
            "path is outside permitted bootstrap roots: {}",
            path.display()
        ))
    }
}

fn scan_supported_content(path: &Path, content: &str) -> Result<(), String> {
    for marker in FORBIDDEN_CONTENT_MARKERS {
        if content.contains(marker) {
            return Err(format!(
                "supported bootstrap surface {} contains forbidden marker '{marker}'",
                path.display()
            ));
        }
    }
    Ok(())
}

fn validate_plain_fixture(root: &Path, fixture: &Path) -> Result<(), String> {
    validate_relative_path(fixture)?;
    let root = canonical(root, "repository root")?;
    let fixture_root = canonical(&root.join(fixture), "plain fixture")?;
    if !fixture_root.starts_with(&root) {
        return Err(format!(
            "plain fixture escapes repository root: {}",
            fixture.display()
        ));
    }

    let index = fixture_root.join("index.html");
    let css = fixture_root.join("consumer.css");
    let readme = fixture_root.join("README.md");
    for required in [&index, &css, &readme] {
        if !required.is_file() {
            return Err(format!(
                "plain fixture is missing required file {}",
                required.display()
            ));
        }
    }

    let html = fs::read_to_string(&index)
        .map_err(|error| format!("read {}: {error}", index.display()))?;
    let stylesheet = fs::read_to_string(&css)
        .map_err(|error| format!("read {}: {error}", css.display()))?;

    if !html.contains("href=\"./consumer.css\"") {
        return Err("plain fixture must load only its local ./consumer.css".to_owned());
    }

    for marker in [
        "<script",
        "tailwind",
        "http://",
        "https://",
        "@luna/",
        "cargo",
        "rustup",
        "node_modules",
        "lg-workstreams",
        "Build/",
    ] {
        if html.to_ascii_lowercase().contains(&marker.to_ascii_lowercase()) {
            return Err(format!("plain fixture HTML contains forbidden marker '{marker}'"));
        }
    }

    for marker in ["@import", "tailwind", "http://", "https://", "@luna/", "url("] {
        if stylesheet
            .to_ascii_lowercase()
            .contains(&marker.to_ascii_lowercase())
        {
            return Err(format!(
                "plain fixture stylesheet contains forbidden marker '{marker}'"
            ));
        }
    }

    Ok(())
}

fn canonical(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("resolve {label} {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn accepted_boundary_fixture_passes() {
        let root = repository_root();
        let result = validate_manifest(
            &root,
            Path::new("fixtures/bootstrap-boundary/accepted.tsv"),
        );
        assert_eq!(result.expect("accepted boundary fixture"), 1);
    }

    #[test]
    fn rejected_private_boundary_fixture_fails() {
        let root = repository_root();
        let error = validate_manifest(
            &root,
            Path::new("fixtures/bootstrap-boundary/rejected-private.tsv"),
        )
        .expect_err("private boundary fixture must fail");
        assert!(error.contains("forbidden component"));
    }

    #[test]
    fn plain_fixture_passes() {
        validate_plain_fixture(&repository_root(), Path::new("fixtures/plain-html"))
            .expect("plain fixture");
    }

    #[test]
    fn public_stable_is_not_a_bootstrap_class() {
        let error = parse_manifest("public-stable\tpackages/frontend/css/README.md")
            .expect_err("public-stable must not be accepted at T3");
        assert!(error.contains("unsupported class"));
    }
}
