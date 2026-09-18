#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

const ALLOWED_CLASSES: [&str; 2] = ["internal", "public-preview"];
const ALLOWED_ROOTS: [&str; 15] = [
    ".github",
    ".gitignore",
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
    "WORKSTREAMS.md",
    "bootstrap-surfaces.tsv",
    "design-system.descriptor.toml",
];
const FORBIDDEN_CONTENT_MARKERS: [&str; 6] = [
    "lg-workstreams",
    "Build/bin/",
    "Build/src/",
    "git@github.com:luckgrid/",
    "ssh://",
    "../",
];

/// Third manifest field marking a surface whose contents are deliberately not
/// privacy-scanned. Every exemption must carry a reason so the exclusion is
/// reviewable in the manifest instead of being inferred from a path's type.
const SCAN_EXEMPT_PREFIX: &str = "scan-exempt:";

/// Directories never walked when recursing a directory surface. They hold build
/// output and Git internals rather than authored bootstrap content.
const UNWALKED_DIRS: [&str; 2] = [".git", "target"];

/// Names skipped when enumerating repository files for classification
/// completeness. They mirror `.gitignore`: Git internals, build output, and
/// editor/OS scratch that is never authored bootstrap source.
const UNCLASSIFIED_SKIP: [&str; 8] = [
    ".git",
    "target",
    "dist",
    "node_modules",
    ".cursor",
    ".vscode",
    ".idea",
    ".DS_Store",
];

/// Workspace-lint opt-in that every Cargo member must declare so the root
/// `[workspace.lints]` policy actually applies to it.
const MEMBER_LINT_OPT_IN: &str = "workspace = true";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Surface {
    class: String,
    path: PathBuf,
    scan_exempt: Option<String>,
}

/// What the manifest pass actually covered, so the reported result cannot imply
/// a privacy scan that did not run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Coverage {
    surfaces: usize,
    scanned: usize,
    exempt: usize,
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

    let coverage = validate_manifest(&root, manifest)?;
    let classified = validate_manifest_completeness(&root, manifest)?;
    validate_plain_fixture(&root, fixture)?;
    let members = validate_lint_inheritance(&root)?;

    Ok(format!(
        "validated {} bootstrap surfaces ({} files scanned, {} exempt), \
{} repository file(s) confirmed classified, \
{} workspace member(s) inheriting workspace lints, and plain fixture {}",
        coverage.surfaces,
        coverage.scanned,
        coverage.exempt,
        classified,
        members,
        fixture.display()
    ))
}

fn validate_manifest(root: &Path, manifest: &Path) -> Result<Coverage, String> {
    validate_relative_path(manifest)?;
    let root = canonical(root, "repository root")?;
    let manifest_path = root.join(manifest);
    let text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("read manifest {}: {error}", manifest.display()))?;
    let surfaces = parse_manifest(&text)?;

    if surfaces.is_empty() {
        return Err("manifest contains no surfaces".to_owned());
    }

    let mut coverage = Coverage {
        surfaces: surfaces.len(),
        ..Coverage::default()
    };

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

        // Directory surfaces are walked, not skipped. Scan exemptions are
        // deliberately file-scoped so a new file cannot silently inherit one.
        if resolved.is_dir() && surface.scan_exempt.is_some() {
            return Err(format!(
                "directory surface {} cannot be scan-exempt; classify exempt files individually",
                surface.path.display()
            ));
        }
        let files = if resolved.is_dir() {
            walk_files(&resolved)?
        } else {
            vec![resolved]
        };

        for file in files {
            if surface.scan_exempt.is_some() {
                coverage.exempt += 1;
                continue;
            }
            let bytes = fs::read(&file)
                .map_err(|error| format!("read surface file {}: {error}", file.display()))?;
            let content = String::from_utf8_lossy(&bytes);
            scan_supported_content(surface, &relative_to(&root, &file), &content)?;
            coverage.scanned += 1;
        }
    }

    Ok(coverage)
}

/// Every repository file must be covered by some manifest row. Without this
/// pass the inventory's exhaustiveness would only ever be a hand-maintained
/// snapshot: a newly added file would be neither classified nor privacy-scanned
/// while the bootstrap check stayed green. Returns the number of repository
/// files confirmed classified.
fn validate_manifest_completeness(root: &Path, manifest: &Path) -> Result<usize, String> {
    validate_relative_path(manifest)?;
    let root = canonical(root, "repository root")?;
    let text = fs::read_to_string(root.join(manifest))
        .map_err(|error| format!("read manifest {}: {error}", manifest.display()))?;
    let classified: Vec<PathBuf> = parse_manifest(&text)?
        .iter()
        .map(|surface| root.join(&surface.path))
        .collect();

    let files = walk_with_skips(&root, &UNCLASSIFIED_SKIP)?;
    for file in &files {
        // A file row covers only itself; a directory row covers everything
        // beneath it. Either way the file must be named by the inventory.
        if !classified.iter().any(|entry| file.starts_with(entry)) {
            return Err(format!(
                "repository file {} is not classified in {}",
                relative_to(&root, file).display(),
                manifest.display()
            ));
        }
    }

    Ok(files.len())
}

fn parse_manifest(text: &str) -> Result<Vec<Surface>, String> {
    let mut surfaces = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut fields = line.split('\t');
        let class = fields
            .next()
            .ok_or_else(|| format!("manifest line {line_number} must be <class><tab><path>"))?;
        let path = fields
            .next()
            .ok_or_else(|| format!("manifest line {line_number} must be <class><tab><path>"))?;
        let exempt_field = fields.next();
        if fields.next().is_some() {
            return Err(format!(
                "manifest line {line_number} has more than three tab-separated fields"
            ));
        }

        if !ALLOWED_CLASSES.contains(&class) {
            return Err(format!(
                "manifest line {line_number} uses unsupported class '{class}'"
            ));
        }
        if path.trim().is_empty() {
            return Err(format!("manifest line {line_number} has an empty path"));
        }

        let scan_exempt = match exempt_field {
            None => None,
            Some(field) => Some(parse_scan_exemption(line_number, field.trim())?),
        };

        surfaces.push(Surface {
            class: class.to_owned(),
            path: PathBuf::from(path),
            scan_exempt,
        });
    }

    Ok(surfaces)
}

fn parse_scan_exemption(line_number: usize, field: &str) -> Result<String, String> {
    let reason = field.strip_prefix(SCAN_EXEMPT_PREFIX).ok_or_else(|| {
        format!(
            "manifest line {line_number} third field must start with '{SCAN_EXEMPT_PREFIX}', got '{field}'"
        )
    })?;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(format!(
            "manifest line {line_number} scan exemption must state a reason"
        ));
    }
    Ok(reason.to_owned())
}

/// Deterministic, sorted, depth-first file listing for a directory surface.
fn walk_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    walk_with_skips(directory, &UNWALKED_DIRS)
}

/// Deterministic, sorted, depth-first file listing that omits any entry whose
/// file name appears in `skip`.
fn walk_with_skips(directory: &Path, skip: &[&str]) -> Result<Vec<PathBuf>, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|error| format!("read directory {}: {error}", directory.display()))?
        .map(|entry| {
            entry.map(|entry| entry.path()).map_err(|error| {
                format!("read directory entry in {}: {error}", directory.display())
            })
        })
        .collect::<Result<Vec<PathBuf>, String>>()?;
    entries.sort();

    let mut files = Vec::new();
    for entry in entries {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if skip.contains(&name.as_str()) {
            continue;
        }
        if entry.is_dir() {
            files.extend(walk_with_skips(&entry, skip)?);
        } else if entry.is_file() {
            files.push(entry);
        }
    }

    Ok(files)
}

fn relative_to(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).unwrap_or(file).to_path_buf()
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(format!(
            "path must be non-empty and relative: {}",
            path.display()
        ));
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

fn scan_supported_content(surface: &Surface, path: &Path, content: &str) -> Result<(), String> {
    for marker in FORBIDDEN_CONTENT_MARKERS {
        if content.contains(marker) {
            return Err(format!(
                "{} bootstrap surface {} contains forbidden marker '{marker}'",
                surface.class,
                path.display()
            ));
        }
    }
    Ok(())
}

/// Prove that the root `[workspace.lints]` policy actually reaches every member.
/// Returns the number of members checked.
fn validate_lint_inheritance(root: &Path) -> Result<usize, String> {
    let root = canonical(root, "repository root")?;
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = fs::read_to_string(&root_manifest_path)
        .map_err(|error| format!("read root Cargo.toml: {error}"))?;
    let members = parse_workspace_members(&root_manifest)?;

    for member in &members {
        let member_path = PathBuf::from(member);
        validate_relative_path(&member_path)?;
        let manifest_path = root.join(&member_path).join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("read member manifest {member}/Cargo.toml: {error}"))?;
        if !member_inherits_workspace_lints(&manifest) {
            return Err(format!(
                "Cargo member {member} does not declare '[lints] {MEMBER_LINT_OPT_IN}'"
            ));
        }
    }

    Ok(members.len())
}

fn parse_workspace_members(manifest: &str) -> Result<Vec<String>, String> {
    let workspace = table(manifest, "[workspace]")
        .ok_or_else(|| "root manifest declares no [workspace] table".to_owned())?;

    let members_value = workspace
        .lines()
        .position(|line| {
            line.trim_start()
                .strip_prefix("members")
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map(|start| {
            workspace
                .lines()
                .skip(start)
                .collect::<Vec<&str>>()
                .join(" ")
        })
        .ok_or_else(|| "root manifest [workspace] declares no members".to_owned())?;

    let members = quoted_values(&members_value);
    if members.is_empty() {
        return Err("root manifest [workspace] members list is empty".to_owned());
    }
    Ok(members)
}

fn member_inherits_workspace_lints(manifest: &str) -> bool {
    table(manifest, "[lints]").is_some_and(|lints| {
        lints
            .lines()
            .any(|line| normalize_assignment(line) == MEMBER_LINT_OPT_IN)
    })
}

/// Body of a top-level TOML table, up to the next table header.
fn table(manifest: &str, header: &str) -> Option<String> {
    let mut found = false;
    let mut inside = false;
    let mut body = String::new();

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if inside {
                break;
            }
            inside = trimmed == header;
            found |= inside;
            continue;
        }
        if inside {
            body.push_str(trimmed);
            body.push('\n');
        }
    }

    if found { Some(body) } else { None }
}

fn quoted_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        rest = &rest[open + 1..];
        match rest.find('"') {
            Some(close) => {
                values.push(rest[..close].to_owned());
                rest = &rest[close + 1..];
            }
            None => break,
        }
    }
    values
}

fn normalize_assignment(line: &str) -> String {
    let mut parts = line.splitn(2, '=');
    match (parts.next(), parts.next()) {
        (Some(key), Some(value)) => format!("{} = {}", key.trim(), value.trim()),
        _ => line.trim().to_owned(),
    }
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

    let html =
        fs::read_to_string(&index).map_err(|error| format!("read {}: {error}", index.display()))?;
    let stylesheet =
        fs::read_to_string(&css).map_err(|error| format!("read {}: {error}", css.display()))?;

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
        if html
            .to_ascii_lowercase()
            .contains(&marker.to_ascii_lowercase())
        {
            return Err(format!(
                "plain fixture HTML contains forbidden marker '{marker}'"
            ));
        }
    }

    for marker in [
        "@import", "tailwind", "http://", "https://", "@luna/", "url(",
    ] {
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

    fn boundary(name: &str) -> PathBuf {
        Path::new("fixtures/bootstrap-boundary").join(name)
    }

    fn reject(name: &str) -> String {
        validate_manifest(&repository_root(), &boundary(name))
            .expect_err("boundary fixture must be rejected")
    }

    #[test]
    fn accepted_boundary_fixture_passes() {
        let coverage = validate_manifest(&repository_root(), &boundary("accepted.tsv"))
            .expect("accepted boundary fixture");
        assert_eq!(
            coverage,
            Coverage {
                surfaces: 2,
                scanned: 1,
                exempt: 1,
            }
        );
    }

    #[test]
    fn rejected_private_boundary_fixture_fails() {
        assert!(reject("rejected-private.tsv").contains("forbidden component"));
    }

    #[test]
    fn rejected_missing_boundary_fixture_fails() {
        assert!(reject("rejected-missing.tsv").contains("resolve surface"));
    }

    #[test]
    fn rejected_outside_root_boundary_fixture_fails() {
        assert!(reject("rejected-outside-root.tsv").contains("outside permitted bootstrap roots"));
    }

    /// The privacy scan must reach a listed file, not merely the manifest rows.
    #[test]
    fn rejected_private_marker_boundary_fixture_fails() {
        assert!(reject("rejected-private-marker.tsv").contains("forbidden marker"));
    }

    #[test]
    fn rejected_bad_exemption_boundary_fixture_fails() {
        assert!(reject("rejected-bad-exemption.tsv").contains(SCAN_EXEMPT_PREFIX));
    }

    #[test]
    fn directory_surface_cannot_be_scan_exempt() {
        assert!(
            reject("rejected-directory-exemption.tsv")
                .contains("directory surface fixtures/plain-html cannot be scan-exempt")
        );
    }

    /// A directory surface must be walked, so a marker anywhere beneath an
    /// unexempted directory is still rejected.
    #[test]
    fn directory_surface_contents_are_scanned() {
        let coverage = validate_manifest(&repository_root(), &boundary("accepted-directory.tsv"))
            .expect("directory surface fixture");
        assert!(
            coverage.scanned > 1,
            "directory surface must scan more than one file, scanned {}",
            coverage.scanned
        );
        assert_eq!(coverage.exempt, 0);
    }

    /// Exhaustive classification must be machine-enforced, not asserted in
    /// prose, so a newly added file cannot arrive unclassified and unscanned.
    #[test]
    fn every_repository_file_is_classified() {
        let inventory = Path::new("bootstrap-surfaces.tsv");
        let classified = validate_manifest_completeness(&repository_root(), inventory)
            .expect("bootstrap inventory must classify every repository file");
        let text = fs::read_to_string(repository_root().join(inventory))
            .expect("read bootstrap inventory");
        let surfaces = parse_manifest(&text).expect("parse bootstrap inventory");
        assert_eq!(classified, surfaces.len());
    }

    #[test]
    fn incomplete_inventory_is_rejected() {
        let error = validate_manifest_completeness(
            &repository_root(),
            &boundary("rejected-incomplete.tsv"),
        )
        .expect_err("an inventory that omits repository files must be rejected");
        assert!(error.contains("is not classified in"), "{error}");
    }

    #[test]
    fn plain_fixture_passes() {
        validate_plain_fixture(&repository_root(), Path::new("fixtures/plain-html"))
            .expect("plain fixture");
    }

    #[test]
    fn public_stable_is_not_a_bootstrap_class() {
        let error = parse_manifest("public-stable\tpackages/styles/README.md")
            .expect_err("public-stable must not be accepted at T3");
        assert!(error.contains("unsupported class"));
    }

    #[test]
    fn workspace_members_inherit_workspace_lints() {
        let members = validate_lint_inheritance(&repository_root()).expect("lint inheritance");
        assert_eq!(members, 1);
    }

    #[test]
    fn workspace_members_are_parsed_from_the_root_manifest() {
        let manifest =
            fs::read_to_string(repository_root().join("Cargo.toml")).expect("read root manifest");
        assert_eq!(
            parse_workspace_members(&manifest).expect("members"),
            vec!["apps/ds-check".to_owned()]
        );
    }

    #[test]
    fn inheriting_member_manifest_fixture_is_accepted() {
        let manifest =
            fs::read_to_string(repository_root().join(boundary("member-lints-inheriting.toml")))
                .expect("read fixture");
        assert!(member_inherits_workspace_lints(&manifest));
    }

    #[test]
    fn member_manifest_fixture_without_lint_opt_in_is_rejected() {
        let manifest =
            fs::read_to_string(repository_root().join(boundary("member-lints-missing.toml")))
                .expect("read fixture");
        assert!(!member_inherits_workspace_lints(&manifest));
    }
}
