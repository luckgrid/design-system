#![forbid(unsafe_code)]

mod css;

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

const ALLOWED_CLASSES: [&str; 2] = ["internal", "public-preview"];
const ALLOWED_ROOTS: [&str; 17] = [
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
    "exports.tsv",
    "tests",
];

/// The only compatibility class a CSS export may carry until a reviewed contract
/// earns `public-stable`.
const EXPORT_CLASS: &str = "public-preview";

/// Directory that holds authored Design System stylesheet source.
const STYLES_ROOT: &str = "packages/styles";

/// The consumer fixture's own stylesheet, the only non-export link it may use.
const FIXTURE_CONSUMER_STYLESHEET: &str = "./consumer.css";
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

/// One declared CSS export from `exports.tsv`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Export {
    class: String,
    name: String,
    path: PathBuf,
}

const USAGE: &str = "usage: ds-check check <bootstrap-surfaces.tsv> <plain-fixture-dir>\n       ds-check layers <exports.tsv> <bootstrap-surfaces.tsv> <plain-fixture-dir>";

fn run(args: &[String]) -> Result<String, String> {
    let root = env::current_dir().map_err(|error| format!("resolve repository root: {error}"))?;
    match args {
        [command, manifest, fixture] if command == "check" => {
            run_check(&root, Path::new(manifest), Path::new(fixture))
        }
        [command, exports, manifest, fixture] if command == "layers" => run_layers(
            &root,
            Path::new(exports),
            Path::new(manifest),
            Path::new(fixture),
        ),
        _ => Err(USAGE.to_owned()),
    }
}

fn run_check(root: &Path, manifest: &Path, fixture: &Path) -> Result<String, String> {
    let coverage = validate_manifest(root, manifest)?;
    let classified = validate_manifest_completeness(root, manifest)?;
    validate_plain_fixture(root, fixture)?;
    let members = validate_lint_inheritance(root)?;

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
            walk_files(&root, &resolved)?
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

/// Every tracked repository file must be covered by some manifest row. Without this
/// pass the inventory's exhaustiveness would only ever be a hand-maintained
/// snapshot: a newly added file would be neither classified nor privacy-scanned
/// while the bootstrap check stayed green. Returns the number of repository
/// files confirmed classified.
fn validate_manifest_completeness(root: &Path, manifest: &Path) -> Result<usize, String> {
    validate_relative_path(manifest)?;
    let root = canonical(root, "repository root")?;
    let text = fs::read_to_string(root.join(manifest))
        .map_err(|error| format!("read manifest {}: {error}", manifest.display()))?;
    let surfaces = parse_manifest(&text)?;
    let classified: Vec<(PathBuf, bool)> = surfaces
        .iter()
        .map(|surface| {
            let path = root.join(&surface.path);
            let is_directory = path.is_dir();
            (path, is_directory)
        })
        .collect();

    let files = tracked_files(&root)?;
    for file in &files {
        // A file row covers only itself; a directory row covers everything
        // beneath it. Trackedness comes from Git rather than a filesystem skip
        // list, so a force-tracked file cannot hide under an ignored-looking
        // directory name such as `target` or `dist`.
        if !classified.iter().any(|(entry, is_directory)| {
            file == entry || (*is_directory && file.starts_with(entry))
        }) {
            return Err(format!(
                "tracked repository file {} is not classified in {}",
                relative_to(&root, file).display(),
                manifest.display()
            ));
        }
    }

    Ok(files.len())
}

/// Return the exact tracked-file set from the repository index. The bootstrap
/// contract is about tracked source, not transient build/editor output, so Git
/// is the authority rather than an approximation of `.gitignore` semantics.
fn tracked_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|error| format!("run git ls-files: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git ls-files failed with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("decode git ls-files output as UTF-8: {error}"))?;
    let mut files: Vec<PathBuf> = stdout
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| root.join(path))
        .collect();
    files.sort();
    Ok(files)
}

fn parse_manifest(text: &str) -> Result<Vec<Surface>, String> {
    let mut surfaces: Vec<Surface> = Vec::new();

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
        let path = PathBuf::from(path);
        validate_relative_path(&path)?;
        if surfaces.iter().any(|surface| surface.path == path) {
            return Err(format!(
                "manifest line {line_number} duplicates path {}",
                path.display()
            ));
        }

        let scan_exempt = match exempt_field {
            None => None,
            Some(field) => Some(parse_scan_exemption(line_number, field.trim())?),
        };

        surfaces.push(Surface {
            class: class.to_owned(),
            path,
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

/// Sorted files beneath a directory surface. Like inventory completeness, this
/// derives from Git's tracked-file set rather than a filesystem walk with a
/// directory-name skip list, so a tracked file is never skipped and untracked
/// build or editor output is never scanned.
fn walk_files(root: &Path, directory: &Path) -> Result<Vec<PathBuf>, String> {
    Ok(tracked_files(root)?
        .into_iter()
        .filter(|file| file.starts_with(directory))
        .collect())
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
        return Err("plain fixture must load its local ./consumer.css".to_owned());
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

/// Validate the supported CSS exports, their stylesheet graphs, and the plain
/// consumer fixture's use of them.
fn run_layers(
    root: &Path,
    exports_file: &Path,
    manifest: &Path,
    fixture: &Path,
) -> Result<String, String> {
    validate_relative_path(exports_file)?;
    validate_relative_path(manifest)?;
    let root = canonical(root, "repository root")?;

    let exports_text = fs::read_to_string(root.join(exports_file))
        .map_err(|error| format!("read exports {}: {error}", exports_file.display()))?;
    let exports = parse_exports(&exports_text)?;
    let manifest_text = fs::read_to_string(root.join(manifest))
        .map_err(|error| format!("read manifest {}: {error}", manifest.display()))?;
    let surfaces = parse_manifest(&manifest_text)?;
    validate_export_classification(&exports, &surfaces, manifest)?;

    let tracked = tracked_files(&root)?;
    let mut reached = BTreeSet::new();
    let mut owned = BTreeSet::new();
    for export in &exports {
        validate_entry_publishes_order(&root, export)?;
        let graph = css::validate_graph(&root, &export.path, Path::new(STYLES_ROOT))?;
        owned.extend(graph.owners.into_keys());
        reached.extend(graph.stylesheets);
    }
    css::validate_reachability(&root, Path::new(STYLES_ROOT), &reached, &tracked)?;

    validate_relative_path(fixture)?;
    let index = root.join(fixture).join("index.html");
    let html =
        fs::read_to_string(&index).map_err(|error| format!("read {}: {error}", index.display()))?;
    validate_fixture_links(&html, &exports)?;

    Ok(format!(
        "validated {} CSS export(s) ({} stylesheet(s) reached, {} owned layer(s)); \
plain fixture {} loads only declared exports and its consumer stylesheet",
        exports.len(),
        reached.len(),
        owned.len(),
        fixture.display()
    ))
}

fn parse_exports(text: &str) -> Result<Vec<Export>, String> {
    let mut exports: Vec<Export> = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        let [class, name, path] = fields.as_slice() else {
            return Err(format!(
                "exports line {line_number} must be <class><tab><export-name><tab><path>"
            ));
        };

        if *class != EXPORT_CLASS {
            return Err(format!(
                "exports line {line_number} uses class '{class}'; CSS exports must be {EXPORT_CLASS} until a reviewed contract earns public-stable"
            ));
        }
        if !name.ends_with(".css") || name.contains('/') {
            return Err(format!(
                "exports line {line_number} export name '{name}' must be a bare .css file name"
            ));
        }
        let path = PathBuf::from(path);
        validate_relative_path(&path)?;
        if !path.starts_with(STYLES_ROOT) || path.extension().is_none_or(|ext| ext != "css") {
            return Err(format!(
                "exports line {line_number} path {} must be a .css file under {STYLES_ROOT}/",
                path.display()
            ));
        }
        if exports
            .iter()
            .any(|export| export.name == *name || export.path == path)
        {
            return Err(format!(
                "exports line {line_number} duplicates an export name or path"
            ));
        }

        exports.push(Export {
            class: (*class).to_owned(),
            name: (*name).to_owned(),
            path,
        });
    }

    if exports.is_empty() {
        return Err("exports declare no supported CSS entrypoint".to_owned());
    }
    Ok(exports)
}

/// Every export must be classified with the same class in the compatibility
/// inventory, and every classified public stylesheet must be exported.
fn validate_export_classification(
    exports: &[Export],
    surfaces: &[Surface],
    manifest: &Path,
) -> Result<(), String> {
    for export in exports {
        let classified = surfaces
            .iter()
            .any(|surface| surface.path == export.path && surface.class == export.class);
        if !classified {
            return Err(format!(
                "export {} ({}) is not classified {} in {}",
                export.name,
                export.path.display(),
                export.class,
                manifest.display()
            ));
        }
    }

    for surface in surfaces {
        let is_stylesheet = surface.path.extension().is_some_and(|ext| ext == "css");
        if surface.class != "internal"
            && is_stylesheet
            && !exports.iter().any(|export| export.path == surface.path)
        {
            return Err(format!(
                "{} is classified {} but is not a declared export",
                surface.path.display(),
                surface.class
            ));
        }
    }
    Ok(())
}

/// An entrypoint must publish its layer order before any populated rule or
/// import, so consumer layers declared afterwards sort after it.
fn validate_entry_publishes_order(root: &Path, export: &Export) -> Result<(), String> {
    let source = fs::read_to_string(root.join(&export.path))
        .map_err(|error| format!("read export {}: {error}", export.path.display()))?;
    let nodes = css::parse(&source)
        .map_err(|error| format!("export {}: {error}", export.path.display()))?;
    let first = nodes
        .iter()
        .find(|node| !matches!(node, css::Node::Statement { name, .. } if name == "charset"));
    match first {
        Some(css::Node::Statement { name, .. }) if name == "layer" => Ok(()),
        _ => Err(format!(
            "export {} must begin with an @layer order statement",
            export.path.display()
        )),
    }
}

/// The consumer fixture may load only declared exports, at their public path,
/// followed by its own consumer stylesheet.
fn validate_fixture_links(html: &str, exports: &[Export]) -> Result<(), String> {
    let lowered = html.to_ascii_lowercase();
    if lowered.contains("<style") || lowered.contains("style=") {
        return Err("plain fixture must not carry inline styles".to_owned());
    }

    let hrefs: Vec<&str> = html
        .split("href=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or_default())
        .collect();
    let mut last_export = None;
    let mut consumer = None;

    for (position, href) in hrefs.iter().enumerate() {
        if *href == FIXTURE_CONSUMER_STYLESHEET {
            consumer = Some(position);
            continue;
        }
        let declared = href
            .strip_prefix('/')
            .is_some_and(|path| exports.iter().any(|export| export.path == Path::new(path)));
        if !declared {
            return Err(format!(
                "plain fixture links '{href}', which is neither a declared export path nor {FIXTURE_CONSUMER_STYLESHEET}"
            ));
        }
        last_export = Some(position);
    }

    match (last_export, consumer) {
        (Some(export), Some(consumer)) if export < consumer => Ok(()),
        (None, _) => Err("plain fixture must load a declared Design System export".to_owned()),
        _ => Err(format!(
            "plain fixture must load {FIXTURE_CONSUMER_STYLESHEET} after the Design System export"
        )),
    }
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
    fn manifest_rejects_duplicate_paths_and_aliases() {
        let duplicate = parse_manifest(
            "internal\tpackages/styles/index.css\npublic-preview\tpackages/styles/index.css",
        )
        .expect_err("duplicate manifest path must be rejected");
        assert!(duplicate.contains("duplicates path"), "{duplicate}");

        let alias = parse_manifest(
            "internal\tpackages/styles/index.css\npublic-preview\t./packages/styles/index.css",
        )
        .expect_err("dot-segment alias must be rejected");
        assert!(alias.contains("forbidden component"), "{alias}");
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

    fn export(path: &str) -> Export {
        Export {
            class: EXPORT_CLASS.to_owned(),
            name: "core.css".to_owned(),
            path: PathBuf::from(path),
        }
    }

    /// The repository's own exports, inventory, and consumer fixture satisfy the
    /// layer contract end to end.
    #[test]
    fn repository_layer_contract_passes() {
        let output = run_layers(
            &repository_root(),
            Path::new("exports.tsv"),
            Path::new("bootstrap-surfaces.tsv"),
            Path::new("fixtures/plain-html"),
        )
        .expect("repository layer contract");
        assert!(output.starts_with("validated 1 CSS export(s)"), "{output}");
    }

    #[test]
    fn exports_reject_unearned_or_internal_classes() {
        let stable = parse_exports("public-stable\tcore.css\tpackages/styles/index.css")
            .expect_err("public-stable export");
        assert!(stable.contains("must be public-preview"), "{stable}");
        let internal = parse_exports("internal\tcore.css\tpackages/styles/index.css")
            .expect_err("internal export");
        assert!(internal.contains("must be public-preview"), "{internal}");
    }

    #[test]
    fn exports_must_be_stylesheets_under_the_styles_root() {
        let outside = parse_exports("public-preview\tcore.css\tfixtures/plain-html/consumer.css")
            .expect_err("export outside styles root");
        assert!(outside.contains("under packages/styles/"), "{outside}");
        let duplicate = parse_exports(
            "public-preview\tcore.css\tpackages/styles/index.css\npublic-preview\tcore.css\tpackages/styles/other.css",
        )
        .expect_err("duplicate export name");
        assert!(duplicate.contains("duplicates"), "{duplicate}");
        assert!(parse_exports("# comment only").is_err());
    }

    #[test]
    fn exports_and_inventory_classes_must_agree() {
        let exports = vec![export("packages/styles/index.css")];
        let internal = parse_manifest("internal\tpackages/styles/index.css").expect("manifest");
        let error = validate_export_classification(&exports, &internal, Path::new("m.tsv"))
            .expect_err("export classified internal");
        assert!(
            error.contains("is not classified public-preview"),
            "{error}"
        );

        let extra = parse_manifest(
            "public-preview\tpackages/styles/index.css\npublic-preview\tpackages/styles/extra.css",
        )
        .expect("manifest");
        let error = validate_export_classification(&exports, &extra, Path::new("m.tsv"))
            .expect_err("public stylesheet without export");
        assert!(error.contains("is not a declared export"), "{error}");
    }

    #[test]
    fn fixture_links_only_declared_exports_then_consumer_css() {
        let exports = vec![export("packages/styles/index.css")];
        let valid = r#"<link rel="stylesheet" href="/packages/styles/index.css"><link rel="stylesheet" href="./consumer.css">"#;
        validate_fixture_links(valid, &exports).expect("declared export then consumer");

        let private = r#"<link href="/packages/styles/internal.css"><link href="./consumer.css">"#;
        let error = validate_fixture_links(private, &exports).expect_err("undeclared path");
        assert!(error.contains("neither a declared export"), "{error}");

        let reversed = r#"<link href="./consumer.css"><link href="/packages/styles/index.css">"#;
        let error = validate_fixture_links(reversed, &exports).expect_err("consumer first");
        assert!(error.contains("after the Design System export"), "{error}");

        let inline = r#"<link href="/packages/styles/index.css"><style>a{}</style><link href="./consumer.css">"#;
        assert!(validate_fixture_links(inline, &exports).is_err());
    }

    /// Directory surfaces are walked from Git's tracked-file set, so the walk and
    /// inventory completeness share one authority.
    #[test]
    fn directory_walk_uses_tracked_files() {
        let root = repository_root().canonicalize().expect("root");
        let directory = root.join("fixtures/plain-html");
        let walked = walk_files(&root, &directory).expect("walk");
        let tracked: Vec<PathBuf> = tracked_files(&root)
            .expect("tracked")
            .into_iter()
            .filter(|file| file.starts_with(&directory))
            .collect();
        assert_eq!(walked, tracked);
        assert!(walked.iter().all(|file| file.starts_with(&directory)));
    }

    #[test]
    fn member_manifest_fixture_without_lint_opt_in_is_rejected() {
        let manifest =
            fs::read_to_string(repository_root().join(boundary("member-lints-missing.toml")))
                .expect("read fixture");
        assert!(!member_inherits_workspace_lints(&manifest));
    }
}
