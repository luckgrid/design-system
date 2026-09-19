//! Fail-closed structural scan of Design System stylesheets for the cascade
//! layer-ownership contract.
//!
//! This is not a general CSS parser. It recognizes only the structure the layer
//! contract depends on (`@import`, `@layer`, group rules, style rules, and
//! comments/strings) and rejects anything it cannot classify rather than guessing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Group at-rules whose bodies contain rules and are therefore descended into.
const GROUP_AT_RULES: [&str; 6] = [
    "layer",
    "media",
    "supports",
    "container",
    "scope",
    "starting-style",
];

/// At-rules whose bodies hold declarations or keyframes and are not descended.
const OPAQUE_AT_RULES: [&str; 6] = [
    "font-face",
    "property",
    "keyframes",
    "counter-style",
    "page",
    "font-palette-values",
];

/// Tailwind directives and functions. The portable core must be correct as
/// plain browser CSS, so any of these fails validation.
const TAILWIND_AT_RULES: [&str; 11] = [
    "tailwind",
    "theme",
    "utility",
    "variant",
    "custom-variant",
    "apply",
    "source",
    "plugin",
    "config",
    "reference",
    "slot",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// `@name prelude;`
    Statement { name: String, prelude: String },
    /// `@name prelude { rules }` for a group rule that is descended into.
    Group {
        name: String,
        prelude: String,
        children: Vec<Node>,
    },
    /// `@name prelude { declarations }` that is not descended into.
    Opaque { name: String, body: String },
    /// `selector { declarations }`
    Style { prelude: String, body: String },
}

/// What a validated stylesheet graph contains.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Graph {
    /// Every stylesheet reached from the entry, each exactly once.
    pub stylesheets: BTreeSet<PathBuf>,
    /// Full layer name to the one stylesheet that owns it.
    pub owners: BTreeMap<String, PathBuf>,
}

/// Parse stylesheet text into top-level nodes.
pub fn parse(source: &str) -> Result<Vec<Node>, String> {
    let text = strip_comments(source)?;
    let chars: Vec<char> = text.chars().collect();
    parse_nodes(&chars)
}

/// Validate the stylesheet graph reached from `entry`.
///
/// `root` is the canonical repository root; `entry` and `styles_root` are
/// repository-relative. Imports must stay inside `styles_root`, and each
/// stylesheet may enter the graph only once.
pub fn validate_graph(root: &Path, entry: &Path, styles_root: &Path) -> Result<Graph, String> {
    let styles_dir = root
        .join(styles_root)
        .canonicalize()
        .map_err(|error| format!("resolve styles root {}: {error}", styles_root.display()))?;
    let entry_path = root
        .join(entry)
        .canonicalize()
        .map_err(|error| format!("resolve stylesheet {}: {error}", entry.display()))?;
    if !entry_path.starts_with(&styles_dir) {
        return Err(format!(
            "stylesheet {} is outside its styles root {}",
            entry.display(),
            styles_root.display()
        ));
    }

    let mut graph = Graph::default();
    let mut pending = vec![(entry_path, None::<String>)];
    while let Some((file, owner)) = pending.pop() {
        let imports = visit(root, &styles_dir, &file, owner, &mut graph)?;
        pending.extend(imports);
    }

    Ok(graph)
}

/// Every tracked stylesheet beneath `styles_root` must be reached from some
/// declared entrypoint; an orphaned stylesheet has no defined layer placement.
pub fn validate_reachability(
    root: &Path,
    styles_root: &Path,
    reached: &BTreeSet<PathBuf>,
    tracked: &[PathBuf],
) -> Result<(), String> {
    let styles_dir = root
        .join(styles_root)
        .canonicalize()
        .map_err(|error| format!("resolve styles root {}: {error}", styles_root.display()))?;
    for file in tracked {
        let is_css = file.extension().is_some_and(|extension| extension == "css");
        if is_css && file.starts_with(&styles_dir) && !reached.contains(file) {
            return Err(format!(
                "stylesheet {} is not reached from any declared entrypoint; every source stylesheet must enter the cascade through one",
                display(root, file)
            ));
        }
    }
    Ok(())
}

/// Validate one stylesheet and return the imports it contributes, each paired
/// with the full layer name that the import supplies (if any).
fn visit(
    root: &Path,
    styles_dir: &Path,
    file: &Path,
    owner: Option<String>,
    graph: &mut Graph,
) -> Result<Vec<(PathBuf, Option<String>)>, String> {
    let shown = display(root, file);
    if !graph.stylesheets.insert(file.to_path_buf()) {
        return Err(format!(
            "stylesheet {shown} enters the cascade more than once; import each stylesheet exactly once"
        ));
    }

    let source =
        fs::read_to_string(file).map_err(|error| format!("read stylesheet {shown}: {error}"))?;
    let nodes = parse(&source).map_err(|error| format!("stylesheet {shown}: {error}"))?;

    if let Some(parent) = &owner {
        claim(graph, parent, file, root)?;
    }

    let context = Context {
        file: &shown,
        owner: owner.as_deref(),
        layered: owner.is_some(),
        top_level: true,
    };

    let mut imports = Vec::new();
    let mut preamble = true;
    for node in &nodes {
        let is_preamble = matches!(
            node,
            Node::Statement { name, .. } if name == "import" || name == "layer" || name == "charset"
        );
        if !is_preamble {
            preamble = false;
        }

        if let Node::Statement { name, prelude } = node
            && name == "import"
        {
            if !preamble {
                return Err(format!(
                    "stylesheet {shown} has @import after other rules; imports must come first"
                ));
            }
            let (target, layer) = parse_import(&shown, prelude)?;
            let resolved = resolve_import(file, styles_dir, &shown, &target)?;
            let child_owner = match layer {
                Some(name) => {
                    check_layer_name(&context, &name)?;
                    Some(match &owner {
                        Some(parent) => format!("{parent}.{name}"),
                        None => name,
                    })
                }
                None => owner.clone(),
            };
            imports.push((resolved, child_owner));
            continue;
        }

        check_node(&context, node, graph, file, root)?;
    }

    Ok(imports)
}

#[derive(Clone, Copy)]
struct Context<'a> {
    file: &'a str,
    /// Full layer name supplied by the import that placed this stylesheet.
    owner: Option<&'a str>,
    /// Whether rules at this point are inside some named layer.
    layered: bool,
    /// Whether this node is at the stylesheet's top level.
    top_level: bool,
}

fn check_node(
    context: &Context<'_>,
    node: &Node,
    graph: &mut Graph,
    file: &Path,
    root: &Path,
) -> Result<(), String> {
    let file_name = context.file;
    match node {
        Node::Statement { name, prelude } => {
            reject_tailwind(file_name, name)?;
            match name.as_str() {
                "charset" => Ok(()),
                "layer" => {
                    let names: Vec<&str> = prelude.split(',').map(str::trim).collect();
                    if names.iter().any(|name| name.is_empty()) {
                        return Err(format!(
                            "stylesheet {file_name} declares an empty or anonymous layer name"
                        ));
                    }
                    for name in names {
                        check_layer_name(context, name)?;
                    }
                    Ok(())
                }
                "import" => Err(format!(
                    "stylesheet {file_name} has @import inside a nested rule"
                )),
                other => Err(format!(
                    "stylesheet {file_name} uses unsupported at-rule statement @{other}"
                )),
            }
        }
        Node::Group {
            name,
            prelude,
            children,
        } => {
            reject_tailwind(file_name, name)?;
            let mut inner = Context {
                top_level: false,
                ..*context
            };
            if name == "layer" {
                let layer = prelude.trim();
                if layer.is_empty() {
                    return Err(format!(
                        "stylesheet {file_name} contains an anonymous @layer block; every layer must be named"
                    ));
                }
                if layer.contains(',') {
                    return Err(format!(
                        "stylesheet {file_name} names more than one layer on an @layer block"
                    ));
                }
                check_layer_name(context, layer)?;
                if context.owner.is_none() && context.top_level {
                    claim(graph, layer, file, root)?;
                }
                inner.layered = true;
            } else if !context.layered {
                return Err(format!(
                    "stylesheet {file_name} has an unlayered @{name} rule; place Design System rules in a named layer"
                ));
            }
            for child in children {
                check_node(&inner, child, graph, file, root)?;
            }
            Ok(())
        }
        Node::Opaque { name, body } => {
            reject_tailwind(file_name, name)?;
            if !OPAQUE_AT_RULES.contains(&name.as_str()) {
                return Err(format!(
                    "stylesheet {file_name} uses unsupported at-rule @{name}"
                ));
            }
            if !context.layered {
                return Err(format!(
                    "stylesheet {file_name} has an unlayered @{name} rule; place Design System rules in a named layer"
                ));
            }
            reject_nested_directives(file_name, body)
        }
        Node::Style { prelude, body } => {
            if !context.layered {
                return Err(format!(
                    "stylesheet {file_name} has an unlayered style rule `{prelude}`; place Design System rules in a named layer"
                ));
            }
            reject_nested_directives(file_name, body)
        }
    }
}

/// Enforce layer-name shape and the import-owned parent rule.
fn check_layer_name(context: &Context<'_>, name: &str) -> Result<(), String> {
    let file = context.file;
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && !segment.starts_with(|c: char| c.is_ascii_digit())
    };
    if !name.split('.').all(valid_segment) {
        return Err(format!(
            "stylesheet {file} declares invalid layer name '{name}'"
        ));
    }

    // A stylesheet that no import placed owns its layers directly. At its top
    // level every layer must sit under the shared `ds` namespace, so Design
    // System source cannot create a sibling of `ds` that outranks or undercuts it.
    if context.owner.is_none() && context.top_level && name != "ds" && !name.starts_with("ds.") {
        return Err(format!(
            "stylesheet {file} declares top-level layer '{name}' outside the `ds` namespace"
        ));
    }

    if let Some(owner) = context.owner {
        let local = owner.rsplit('.').next().unwrap_or(owner);
        if name == owner || name == local {
            return Err(format!(
                "stylesheet {file} redeclares its import-owned parent layer '{owner}' as '{name}'; a stylesheet enters its layer exactly once"
            ));
        }
        if name.starts_with(&format!("{owner}.")) || name.starts_with(&format!("{local}.")) {
            return Err(format!(
                "stylesheet {file} declares prefix-qualified layer '{name}' under import-owned parent '{owner}'; use unqualified local sub-layer names"
            ));
        }
        if name.contains('.') {
            return Err(format!(
                "stylesheet {file} declares qualified layer '{name}' under import-owned parent '{owner}'; use unqualified local sub-layer names"
            ));
        }
    }
    Ok(())
}

/// Record the single owning stylesheet of a full layer name.
fn claim(graph: &mut Graph, layer: &str, file: &Path, root: &Path) -> Result<(), String> {
    match graph.owners.get(layer) {
        Some(existing) if existing != file => Err(format!(
            "layer '{layer}' has two owning stylesheets: {} and {}",
            display(root, existing),
            display(root, file)
        )),
        _ => {
            graph.owners.insert(layer.to_owned(), file.to_path_buf());
            Ok(())
        }
    }
}

fn reject_tailwind(file: &str, name: &str) -> Result<(), String> {
    if TAILWIND_AT_RULES.contains(&name) {
        return Err(format!(
            "stylesheet {file} uses Tailwind directive @{name}; the portable core must be plain browser CSS"
        ));
    }
    Ok(())
}

/// Style and opaque bodies are not descended, so reject the constructs that
/// would otherwise hide inside them.
fn reject_nested_directives(file: &str, body: &str) -> Result<(), String> {
    let body = &blank_strings(body);
    if has_escaped_at_rule(body) {
        return Err(format!(
            "stylesheet {file} uses an escaped at-rule name; write at-rule names literally"
        ));
    }
    for name in TAILWIND_AT_RULES {
        if contains_at_rule(body, name) {
            return Err(format!(
                "stylesheet {file} uses Tailwind directive @{name}; the portable core must be plain browser CSS"
            ));
        }
    }
    if contains_at_rule(body, "layer") || contains_at_rule(body, "import") {
        return Err(format!(
            "stylesheet {file} nests @layer or @import inside a style rule; declare layers at stylesheet or group level"
        ));
    }
    if has_unescaped_bang(body) {
        return Err(format!(
            "stylesheet {file} uses `!` in a declaration; Design System rules never use !important, so consumer layers win by order alone"
        ));
    }
    Ok(())
}

/// Whether the body has a `!` that is not an escaped code point. Outside strings
/// a declaration only uses `!` for a priority such as `!important` (in any case,
/// spacing, or escaped spelling), so every such `!` is rejected.
fn has_unescaped_bang(body: &str) -> bool {
    body.match_indices('!').any(|(index, _)| {
        let escaping = body[..index]
            .chars()
            .rev()
            .take_while(|c| *c == '\\')
            .count();
        escaping % 2 == 0
    })
}

/// Blank the contents of quoted strings so literal text such as
/// `content: "@theme"` is not mistaken for a directive.
fn blank_strings(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in body.chars() {
        match quote {
            Some(open) => {
                if escaped {
                    escaped = false;
                    out.push(' ');
                } else if c == '\\' {
                    escaped = true;
                    out.push(' ');
                } else if c == open {
                    quote = None;
                    out.push(c);
                } else {
                    out.push(' ');
                }
            }
            None => {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' || c == '\'' {
                    quote = Some(c);
                }
                out.push(c);
            }
        }
    }
    out
}

/// Whether any at-keyword's name contains a CSS escape, which the literal
/// name checks below would otherwise miss (`@l\61yer`, `@\61pply`).
fn has_escaped_at_rule(body: &str) -> bool {
    body.match_indices('@').any(|(index, _)| {
        // `\@` is an escaped code point inside a selector, not an at-keyword.
        let escaping = body[..index]
            .chars()
            .rev()
            .take_while(|c| *c == '\\')
            .count();
        escaping % 2 == 0
            && body[index + 1..]
                .chars()
                .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
                == Some('\\')
    })
}

fn contains_at_rule(body: &str, name: &str) -> bool {
    let needle = format!("@{name}");
    body.match_indices(&needle).any(|(index, _)| {
        body[index + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '-' || next == '_'))
    })
}

/// Parse an `@import` prelude into its target and optional `layer(name)`.
fn parse_import(file: &str, prelude: &str) -> Result<(String, Option<String>), String> {
    let prelude = prelude.trim();
    let quote = prelude
        .chars()
        .next()
        .filter(|c| *c == '"' || *c == '\'')
        .ok_or_else(|| {
            format!(
                "stylesheet {file} import `{prelude}` must be a quoted relative path; url() and bare imports are not supported"
            )
        })?;
    let rest = &prelude[1..];
    let close = rest
        .find(quote)
        .ok_or_else(|| format!("stylesheet {file} import `{prelude}` has an unterminated path"))?;
    let target = rest[..close].to_owned();
    let tail = rest[close + 1..].trim();

    let layer = if tail.is_empty() {
        None
    } else if let Some(inner) = tail
        .strip_prefix("layer(")
        .and_then(|inner| inner.strip_suffix(')'))
    {
        let name = inner.trim();
        if name.is_empty() {
            return Err(format!(
                "stylesheet {file} import `{prelude}` uses an empty layer()"
            ));
        }
        Some(name.to_owned())
    } else if tail == "layer" {
        return Err(format!(
            "stylesheet {file} import `{prelude}` creates an anonymous layer; name the layer"
        ));
    } else {
        return Err(format!(
            "stylesheet {file} import `{prelude}` has unsupported conditions `{tail}`"
        ));
    };

    Ok((target, layer))
}

/// Resolve an import target against the importing file, confined to the
/// styles root.
fn resolve_import(
    file: &Path,
    styles_dir: &Path,
    shown: &str,
    target: &str,
) -> Result<PathBuf, String> {
    let relative = target.strip_prefix("./").ok_or_else(|| {
        format!(
            "stylesheet {shown} imports '{target}'; imports must be relative paths starting with ./"
        )
    })?;
    let relative_path = Path::new(relative);
    let confined = !relative.is_empty()
        && relative_path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !confined {
        return Err(format!(
            "stylesheet {shown} imports '{target}'; import paths must not leave the importing directory"
        ));
    }

    let directory = file
        .parent()
        .ok_or_else(|| format!("stylesheet {shown} has no parent directory"))?;
    let resolved = directory
        .join(relative_path)
        .canonicalize()
        .map_err(|error| format!("stylesheet {shown} imports '{target}': {error}"))?;
    if !resolved.starts_with(styles_dir) {
        return Err(format!(
            "stylesheet {shown} imports '{target}', which resolves outside the styles root"
        ));
    }
    Ok(resolved)
}

fn display(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
}

/// CSS newlines end a string token, so a string must not span one unescaped.
fn is_newline(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{c}')
}

/// Replace comments with a space, leaving strings intact.
fn strip_comments(source: &str) -> Result<String, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut quote: Option<char> = None;
    let mut index = 0;

    while index < chars.len() {
        let c = chars[index];
        if let Some(open) = quote {
            if is_newline(c) {
                return Err(
                    "unterminated string: a string may not contain an unescaped newline".to_owned(),
                );
            }
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.get(index + 1) {
                    out.push(*next);
                }
                index += 2;
                continue;
            }
            if c == open {
                quote = None;
            }
            index += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            out.push(c);
            index += 1;
            continue;
        }
        if c == '\\' {
            // An escaped code point is part of an identifier, never structure.
            out.push(c);
            if let Some(next) = chars.get(index + 1) {
                out.push(*next);
            }
            index += 2;
            continue;
        }
        if c == '/' && chars.get(index + 1) == Some(&'*') {
            let end = (index + 2..chars.len().saturating_sub(1))
                .find(|&at| chars[at] == '*' && chars[at + 1] == '/')
                .ok_or_else(|| "unterminated comment".to_owned())?;
            out.push(' ');
            index = end + 2;
            continue;
        }
        out.push(c);
        index += 1;
    }

    if quote.is_some() {
        return Err("unterminated string".to_owned());
    }
    Ok(out)
}

fn parse_nodes(chars: &[char]) -> Result<Vec<Node>, String> {
    let mut nodes = Vec::new();
    let mut index = 0;

    loop {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }

        let (end, delimiter) = scan_prelude(chars, index)?;
        let prelude: String = chars[index..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned();

        if let Some(at_rule) = prelude.strip_prefix('@') {
            let name_len = at_rule
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .unwrap_or(at_rule.len());
            if at_rule[name_len..].starts_with('\\') {
                return Err(format!(
                    "at-rule name uses a CSS escape; write it literally: `{prelude}`"
                ));
            }
            let name = at_rule[..name_len].to_ascii_lowercase();
            if name.is_empty() {
                return Err(format!("at-rule without a name: `{prelude}`"));
            }
            let rest = at_rule[name_len..].trim().to_owned();

            if delimiter == ';' {
                nodes.push(Node::Statement {
                    name,
                    prelude: rest,
                });
                index = end + 1;
            } else {
                let close = matching_brace(chars, end)?;
                let body = &chars[end + 1..close];
                if GROUP_AT_RULES.contains(&name.as_str()) {
                    nodes.push(Node::Group {
                        name,
                        prelude: rest,
                        children: parse_nodes(body)?,
                    });
                } else {
                    nodes.push(Node::Opaque {
                        name,
                        body: body.iter().collect(),
                    });
                }
                index = close + 1;
            }
        } else {
            if delimiter == ';' {
                return Err(format!("declaration outside any rule: `{prelude}`"));
            }
            let close = matching_brace(chars, end)?;
            nodes.push(Node::Style {
                prelude,
                body: chars[end + 1..close].iter().collect(),
            });
            index = close + 1;
        }
    }

    Ok(nodes)
}

/// Find the `;` or `{` that ends the prelude starting at `start`.
fn scan_prelude(chars: &[char], start: usize) -> Result<(usize, char), String> {
    let mut quote: Option<char> = None;
    let mut parens = 0usize;
    let mut index = start;

    while index < chars.len() {
        let c = chars[index];
        if let Some(open) = quote {
            if c == '\\' {
                index += 2;
                continue;
            }
            if c == open {
                quote = None;
            }
        } else {
            match c {
                '\\' => {
                    index += 2;
                    continue;
                }
                '"' | '\'' => quote = Some(c),
                '(' => parens += 1,
                ')' => {
                    parens = parens
                        .checked_sub(1)
                        .ok_or_else(|| "unbalanced ')'".to_owned())?;
                }
                ';' | '{' if parens == 0 => return Ok((index, c)),
                '}' => return Err("unbalanced '}'".to_owned()),
                _ => {}
            }
        }
        index += 1;
    }

    let rest: String = chars[start..].iter().collect();
    Err(format!("unterminated rule: `{}`", rest.trim()))
}

/// Index of the `}` matching the `{` at `open`.
fn matching_brace(chars: &[char], open: usize) -> Result<usize, String> {
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut index = open;

    while index < chars.len() {
        let c = chars[index];
        if let Some(open_quote) = quote {
            if c == '\\' {
                index += 2;
                continue;
            }
            if c == open_quote {
                quote = None;
            }
        } else {
            match c {
                '\\' => {
                    index += 2;
                    continue;
                }
                '"' | '\'' => quote = Some(c),
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(index);
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }

    Err("unterminated block".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: &str =
        "@layer ds.tokens, ds.base, ds.layouts, ds.primitives, ds.components, ds.utilities;";

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repository root")
    }

    fn case(name: &str) -> Result<Graph, String> {
        let styles_root = Path::new("fixtures/layer-ownership").join(name);
        let root = repository_root();
        let graph = validate_graph(&root, &styles_root.join("index.css"), &styles_root)?;
        let tracked = crate::tracked_files(&root).expect("tracked files");
        validate_reachability(&root, &styles_root, &graph.stylesheets, &tracked)?;
        Ok(graph)
    }

    fn rejected(name: &str) -> String {
        case(name).expect_err("layer-ownership fixture must be rejected")
    }

    fn file_check(source: &str, owner: Option<&str>) -> Result<(), String> {
        let nodes = parse(source)?;
        let mut graph = Graph::default();
        let context = Context {
            file: "inline.css",
            owner,
            layered: owner.is_some(),
            top_level: true,
        };
        for node in &nodes {
            check_node(
                &context,
                node,
                &mut graph,
                Path::new("inline.css"),
                Path::new(""),
            )?;
        }
        Ok(())
    }

    #[test]
    fn import_owned_fixture_passes_with_unqualified_sub_layers() {
        let graph = case("accepted-import-owned").expect("import-owned fixture");
        assert_eq!(graph.stylesheets.len(), 2);
        assert_eq!(
            graph.owners.keys().collect::<Vec<_>>(),
            vec!["ds.components"]
        );
    }

    #[test]
    fn source_owned_fixture_passes() {
        let graph = case("accepted-source-owned").expect("source-owned fixture");
        assert_eq!(graph.stylesheets.len(), 2);
        assert_eq!(graph.owners.keys().collect::<Vec<_>>(), vec!["ds.tokens"]);
    }

    #[test]
    fn exact_duplicate_parent_is_rejected() {
        let error = rejected("rejected-exact-duplicate");
        assert!(
            error.contains("redeclares its import-owned parent layer"),
            "{error}"
        );
    }

    #[test]
    fn prefix_qualified_child_is_rejected() {
        let error = rejected("rejected-prefix-qualified");
        assert!(error.contains("prefix-qualified layer"), "{error}");
    }

    #[test]
    fn double_import_is_rejected() {
        let error = rejected("rejected-double-import");
        assert!(
            error.contains("enters the cascade more than once"),
            "{error}"
        );
    }

    #[test]
    fn unlayered_rule_is_rejected() {
        let error = rejected("rejected-unlayered-rule");
        assert!(error.contains("unlayered style rule"), "{error}");
    }

    #[test]
    fn tailwind_directive_is_rejected() {
        let error = rejected("rejected-tailwind-directive");
        assert!(error.contains("Tailwind directive @theme"), "{error}");
    }

    #[test]
    fn url_import_is_rejected() {
        let error = rejected("rejected-non-relative-import");
        assert!(error.contains("must be a quoted relative path"), "{error}");
    }

    #[test]
    fn root_absolute_and_parent_imports_are_rejected() {
        let file = Path::new("/nowhere/index.css");
        let styles = Path::new("/nowhere");
        let absolute = resolve_import(file, styles, "inline.css", "/packages/styles/index.css")
            .expect_err("root-absolute import must be rejected");
        assert!(absolute.contains("must be relative paths"), "{absolute}");
        let parent = resolve_import(file, styles, "inline.css", concat!("./..", "/outside.css"))
            .expect_err("parent-directory import must be rejected");
        assert!(parent.contains("must not leave"), "{parent}");
    }

    #[test]
    fn anonymous_layers_are_rejected() {
        let block = file_check("@layer { a { color: red; } }", None).expect_err("anonymous block");
        assert!(block.contains("anonymous @layer block"), "{block}");
        let import = parse_import("inline.css", "\"./a.css\" layer").expect_err("anonymous import");
        assert!(import.contains("anonymous layer"), "{import}");
    }

    #[test]
    fn nested_import_owned_names_must_stay_unqualified() {
        let nested = "@layer base { @layer ds.components.inner { a { color: red; } } }";
        let error = file_check(nested, Some("ds.components")).expect_err("nested prefix");
        assert!(error.contains("prefix-qualified layer"), "{error}");
        let other = file_check("@layer ds.base;", Some("ds.components")).expect_err("qualified");
        assert!(other.contains("qualified layer"), "{other}");
        file_check(
            "@layer base, variants; @layer base { a { color: red; } }",
            Some("ds.components"),
        )
        .expect("unqualified local sub-layers");
    }

    #[test]
    fn unlayered_group_and_opaque_rules_are_rejected() {
        let media = file_check("@media (width > 1px) { a { color: red; } }", None)
            .expect_err("unlayered media");
        assert!(media.contains("unlayered @media"), "{media}");
        let font = file_check("@font-face { font-family: x; }", None).expect_err("unlayered font");
        assert!(font.contains("unlayered @font-face"), "{font}");
        file_check(
            "@layer ds.base { @media (width > 1px) { a { color: red; } } }",
            None,
        )
        .expect("layered media");
    }

    #[test]
    fn nested_tailwind_apply_is_rejected() {
        let error = file_check("@layer ds.base { a { @apply text-red-500; } }", None)
            .expect_err("nested apply");
        assert!(error.contains("Tailwind directive @apply"), "{error}");
    }

    #[test]
    fn comments_and_strings_do_not_confuse_the_scanner() {
        let source = format!(
            "/* {{ @layer ds.fake; }} */\n{ORDER}\n@layer ds.base {{ a::before {{ content: \"}} @theme {{\"; }} }}"
        );
        file_check(&source, None).expect("comments and strings are inert");
        assert!(parse("a { color: red;").is_err());
        assert!(parse("/* open").is_err());
    }

    /// A browser ends a string at an unescaped newline, so a scanner that let
    /// the string continue would hide rules the browser places outside the layer.
    #[test]
    fn strings_may_not_span_an_unescaped_newline() {
        let hidden =
            "@layer ds.base { a { content: \"\n} } #probe { color: red } q { content: \"; } }";
        let error = parse(hidden).expect_err("newline inside a string");
        assert!(error.contains("unescaped newline"), "{error}");
        for newline in ["\r", "\u{c}"] {
            let source = format!("@layer ds.base {{ a {{ content: \"{newline}\"; }} }}");
            assert!(parse(&source).is_err(), "{newline:?}");
        }
        file_check("@layer ds.base { a { content: \"line\\\nbreak\"; } }", None)
            .expect("an escaped newline continues the string");
    }

    /// An escaped brace is an identifier code point in the browser, so it must
    /// not open or close a block in the scanner either.
    #[test]
    fn escaped_braces_are_not_structural() {
        let hidden = "@layer ds.base { .a\\{ } #probe { color: red } }";
        assert!(
            parse(hidden).is_err(),
            "escaped brace must not balance a block"
        );
        file_check(
            "@layer ds.utilities { .sm\\:flex, .w-1\\/2, .x\\{ { color: red; } }",
            None,
        )
        .expect("escaped selector code points stay inside the layer");
        file_check(
            "@layer ds.utilities { .card { &.\\@md\\:flex { color: red; } } }",
            None,
        )
        .expect("an escaped @ in a selector is not an at-rule");
        let nodes = parse("@layer ds.base { .x\\} { color: red; } }").expect("escaped close");
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn escaped_at_rule_names_are_rejected() {
        let top = parse("@l\\61yer ds.base { a { color: red; } }").expect_err("escaped layer");
        assert!(top.contains("CSS escape"), "{top}");
        let nested = file_check("@layer ds.base { a { @\\61pply text-red; } }", None)
            .expect_err("escaped nested apply");
        assert!(nested.contains("escaped at-rule name"), "{nested}");
        let layer = file_check("@layer ds.base { a { @l\\61yer x { color: red; } } }", None)
            .expect_err("escaped nested layer");
        assert!(layer.contains("escaped at-rule name"), "{layer}");
    }

    #[test]
    fn a_layer_has_one_owning_stylesheet() {
        let mut graph = Graph::default();
        let root = Path::new("/r");
        claim(&mut graph, "ds.base", Path::new("/r/a.css"), root).expect("first owner");
        claim(&mut graph, "ds.base", Path::new("/r/a.css"), root).expect("same owner");
        let error =
            claim(&mut graph, "ds.base", Path::new("/r/b.css"), root).expect_err("second owner");
        assert!(error.contains("two owning stylesheets"), "{error}");
    }

    #[test]
    fn source_owned_layers_stay_under_the_ds_namespace() {
        for source in [
            "@layer app;",
            "@layer dsx { a { color: red; } }",
            "@layer ds, app;",
            "@import \"./a.css\" layer(vendor);",
        ] {
            let error = if source.starts_with("@import") {
                let context = Context {
                    file: "inline.css",
                    owner: None,
                    layered: false,
                    top_level: true,
                };
                check_layer_name(&context, "vendor").expect_err("import layer")
            } else {
                file_check(source, None).expect_err(source)
            };
            assert!(error.contains("outside the `ds` namespace"), "{error}");
        }
        file_check("@layer ds;", None).expect("the ds namespace itself");
        file_check(
            "@layer ds.tokens { @layer local { a { color: red; } } }",
            None,
        )
        .expect("nested local layer inside a ds layer");
        file_check("@layer local { a { color: red; } }", Some("ds.tokens"))
            .expect("import-owned sub-layer");
    }

    #[test]
    fn important_declarations_are_rejected() {
        for body in [
            "color: red !important;",
            "color: red ! important;",
            "color: red !IMPORTANT;",
            "color: red !imp\\ortant;",
        ] {
            let source = format!("@layer ds.base {{ a {{ {body} }} }}");
            let error = file_check(&source, None).expect_err(body);
            assert!(error.contains("!important"), "{error}");
        }
        file_check(
            "@layer ds.base { a { content: \"!important\"; } .a\\! { color: red; } }",
            None,
        )
        .expect("a quoted or escaped `!` is not a priority");
    }
}
