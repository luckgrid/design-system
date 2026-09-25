//! Release verification: the source-side artifact inventory and the packaged-form
//! check of an unpacked release archive.
//!
//! `inventory` proves the authored inventory against the declared exports, the
//! compatibility classification, the release identity, and the license metadata.
//! `release` proves an unpacked archive on its own: identity, manifest checksums,
//! the public surface tables against the shipped CSS, the packaged import graph,
//! documentation links, and a privacy scan over every file. Given the source
//! root, it also proves each archived file is the inventory source, byte for byte,
//! apart from the two recorded transforms.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::static_renderer::sha256_hex;
use crate::{
    layout, lexical, primitive, relative_to, theme, tokens, tracked_files, validate_relative_path,
};

const IDENTITY_FILE: &str = "IDENTITY.tsv";
const MANIFEST_FILE: &str = "MANIFEST.tsv";
const MATURITY: &str = "preview";
const LICENSE_ID: &str = "MIT";
const CLASSES: [&str; 3] = ["public-preview", "internal", "documentation"];
const KINDS: [&str; 7] = [
    "css", "surface", "doc", "license", "tool", "identity", "manifest",
];
const TRANSFORMS: [&str; 3] = ["none", "identity", "tailwind-core-import"];
const CORE_IMPORT_SOURCE: &str = "@import \"../../packages/styles/index.css\";";
const CORE_IMPORT_ARCHIVE: &str = "@import \"./core.css\";";
const LAYER_ORDER: &str =
    "ds.tokens, ds.base, ds.layouts, ds.primitives, ds.components, ds.utilities";
const CHANNEL: &str = "source-tag-and-github-release-archive";

/// Surface tables the archive ships, archive path to source path.
const SURFACES: [(&str, &str); 8] = [
    ("surface/exports.tsv", "exports.tsv"),
    ("surface/adapter-exports.tsv", "adapter-exports.tsv"),
    (
        "surface/tailwind-projection.tsv",
        "adapters/tailwind/projection.tsv",
    ),
    ("surface/tokens.tsv", "tokens.tsv"),
    ("surface/theme.tsv", "theme.tsv"),
    ("surface/base.tsv", "base.tsv"),
    ("surface/layouts.tsv", "layouts.tsv"),
    ("surface/primitives.tsv", "primitives.tsv"),
];

/// Documentation every release must ship.
const REQUIRED_DOCS: [&str; 12] = [
    "README.md",
    "RELEASE.md",
    "LICENSE",
    "docs/browser-support.md",
    "docs/architecture/README.md",
    "docs/architecture/css-entrypoint.md",
    "docs/architecture/tokens.md",
    "docs/architecture/theme.md",
    "docs/architecture/base.md",
    "docs/architecture/layouts.md",
    "docs/architecture/primitives.md",
    "docs/architecture/hooks.md",
];

/// Wording the published browser-support statement must carry (S102 R1-R4 and the
/// S101 amendment) and wording it must not carry.
const SUPPORT_REQUIRED: [(&str, &str); 11] = [
    (
        "Chrome for Testing 123.0.6312.122",
        "R2 names Chrome for Testing",
    ),
    ("Windows desktop", "R3 names Windows desktop unverified"),
    ("Linux desktop", "R3 names Linux desktop unverified"),
    ("Windows Edge", "R3 names Windows Edge unverified"),
    ("DS-E05.S4.T1-C1", "the deferred-row reference"),
    (
        "Playwright-patched Gecko 121.0",
        "R1 names the patched Gecko build",
    ),
    (
        "Firefox 155",
        "R1/R4 attribute Firefox evidence to Firefox 155",
    ),
    ("no real system", "R1 states no system toggle was tested"),
    (
        "Mozilla bug 187508",
        "R1 cites external documentation as such",
    ),
    (
        "applies no forced palette",
        "R4 states the Gecko 121 emulation limit",
    ),
    (
        "WebKit forced-colors emulation",
        "R4 states the WebKit emulation limit",
    ),
];
const SUPPORT_FORBIDDEN: [(&str, &str); 3] = [
    (
        "Firefox follows the system keyboard-navigation setting",
        "R1: an unobserved generalization",
    ),
    (
        "draft compatibility statement",
        "the statement is published",
    ),
    (
        "is not published as a release claim",
        "the statement is published",
    ),
];

/// Markers no archived file may contain, with the reason. Matching is at token
/// boundaries, lowercase. The list is specific on purpose: a broad word such as
/// `token` or `password` is ordinary Design System vocabulary.
const PRIVATE_MARKERS: [(&str, &str); 26] = [
    ("lg-workstreams", "private repository name"),
    ("build/src/", "private workspace path"),
    ("build/bin/", "private workspace path"),
    ("/users/", "local user path"),
    ("/home/", "local user path"),
    ("c:\\users", "local user path"),
    ("git@github.com", "private remote"),
    ("ssh://", "private remote"),
    ("@luna/", "unpublished alias"),
    ("@design-system/", "unpublished alias"),
    ("@ds/", "unpublished alias"),
    ("node_modules", "source-tree coupling"),
    ("npm.pkg.github.com", "private registry"),
    ("_authtoken", "registry credential"),
    (".internal/", "private host"),
    ("-----begin", "key material"),
    ("ghp_", "GitHub credential"),
    ("gho_", "GitHub credential"),
    ("github_pat_", "GitHub credential"),
    ("sk-ant-", "API credential"),
    ("akia", "cloud credential"),
    ("xoxb-", "chat credential"),
    ("api_key=", "credential assignment"),
    ("password=", "credential assignment"),
    ("secret=", "credential assignment"),
    ("bearer ey", "bearer credential"),
];

/// Commands the consumer script must not run: no Rust, Node, or Tailwind.
const FORBIDDEN_TOOLS: [&str; 9] = [
    "cargo",
    "rustc",
    "node",
    "npm",
    "npx",
    "pnpm",
    "yarn",
    "tailwindcss",
    "postcss",
];

type Table = BTreeMap<String, String>;

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn rows(text: &str) -> Vec<Vec<&str>> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| line.split('\t').collect())
        .collect()
}

/// `key = "value"` pairs under one TOML table header.
fn toml_table(text: &str, table: &str) -> Table {
    let mut values = Table::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == format!("[{table}]");
            continue;
        }
        if inside
            && !line.starts_with('#')
            && let Some((key, value)) = line.split_once('=')
        {
            let value = value.trim().trim_matches('"');
            values.insert(key.trim().to_owned(), value.to_owned());
        }
    }
    values
}

fn require<'a>(table: &'a Table, key: &str, label: &str) -> Result<&'a str, String> {
    table
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("{label} has no `{key}`"))
}

/// A `0.y.z-preview.n` version: never `1.0.0` or later, and marked as a preview.
fn validate_version(version: &str) -> Result<(), String> {
    let core_and_pre = version
        .split_once('-')
        .ok_or_else(|| format!("version `{version}` must carry a -preview.<n> pre-release"))?;
    let core: Vec<&str> = core_and_pre.0.split('.').collect();
    if core.len() != 3
        || core
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!(
            "version `{version}` core must be <major>.<minor>.<patch>"
        ));
    }
    if core[0] != "0" {
        return Err(format!(
            "version `{version}` is 1.0.0 or later; `stable` needs separate evidence"
        ));
    }
    let pre = core_and_pre.1;
    let number = pre
        .strip_prefix("preview.")
        .or_else(|| pre.strip_prefix("rehearsal."));
    if !number.is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) {
        return Err(format!(
            "version `{version}` pre-release must be preview.<n> (or rehearsal.<n> for a rehearsal build)"
        ));
    }
    Ok(())
}

fn check_license_text(text: &str) -> Result<(), String> {
    let mut lines = text.lines();
    if lines.next() != Some("MIT License") {
        return Err("LICENSE must be the MIT License text".to_owned());
    }
    if !text.contains("Permission is hereby granted, free of charge")
        || !text.contains("THE SOFTWARE IS PROVIDED \"AS IS\"")
        || !text.lines().any(|line| line.starts_with("Copyright (c) "))
    {
        return Err("LICENSE is missing MIT grant, copyright, or warranty text".to_owned());
    }
    Ok(())
}

fn workspace_value(cargo: &str, key: &str) -> Option<String> {
    toml_table(cargo, "workspace.package").remove(key)
}

/// Source-side inventory proof.
pub fn run_inventory(
    root: &Path,
    inventory: &Path,
    identity: &Path,
    exports_file: &Path,
    adapter_exports: &Path,
    manifest: &Path,
) -> Result<String, String> {
    for path in [inventory, identity, exports_file, adapter_exports, manifest] {
        validate_relative_path(path)?;
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("resolve repository root: {error}"))?;

    // Identity and source metadata: one version, one license, preview maturity.
    let identity_text = read(&root.join(identity))?;
    let release = toml_table(&identity_text, "release");
    let version = require(&release, "version", "release/identity.toml")?;
    validate_version(version)?;
    if version.contains("rehearsal") || !version.contains("-preview.") {
        return Err("identity version must be a -preview.<n> release version".to_owned());
    }
    if require(&release, "tag", "identity")? != format!("v{version}") {
        return Err("identity tag must be v<version>".to_owned());
    }
    if require(&release, "maturity", "identity")? != MATURITY {
        return Err(format!(
            "identity maturity must be `{MATURITY}`; stable needs separate evidence"
        ));
    }
    if require(&release, "license", "identity")? != LICENSE_ID {
        return Err(format!("identity license must be {LICENSE_ID}"));
    }
    if require(&release, "channel", "identity")? != CHANNEL {
        return Err(format!(
            "identity channel must be {CHANNEL}; a registry needs a recorded decision"
        ));
    }
    if require(&release, "archive_format", "identity")? != "tar.gz" {
        return Err("identity archive_format must be tar.gz".to_owned());
    }
    let cargo = read(&root.join("Cargo.toml"))?;
    if workspace_value(&cargo, "version").as_deref() != Some(version) {
        return Err("Cargo.toml workspace version must equal the identity version".to_owned());
    }
    if workspace_value(&cargo, "license").as_deref() != Some(LICENSE_ID) {
        return Err(format!("Cargo.toml workspace license must be {LICENSE_ID}"));
    }
    let descriptor = toml_table(
        &read(&root.join("design-system.descriptor.toml"))?,
        "release",
    );
    if descriptor.get("license").map(String::as_str) != Some(LICENSE_ID)
        || descriptor.get("maturity").map(String::as_str) != Some(MATURITY)
    {
        return Err("descriptor [release] must state license MIT and maturity preview".to_owned());
    }
    check_license_text(&read(&root.join("LICENSE"))?)?;
    for document in ["README.md", "CONTRIBUTING.md", "SECURITY.md"] {
        if read(&root.join(document))?
            .to_ascii_lowercase()
            .contains("unlicensed")
        {
            return Err(format!(
                "{document} still describes the repository as unlicensed"
            ));
        }
    }
    let readme = read(&root.join("README.md"))?;
    if !readme.contains(LICENSE_ID) {
        return Err("README.md must state the MIT license".to_owned());
    }
    if !readme.contains(version) {
        return Err(format!("README.md must name the release version {version}"));
    }

    // The inventory rows.
    let text = read(&root.join(inventory))?;
    let tracked: BTreeSet<String> = tracked_files(&root)?
        .iter()
        .map(|file| relative_to(&root, file).display().to_string())
        .collect();
    let surfaces_text = read(&root.join(manifest))?;
    let classes: BTreeMap<String, String> = rows(&surfaces_text)
        .into_iter()
        .filter(|row| row.len() >= 2)
        .map(|row| (row[1].to_owned(), row[0].to_owned()))
        .collect();
    let mut by_archive: BTreeMap<String, (String, String, String, String)> = BTreeMap::new();
    for row in rows(&text) {
        let [class, kind, archive, source, transform] = row.as_slice() else {
            return Err(format!(
                "inventory row must be <class><tab><kind><tab><archive><tab><source><tab><transform>: {}",
                row.join("\t")
            ));
        };
        if !CLASSES.contains(class) || (*class == "internal" && *kind != "css") {
            return Err(format!(
                "inventory class `{class}` is not valid for {archive}"
            ));
        }
        if !KINDS.contains(kind) || *kind == "identity" || *kind == "manifest" {
            return Err(format!(
                "inventory kind `{kind}` is not valid for {archive}"
            ));
        }
        if !TRANSFORMS.contains(transform) {
            return Err(format!(
                "inventory transform `{transform}` is unknown for {archive}"
            ));
        }
        validate_relative_path(Path::new(archive))?;
        validate_relative_path(Path::new(source))?;
        if by_archive
            .insert(
                (*archive).to_owned(),
                (
                    (*class).to_owned(),
                    (*kind).to_owned(),
                    (*source).to_owned(),
                    (*transform).to_owned(),
                ),
            )
            .is_some()
        {
            return Err(format!("inventory lists archive path {archive} twice"));
        }
        if !tracked.contains(*source) {
            return Err(format!("inventory source {source} is not tracked by Git"));
        }
        let bootstrap = classes.get(*source).ok_or_else(|| {
            format!("inventory source {source} is not classified in the bootstrap inventory")
        })?;
        let matches = match *class {
            "documentation" => bootstrap == "internal",
            other => bootstrap == other,
        };
        if !matches {
            return Err(format!(
                "inventory row {archive} is `{class}` but {source} is `{bootstrap}` in the bootstrap inventory"
            ));
        }
        let expected_prefix = match *kind {
            "css" => Some("css/"),
            "surface" => Some("surface/"),
            "tool" => Some("consumer/"),
            _ => None,
        };
        if expected_prefix.is_some_and(|prefix| !archive.starts_with(prefix)) {
            return Err(format!(
                "{kind} row {archive} is outside its archive directory"
            ));
        }
        if *kind != "css" && *transform == "tailwind-core-import" {
            return Err(format!(
                "{archive} may not use the Tailwind import transform"
            ));
        }
    }

    // CSS: exactly the export graph plus the declared adapter exports.
    let exports_text = read(&root.join(exports_file))?;
    let exports = crate::parse_exports(&exports_text)?;
    let staged = crate::static_renderer::derive_staged_graph(&root, &exports)?;
    let mut expected_css: BTreeMap<String, (String, &str)> = BTreeMap::new();
    for item in &staged {
        expected_css.insert(
            format!("css/{}", item.relative),
            (
                relative_to(&root, &item.source).display().to_string(),
                "none",
            ),
        );
    }
    let adapter_text = read(&root.join(adapter_exports))?;
    for row in rows(&adapter_text) {
        let [class, name, path] = row.as_slice() else {
            return Err("adapter export rows must be <class><tab><name><tab><path>".to_owned());
        };
        if *class != "public-preview" || !name.ends_with(".css") || name.contains('/') {
            return Err(format!(
                "adapter export {name} is not a public-preview bare .css name"
            ));
        }
        if expected_css
            .insert(
                format!("css/{name}"),
                ((*path).to_owned(), "tailwind-core-import"),
            )
            .is_some()
        {
            return Err(format!(
                "adapter export {name} collides with a core stylesheet"
            ));
        }
    }
    let inventory_css: BTreeMap<&String, &(String, String, String, String)> = by_archive
        .iter()
        .filter(|(_, row)| row.1 == "css")
        .collect();
    if let Some(missing) = expected_css
        .keys()
        .find(|archive| !inventory_css.contains_key(archive))
    {
        return Err(format!(
            "inventory omits {missing}, which a declared export publishes"
        ));
    }
    for (archive, row) in &inventory_css {
        let Some((source, transform)) = expected_css.get(*archive) else {
            return Err(format!(
                "inventory ships {archive}, which no declared export publishes"
            ));
        };
        if row.2 != *source || row.3 != *transform {
            return Err(format!(
                "inventory row {archive} must be sourced from {source} with transform {transform}"
            ));
        }
    }
    // Entrypoints (the exports) are public-preview; imported modules are internal.
    let entrypoints: BTreeSet<String> = exports
        .iter()
        .map(|export| format!("css/{}", export.name))
        .chain(
            rows(&adapter_text)
                .iter()
                .map(|row| format!("css/{}", row[1])),
        )
        .collect();
    for (archive, row) in &inventory_css {
        let want = if entrypoints.contains(*archive) {
            "public-preview"
        } else {
            "internal"
        };
        if row.0 != want {
            return Err(format!(
                "{archive} must be classified {want} in the inventory"
            ));
        }
    }

    // Surface tables: the complete public surface, from the fixed set.
    for (archive, source) in SURFACES {
        match by_archive.get(archive) {
            Some(row) if row.1 == "surface" && row.2 == source && row.3 == "none" => {}
            _ => {
                return Err(format!(
                    "inventory must ship {source} as surface {archive} with no transform"
                ));
            }
        }
    }
    if by_archive.values().filter(|row| row.1 == "surface").count() != SURFACES.len() {
        return Err("inventory ships a surface table outside the public surface set".to_owned());
    }
    if by_archive
        .get("surface/exports.tsv")
        .map(|row| row.2.as_str())
        != Some(exports_file.display().to_string().as_str())
        || by_archive
            .get("surface/adapter-exports.tsv")
            .map(|row| row.2.as_str())
            != Some(adapter_exports.display().to_string().as_str())
    {
        return Err("inventory surface rows must name the declared exports files".to_owned());
    }

    // Documentation, license, and consumer script.
    for required in REQUIRED_DOCS {
        if !by_archive.contains_key(required) {
            return Err(format!("inventory omits required {required}"));
        }
    }
    match by_archive.get("LICENSE") {
        Some(row) if row.1 == "license" && row.2 == "LICENSE" && row.3 == "none" => {}
        _ => return Err("inventory must ship LICENSE byte for byte from LICENSE".to_owned()),
    }
    match by_archive.get("consumer/ds-consumer.sh") {
        Some(row) if row.1 == "tool" && row.0 == "public-preview" => {}
        _ => {
            return Err(
                "inventory must ship consumer/ds-consumer.sh as a public-preview tool".to_owned(),
            );
        }
    }
    for archive in ["README.md", "RELEASE.md"] {
        if by_archive[archive].3 != "identity" {
            return Err(format!("{archive} must use the identity transform"));
        }
    }
    let support = read(&root.join(&by_archive["docs/browser-support.md"].2))?;
    for (needle, reason) in SUPPORT_REQUIRED {
        if !support.contains(needle) {
            return Err(format!(
                "docs/browser-support.md lacks `{needle}`: {reason}"
            ));
        }
    }
    for (needle, reason) in SUPPORT_FORBIDDEN {
        if support.contains(needle) {
            return Err(format!(
                "docs/browser-support.md still says `{needle}`: {reason}"
            ));
        }
    }

    Ok(format!(
        "release inventory {}: version {version} maturity {MATURITY} license {LICENSE_ID}; {} archive path(s) \
({} css matching the export graph, {} surface table(s), {} entrypoint(s) public-preview); classes match the \
bootstrap inventory; browser-support gates R1-R4 present",
        inventory.display(),
        by_archive.len(),
        inventory_css.len(),
        SURFACES.len(),
        entrypoints.len(),
    ))
}

/// One archived file's manifest row.
struct Entry {
    sha256: String,
    size: usize,
    class: String,
    kind: String,
}

fn walk(directory: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in
            fs::read_dir(&next).map_err(|error| format!("read {}: {error}", next.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            let meta = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if meta.file_type().is_symlink() {
                return Err(format!(
                    "archive contains a symbolic link: {}",
                    path.display()
                ));
            }
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file() {
                files.insert(relative_to(directory, &path).display().to_string());
            } else {
                return Err(format!(
                    "archive contains a non-regular file: {}",
                    path.display()
                ));
            }
        }
    }
    Ok(files)
}

/// Every `@import` target in normalized CSS: a quoted string or `url()` argument.
fn imports(normalized: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for (index, _) in normalized.match_indices("@import") {
        let rest = normalized[index + "@import".len()..].trim_start();
        let target = if let Some(quoted) = rest.strip_prefix('"') {
            quoted.split('"').next().unwrap_or("")
        } else if let Some(quoted) = rest.strip_prefix('\'') {
            quoted.split('\'').next().unwrap_or("")
        } else if let Some(function) = rest.strip_prefix("url(") {
            function
                .split(')')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
        } else {
            ""
        };
        targets.push(target.to_owned());
    }
    targets
}

/// Class hooks (`.ds-name`) named in normalized CSS.
fn hook_classes(normalized: &str) -> BTreeSet<String> {
    let mut hooks = BTreeSet::new();
    for (index, _) in normalized.match_indices(".ds-") {
        let name: String = normalized[index + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        hooks.insert(name);
    }
    hooks
}

fn markdown_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else { break };
            links.push(after[..end].to_owned());
            rest = &after[end + 1..];
        }
    }
    links
}

/// Packaged-form proof of an unpacked archive, optionally against the source.
pub fn run_release(archive: &Path, source: Option<&Path>) -> Result<String, String> {
    let archive = archive
        .canonicalize()
        .map_err(|error| format!("resolve archive directory {}: {error}", archive.display()))?;

    // Identity.
    let mut identity = Table::new();
    for row in rows(&read(&archive.join(IDENTITY_FILE))?) {
        let [key, value] = row.as_slice() else {
            return Err("IDENTITY.tsv rows must be <key><tab><value>".to_owned());
        };
        if identity
            .insert((*key).to_owned(), (*value).to_owned())
            .is_some()
        {
            return Err(format!("IDENTITY.tsv repeats `{key}`"));
        }
    }
    let version = require(&identity, "version", IDENTITY_FILE)?.to_owned();
    let kind = require(&identity, "identity", IDENTITY_FILE)?.to_owned();
    validate_version(&version)?;
    for (key, want) in [
        ("product", "design-system"),
        ("maturity", MATURITY),
        ("license", LICENSE_ID),
        ("channel", CHANNEL),
        ("archive_format", "tar.gz"),
    ] {
        if require(&identity, key, IDENTITY_FILE)? != want {
            return Err(format!("IDENTITY.tsv `{key}` must be `{want}`"));
        }
    }
    let commit = require(&identity, "source_commit", IDENTITY_FILE)?.to_owned();
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("IDENTITY.tsv source_commit must be a full lowercase sha".to_owned());
    }
    let tag = require(&identity, "tag", IDENTITY_FILE)?.to_owned();
    match kind.as_str() {
        "release" => {
            if version.contains("rehearsal")
                || !version.contains("-preview.")
                || tag != format!("v{version}")
            {
                return Err(
                    "a release identity needs a -preview.<n> version and tag v<version>".to_owned(),
                );
            }
        }
        "rehearsal" => {
            if !version.contains("-rehearsal.") || tag != "none" {
                return Err(
                    "a rehearsal identity needs a -rehearsal.<n> version and tag `none`".to_owned(),
                );
            }
        }
        other => {
            return Err(format!(
                "IDENTITY.tsv identity `{other}` is neither release nor rehearsal"
            ));
        }
    }
    if archive.file_name().and_then(|name| name.to_str())
        != Some(format!("design-system-{version}").as_str())
    {
        return Err(format!(
            "archive directory must be named design-system-{version}"
        ));
    }

    // Manifest completeness and checksums.
    let manifest_text = read(&archive.join(MANIFEST_FILE))?;
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    let mut previous = String::new();
    for row in rows(&manifest_text) {
        let [sha256, size, class, kind, path] = row.as_slice() else {
            return Err(
                "MANIFEST.tsv rows must be <sha256><tab><size><tab><class><tab><kind><tab><path>"
                    .to_owned(),
            );
        };
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(format!(
                "MANIFEST.tsv sha256 for {path} is not 64 lowercase hex digits"
            ));
        }
        if !CLASSES.contains(class) || !KINDS.contains(kind) {
            return Err(format!(
                "MANIFEST.tsv row {path} has class `{class}` or kind `{kind}` outside the vocabulary"
            ));
        }
        validate_relative_path(Path::new(path))?;
        if path.as_bytes() <= previous.as_bytes() {
            return Err("MANIFEST.tsv must list unique paths in byte order".to_owned());
        }
        previous = (*path).to_owned();
        entries.insert(
            (*path).to_owned(),
            Entry {
                sha256: (*sha256).to_owned(),
                size: size
                    .parse()
                    .map_err(|_| format!("MANIFEST.tsv size for {path} is not a number"))?,
                class: (*class).to_owned(),
                kind: (*kind).to_owned(),
            },
        );
    }
    let on_disk = walk(&archive)?;
    let listed: BTreeSet<String> = entries
        .keys()
        .cloned()
        .chain([MANIFEST_FILE.to_owned()])
        .collect();
    if let Some(extra) = on_disk.difference(&listed).next() {
        return Err(format!(
            "archive holds `{extra}`, which MANIFEST.tsv does not list"
        ));
    }
    if let Some(missing) = listed.difference(&on_disk).next() {
        return Err(format!(
            "archive is missing `{missing}`, which MANIFEST.tsv lists"
        ));
    }
    let mut texts: BTreeMap<String, String> = BTreeMap::new();
    for (path, entry) in &entries {
        let bytes = fs::read(archive.join(path)).map_err(|error| error.to_string())?;
        if sha256_hex(&bytes) != entry.sha256 || bytes.len() != entry.size {
            return Err(format!(
                "archive file {path} does not match its MANIFEST.tsv checksum or size"
            ));
        }
        let text =
            String::from_utf8(bytes).map_err(|_| format!("archive file {path} is not text"))?;
        texts.insert(path.clone(), text);
        check_mode(&archive, path, &entry.kind)?;
        let shape_ok = match entry.kind.as_str() {
            "css" => {
                path.starts_with("css/") && path.ends_with(".css") && entry.class != "documentation"
            }
            "surface" => path.starts_with("surface/") && entry.class == "documentation",
            "tool" => path.starts_with("consumer/") && entry.class == "public-preview",
            "doc" | "license" | "identity" => entry.class == "documentation",
            _ => false,
        };
        if !shape_ok {
            return Err(format!(
                "MANIFEST.tsv row {path} has a class or kind inconsistent with its location"
            ));
        }
    }
    for required in REQUIRED_DOCS
        .iter()
        .chain(SURFACES.iter().map(|(archive, _)| archive))
    {
        if !entries.contains_key(*required) {
            return Err(format!("archive lacks required {required}"));
        }
    }

    // License and identity consistency across archive contents.
    check_license_text(&texts["LICENSE"])?;
    let rehearsal_banner = "REHEARSAL BUILD";
    for document in ["README.md", "RELEASE.md"] {
        let text = &texts[document];
        if !text.contains(&version) || !text.contains(&commit) || !text.contains(LICENSE_ID) {
            return Err(format!(
                "{document} must state the identity version, source commit, and license"
            ));
        }
        if text.contains(rehearsal_banner) != (kind == "rehearsal") {
            return Err(format!(
                "{document} rehearsal marking disagrees with the identity"
            ));
        }
    }
    for (path, text) in &texts {
        if let Some(placeholder) = find_placeholder(text) {
            return Err(format!(
                "{path} holds the unreplaced placeholder {placeholder}"
            ));
        }
    }

    // CSS: packaged import graph, escapes, and Tailwind confinement.
    let css_files: Vec<&String> = entries
        .keys()
        .filter(|path| path.starts_with("css/"))
        .collect();
    let mut normalized: BTreeMap<&str, String> = BTreeMap::new();
    for path in &css_files {
        normalized.insert(path.as_str(), lexical::css_normalize(&texts[*path]));
    }
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<String> = ["css/core.css", "css/tailwind.css"]
        .into_iter()
        .filter(|path| entries.contains_key(*path))
        .map(str::to_owned)
        .collect();
    if !entries.contains_key("css/core.css") {
        return Err("archive has no css/core.css, the browser-ready entrypoint".to_owned());
    }
    while let Some(file) = pending.pop() {
        if !reached.insert(file.clone()) {
            continue;
        }
        let text = &normalized[file.as_str()];
        if lexical::has_important(text) {
            return Err(format!("{file} contains !important"));
        }
        for marker in ["url(", "@charset"] {
            if text.contains(marker) {
                return Err(format!(
                    "{file} contains `{marker}`, which the packaged form does not use"
                ));
            }
        }
        for target in imports(text) {
            let package_import = file == "css/tailwind.css"
                && (target == "tailwindcss/theme" || target == "tailwindcss/utilities");
            if package_import {
                continue;
            }
            let Some(relative) = target.strip_prefix("./") else {
                return Err(format!(
                    "{file} imports `{target}`; packaged imports must be quoted ./ paths inside the archive"
                ));
            };
            if relative.contains("..") || relative.starts_with('/') || relative.contains("//") {
                return Err(format!(
                    "{file} imports `{target}`, which leaves its directory"
                ));
            }
            let dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
            let resolved = format!("{dir}/{relative}");
            if !entries.contains_key(&resolved) {
                return Err(format!(
                    "{file} imports {target}, which resolves to {resolved}: not in the archive"
                ));
            }
            pending.push(resolved);
        }
    }
    if let Some(orphan) = css_files
        .iter()
        .find(|path| !reached.contains(path.as_str()))
    {
        return Err(format!(
            "{orphan} is not reached from css/core.css or css/tailwind.css"
        ));
    }
    for (path, text) in &normalized {
        if *path != "css/tailwind.css" {
            for marker in [
                "@theme",
                "@apply",
                "@utility",
                "@source",
                "@plugin",
                "tailwindcss",
            ] {
                if text.contains(marker) {
                    return Err(format!(
                        "{path} contains `{marker}`; the portable core is Tailwind-free"
                    ));
                }
            }
        }
    }
    if let Some(adapter) = normalized.get("css/tailwind.css") {
        for marker in ["--ds-ref-", "@source", "../"] {
            if adapter.contains(marker) {
                return Err(format!("css/tailwind.css contains `{marker}`"));
            }
        }
        if !imports(adapter)
            .first()
            .is_some_and(|first| first == "./core.css")
        {
            return Err("css/tailwind.css must import ./core.css first".to_owned());
        }
    }
    let core = &normalized["css/core.css"];
    let order = format!("@layer {LAYER_ORDER};");
    if !core.contains(&order) {
        return Err(format!("css/core.css must publish `{order}`"));
    }
    if !texts["docs/architecture/css-entrypoint.md"].contains(LAYER_ORDER) {
        return Err(
            "docs/architecture/css-entrypoint.md does not document the shipped layer order"
                .to_owned(),
        );
    }

    // Public surface: tokens, theme, hooks against the shipped CSS.
    let rows_tokens = tokens::parse_inventory(&texts["surface/tokens.tsv"])?;
    let mut declared = BTreeSet::new();
    for path in &css_files {
        if path.as_str() == "css/tailwind.css" {
            continue;
        }
        for declaration in tokens::declarations(path, &texts[*path])? {
            declared.insert(declaration.name);
        }
    }
    let inventoried: BTreeSet<String> = rows_tokens.iter().map(|row| row.name.clone()).collect();
    if let Some(name) = declared.difference(&inventoried).next() {
        return Err(format!(
            "shipped CSS declares {name}, which surface/tokens.tsv does not classify"
        ));
    }
    if let Some(name) = inventoried.difference(&declared).next() {
        return Err(format!(
            "surface/tokens.tsv lists {name}, which no shipped stylesheet declares"
        ));
    }
    let public_roles: BTreeSet<&str> = rows_tokens
        .iter()
        .filter(|row| row.class == "public-preview")
        .map(|row| row.name.as_str())
        .collect();
    if let Some(adapter) = normalized.get("css/tailwind.css") {
        for (index, _) in adapter.match_indices("var(--") {
            let name: String = adapter[index + 4..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if !public_roles.contains(name.as_str()) {
                return Err(format!(
                    "css/tailwind.css reads {name}, which is not a public-preview semantic role"
                ));
            }
        }
    }
    let layouts = layout::parse_manifest(&texts["surface/layouts.tsv"])?;
    let primitives = primitive::parse_manifest(&texts["surface/primitives.tsv"])?;
    let mut hooks = primitive::layout_hooks(&layouts);
    hooks.extend(primitives.hooks());
    let mut used = BTreeSet::new();
    for (path, text) in &normalized {
        if *path != "css/tailwind.css" {
            used.extend(hook_classes(text));
        }
    }
    if let Some(extra) = used.difference(&hooks).next() {
        return Err(format!(
            "shipped CSS selects `.{extra}`, which the surface tables do not promote"
        ));
    }
    if let Some(missing) = hooks.difference(&used).next() {
        return Err(format!(
            "surface tables promote `.{missing}`, which no shipped stylesheet styles"
        ));
    }
    let theme_manifest = theme::parse_manifest(&texts["surface/theme.tsv"])?;
    if !normalized["css/tokens/theme.css"].contains(&theme_manifest.attribute) {
        return Err(format!(
            "css/tokens/theme.css does not implement the {} hook",
            theme_manifest.attribute
        ));
    }

    // Documentation links and the browser-support gates.
    for (path, text) in &texts {
        if !path.ends_with(".md") {
            continue;
        }
        for link in markdown_links(text) {
            if link.starts_with("http://")
                || link.starts_with("https://")
                || link.starts_with('#')
                || link.starts_with("mailto:")
            {
                continue;
            }
            let target = link.split('#').next().unwrap_or("");
            let resolved = if let Some(absolute) = target.strip_prefix('/') {
                absolute.to_owned()
            } else {
                let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
                normalize_join(dir, target)
                    .ok_or_else(|| format!("{path} links {link} outside the archive"))?
            };
            if !entries.contains_key(&resolved) {
                return Err(format!("{path} links {link}, which is not in the archive"));
            }
        }
    }
    let support = &texts["docs/browser-support.md"];
    for (needle, reason) in SUPPORT_REQUIRED {
        if !support.contains(needle) {
            return Err(format!(
                "docs/browser-support.md lacks `{needle}`: {reason}"
            ));
        }
    }
    for (needle, reason) in SUPPORT_FORBIDDEN {
        if support.contains(needle) {
            return Err(format!(
                "docs/browser-support.md still says `{needle}`: {reason}"
            ));
        }
    }

    // Consumer script: no Rust, Node, or Tailwind.
    let script = &texts["consumer/ds-consumer.sh"];
    let code: String = script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for tool in FORBIDDEN_TOOLS {
        if code
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .any(|word| word == tool)
        {
            return Err(format!(
                "consumer/ds-consumer.sh runs or names `{tool}`; the consumer path is toolchain-free"
            ));
        }
    }

    // Privacy scan over every archived file.
    let mut scanned = 0;
    for (path, text) in &texts {
        let lower = text.to_ascii_lowercase();
        let decoded = path.ends_with(".css").then(|| lexical::css_normalize(text));
        for (marker, reason) in PRIVATE_MARKERS {
            let hit = lexical::contains_marker(&lower, marker)
                || decoded
                    .as_ref()
                    .is_some_and(|css| lexical::contains_marker(css, marker));
            if hit {
                return Err(format!("{path} contains `{marker}`: {reason}"));
            }
        }
        if path.starts_with("css/") && path != "css/tailwind.css" && lower.contains("../") {
            return Err(format!("{path} contains a parent-directory path"));
        }
        scanned += 1;
    }

    let mut summary = format!(
        "release archive design-system-{version} ({kind}): {} file(s) match MANIFEST.tsv checksums; {} stylesheet(s) reached through ./ imports inside the archive; {} semantic role(s) and {} reference value(s) match surface/tokens.tsv; {} hook(s) match the surface tables; browser-support gates R1-R4 present; consumer script toolchain-free; {scanned} file(s) privacy-scanned",
        entries.len(),
        css_files.len(),
        public_roles.len(),
        inventoried.len() - public_roles.len(),
        hooks.len(),
    );
    if let Some(source) = source {
        summary.push_str(&format!(
            "; {}",
            compare_source(source, &texts, &entries, &identity)?
        ));
    }
    Ok(summary)
}

fn find_placeholder(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let rest = &text[i + 1..];
            let end = rest
                .bytes()
                .position(|b| !(b.is_ascii_uppercase() || b == b'_'))
                .unwrap_or(rest.len());
            if end >= 3 && rest.as_bytes().get(end) == Some(&b'@') {
                return Some(format!("@{}@", &rest[..end]));
            }
        }
        i += 1;
    }
    None
}

fn normalize_join(dir: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

#[cfg(unix)]
fn check_mode(archive: &Path, path: &str, kind: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(archive.join(path))
        .map_err(|error| error.to_string())?
        .permissions()
        .mode();
    let executable = mode & 0o111 != 0;
    if executable != (kind == "tool") {
        return Err(format!(
            "archive file {path} has an unexpected executable bit"
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_mode(_: &Path, _: &str, _: &str) -> Result<(), String> {
    Ok(())
}

/// Each archived file is its inventory source, byte for byte, apart from the two
/// recorded transforms; the archive holds exactly the inventory plus its two
/// generated files.
fn compare_source(
    source: &Path,
    texts: &BTreeMap<String, String>,
    entries: &BTreeMap<String, Entry>,
    identity: &Table,
) -> Result<String, String> {
    let root: PathBuf = source
        .canonicalize()
        .map_err(|error| format!("resolve source root {}: {error}", source.display()))?;
    let inventory = read(&root.join("release/inventory.tsv"))?;
    let mut expected: BTreeSet<String> = BTreeSet::new();
    let notice = if identity["identity"] == "rehearsal" {
        "> **REHEARSAL BUILD. This is not a release. Do not tag, publish, or pin it.**"
    } else {
        ""
    };
    for row in rows(&inventory) {
        let [class, kind, path, from, transform] = row.as_slice() else {
            return Err("inventory row has the wrong number of fields".to_owned());
        };
        let entry = entries
            .get(*path)
            .ok_or_else(|| format!("archive lacks inventory path {path}"))?;
        if entry.class != *class || entry.kind != *kind {
            return Err(format!(
                "archive row {path} disagrees with the inventory class or kind"
            ));
        }
        let original = read(&root.join(from))?;
        let want = match *transform {
            "none" => original,
            "identity" => original
                .replace("@VERSION@", &identity["version"])
                .replace("@TAG@", &identity["tag"])
                .replace("@COMMIT@", &identity["source_commit"])
                .replace("@IDENTITY_KIND@", &identity["identity"])
                .replace("@REHEARSAL_NOTICE@", notice),
            "tailwind-core-import" => {
                if original.matches(CORE_IMPORT_SOURCE).count() != 1 {
                    return Err(format!(
                        "{from} must hold exactly one repository-relative core import"
                    ));
                }
                original.replace(CORE_IMPORT_SOURCE, CORE_IMPORT_ARCHIVE)
            }
            other => return Err(format!("unknown transform {other}")),
        };
        if texts[*path] != want {
            return Err(format!(
                "archive file {path} differs from {from} (transform {transform})"
            ));
        }
        expected.insert((*path).to_owned());
    }
    expected.insert(IDENTITY_FILE.to_owned());
    let actual: BTreeSet<String> = entries.keys().cloned().collect();
    if actual != expected {
        return Err("archive contents differ from the inventory".to_owned());
    }
    if identity["source_repository"].is_empty() {
        return Err("identity source_repository is empty".to_owned());
    }
    Ok(format!(
        "all {} inventoried file(s) equal their source (transforms: none, identity, tailwind-core-import only)",
        expected.len() - 1
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_preview_and_never_stable() {
        for ok in ["0.1.0-preview.1", "0.1.0-rehearsal.3", "0.2.0-preview.12"] {
            assert!(validate_version(ok).is_ok(), "{ok}");
        }
        for bad in [
            "1.0.0",
            "1.0.0-preview.1",
            "0.1.0",
            "0.1.0-rc.1",
            "0.1-preview.1",
            "0.1.0-preview.",
            "0.1.0-preview.x",
        ] {
            assert!(validate_version(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn license_text_must_be_mit() {
        let mit = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../LICENSE"))
            .expect("LICENSE");
        assert!(check_license_text(&mit).is_ok());
        assert!(check_license_text("Apache License").is_err());
        assert!(check_license_text(&mit.replace("Copyright (c)", "Copyleft")).is_err());
    }

    #[test]
    fn imports_read_quoted_and_url_forms() {
        let css = lexical::css_normalize(
            "@import \"./a.css\" layer(ds.a); @\\69mport url('./b.css'); @import 'c.css';",
        );
        assert_eq!(imports(&css), ["./a.css", "./b.css", "c.css"]);
    }

    #[test]
    fn hook_classes_read_names_not_custom_properties() {
        let hooks =
            hook_classes(":where(.ds-stack) > .ds-action-primary { color: var(--ds-color-text); }");
        assert_eq!(
            hooks,
            BTreeSet::from(["ds-stack".to_owned(), "ds-action-primary".to_owned()])
        );
    }

    #[test]
    fn placeholders_are_found() {
        assert_eq!(
            find_placeholder("a @VERSION@ b").as_deref(),
            Some("@VERSION@")
        );
        assert_eq!(find_placeholder("a@b.example and @media"), None);
    }

    #[test]
    fn links_resolve_relative_and_root_forms() {
        assert_eq!(
            normalize_join("docs/architecture", "../browser-support.md").as_deref(),
            Some("docs/browser-support.md")
        );
        assert_eq!(
            normalize_join("docs", "architecture/hooks.md").as_deref(),
            Some("docs/architecture/hooks.md")
        );
        assert_eq!(normalize_join("", "../x"), None);
        assert_eq!(
            markdown_links("see [a](b.md) and [c](https://x.example)\n```\n[d](e.md)\n```"),
            ["b.md", "https://x.example"]
        );
    }

    #[test]
    fn privacy_markers_are_specific() {
        let hit = |text: &str| {
            PRIVATE_MARKERS
                .iter()
                .any(|(marker, _)| lexical::contains_marker(&text.to_ascii_lowercase(), marker))
        };
        assert!(hit("see /Users/someone/project"));
        assert!(hit("registry=https://npm.pkg.github.com"));
        assert!(hit("token ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(!hit("input[type=password] and a design token"));
        assert!(!hit("the --ds-color-text token"));
    }
}
