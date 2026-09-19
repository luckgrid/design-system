//! Classless base checks: the stylesheets under `packages/styles/base/` style
//! exactly the subjects `base.tsv` classifies, through zero-specificity
//! `:where()` selectors that carry no product hook, with values bound to the
//! public semantic roles. The base document and the plain fixture must cover
//! the same inventory.
//!
//! Like `tokens` and `theme`, this recognizes only the shapes the base uses and
//! rejects anything else rather than guessing.

use std::collections::{BTreeMap, BTreeSet};

use crate::css::{self, Node};
use crate::theme::{self, Simple};
use crate::tokens;

/// Repository-relative stylesheet that imports the base modules.
pub const BASE_ENTRY: &str = "packages/styles/base.css";
/// Directory that holds one base module per group.
const BASE_DIR: &str = "packages/styles/base/";

const PUBLIC_PREVIEW: &str = "public-preview";
const EXCLUDED: &str = "excluded";
/// The base groups, in sub-layer order; each is one module `base/<group>.css`.
const GROUPS: [&str; 4] = ["document", "content", "forms", "interactive"];

/// Pseudo-classes a base selector may use: native link, pointer, focus, and
/// form states, and the logical combinators.
const PSEUDO_CLASSES: [&str; 9] = [
    "any-link",
    "hover",
    "visited",
    "focus-visible",
    "disabled",
    "user-invalid",
    "not",
    "is",
    "where",
];
/// Native attributes a base selector may test.
const ATTRIBUTES: [&str; 5] = ["type", "popover", "multiple", "size", "open"];
/// Pseudo-elements that may follow a base `:where()`.
const PSEUDO_ELEMENTS: [&str; 2] = ["placeholder", "file-selector-button"];
/// Value functions a base declaration may call.
const FUNCTIONS: [&str; 4] = ["var", "calc", "min", "max"];
/// Units a base value may use: font-relative lengths and percentages. Every
/// other length comes from a semantic role.
const UNITS: [&str; 4] = ["em", "ch", "lh", "%"];
/// Keywords a base value may use. A named color is not a keyword here, so a
/// color can only come from a role, `currentcolor`, or `transparent`.
const KEYWORDS: [&str; 30] = [
    "inherit",
    "initial",
    "unset",
    "revert",
    "currentcolor",
    "transparent",
    "none",
    "auto",
    "normal",
    "solid",
    "dotted",
    "dashed",
    "underline",
    "break-word",
    "anywhere",
    "balance",
    "pretty",
    "collapse",
    "separate",
    "start",
    "end",
    "top",
    "middle",
    "baseline",
    "pointer",
    "default",
    "not-allowed",
    "italic",
    "block",
    "inline-block",
];

/// The value vocabulary of the classless base.
const BASE_VALUES: Vocabulary = Vocabulary {
    label: "the base",
    functions: &FUNCTIONS,
    units: &UNITS,
    keywords: &KEYWORDS,
};

/// Owner of an exclusion that the shared base does not style: the consumer
/// keeps it, as it keeps page layout.
pub(crate) const CONSUMER_OWNER: &str = "consumer";
const NATIVE_PATTERN_OWNER: &str = "native-pattern-refinement";
const EXCLUSION_OWNERS: [&str; 2] = [CONSUMER_OWNER, NATIVE_PATTERN_OWNER];

/// What a checked value may be built from, besides numbers, `var(--ds-*)`
/// references, and separators.
pub(crate) struct Vocabulary {
    /// How errors name the surface, for example "the base".
    pub label: &'static str,
    pub functions: &'static [&'static str],
    pub units: &'static [&'static str],
    pub keywords: &'static [&'static str],
}

/// The validated `base.tsv` inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Owned subject to its group, in manifest order.
    pub owned: Vec<(String, String)>,
    /// Excluded subject to the program task that owns it.
    pub excluded: Vec<(String, String)>,
}

impl Manifest {
    fn is_owned(&self, subject: &str) -> bool {
        self.owned.iter().any(|(name, _)| name == subject)
    }

    fn is_excluded(&self, subject: &str) -> bool {
        self.excluded.iter().any(|(name, _)| name == subject)
    }
}

/// Parse and validate the base inventory.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut owned = Vec::new();
    let mut excluded = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [class, group, subject] = fields.as_slice() else {
            return Err(format!(
                "base line {line_number} must be <class><tab><group or owner><tab><subject>"
            ));
        };
        if !is_subject(subject) {
            return Err(format!(
                "base line {line_number} subject '{subject}' is not a lowercase element, [attribute], or :pseudo-class"
            ));
        }
        if !seen.insert((*subject).to_owned()) {
            return Err(format!(
                "base line {line_number} repeats subject '{subject}'; a subject is owned or excluded exactly once"
            ));
        }
        match *class {
            PUBLIC_PREVIEW => {
                if !GROUPS.contains(group) {
                    return Err(format!(
                        "base line {line_number} group '{group}' is not one of {}",
                        GROUPS.join(", ")
                    ));
                }
                owned.push(((*subject).to_owned(), (*group).to_owned()));
            }
            EXCLUDED => {
                if !EXCLUSION_OWNERS.contains(group) {
                    return Err(format!(
                        "base line {line_number} excludes '{subject}' with unknown owner '{group}'; expected {}",
                        EXCLUSION_OWNERS.join(" or ")
                    ));
                }
                if !is_element(subject) {
                    return Err(format!(
                        "base line {line_number} excludes '{subject}'; exclusions name elements"
                    ));
                }
                excluded.push(((*subject).to_owned(), (*group).to_owned()));
            }
            other => {
                return Err(format!(
                    "base line {line_number} classifies {subject} as '{other}'; base subjects are {PUBLIC_PREVIEW} or {EXCLUDED}"
                ));
            }
        }
    }
    if owned.is_empty() {
        return Err("base inventory owns no subject".to_owned());
    }
    Ok(Manifest { owned, excluded })
}

fn is_element(subject: &str) -> bool {
    let mut chars = subject.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

fn is_subject(subject: &str) -> bool {
    let lower_name = |name: &str| {
        !name.is_empty()
            && !name.starts_with('-')
            && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
    };
    if let Some(name) = subject
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return lower_name(name);
    }
    if let Some(name) = subject.strip_prefix(':') {
        return lower_name(name);
    }
    is_element(subject)
}

/// What a validated base covers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub modules: usize,
    pub rules: usize,
    pub declarations: usize,
}

/// Validate the base stylesheets among the reached Design System stylesheets.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_stylesheets(
    stylesheets: &[(String, String)],
    manifest: &Manifest,
) -> Result<Summary, String> {
    let entry = stylesheets
        .iter()
        .find(|(file, _)| file == BASE_ENTRY)
        .ok_or_else(|| {
            format!("{BASE_ENTRY} is not reached from any declared export; the base must enter the cascade")
        })?;
    validate_entry(&entry.0, &entry.1)?;

    let mut modules = BTreeMap::new();
    for (file, source) in stylesheets {
        if let Some(name) = file.strip_prefix(BASE_DIR) {
            let group = name.strip_suffix(".css").unwrap_or(name);
            if !GROUPS.contains(&group) {
                return Err(format!(
                    "{file} is not a base module; the base groups are {}",
                    GROUPS.join(", ")
                ));
            }
            modules.insert(group.to_owned(), (file.clone(), source.clone()));
        }
    }
    for group in GROUPS {
        if !modules.contains_key(group) {
            return Err(format!(
                "{BASE_DIR}{group}.css is not reached; every base group enters the cascade"
            ));
        }
    }

    let mut summary = Summary {
        modules: modules.len(),
        ..Summary::default()
    };
    let mut used = BTreeSet::new();
    for (file, source) in modules.values() {
        theme::reject_selector_comments(file, source)?;
        let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
        for node in &nodes {
            let Node::Style { prelude, body } = node else {
                return Err(format!(
                    "{file} holds an at-rule; base modules hold unconditional style rules only"
                ));
            };
            let selector = prelude.split_whitespace().collect::<Vec<_>>().join(" ");
            for subject in check_selector(&selector, manifest)
                .map_err(|error| format!("{file} `{selector}`: {error}"))?
            {
                used.insert(subject);
            }
            let declarations = tokens::split_declarations(file, body)?;
            if declarations.is_empty() {
                return Err(format!("{file} `{selector}` declares nothing"));
            }
            for (name, value) in &declarations {
                check_declaration(name, value)
                    .map_err(|error| format!("{file} `{selector}` {name}: {error}"))?;
            }
            summary.rules += 1;
            summary.declarations += declarations.len();
        }
    }

    if let Some((subject, _)) = manifest
        .owned
        .iter()
        .find(|(subject, _)| !used.contains(subject))
    {
        return Err(format!(
            "base.tsv owns `{subject}`, but no base rule selects it"
        ));
    }
    Ok(summary)
}

/// `base.css` holds only its sub-layer order and one import per group.
fn validate_entry(file: &str, source: &str) -> Result<(), String> {
    let mut imported = Vec::new();
    for node in css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))? {
        match node {
            Node::Statement { name, .. } if name == "layer" => {}
            Node::Statement { name, prelude } if name == "import" => {
                imported.push(prelude.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            _ => {
                return Err(format!(
                    "{file} holds a rule; it only orders and imports the base modules"
                ));
            }
        }
    }
    let expected: Vec<String> = GROUPS
        .iter()
        .map(|group| format!("\"./base/{group}.css\" layer({group})"))
        .collect();
    if imported != expected {
        return Err(format!(
            "{file} must import exactly {} in that order",
            expected.join(", ")
        ));
    }
    Ok(())
}

/// Check one selector list and return the subjects it styles.
///
/// Every complex selector at the top level must be one `:where(...)`, written
/// literally in lowercase, optionally followed by an allowlisted
/// pseudo-element. Inside it, only element names, allowlisted pseudo-classes,
/// and allowlisted attributes may appear.
fn check_selector(selector: &str, manifest: &Manifest) -> Result<Vec<String>, String> {
    let mut subjects = Vec::new();
    for part in split_top_level(selector)? {
        let inner = where_argument(part)?;
        for complex in theme::scan_selector(inner)? {
            for simple in complex.iter().flatten() {
                match simple {
                    Simple::Class(name) => {
                        return Err(format!(
                            "class `.{name}`: the classless base styles elements, not class hooks"
                        ));
                    }
                    Simple::Other => {
                        return Err(
                            "an id, `*`, or `&` selector: the base styles named elements only"
                                .to_owned(),
                        );
                    }
                    Simple::Attribute(name) if !ATTRIBUTES.contains(&name.as_str()) => {
                        return Err(format!(
                            "attribute `[{name}]`: only the native {} attributes are base hooks",
                            ATTRIBUTES.join(", ")
                        ));
                    }
                    Simple::Pseudo(name) if !PSEUDO_CLASSES.contains(&name.as_str()) => {
                        return Err(format!(
                            "pseudo-class `:{name}` is not one of the base's native states"
                        ));
                    }
                    Simple::Type(name) if manifest.is_excluded(name) => {
                        return Err(format!(
                            "`{name}` is excluded in base.tsv; its owner styles it"
                        ));
                    }
                    Simple::Type(name) if !manifest.is_owned(name) => {
                        return Err(format!("`{name}` is not an owned subject in base.tsv"));
                    }
                    _ => {}
                }
            }
            let subject = complex
                .last()
                .and_then(|compound| subject_of(compound))
                .ok_or("cannot tell which subject this selector styles")?;
            if !manifest.is_owned(&subject) {
                return Err(format!("subject `{subject}` is not owned in base.tsv"));
            }
            subjects.push(subject);
        }
    }
    Ok(subjects)
}

/// The subject a compound styles: its element, else its attribute, else its
/// first non-logical pseudo-class.
fn subject_of(compound: &[Simple]) -> Option<String> {
    compound
        .iter()
        .find_map(|simple| match simple {
            Simple::Type(name) => Some(name.clone()),
            _ => None,
        })
        .or_else(|| {
            compound.iter().find_map(|simple| match simple {
                Simple::Attribute(name) => Some(format!("[{name}]")),
                _ => None,
            })
        })
        .or_else(|| {
            compound.iter().find_map(|simple| match simple {
                Simple::Pseudo(name) if !matches!(name.as_str(), "not" | "is" | "where") => {
                    Some(format!(":{name}"))
                }
                _ => None,
            })
        })
}

/// Split a selector list on its top-level commas.
fn split_top_level(selector: &str) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut quote = None;
    let mut chars = selector.char_indices();
    while let Some((index, c)) = chars.next() {
        if let Some(open) = quote {
            if c == '\\' {
                chars.next();
            } else if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '\\' => {
                chars.next();
            }
            '"' | '\'' => quote = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("unbalanced `)` or `]`".to_owned())?;
            }
            ',' if depth == 0 => {
                parts.push(selector[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 || quote.is_some() {
        return Err("unbalanced selector".to_owned());
    }
    parts.push(selector[start..].trim());
    if parts.iter().any(|part| part.is_empty()) {
        return Err("empty selector in a list".to_owned());
    }
    Ok(parts)
}

/// The argument of the one `:where(...)` a base complex selector consists of.
fn where_argument(part: &str) -> Result<&str, String> {
    let rest = part.strip_prefix(":where(").ok_or_else(|| {
        format!(
            "`{part}` is not wrapped in one literal :where(); base selectors carry zero specificity"
        )
    })?;
    let mut depth = 1usize;
    let mut quote = None;
    let mut close = None;
    let mut chars = rest.char_indices();
    while let Some((index, c)) = chars.next() {
        if let Some(open) = quote {
            if c == '\\' {
                chars.next();
            } else if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '\\' => {
                chars.next();
            }
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close.ok_or_else(|| format!("`{part}` has an unterminated :where("))?;
    let inner = &rest[..close];
    let tail = &rest[close + 1..];
    if !tail.is_empty() {
        let element = tail.strip_prefix("::").unwrap_or("");
        if !PSEUDO_ELEMENTS.contains(&element) {
            return Err(format!(
                "`{part}` continues after :where() with `{tail}`; only {} may follow",
                PSEUDO_ELEMENTS
                    .iter()
                    .map(|name| format!("::{name}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if inner.trim().is_empty() {
        return Err(format!("`{part}` has an empty :where()"));
    }
    Ok(inner)
}

/// Check one declaration's property and value.
fn check_declaration(name: &str, value: &str) -> Result<(), String> {
    let property = name.to_ascii_lowercase();
    if property.starts_with("--") {
        return Err(
            "declares a custom property; the token stylesheets are the only token authority"
                .to_owned(),
        );
    }
    if property.starts_with("transition") || property.starts_with("animation") {
        return Err("adds motion; the base declares no transition or animation".to_owned());
    }
    if property == "color-scheme" {
        return Err("sets color-scheme; only the theme contract selects the scheme".to_owned());
    }
    if property == "appearance" || property.ends_with("-appearance") {
        return Err("sets appearance; the base keeps native control appearance".to_owned());
    }
    let lower = value.to_ascii_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let removes_outline = match property.as_str() {
        "outline" => words.iter().any(|word| matches!(*word, "none" | "0")),
        "outline-style" => words.contains(&"none"),
        "outline-width" => words == ["0"],
        _ => false,
    };
    if removes_outline {
        return Err("removes the outline; the base never defeats focus indication".to_owned());
    }
    check_value(&lower, &BASE_VALUES)
}

/// A checked value is built from semantic roles and the vocabulary's units,
/// functions, and keywords. `value` is lowercase.
pub(crate) fn check_value(value: &str, vocabulary: &Vocabulary) -> Result<(), String> {
    let label = vocabulary.label;
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        if c.is_whitespace() || matches!(c, ',' | '/' | '(' | ')' | '*') {
            index += 1;
        } else if c == '#' {
            return Err("uses a hex color; colors come from semantic roles".to_owned());
        } else if c == '"' || c == '\'' {
            return Err(format!("uses a string; {label} generates no content"));
        } else if c.is_ascii_digit()
            || (c == '.' && next.is_some_and(|n| n.is_ascii_digit()))
            || (matches!(c, '+' | '-') && next.is_some_and(|n| n.is_ascii_digit() || n == '.'))
        {
            index += 1;
            while chars
                .get(index)
                .is_some_and(|c| c.is_ascii_digit() || *c == '.')
            {
                index += 1;
            }
            let start = index;
            while chars
                .get(index)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == '%')
            {
                index += 1;
            }
            let unit: String = chars[start..index].iter().collect();
            if !unit.is_empty() && !vocabulary.units.contains(&unit.as_str()) {
                return Err(format!(
                    "uses the unit `{unit}`; lengths come from semantic roles or {}",
                    vocabulary.units.join(", ")
                ));
            }
        } else if matches!(c, '+' | '-') && next.is_none_or(char::is_whitespace) {
            index += 1;
        } else if c.is_ascii_alphabetic() || c == '-' {
            let start = index;
            while chars
                .get(index)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-')
            {
                index += 1;
            }
            let word: String = chars[start..index].iter().collect();
            if chars.get(index) == Some(&'(') {
                if !vocabulary.functions.contains(&word.as_str()) {
                    return Err(format!(
                        "calls `{word}()`; {label} uses only {}",
                        vocabulary
                            .functions
                            .iter()
                            .map(|name| format!("{name}()"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if word == "var" {
                    index += 1;
                    let start = index;
                    while chars
                        .get(index)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-')
                    {
                        index += 1;
                    }
                    let reference: String = chars[start..index].iter().collect();
                    if reference.starts_with("--ds-ref-") {
                        return Err(format!(
                            "reads internal reference value {reference}; {label} binds to public semantic roles"
                        ));
                    }
                    if !reference.starts_with("--ds-") {
                        return Err(format!(
                            "reads var({reference}); {label} binds to Design System semantic roles only"
                        ));
                    }
                    if chars.get(index) != Some(&')') {
                        return Err(format!(
                            "var({reference}) has a fallback; roles are always declared"
                        ));
                    }
                }
            } else if !vocabulary.keywords.contains(&word.as_str()) {
                return Err(format!(
                    "uses `{word}`, which is not a keyword {label} classifies; colors and fonts come from semantic roles"
                ));
            }
        } else {
            return Err(format!(
                "uses `{c}`, which the value check for {label} does not classify"
            ));
        }
    }
    Ok(())
}

/// Heading of the base document section that lists the owned subjects.
const OWNED_SECTION: &str = "## Owned defaults";
/// Heading of the base document section that lists the exclusions.
const EXCLUSIONS_SECTION: &str = "## Exclusions";

/// The base document states the inventory: the owned section has one `###`
/// heading per group and tables whose first column names exactly the owned
/// subjects; the exclusions section names every excluded element.
pub fn validate_document(document: &str, manifest: &Manifest) -> Result<(), String> {
    let owned = section(document, OWNED_SECTION)?;
    for group in GROUPS {
        let heading = format!("### {group}");
        if !owned.lines().any(|line| line.trim_end() == heading) {
            return Err(format!(
                "the base document `{OWNED_SECTION}` section has no `{heading}` heading"
            ));
        }
    }
    let mut listed = BTreeSet::new();
    for line in owned.lines().filter(|line| line.starts_with('|')) {
        let first = line.trim_start_matches('|').split('|').next().unwrap_or("");
        for subject in backticked(first) {
            if !manifest.is_owned(&subject) {
                return Err(format!(
                    "the base document lists `{subject}` as owned, which base.tsv does not own"
                ));
            }
            listed.insert(subject);
        }
    }
    if let Some((subject, _)) = manifest
        .owned
        .iter()
        .find(|(subject, _)| !listed.contains(subject))
    {
        return Err(format!(
            "the base document `{OWNED_SECTION}` tables do not list owned subject `{subject}`"
        ));
    }

    let exclusions = section(document, EXCLUSIONS_SECTION)?;
    let named: BTreeSet<String> = backticked(exclusions).into_iter().collect();
    if let Some((subject, _)) = manifest
        .excluded
        .iter()
        .find(|(subject, _)| !named.contains(subject))
    {
        return Err(format!(
            "the base document `{EXCLUSIONS_SECTION}` section does not name excluded `{subject}`"
        ));
    }
    Ok(())
}

pub(crate) fn backticked(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(|item| {
            item.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
        .collect()
}

/// The body of the one `heading` section, up to the next `#` or `##` heading.
pub(crate) fn section<'a>(document: &'a str, heading: &str) -> Result<&'a str, String> {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in document.split_inclusive('\n') {
        if line.trim_end() == heading {
            starts.push(offset + line.len());
        }
        offset += line.len();
    }
    let [start] = starts.as_slice() else {
        return Err(format!(
            "the document must have exactly one `{heading}` section, found {}",
            starts.len()
        ));
    };
    let body = &document[*start..];
    let mut end = 0;
    for line in body.split_inclusive('\n') {
        if line.starts_with("# ") || line.starts_with("## ") {
            break;
        }
        end += line.len();
    }
    Ok(&body[..end])
}

/// The plain fixture exercises every owned element and attribute subject.
///
/// The plain fixture is classless: it carries no `class` attribute, so the base
/// is proven on ordinary HTML alone.
pub fn validate_fixture(html: &str, manifest: &Manifest) -> Result<usize, String> {
    let lower = html.to_ascii_lowercase();
    if has_attribute(&lower, "class") {
        return Err(
            "the plain fixture carries a class attribute; it proves the base on classless HTML"
                .to_owned(),
        );
    }
    let mut covered = 0;
    for (subject, _) in &manifest.owned {
        let present = if let Some(attribute) = subject
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            has_attribute(&lower, attribute)
        } else if subject.starts_with(':') {
            continue;
        } else {
            has_element(&lower, subject)
        };
        if !present {
            return Err(format!(
                "the plain fixture has no `{subject}`, which base.tsv owns; every owned default needs fixture coverage"
            ));
        }
        covered += 1;
    }
    Ok(covered)
}

pub(crate) fn has_element(html: &str, name: &str) -> bool {
    let open = format!("<{name}");
    html.match_indices(&open).any(|(index, _)| {
        html[index + open.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '>' || c == '/')
    })
}

fn has_attribute(html: &str, name: &str) -> bool {
    html.match_indices(name).any(|(index, _)| {
        html[..index].ends_with(char::is_whitespace)
            && html[index + name.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace() || c == '>' || c == '=')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
# comment
public-preview\tdocument\thtml
public-preview\tdocument\t:focus-visible
public-preview\tcontent\ta
public-preview\tcontent\tpre
public-preview\tcontent\tcode
public-preview\tforms\tinput
public-preview\tinteractive\tdetails
public-preview\tinteractive\t[popover]
excluded\tconsumer\tnav
";

    const ENTRY: &str = "@layer document, content, forms, interactive;
@import \"./base/document.css\" layer(document);
@import \"./base/content.css\" layer(content);
@import \"./base/forms.css\" layer(forms);
@import \"./base/interactive.css\" layer(interactive);
";

    const DOCUMENT: &str = ":where(html) { color: var(--ds-color-text); }
:where(:focus-visible) { outline: var(--ds-focus-width) solid var(--ds-color-focus); }";
    const CONTENT: &str = ":where(a:any-link) { text-underline-offset: 0.15em; }
:where(pre) { overflow-x: auto; }
:where(pre code) { padding: 0; }";
    const FORMS: &str = ":where(input:not([type=\"checkbox\"]))::placeholder { color: var(--ds-color-text-muted); }";
    const INTERACTIVE: &str = ":where(details[open]) { padding-block-end: var(--ds-space-control-block); }\n:where([popover]) { border: var(--ds-border-width) solid var(--ds-color-border); }";

    fn manifest() -> Manifest {
        parse_manifest(MANIFEST).expect("manifest")
    }

    fn sheets(content: &str) -> Vec<(String, String)> {
        vec![
            (BASE_ENTRY.to_owned(), ENTRY.to_owned()),
            (
                "packages/styles/base/document.css".to_owned(),
                DOCUMENT.to_owned(),
            ),
            (
                "packages/styles/base/content.css".to_owned(),
                content.to_owned(),
            ),
            (
                "packages/styles/base/forms.css".to_owned(),
                FORMS.to_owned(),
            ),
            (
                "packages/styles/base/interactive.css".to_owned(),
                INTERACTIVE.to_owned(),
            ),
        ]
    }

    fn with_rule(rule: &str) -> Result<Summary, String> {
        validate_stylesheets(&sheets(&format!("{CONTENT}\n{rule}")), &manifest())
    }

    #[test]
    fn accepts_a_conforming_base() {
        let summary = validate_stylesheets(&sheets(CONTENT), &manifest()).expect("base");
        assert_eq!(summary.modules, 4);
        assert_eq!(summary.rules, 8);
    }

    #[test]
    fn manifest_rejects_bad_rows() {
        for (text, needle) in [
            ("internal\tdocument\thtml", "public-preview or excluded"),
            ("public-preview\tlayout\thtml", "group 'layout'"),
            ("public-preview\tdocument\tHTML", "not a lowercase"),
            ("public-preview\tdocument\t.card", "not a lowercase"),
            ("excluded\tlater\tnav", "unknown owner"),
            ("excluded\tconsumers\tnav", "unknown owner"),
            (
                "excluded\tnative-pattern-refinement\t[popover]",
                "exclusions name elements",
            ),
            (
                "public-preview\tdocument\thtml\nexcluded\tconsumer\thtml",
                "repeats subject",
            ),
            ("excluded\tconsumer\tnav", "owns no subject"),
        ] {
            let error = parse_manifest(text).expect_err(text);
            assert!(error.contains(needle), "{text}: {error}");
        }
    }

    #[test]
    fn rejects_selectors_outside_where() {
        for rule in [
            "a { color: inherit; }",
            ":is(a) { color: inherit; }",
            ":where(a), pre { color: inherit; }",
            ":where(a) code { color: inherit; }",
            ":where(a):hover { color: inherit; }",
            ":WHERE(a) { color: inherit; }",
            ":wher\\65(a) { color: inherit; }",
            ":where(a)::before { color: inherit; }",
        ] {
            let error = with_rule(rule).expect_err(rule);
            assert!(error.contains(":where()"), "{rule}: {error}");
        }
    }

    #[test]
    fn rejects_product_hooks_inside_where() {
        for (rule, needle) in [
            (":where(.card) { color: inherit; }", "class"),
            (":where(a.button) { color: inherit; }", "class"),
            (":where(.d\\61rk a) { color: inherit; }", "class `.dark`"),
            (":where(#main) { color: inherit; }", "an id"),
            (":where(*) { color: inherit; }", "an id"),
            (":where([data-x]) { color: inherit; }", "[data-x]"),
            (":where([ data-foo]) { color: inherit; }", "[data-foo]"),
            (":where([data\\-foo]) { color: inherit; }", "[data-foo]"),
            (":where([role=\"button\"]) { color: inherit; }", "[role]"),
            (":where(a:not([data-x])) { color: inherit; }", "[data-x]"),
            (":where(a:first-child) { color: inherit; }", ":first-child"),
            (":where(pre:has(code)) { color: inherit; }", "`:has`"),
        ] {
            let error = with_rule(rule).expect_err(rule);
            assert!(error.contains(needle), "{rule}: {error}");
        }
    }

    #[test]
    fn permits_the_native_open_attribute_only_on_an_owned_subject() {
        let accepted = check_selector(":where(details[open])", &manifest()).expect("open state");
        assert_eq!(accepted, ["details"]);

        let error = check_selector(":where(nav[open])", &manifest()).expect_err("excluded subject");
        assert!(error.contains("excluded"), "{error}");
    }

    #[test]
    fn rejects_selector_comments() {
        for rule in [
            ":where(.x/**/y) { color: inherit; }",
            ":where(a/**/.dark) { color: inherit; }",
            ".x\\\", :where(.d/**/ark) { color: inherit; }",
        ] {
            let error = with_rule(rule).expect_err(rule);
            assert!(error.contains("comment"), "{rule}: {error}");
        }
    }

    #[test]
    fn rejects_unowned_and_excluded_subjects() {
        let excluded = with_rule(":where(nav) { color: inherit; }").expect_err("nav");
        assert!(excluded.contains("excluded"), "{excluded}");
        let nested = with_rule(":where(nav a) { color: inherit; }").expect_err("nav a");
        assert!(nested.contains("excluded"), "{nested}");
        let unowned = with_rule(":where(section) { color: inherit; }").expect_err("section");
        assert!(unowned.contains("not an owned"), "{unowned}");
        let attribute = with_rule(":where([type]) { color: inherit; }").expect_err("[type]");
        assert!(attribute.contains("`[type]` is not owned"), "{attribute}");
    }

    #[test]
    fn requires_every_owned_subject() {
        let error = validate_stylesheets(
            &sheets(":where(a:any-link) { color: inherit; }"),
            &manifest(),
        )
        .expect_err("pre/code unused");
        assert!(error.contains("no base rule selects it"), "{error}");
    }

    #[test]
    fn rejects_local_values() {
        for (declaration, needle) in [
            ("color: #06c;", "hex color"),
            ("color: red;", "`red`"),
            ("color: oklch(50% 0.1 250);", "`oklch()`"),
            (
                "color: color-mix(in srgb, var(--ds-color-text), white);",
                "`color-mix()`",
            ),
            ("padding: 4px;", "unit `px`"),
            ("padding: 0.25rem;", "unit `rem`"),
            ("color: var(--ds-ref-blue-54);", "internal reference"),
            ("color: var(--brand-ink);", "var(--brand-ink)"),
            ("color: var(--ds-color-text, black);", "fallback"),
            ("--ds-color-text: inherit;", "custom property"),
            ("transition: color 0.2s;", "motion"),
            ("animation-name: pulse;", "motion"),
            ("color-scheme: dark;", "color-scheme"),
            ("content: \"x\";", "string"),
            ("appearance: none;", "appearance"),
            ("-webkit-appearance: none;", "appearance"),
            ("APPEARANCE: auto;", "appearance"),
        ] {
            let error = with_rule(&format!(":where(a:any-link) {{ {declaration} }}"))
                .expect_err(declaration);
            assert!(error.contains(needle), "{declaration}: {error}");
        }
    }

    #[test]
    fn rejects_removed_focus_outline() {
        for declaration in [
            "outline: none;",
            "outline: 0;",
            "outline-style: none;",
            "outline-width: 0;",
            "OUTLINE: NONE;",
        ] {
            let error = with_rule(&format!(":where(:focus-visible) {{ {declaration} }}"))
                .expect_err(declaration);
            assert!(error.contains("focus indication"), "{declaration}: {error}");
        }
    }

    #[test]
    fn accepts_relative_units_and_calc() {
        with_rule(":where(a:any-link) { margin: calc(var(--ds-space-flow) - 0.5em) 0 1ch 50%; }")
            .expect("relative values");
    }

    #[test]
    fn rejects_at_rules_and_malformed_entry() {
        let media =
            with_rule("@media (min-width: 40em) { :where(a:any-link) { color: inherit; } }")
                .expect_err("media");
        assert!(media.contains("at-rule"), "{media}");
        let mut reordered = sheets(CONTENT);
        reordered[0].1 = ENTRY.replace("layer(forms)", "layer(interactive)");
        let error = validate_stylesheets(&reordered, &manifest()).expect_err("entry");
        assert!(error.contains("must import exactly"), "{error}");
        let mut styled = sheets(CONTENT);
        styled[0].1.push_str(":where(a) { color: inherit; }");
        let error = validate_stylesheets(&styled, &manifest()).expect_err("entry rule");
        assert!(error.contains("holds a rule"), "{error}");
    }

    #[test]
    fn requires_the_base_to_be_reached() {
        let mut missing = sheets(CONTENT);
        missing.remove(0);
        let error = validate_stylesheets(&missing, &manifest()).expect_err("no entry");
        assert!(error.contains("not reached"), "{error}");
        let mut module = sheets(CONTENT);
        module.remove(4);
        let error = validate_stylesheets(&module, &manifest()).expect_err("no module");
        assert!(error.contains("interactive.css is not reached"), "{error}");
        let mut extra = sheets(CONTENT);
        extra.push((
            "packages/styles/base/widgets.css".to_owned(),
            ":where(a) { color: inherit; }".to_owned(),
        ));
        let error = validate_stylesheets(&extra, &manifest()).expect_err("extra module");
        assert!(error.contains("not a base module"), "{error}");
    }

    const DOC: &str = "\
# Base

## Owned defaults

### document

| Subject | Default |
|---|---|
| `html`, `:focus-visible` | canvas |

### content

| Subject | Default |
|---|---|
| `a` | link |
| `pre`, `code` | code |

### forms

| `input` | control |

### interactive

| `details`, `[popover]` | native surface |

## Exclusions

| `nav` | consumer |
";

    #[test]
    fn document_must_state_the_inventory() {
        validate_document(DOC, &manifest()).expect("document");
        let error = validate_document(&DOC.replace("### forms\n", ""), &manifest())
            .expect_err("group heading");
        assert!(error.contains("### forms"), "{error}");
        let error = validate_document(&DOC.replace("| `a` | link |\n", ""), &manifest())
            .expect_err("owned subject");
        assert!(error.contains("`a`"), "{error}");
        let error = validate_document(
            &DOC.replace("| `a` | link |", "| `a`, `nav` | link |"),
            &manifest(),
        )
        .expect_err("excluded listed as owned");
        assert!(error.contains("lists `nav` as owned"), "{error}");
        let error = validate_document(&DOC.replace("| `nav` | consumer |\n", ""), &manifest())
            .expect_err("exclusion");
        assert!(error.contains("excluded `nav`"), "{error}");
    }

    #[test]
    fn fixture_must_cover_owned_subjects() {
        let html = "<html><body><a href=\"#x\">x</a><pre><code>x</code></pre>\
<input id=\"x\"><details><summary>x</summary></details><div popover id=\"p\">p</div></body></html>";
        assert_eq!(validate_fixture(html, &manifest()), Ok(7));
        let error =
            validate_fixture(&html.replace("<pre>", "<div>"), &manifest()).expect_err("no pre");
        assert!(error.contains("`pre`"), "{error}");
        let error = validate_fixture(&html.replace(" popover", " data-popover"), &manifest())
            .expect_err("no popover");
        assert!(error.contains("`[popover]`"), "{error}");
        let error = validate_fixture(&html.replace("<a href", "<abbr href"), &manifest())
            .expect_err("abbr is not a");
        assert!(error.contains("`a`"), "{error}");
        for classed in [
            html.replace("<pre>", "<pre class=\"x\">"),
            html.replace("<pre>", "<pre\nCLASS=x>"),
        ] {
            let error = validate_fixture(&classed, &manifest()).expect_err("class");
            assert!(error.contains("class attribute"), "{error}");
        }
        let consumer = parse_manifest("public-preview\tcontent\ta\nexcluded\tconsumer\tnav")
            .expect("consumer owner");
        assert_eq!(
            consumer.excluded,
            [("nav".to_owned(), "consumer".to_owned())]
        );
    }
}
