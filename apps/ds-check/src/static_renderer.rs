//! Static-renderer consumer fixture checks. The fixture consumes a release-shaped
//! staged copy of the declared public exports through Hugo. This module fails
//! closed on private-path, unpublished-alias, internal-module, Tailwind, and
//! source-tree coupling, on a staged boundary that differs from the declared
//! export graph, and on any promoted hook category the rendered site does not
//! exercise. It does not teach the core checker any renderer syntax.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    base, css, lexical, parse_exports, relative_to, tracked_files, validate_relative_path,
};

const STAGE_DIR: &str = "stage/design-system";
const STAGE_MANIFEST: &str = "stage/MANIFEST.tsv";
const PUBLIC_DIR: &str = "public";
const PUBLISHED_PREFIX: &str = "design-system";
const CONSUMER_DIR: &str = "consumer";
const PIN_FILE: &str = "HUGO_VERSION";

/// Fixture files that are documentation or the negative probe itself. The
/// toolchain-free probe must name the forbidden tools to reject them.
const UNSCANNED: [&str; 3] = ["README.md", "verify-toolchain-free.sh", PIN_FILE];

/// Hugo mounts a fixture may declare: its own inputs and the staged boundary.
const ALLOWED_MOUNT_SOURCES: [&str; 5] = [
    "content",
    "layouts",
    "static",
    CONSUMER_DIR,
    "stage/design-system",
];

/// Substring markers no consumer input may contain, with the reason.
const FORBIDDEN_INPUT: [(&str, &str); 34] = [
    ("packages/styles", "private repository path"),
    ("adapters/", "private repository path"),
    ("build/src", "private workspace path"),
    ("build/bin", "private workspace path"),
    ("lg-workstreams", "private repository name"),
    ("../", "parent-directory path"),
    ("git@github.com", "private remote"),
    ("file://", "local file reference"),
    ("node_modules", "source-tree coupling"),
    ("target/", "source-tree build output"),
    ("@luna/", "unpublished alias"),
    ("@design-system/", "unpublished alias"),
    ("@ds/", "unpublished alias"),
    ("~/", "home-relative alias"),
    ("--ds-ref-", "internal reference token"),
    ("@import", "consumer input imports a stylesheet"),
    ("tailwind", "Tailwind coupling"),
    ("@apply", "Tailwind directive"),
    ("@theme", "Tailwind directive"),
    ("@source", "Tailwind directive"),
    ("@utility", "Tailwind directive"),
    ("@variant", "Tailwind directive"),
    ("@custom-variant", "Tailwind directive"),
    ("@plugin", "Tailwind directive"),
    ("@config", "Tailwind directive"),
    ("postcss", "CSS build-tool coupling"),
    ("<script", "JavaScript in the render path"),
    ("package.json", "Node package coupling"),
    ("npm ", "Node tooling"),
    ("npx ", "Node tooling"),
    ("pnpm", "Node tooling"),
    ("module.imports", "external Hugo module import"),
    ("resources.postcss", "CSS build-tool coupling"),
    ("js.build", "JavaScript build in the render path"),
];

/// Markers for the fixture's stage and build scripts, checked outside comments.
const FORBIDDEN_SCRIPT: [&str; 11] = [
    "tailwind",
    "npm ",
    "npx ",
    "pnpm",
    "yarn",
    "node ",
    "cargo",
    "rustc",
    "postcss",
    "packages/styles",
    "../",
];

pub fn run(
    root: &Path,
    exports_file: &Path,
    layouts: &Path,
    primitives: &Path,
    theme: &Path,
    base_inventory: &Path,
    fixture: &Path,
) -> Result<String, String> {
    for path in [
        exports_file,
        layouts,
        primitives,
        theme,
        base_inventory,
        fixture,
    ] {
        validate_relative_path(path)?;
    }
    let exports_text = read(&root.join(exports_file))?;
    let exports = parse_exports(&exports_text)?;

    let fixture_root = root.join(fixture);
    let tracked = tracked_files(root)?;
    let mut inputs = Vec::new();
    for file in tracked
        .iter()
        .filter(|file| file.starts_with(&fixture_root))
    {
        let relative = relative_to(&fixture_root, file).display().to_string();
        inputs.push((relative, read(file)?));
    }
    if inputs.is_empty() {
        return Err(format!(
            "no tracked fixture files under {}",
            fixture.display()
        ));
    }
    reject_committed_output(&inputs)?;
    reject_untracked_inputs(&fixture_root, &inputs)?;
    scan_inputs(&inputs)?;
    let pinned = validate_pin(&inputs)?;
    validate_mounts(&inputs)?;

    let staged = derive_staged_graph(root, &exports)?;
    let stage_files = validate_stage(&fixture_root, &staged)?;
    let sheets = allowed_sheets(&exports, &inputs);
    let pages = read_pages(&fixture_root.join(PUBLIC_DIR))?;
    validate_public_design_system(&fixture_root.join(PUBLIC_DIR), &staged)?;
    let generated = scan_generated(&fixture_root.join(PUBLIC_DIR))?;
    let linked = validate_pages(&pages, &sheets)?;

    let hooks = validate_hooks(
        &pages,
        &read(&root.join(layouts))?,
        &read(&root.join(primitives))?,
        &read(&root.join(theme))?,
    )?;
    let classless = validate_classless_base(&pages, &read(&root.join(base_inventory))?)?;
    validate_consumer_overrides(&inputs)?;

    Ok(format!(
        "static renderer fixture {}: hugo {pinned} pinned; {} staged file(s) match the declared \
export graph byte for byte; {} page(s) link only {} declared stylesheet(s); {hooks}; \
classless base covers {classless} owned subject(s) on one classless page; consumer semantic \
theme and cascade overrides present; {} consumer input(s) and {generated} generated file(s) free \
of private-path, unpublished-alias, internal-module, Tailwind, script, and source-tree coupling",
        fixture.display(),
        stage_files,
        pages.len(),
        linked,
        inputs
            .iter()
            .filter(|(name, _)| !UNSCANNED.contains(&name.as_str()))
            .count(),
    ))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn reject_committed_output(inputs: &[(String, String)]) -> Result<(), String> {
    for (name, _) in inputs {
        for generated in ["stage/", "public/", "resources/"] {
            if name.starts_with(generated) {
                return Err(format!(
                    "{name} is tracked; staged and rendered output must stay generated and ignored"
                ));
            }
        }
    }
    Ok(())
}

/// Reject any consumer input that names a private path, an unpublished alias,
/// an internal module, a Tailwind construct, or a script.
pub fn scan_inputs(inputs: &[(String, String)]) -> Result<(), String> {
    for (name, content) in inputs {
        if UNSCANNED.contains(&name.as_str()) {
            continue;
        }
        if name.ends_with(".sh") {
            scan_script(name, content)?;
            continue;
        }
        let lower = content.to_ascii_lowercase();
        scan_markers(name, &lower, &FORBIDDEN_INPUT)?;
        if name.ends_with(".css") {
            // Escapes and comment separators must not hide a marker.
            scan_markers(name, &lexical::css_normalize(content), &FORBIDDEN_INPUT)?;
        }
        // The only Design System file a consumer may name is a declared export.
        for (index, _) in lower.match_indices(&format!("{PUBLISHED_PREFIX}/")) {
            let rest = &lower[index + PUBLISHED_PREFIX.len() + 1..];
            let target: String = rest
                .chars()
                .take_while(|c| !matches!(c, '"' | '\'' | ')' | '<' | '>') && !c.is_whitespace())
                .collect();
            if target != "core.css" {
                return Err(format!(
                    "{name} references `{PUBLISHED_PREFIX}/{target}`: only the declared core.css export may be named"
                ));
            }
        }
    }
    Ok(())
}

fn scan_markers(name: &str, text: &str, markers: &[(&str, &str)]) -> Result<(), String> {
    for (marker, reason) in markers {
        if text.contains(marker) {
            return Err(format!("{name} contains `{marker}`: {reason}"));
        }
    }
    Ok(())
}

fn scan_script(name: &str, content: &str) -> Result<(), String> {
    let code: String = content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    for marker in FORBIDDEN_SCRIPT {
        if code.contains(marker) {
            return Err(format!("{name} contains `{marker}` outside a comment"));
        }
    }
    Ok(())
}

/// Fixture directories that hold only generated output and are ignored by Git.
const GENERATED_ENTRIES: [&str; 4] = ["stage", "public", "resources", ".hugo_build.lock"];

/// The fixture's tracked files must be every file Hugo could read from it. A file
/// present on disk but untracked (or ignored) would be mounted and published
/// without the tracked-file scan ever seeing it.
fn reject_untracked_inputs(fixture_root: &Path, inputs: &[(String, String)]) -> Result<(), String> {
    let tracked: BTreeSet<&str> = inputs.iter().map(|(name, _)| name.as_str()).collect();
    let mut pending = vec![fixture_root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("read {}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            let relative = relative_to(fixture_root, &path).display().to_string();
            let top = relative.split('/').next().unwrap_or("");
            if GENERATED_ENTRIES.contains(&top) || relative.ends_with(".DS_Store") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if !tracked.contains(relative.as_str()) {
                return Err(format!(
                    "{relative} is in the fixture directory but not tracked by Git; the scan only reads tracked files, so track it or remove it"
                ));
            }
        }
    }
    Ok(())
}

/// File types a rendered fixture site may contain outside the byte-compared
/// design-system directory.
const GENERATED_EXTENSIONS: [&str; 3] = ["html", "css", "svg"];

/// Scan the rendered site with the same coupling markers as the inputs, and reject
/// any file type the fixture is not meant to publish. This is the fixture's
/// generated-artifact scan: input scanning alone cannot see what a mount or a
/// layout emits. A rendered page legitimately carries `../` (relative URLs), so
/// that one marker is not applied here; pages are separately confined to declared
/// stylesheets by `validate_pages`.
fn scan_generated(public: &Path) -> Result<usize, String> {
    let mut scanned = 0;
    for file in list_files(public)? {
        if file.starts_with(&format!("{PUBLISHED_PREFIX}/")) {
            continue;
        }
        let extension = file.rsplit_once('.').map_or("", |(_, extension)| extension);
        if !GENERATED_EXTENSIONS.contains(&extension) {
            return Err(format!(
                "rendered output holds `{file}`, a file type the fixture does not publish"
            ));
        }
        let content = read(&public.join(&file))?;
        let lower = content.to_ascii_lowercase();
        let markers: Vec<(&str, &str)> = FORBIDDEN_INPUT
            .iter()
            .copied()
            .filter(|(marker, _)| *marker != "../")
            .collect();
        scan_markers(&file, &lower, &markers)?;
        if extension == "css" {
            scan_markers(&file, &lexical::css_normalize(&content), &markers)?;
        }
        scanned += 1;
    }
    Ok(scanned)
}

fn validate_pin(inputs: &[(String, String)]) -> Result<String, String> {
    let pin = inputs
        .iter()
        .find(|(name, _)| name == PIN_FILE)
        .ok_or_else(|| format!("fixture has no {PIN_FILE} renderer pin"))?
        .1
        .trim()
        .to_owned();
    let parts: Vec<&str> = pin.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!(
            "{PIN_FILE} must pin one exact x.y.z version, found `{pin}`"
        ));
    }
    let build = inputs
        .iter()
        .find(|(name, _)| name == "build.sh")
        .ok_or("fixture has no build.sh")?;
    if !build.1.contains("\"$have\" = \"$want\"") {
        return Err(
            "build.sh must fail unless the installed Hugo equals the pinned version".to_owned(),
        );
    }
    Ok(pin)
}

fn validate_mounts(inputs: &[(String, String)]) -> Result<(), String> {
    let config = &inputs
        .iter()
        .find(|(name, _)| name == "hugo.toml")
        .ok_or("fixture has no hugo.toml")?
        .1;
    let sources = toml_string_values(config, "source");
    for value in &sources {
        if !ALLOWED_MOUNT_SOURCES.contains(&value.as_str()) {
            return Err(format!(
                "hugo.toml mounts `{value}`, which is not a fixture input"
            ));
        }
    }
    // Every mount must be accounted for as a `source` the check just verified: a
    // mount table or inline table without one, or a `mounts` key spelled some
    // other way, fails closed instead of publishing an unchecked directory.
    let compact: String = config
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(|line| {
            line.chars()
                .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
        })
        .collect();
    let headers = compact.matches("[[module.mounts]]").count();
    let inline_tables = compact.matches('{').count();
    if headers + inline_tables != sources.len() {
        return Err(
            "hugo.toml declares a mount this check cannot account for; use [[module.mounts]] tables with a `source` each"
                .to_owned(),
        );
    }
    if sources.is_empty() {
        return Err(
            "hugo.toml declares no mounts; the staged boundary would not be published".to_owned(),
        );
    }
    Ok(())
}

/// Every string assigned to `key` anywhere in a TOML document, whether the key is
/// bare, `"quoted"`, or `'quoted'`, at a line start or inside an inline table.
/// Comments are skipped; a value must be a basic or literal string.
fn toml_string_values(config: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    for line in config.lines() {
        let line = line.split('#').next().unwrap_or("");
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let boundary = i == 0 || matches!(chars[i - 1], '{' | ',' | ' ' | '\t');
            let quoted = matches!(chars[i], '"' | '\'');
            let start = if quoted { i + 1 } else { i };
            let name: String = chars[start..].iter().take(key.len()).collect();
            let after = start + key.len();
            let closes = if quoted {
                chars.get(after) == Some(&chars[i])
            } else {
                !chars
                    .get(after)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            };
            if boundary && name == key && closes {
                let mut j = if quoted { after + 1 } else { after };
                while chars.get(j).is_some_and(|c| c.is_whitespace()) {
                    j += 1;
                }
                if chars.get(j) == Some(&'=') {
                    j += 1;
                    while chars.get(j).is_some_and(|c| c.is_whitespace()) {
                        j += 1;
                    }
                    if let Some(&open) = chars.get(j).filter(|c| matches!(c, '"' | '\'')) {
                        let value: String =
                            chars[j + 1..].iter().take_while(|c| **c != open).collect();
                        j += value.chars().count() + 2;
                        values.push(value);
                        i = j;
                        continue;
                    }
                }
            }
            i += 1;
        }
    }
    values
}

/// One staged file: path relative to the staged export root, and its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Staged {
    pub(crate) relative: String,
    pub(crate) source: PathBuf,
}

pub(crate) fn derive_staged_graph(
    root: &Path,
    exports: &[crate::Export],
) -> Result<Vec<Staged>, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("resolve repository root: {error}"))?;
    let mut staged = BTreeMap::new();
    for export in exports {
        let graph = css::validate_graph(&root, &export.path, Path::new(crate::STYLES_ROOT))?;
        let entry = root
            .join(&export.path)
            .canonicalize()
            .map_err(|error| format!("resolve export {}: {error}", export.path.display()))?;
        let base_dir = entry
            .parent()
            .ok_or("export has no directory")?
            .to_path_buf();
        for sheet in graph.stylesheets {
            let relative = if sheet == entry {
                export.name.clone()
            } else {
                sheet
                    .strip_prefix(&base_dir)
                    .map_err(|_| format!("{} escapes its export directory", sheet.display()))?
                    .display()
                    .to_string()
            };
            if staged.insert(relative.clone(), sheet).is_some() {
                return Err(format!("two exports stage `{relative}`"));
            }
        }
    }
    Ok(staged
        .into_iter()
        .map(|(relative, source)| Staged { relative, source })
        .collect())
}

fn list_files(directory: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in
            fs::read_dir(&next).map_err(|error| format!("read {}: {error}", next.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(relative_to(directory, &path).display().to_string());
            }
        }
    }
    Ok(files)
}

fn validate_stage(fixture_root: &Path, staged: &[Staged]) -> Result<usize, String> {
    let directory = fixture_root.join(STAGE_DIR);
    if !directory.is_dir() {
        return Err(format!(
            "no staged boundary at {}; run stage.sh first",
            directory.display()
        ));
    }
    compare_tree(&directory, staged, "staged boundary")?;
    let manifest = read(&fixture_root.join(STAGE_MANIFEST))?;
    let mut recorded = BTreeMap::new();
    for line in manifest.lines().filter(|line| !line.is_empty()) {
        let (digest, name) = line
            .split_once('\t')
            .ok_or_else(|| format!("staged manifest line `{line}` is not <sha256><tab><path>"))?;
        recorded.insert(name.to_owned(), digest.to_owned());
    }
    for item in staged {
        let bytes = fs::read(&item.source).map_err(|error| error.to_string())?;
        let expected = sha256_hex(&bytes);
        match recorded.get(&item.relative) {
            Some(digest) if *digest == expected => {}
            Some(digest) => {
                return Err(format!(
                    "staged manifest digest for {} is {digest}, the declared export is {expected}",
                    item.relative
                ));
            }
            None => return Err(format!("staged manifest omits {}", item.relative)),
        }
    }
    if recorded.len() != staged.len() {
        return Err("staged manifest lists a file outside the declared export graph".to_owned());
    }
    Ok(staged.len())
}

/// The directory must hold exactly the staged files, each byte-identical to its
/// declared source: nothing missing, nothing extra, nothing edited.
fn compare_tree(directory: &Path, staged: &[Staged], label: &str) -> Result<(), String> {
    let present = list_files(directory)?;
    let expected: BTreeSet<String> = staged.iter().map(|item| item.relative.clone()).collect();
    if let Some(extra) = present.difference(&expected).next() {
        return Err(format!(
            "{label} holds `{extra}`, which no declared export publishes"
        ));
    }
    if let Some(missing) = expected.difference(&present).next() {
        return Err(format!(
            "{label} is missing `{missing}`, which a declared export publishes"
        ));
    }
    for item in staged {
        let staged_bytes =
            fs::read(directory.join(&item.relative)).map_err(|error| error.to_string())?;
        let source_bytes = fs::read(&item.source).map_err(|error| error.to_string())?;
        if staged_bytes != source_bytes {
            return Err(format!(
                "{label} `{}` differs from the declared export source",
                item.relative
            ));
        }
    }
    Ok(())
}

fn validate_public_design_system(public: &Path, staged: &[Staged]) -> Result<(), String> {
    let directory = public.join(PUBLISHED_PREFIX);
    if !directory.is_dir() {
        return Err(format!(
            "rendered output has no {PUBLISHED_PREFIX}/ directory; run build.sh"
        ));
    }
    compare_tree(&directory, staged, "rendered design-system directory")
}

/// Stylesheets a rendered page may link, relative to the site root.
fn allowed_sheets(exports: &[crate::Export], inputs: &[(String, String)]) -> BTreeSet<String> {
    let mut allowed: BTreeSet<String> = exports
        .iter()
        .map(|export| format!("{PUBLISHED_PREFIX}/{}", export.name))
        .collect();
    for (name, _) in inputs {
        if let Some(sheet) = name.strip_prefix(&format!("{CONSUMER_DIR}/"))
            && sheet.ends_with(".css")
        {
            allowed.insert(name.clone());
        }
    }
    allowed
}

#[derive(Debug, Clone)]
struct Tag {
    name: String,
    attrs: Vec<(String, String)>,
}

impl Tag {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn classes(&self) -> Vec<&str> {
        self.attr("class")
            .map(|v| v.split_whitespace().collect())
            .unwrap_or_default()
    }
}

struct Page {
    /// Path relative to the site root, for example `article/index.html`.
    path: String,
    html: String,
    tags: Vec<Tag>,
}

fn read_pages(public: &Path) -> Result<Vec<Page>, String> {
    if !public.is_dir() {
        return Err(format!(
            "no rendered output at {}; run build.sh first",
            public.display()
        ));
    }
    let mut pages = Vec::new();
    for file in list_files(public)? {
        if file.ends_with(".html") {
            let html = read(&public.join(&file))?;
            let tags = parse_tags(&html);
            pages.push(Page {
                path: file,
                html,
                tags,
            });
        }
    }
    if pages.is_empty() {
        return Err("rendered output has no HTML page".to_owned());
    }
    Ok(pages)
}

fn parse_tags(html: &str) -> Vec<Tag> {
    let bytes = html.as_bytes();
    let mut tags = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if html[i..].starts_with("<!--") {
            i = html[i..].find("-->").map_or(bytes.len(), |end| i + end + 3);
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len()
            && !bytes[j].is_ascii_whitespace()
            && bytes[j] != b'>'
            && bytes[j] != b'/'
        {
            j += 1;
        }
        let name = html[start..j].to_ascii_lowercase();
        if name.is_empty() || name.starts_with('!') || name.starts_with('/') {
            i = html[i..].find('>').map_or(bytes.len(), |end| i + end + 1);
            continue;
        }
        let mut attrs = Vec::new();
        loop {
            while j < bytes.len() && (bytes[j].is_ascii_whitespace() || bytes[j] == b'/') {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] == b'>' {
                break;
            }
            let key_start = j;
            while j < bytes.len()
                && !bytes[j].is_ascii_whitespace()
                && !matches!(bytes[j], b'=' | b'>' | b'/')
            {
                j += 1;
            }
            let key = html[key_start..j].to_ascii_lowercase();
            let mut value = String::new();
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                    let quote = bytes[j];
                    j += 1;
                    let value_start = j;
                    while j < bytes.len() && bytes[j] != quote {
                        j += 1;
                    }
                    value = html[value_start..j].to_owned();
                    j += 1;
                } else {
                    let value_start = j;
                    while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b'>' {
                        j += 1;
                    }
                    value = html[value_start..j].to_owned();
                }
            }
            if !key.is_empty() {
                attrs.push((key, value));
            }
        }
        tags.push(Tag { name, attrs });
        i = j;
    }
    tags
}

/// Resolve a page-relative reference to a site-root-relative path.
fn resolve(page: &str, reference: &str) -> Option<String> {
    let reference = reference.split(['#', '?']).next().unwrap_or("");
    let mut parts: Vec<&str> = page.split('/').collect();
    parts.pop();
    for segment in reference.split('/') {
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

fn validate_pages(pages: &[Page], allowed: &BTreeSet<String>) -> Result<usize, String> {
    let mut linked = BTreeSet::new();
    for page in pages {
        if let Some(inert) = page
            .tags
            .iter()
            .find(|tag| matches!(tag.name.as_str(), "script" | "template" | "noscript"))
        {
            return Err(format!(
                "{} contains a {} element; its contents are not ordinary page content",
                page.path, inert.name
            ));
        }
        let mut sheets = Vec::new();
        for tag in &page.tags {
            if tag.name == "link" && tag.attr("rel") == Some("stylesheet") {
                let href = tag
                    .attr("href")
                    .ok_or_else(|| format!("{} has a stylesheet link without href", page.path))?;
                let target = resolve(&page.path, href)
                    .ok_or_else(|| format!("{} links outside the site: {href}", page.path))?;
                if !allowed.contains(&target) {
                    return Err(format!(
                        "{} links `{target}`, which is not a declared export or consumer stylesheet",
                        page.path
                    ));
                }
                sheets.push(target);
            }
            for attribute in ["href", "src", "action"] {
                if let Some(value) = tag.attr(attribute)
                    && (value.starts_with("http:")
                        || value.starts_with("https:")
                        || value.starts_with("//"))
                {
                    return Err(format!("{} references the network: {value}", page.path));
                }
            }
        }
        let core = allowed
            .iter()
            .find(|sheet| sheet.starts_with(PUBLISHED_PREFIX))
            .cloned();
        if sheets.first() != core.as_ref() {
            return Err(format!(
                "{} must link the declared core export first, found {sheets:?}",
                page.path
            ));
        }
        linked.extend(sheets);
    }
    Ok(linked.len())
}

struct Row {
    class: String,
    name: String,
    kind: String,
}

fn rows(text: &str) -> Vec<Row> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some(Row {
                class: fields.next()?.to_owned(),
                name: fields.next()?.to_owned(),
                kind: fields.next()?.to_owned(),
            })
        })
        .collect()
}

fn validate_hooks(
    pages: &[Page],
    layouts: &str,
    primitives: &str,
    theme: &str,
) -> Result<String, String> {
    let all: Vec<&Tag> = pages.iter().flat_map(|page| page.tags.iter()).collect();
    let has_class = |class: &str| all.iter().any(|tag| tag.classes().contains(&class));
    let mut covered = 0;

    for row in rows(layouts)
        .iter()
        .filter(|row| row.class == "public-preview")
    {
        let hook = format!("ds-{}", row.name);
        if !has_class(&hook) {
            return Err(format!(
                "no rendered element carries the promoted layout hook .{hook}"
            ));
        }
        covered += 1;
    }

    let promoted: Vec<Row> = rows(primitives)
        .into_iter()
        .filter(|row| row.class == "public-preview")
        .collect();
    for row in &promoted {
        let hook = format!("ds-{}", row.name);
        match row.kind.as_str() {
            "base-primitive" | "ui-primitive" => {
                if !has_class(&hook) {
                    return Err(format!(
                        "no rendered element carries the promoted primitive hook .{hook}"
                    ));
                }
            }
            "variant" => {
                let owner = promoted
                    .iter()
                    .filter(|other| other.kind == "ui-primitive")
                    .find(|other| row.name.starts_with(&format!("{}-", other.name)))
                    .ok_or_else(|| format!("variant {} has no promoted primitive", row.name))?;
                let owner_hook = format!("ds-{}", owner.name);
                if !all.iter().any(|tag| {
                    let classes = tag.classes();
                    classes.contains(&hook.as_str()) && classes.contains(&owner_hook.as_str())
                }) {
                    return Err(format!(
                        "no rendered element combines .{owner_hook} with .{hook}"
                    ));
                }
            }
            _ => {}
        }
        if row.kind == "state" {
            let (owner, state) = row.name.split_once('-').ok_or("malformed state row")?;
            let owner_hook = format!("ds-{owner}");
            let candidates = all
                .iter()
                .filter(|tag| tag.classes().contains(&owner_hook.as_str()));
            let found = match state {
                // :hover has no markup; the browser suite exercises it.
                "hover" => candidates.clone().next().is_some(),
                "current" => candidates.clone().any(|tag| {
                    tag.attr("aria-current")
                        .is_some_and(|v| !v.is_empty() && !v.eq_ignore_ascii_case("false"))
                }),
                "disabled" => candidates
                    .clone()
                    .any(|tag| tag.name == "button" && tag.attr("disabled").is_some()),
                "unlinked" => candidates
                    .clone()
                    .any(|tag| tag.name == "a" && tag.attr("href").is_none()),
                other => {
                    return Err(format!(
                        "primitives.tsv promotes unknown state `{other}`; extend the fixture and this check"
                    ));
                }
            };
            if !found {
                return Err(format!(
                    "no rendered element exercises the promoted state {}",
                    row.name
                ));
            }
            if state == "current"
                && !candidates.clone().any(|tag| {
                    tag.attr("aria-current")
                        .is_some_and(|v| v.eq_ignore_ascii_case("false"))
                })
            {
                return Err(
                    "no rendered element shows aria-current=\"false\" beside the current state"
                        .to_owned(),
                );
            }
        }
        covered += 1;
    }

    let roots: Vec<&Tag> = pages
        .iter()
        .filter_map(|page| page.tags.iter().find(|tag| tag.name == "html"))
        .collect();
    let mut schemes = BTreeSet::new();
    let mut attribute_rows = 0;
    for line in theme
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() == 4 && fields[0] == "public-preview" && fields[1] == "attribute" {
            attribute_rows += 1;
            if !roots.iter().any(|tag| tag.attr(fields[2]).is_none()) {
                return Err(
                    "every page sets a scheme attribute; the default theme is unexercised"
                        .to_owned(),
                );
            }
            for value in fields[3].split_whitespace() {
                if !roots.iter().any(|tag| tag.attr(fields[2]) == Some(value)) {
                    return Err(format!("no page sets {}=\"{value}\" on <html>", fields[2]));
                }
                schemes.insert(value.to_owned());
            }
            covered += 1;
        }
    }
    if attribute_rows == 0 {
        return Err("theme.tsv promotes no attribute hook to exercise".to_owned());
    }
    Ok(format!(
        "{covered} promoted hook row(s) exercised (layouts, surface, action, variants, states, default and {} explicit scheme value(s))",
        schemes.len()
    ))
}

/// The classless base is proven on one page that carries no class attribute at
/// all, using the same coverage rule as the plain-HTML fixture.
fn validate_classless_base(pages: &[Page], inventory: &str) -> Result<usize, String> {
    let manifest = base::parse_manifest(inventory)?;
    let mut last = String::new();
    for page in pages
        .iter()
        .filter(|page| page.tags.iter().all(|tag| tag.attr("class").is_none()))
    {
        match base::validate_fixture(&page.html, &manifest) {
            Ok(covered) => return Ok(covered),
            Err(error) => last = format!("{}: {error}", page.path),
        }
    }
    if last.is_empty() {
        return Err(
            "no rendered page is classless, so the base is unproven on ordinary HTML".to_owned(),
        );
    }
    Err(last)
}

fn validate_consumer_overrides(inputs: &[(String, String)]) -> Result<(), String> {
    let css: Vec<&(String, String)> = inputs
        .iter()
        .filter(|(name, _)| name.starts_with(&format!("{CONSUMER_DIR}/")) && name.ends_with(".css"))
        .collect();
    if css
        .iter()
        .any(|(_, source)| lexical::has_important(&lexical::css_normalize(source)))
    {
        return Err(
            "consumer stylesheets must override through layer order, not !important".to_owned(),
        );
    }
    let theme = css
        .iter()
        .find(|(name, _)| name.ends_with("theme.css"))
        .ok_or("fixture has no consumer semantic theme stylesheet")?;
    if !theme.1.contains("--ds-color-") || !theme.1.contains("@layer app") {
        return Err(
            "consumer theme must assign public --ds-color-* roles inside `@layer app`".to_owned(),
        );
    }
    let site = css
        .iter()
        .find(|(name, _)| name.ends_with("site.css"))
        .ok_or("fixture has no consumer cascade stylesheet")?;
    if !site.1.contains(".app-pill") || !site.1.contains("@layer app") {
        return Err(
            "consumer cascade stylesheet must override a Design System hook inside `@layer app`"
                .to_owned(),
        );
    }
    Ok(())
}

/// SHA-256 of `bytes` as lowercase hex, so staged identity needs no external tool.
pub fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((bytes.len() as u64) * 8).to_be_bytes());
    for block in message.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = state;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let choice = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let majority = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(majority);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (slot, value) in state.iter_mut().zip(v) {
            *slot = slot.wrapping_add(value);
        }
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, content: &str) -> Vec<(String, String)> {
        vec![(name.to_owned(), content.to_owned())]
    }

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn rejects_each_coupling_class_in_consumer_inputs() {
        for (content, class) in [
            ("@import \"packages/styles/index.css\";", "private path"),
            ("<link href=\"packages/styles/index.css\">", "private path"),
            (
                "a { src: url(adapters/tailwind/index.css); }",
                "private path",
            ),
            ("<link href=\"@luna/core.css\">", "unpublished alias"),
            (
                "<link href=\"design-system/tokens/reference.css\">",
                "internal module",
            ),
            ("<link href=\"design-system/base.css\">", "internal module"),
            (":root { --x: var(--ds-ref-blue-500); }", "internal token"),
            ("@import \"tailwindcss\";", "tailwind"),
            (".a { @apply px-4; }", "tailwind directive"),
            ("@utility x { color: red; }", "tailwind directive"),
            ("<script src=\"app.js\"></script>", "script"),
            ("<link href=\"../design-system/core.css\">", "parent path"),
            (
                "[[module.imports]]\npath = \"example.test/x\"",
                "module import",
            ),
        ] {
            assert!(
                scan_inputs(&input("layouts/baseof.html", content)).is_err(),
                "{class}: {content}"
            );
        }
    }

    #[test]
    fn accepts_the_declared_core_export_reference() {
        assert!(
            scan_inputs(&input(
                "layouts/baseof.html",
                "<link rel=\"stylesheet\" href=\"design-system/core.css\">"
            ))
            .is_ok()
        );
    }

    #[test]
    fn scripts_are_scanned_outside_comments_only() {
        assert!(scan_inputs(&input("build.sh", "# no tailwind or cargo here\nhugo\n")).is_ok());
        assert!(scan_inputs(&input("build.sh", "npm ci\n")).is_err());
        assert!(scan_inputs(&input("stage.sh", "cp packages/styles/index.css out\n")).is_err());
    }

    #[test]
    fn readme_and_the_negative_probe_are_the_only_unscanned_files() {
        assert!(
            scan_inputs(&input(
                "README.md",
                "Tailwind, Node, and cargo are not required."
            ))
            .is_ok()
        );
        assert!(scan_inputs(&input("verify-toolchain-free.sh", "cargo node")).is_ok());
        assert!(scan_inputs(&input("notes.md", "Tailwind")).is_err());
    }

    #[test]
    fn requires_an_exact_renderer_pin() {
        let build = (
            "build.sh".to_owned(),
            "[ \"$have\" = \"$want\" ]".to_owned(),
        );
        for pin in ["0.166", "v0.166.0", "0.166.x", ">=0.166.0", ""] {
            let inputs = vec![(PIN_FILE.to_owned(), pin.to_owned()), build.clone()];
            assert!(validate_pin(&inputs).is_err(), "{pin}");
        }
        let inputs = vec![(PIN_FILE.to_owned(), "0.166.0\n".to_owned()), build];
        assert_eq!(validate_pin(&inputs).as_deref(), Ok("0.166.0"));
    }

    #[test]
    fn rejects_a_mount_outside_the_fixture() {
        let config = "[[module.mounts]]\nsource = \"packages/styles\"\ntarget = \"static\"\n";
        assert!(validate_mounts(&input("hugo.toml", config)).is_err());
        let ok = "[[module.mounts]]\nsource = \"stage/design-system\"\ntarget = \"static/design-system\"\n";
        assert!(validate_mounts(&input("hugo.toml", ok)).is_ok());
    }

    #[test]
    fn rejects_committed_generated_output() {
        assert!(reject_committed_output(&input("stage/design-system/core.css", "")).is_err());
        assert!(reject_committed_output(&input("public/index.html", "")).is_err());
    }

    #[test]
    fn resolves_page_relative_references() {
        assert_eq!(
            resolve("article/index.html", "../design-system/core.css").as_deref(),
            Some("design-system/core.css")
        );
        assert_eq!(
            resolve("index.html", "./consumer/theme.css").as_deref(),
            Some("consumer/theme.css")
        );
        assert_eq!(resolve("index.html", "../escape.css"), None);
    }

    fn page(path: &str, html: &str) -> Page {
        Page {
            path: path.to_owned(),
            html: html.to_owned(),
            tags: parse_tags(html),
        }
    }

    fn allowed() -> BTreeSet<String> {
        ["design-system/core.css", "consumer/theme.css"]
            .map(str::to_owned)
            .into()
    }

    #[test]
    fn rejects_undeclared_stylesheets_scripts_and_network_references() {
        let core = "<link rel=\"stylesheet\" href=\"./design-system/core.css\">";
        assert!(validate_pages(&[page("index.html", core)], &allowed()).is_ok());
        for html in [
            "<link rel=\"stylesheet\" href=\"./design-system/tokens.css\">".to_owned(),
            format!(
                "{core}<link rel=\"stylesheet\" href=\"./design-system/tokens/reference.css\">"
            ),
            format!("{core}<script></script>"),
            format!("{core}<img src=\"https://example.test/x.png\">"),
            "<link rel=\"stylesheet\" href=\"./consumer/theme.css\">".to_owned(),
        ] {
            assert!(
                validate_pages(&[page("index.html", &html)], &allowed()).is_err(),
                "{html}"
            );
        }
    }

    #[test]
    fn hook_coverage_fails_when_a_promoted_hook_is_absent() {
        let layouts = "public-preview\tstack\t.ds-stack\npublic-preview\tgrid\t.ds-grid\n";
        let primitives = "public-preview\tsurface\tbase-primitive\npublic-preview\taction\tui-primitive\npublic-preview\taction-primary\tvariant\npublic-preview\taction-current\tstate\npublic-preview\taction-disabled\tstate\npublic-preview\taction-unlinked\tstate\npublic-preview\taction-hover\tstate\n";
        let theme = "public-preview\tdefault\tcolor-scheme\tlight dark\npublic-preview\tattribute\tdata-ds-scheme\tlight dark\n";
        let full_body = "<div class=\"ds-stack\"></div><ul class=\"ds-grid\"></ul><div class=\"ds-surface\"></div>\
<button class=\"ds-action ds-action-primary\" disabled></button><a class=\"ds-action\"></a>\
<a class=\"ds-action\" href=\"#x\" aria-current=\"page\"></a><a class=\"ds-action\" href=\"#x\" aria-current=\"false\"></a>";
        let pages = |body: &str| {
            vec![
                page("index.html", &format!("<html>{body}")),
                page("a/index.html", "<html data-ds-scheme=\"light\">"),
                page("b/index.html", "<html data-ds-scheme=\"dark\">"),
            ]
        };
        assert!(validate_hooks(&pages(full_body), layouts, primitives, theme).is_ok());
        for (drop, reason) in [
            ("<div class=\"ds-stack\"></div>", "stack"),
            ("<ul class=\"ds-grid\"></ul>", "grid"),
            ("<div class=\"ds-surface\"></div>", "surface"),
            ("<a class=\"ds-action\"></a>", "unlinked"),
            (
                "<a class=\"ds-action\" href=\"#x\" aria-current=\"page\"></a>",
                "current",
            ),
            (
                "<a class=\"ds-action\" href=\"#x\" aria-current=\"false\"></a>",
                "non-current",
            ),
            (
                "<button class=\"ds-action ds-action-primary\" disabled></button>",
                "disabled and variant",
            ),
        ] {
            let body = full_body.replace(drop, "");
            assert!(
                validate_hooks(&pages(&body), layouts, primitives, theme).is_err(),
                "{reason}"
            );
        }
        // The default theme and each explicit scheme value need a page.
        let no_default = vec![
            page(
                "index.html",
                &format!("<html data-ds-scheme=\"light\">{full_body}"),
            ),
            page("b/index.html", "<html data-ds-scheme=\"dark\">"),
        ];
        assert!(validate_hooks(&no_default, layouts, primitives, theme).is_err());
        let no_dark = vec![
            page("index.html", &format!("<html>{full_body}")),
            page("a/index.html", "<html data-ds-scheme=\"light\">"),
        ];
        assert!(validate_hooks(&no_dark, layouts, primitives, theme).is_err());
    }

    #[test]
    fn tag_parser_reads_quoted_bare_and_boolean_attributes() {
        let tags = parse_tags("<a class=\"x y\" href=z disabled><!-- <b> --><img alt='q'/>");
        assert_eq!(tags[0].classes(), ["x", "y"]);
        assert_eq!(tags[0].attr("href"), Some("z"));
        assert_eq!(tags[0].attr("disabled"), Some(""));
        assert_eq!(tags[1].name, "img");
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn consumer_overrides_require_layer_and_forbid_important() {
        let theme = (
            "consumer/theme.css".to_owned(),
            "@layer app { :root { --ds-color-accent: red; } }".to_owned(),
        );
        let site = (
            "consumer/site.css".to_owned(),
            "@layer app { .app-pill { border-radius: 9px; } }".to_owned(),
        );
        assert!(validate_consumer_overrides(&[theme.clone(), site.clone()]).is_ok());
        let commented = (
            "consumer/site.css".to_owned(),
            "/* no !important */ @layer app { .app-pill { border-radius: 9px; } }".to_owned(),
        );
        assert!(validate_consumer_overrides(&[theme.clone(), commented]).is_ok());
        let loud = (
            "consumer/site.css".to_owned(),
            "@layer app { .app-pill { border-radius: 9px !important; } }".to_owned(),
        );
        assert!(validate_consumer_overrides(&[theme.clone(), loud]).is_err());
        assert!(validate_consumer_overrides(std::slice::from_ref(&site)).is_err());
        let unlayered = (
            "consumer/theme.css".to_owned(),
            ":root { --ds-color-accent: red; }".to_owned(),
        );
        assert!(validate_consumer_overrides(&[unlayered, site]).is_err());
    }

    #[test]
    fn rejects_every_toml_mount_spelling_outside_the_fixture() {
        let bad = "packages/styles";
        for config in [
            format!("[[module.mounts]]\nsource = \"{bad}\"\ntarget = \"static\"\n"),
            format!("[[module.mounts]]\n\"source\" = \"{bad}\"\ntarget = \"static\"\n"),
            format!("[[module.mounts]]\n'source' = '{bad}'\ntarget = \"static\"\n"),
            format!("[[module.mounts]]\nsource='{bad}'\ntarget='static'\n"),
            format!("[module]\nmounts = [{{ source = \"{bad}\", target = \"static\" }}]\n"),
            format!("[module]\nmounts = [{{ \"source\" = \"{bad}\", target = \"static\" }}]\n"),
            format!(
                "[[module.mounts]]\nsource = \"content\"\ntarget = \"content\"\n[[module.mounts]]\n\"source\" = \"{bad}\"\ntarget = \"static\"\n"
            ),
        ] {
            assert!(
                validate_mounts(&input("hugo.toml", &config)).is_err(),
                "{config}"
            );
        }
        for config in [
            "[[module.mounts]]\n\"source\" = \"stage/design-system\"\ntarget = \"static/design-system\"\n",
            "[module]\nmounts = [{ source = \"content\", target = \"content\" }, { source = \"layouts\", target = \"layouts\" }]\n",
            "[[module.\"mounts\"]]\nsource = \"static\"\ntarget = \"static\"\n",
        ] {
            assert!(
                validate_mounts(&input("hugo.toml", config)).is_ok(),
                "{config}"
            );
        }
    }

    #[test]
    fn a_mount_without_a_checked_source_fails_closed() {
        let config = "[[module.mounts]]\nsource = \"content\"\ntarget = \"content\"\n[[module.mounts]]\ntarget = \"static\"\n";
        assert!(validate_mounts(&input("hugo.toml", config)).is_err());
        assert!(validate_mounts(&input("hugo.toml", "[module]\n")).is_err());
    }

    #[test]
    fn escaped_css_spellings_are_rejected_in_consumer_inputs() {
        for css in [
            "@\\69mport \"x.css\";",
            "@\\000069 mport url(x.css);",
            ":root { --x: var(--ds-\\72 ef-blue-500); }",
            ":root { --x: var(--ds-\\ref-blue-500); }",
            "/* c */ @/**/import x;",
        ] {
            let result = scan_inputs(&input("consumer/x.css", css));
            // The comment-split form is not an import token; the rest are.
            if css.contains("/**/") {
                assert!(result.is_ok(), "{css}");
            } else {
                assert!(result.is_err(), "{css}");
            }
        }
    }

    #[test]
    fn spaced_and_escaped_important_is_rejected() {
        let theme = (
            "consumer/theme.css".to_owned(),
            "@layer app { :root { --ds-color-accent: red; } }".to_owned(),
        );
        for declaration in [
            "border-radius: 9px ! important;",
            "border-radius: 9px !\\69mportant;",
            "border-radius: 9px !/**/important;",
            "border-radius: 9px!IMPORTANT;",
        ] {
            let site = (
                "consumer/site.css".to_owned(),
                format!("@layer app {{ .app-pill {{ {declaration} }} }}"),
            );
            assert!(
                validate_consumer_overrides(&[theme.clone(), site]).is_err(),
                "{declaration}"
            );
        }
    }

    #[test]
    fn generated_output_is_scanned_and_type_restricted() {
        let directory =
            std::env::temp_dir().join(format!("ds-check-generated-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("design-system")).unwrap();
        fs::write(
            directory.join("index.html"),
            "<a href=\"../design-system/core.css\">",
        )
        .unwrap();
        fs::write(
            directory.join("design-system/core.css"),
            "@import \"./x.css\";",
        )
        .unwrap();
        assert_eq!(scan_generated(&directory), Ok(1));
        fs::write(directory.join("app.js"), "1").unwrap();
        assert!(scan_generated(&directory).is_err());
        fs::remove_file(directory.join("app.js")).unwrap();
        fs::write(directory.join("x.css"), "@\\69mport 'a';").unwrap();
        assert!(scan_generated(&directory).is_err());
        fs::remove_file(directory.join("x.css")).unwrap();
        fs::write(directory.join("y.html"), "<p>packages/styles</p>").unwrap();
        assert!(scan_generated(&directory).is_err());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn untracked_fixture_files_are_rejected() {
        let directory =
            std::env::temp_dir().join(format!("ds-check-untracked-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("static")).unwrap();
        fs::create_dir_all(directory.join("public")).unwrap();
        fs::write(directory.join("static/a.svg"), "").unwrap();
        fs::write(directory.join("public/out.html"), "").unwrap();
        let tracked = vec![("static/a.svg".to_owned(), String::new())];
        assert!(reject_untracked_inputs(&directory, &tracked).is_ok());
        fs::write(directory.join("static/hidden.js"), "").unwrap();
        assert!(reject_untracked_inputs(&directory, &tracked).is_err());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn rendered_pages_reject_inert_containers() {
        let core = "<link rel=\"stylesheet\" href=\"./design-system/core.css\">";
        for extra in ["<template></template>", "<noscript></noscript>"] {
            let html = format!("{core}{extra}");
            assert!(
                validate_pages(&[page("index.html", &html)], &allowed()).is_err(),
                "{extra}"
            );
        }
    }
}
