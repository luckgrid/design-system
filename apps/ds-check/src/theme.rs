//! Theme contract checks: `packages/styles/tokens/theme.css` must implement
//! exactly the default and the one root hook that `theme.tsv` classifies, no
//! other Design System stylesheet may select a color scheme, and the theme must
//! assign no custom property so the token stylesheets stay the only authority.
//!
//! Like `tokens`, this recognizes only the shapes the theme contract uses and
//! rejects anything else rather than guessing.

use std::collections::BTreeSet;

use crate::css::{self, Node};
use crate::tokens;

/// Repository-relative stylesheet that owns the theme contract.
pub const THEME_FILE: &str = "packages/styles/tokens/theme.css";

const PUBLIC_PREVIEW: &str = "public-preview";
const COLOR_SCHEME: &str = "color-scheme";
const HOOK_PREFIX: &str = "data-ds-";
/// The explicit schemes a hook value may select.
const SCHEMES: [&str; 2] = ["light", "dark"];
/// The default values the contract may declare: follow the preference, or one
/// fixed scheme.
const DEFAULTS: [&str; 3] = ["light dark", "light", "dark"];
/// Attribute hooks that are not this contract's. Luna's `.dark` class and
/// `data-theme` attribute are evidence, not portable API.
const ALIAS_ATTRIBUTES: [&str; 2] = ["data-theme", "data-color-scheme"];

/// The validated `theme.tsv` contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The `color-scheme` value on `:root` when no hook value applies.
    pub default: String,
    /// The one root attribute hook.
    pub attribute: String,
    /// The explicit schemes the hook selects, in manifest order.
    pub values: Vec<String>,
}

/// Parse and validate the theme inventory: one default row and one attribute
/// row, both public-preview.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut default: Option<String> = None;
    let mut hook: Option<(String, Vec<String>)> = None;
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [class, kind, name, values] = fields.as_slice() else {
            return Err(format!(
                "theme line {line_number} must be <class><tab><kind><tab><name><tab><values>"
            ));
        };
        if *class != PUBLIC_PREVIEW {
            return Err(format!(
                "theme line {line_number} classifies {name} as '{class}'; theme surfaces are {PUBLIC_PREVIEW} until a reviewed contract earns public-stable"
            ));
        }
        let words: Vec<&str> = values.split_whitespace().collect();
        match *kind {
            "default" => {
                if *name != COLOR_SCHEME {
                    return Err(format!(
                        "theme line {line_number} default row must name {COLOR_SCHEME}, not '{name}'"
                    ));
                }
                let value = words.join(" ");
                if !DEFAULTS.contains(&value.as_str()) {
                    return Err(format!(
                        "theme line {line_number} default '{value}' is not one of {}",
                        DEFAULTS.join(", ")
                    ));
                }
                if default.replace(value).is_some() {
                    return Err(format!(
                        "theme line {line_number} declares a second default"
                    ));
                }
            }
            "attribute" => {
                let valid_name = name.strip_prefix(HOOK_PREFIX).is_some_and(|rest| {
                    !rest.is_empty()
                        && !rest.starts_with('-')
                        && !rest.ends_with('-')
                        && rest.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                });
                if !valid_name {
                    return Err(format!(
                        "theme line {line_number} hook '{name}' must be a lowercase {HOOK_PREFIX}* attribute"
                    ));
                }
                if words.is_empty() {
                    return Err(format!(
                        "theme line {line_number} hook {name} has no values"
                    ));
                }
                let mut seen = BTreeSet::new();
                for word in &words {
                    if !SCHEMES.contains(word) {
                        return Err(format!(
                            "theme line {line_number} hook value '{word}' is not one of {}",
                            SCHEMES.join(", ")
                        ));
                    }
                    if !seen.insert(*word) {
                        return Err(format!(
                            "theme line {line_number} repeats hook value '{word}'"
                        ));
                    }
                }
                if hook.is_some() {
                    return Err(format!(
                        "theme line {line_number} declares a second hook; the contract has exactly one public hook and no aliases"
                    ));
                }
                hook = Some((
                    (*name).to_owned(),
                    words.iter().map(|w| (*w).to_owned()).collect(),
                ));
            }
            other => {
                return Err(format!(
                    "theme line {line_number} kind '{other}' is not default or attribute"
                ));
            }
        }
    }
    let default = default.ok_or("theme inventory declares no default row")?;
    let (attribute, values) = hook.ok_or("theme inventory declares no attribute hook")?;
    Ok(Manifest {
        default,
        attribute,
        values,
    })
}

/// One style rule's selector and declarations, from any nesting depth.
struct Rule {
    selector: String,
    declarations: Vec<(String, String)>,
    /// Whether the rule sits inside a group rule other than `@layer`.
    conditional: bool,
}

fn rules(file: &str, source: &str) -> Result<Vec<Rule>, String> {
    reject_selector_comments(file, source)?;
    let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
    let mut found = Vec::new();
    collect(file, &nodes, false, &mut found)?;
    Ok(found)
}

/// CSS comments are replaced with whitespace by the structural parser. That
/// is correct for declarations, but a comment embedded in a selector can make
/// an alias or root subject look like unrelated selector pieces to the theme
/// scanner. Reject comments that occur inside a prelude rather than guessing
/// which selector spelling the author intended.
pub(crate) fn reject_selector_comments(file: &str, source: &str) -> Result<(), String> {
    let chars: Vec<char> = source.chars().collect();
    let mut segment_has_text = false;
    let mut index = 0;
    let mut quote = None;
    while index < chars.len() {
        let character = chars[index];
        if let Some(open) = quote {
            if character == '\\' {
                index += 2;
            } else {
                if character == open {
                    quote = None;
                }
                index += 1;
            }
            continue;
        }
        if character == '\\' {
            // An escaped code point is part of an identifier, never a quote or
            // comment opener; this matches the structural parser, which would
            // otherwise see a comment this scan had skipped as string content.
            segment_has_text = true;
            index += 2;
            continue;
        }
        if character == '"' || character == '\'' {
            quote = Some(character);
            segment_has_text = true;
            index += 1;
            continue;
        }
        if character == '/' && chars.get(index + 1) == Some(&'*') {
            let comment_start = index;
            index += 2;
            while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            if index + 1 >= chars.len() {
                return Err(format!(
                    "stylesheet {file}: cannot classify selector comment starting at byte-like position {comment_start}"
                ));
            }
            index += 2;
            if segment_has_text {
                let mut next = index;
                while chars
                    .get(next)
                    .is_some_and(|character| !matches!(character, '{' | ';' | '}'))
                {
                    next += 1;
                }
                if chars.get(next) == Some(&'{') {
                    return Err(format!(
                        "stylesheet {file}: cannot classify selector containing an embedded comment"
                    ));
                }
            }
            continue;
        }
        if matches!(character, '{' | '}' | ';') {
            segment_has_text = false;
        } else if !character.is_whitespace() {
            segment_has_text = true;
        }
        index += 1;
    }
    Ok(())
}

fn collect(
    file: &str,
    nodes: &[Node],
    conditional: bool,
    found: &mut Vec<Rule>,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Group { name, children, .. } => {
                collect(file, children, conditional || name != "layer", found)?;
            }
            Node::Style { prelude, body } => found.push(Rule {
                selector: normalize_selector(prelude),
                declarations: tokens::split_declarations(file, body)?,
                conditional,
            }),
            Node::Statement { .. } | Node::Opaque { .. } => {}
        }
    }
    Ok(())
}

fn normalize_selector(selector: &str) -> String {
    selector.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_color_scheme(name: &str) -> bool {
    name.eq_ignore_ascii_case(COLOR_SCHEME)
}

/// One simple selector the theme contract classifies. Simple selectors inside
/// functional pseudo-classes such as `:where()` count as part of the compound
/// that holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Simple {
    /// `.name`, with escapes decoded.
    Class(String),
    /// `[name ...]`, lowercased, with escapes decoded and any namespace dropped.
    Attribute(String),
    /// `:name` or `::name`, lowercased.
    Pseudo(String),
    /// A type selector, lowercased.
    Type(String),
    /// `#id`, `*`, or `&`, which the contract does not classify further.
    Other,
}

/// A selector list: complex selectors, each a sequence of compounds.
pub(crate) type SelectorList = Vec<Vec<Vec<Simple>>>;

/// Scan a selector list into its simple selectors. The scan recognizes the
/// selector grammar the stylesheets use and rejects anything else, so an
/// escaped, spaced, or namespaced spelling cannot hide a theme hook.
pub(crate) fn scan_selector(selector: &str) -> Result<SelectorList, String> {
    let mut scanner = Scanner {
        chars: selector.chars().collect(),
        pos: 0,
    };
    scanner
        .list(false)
        .map_err(|reason| format!("cannot classify selector `{}`: {reason}", selector.trim()))
}

struct Scanner {
    chars: Vec<char>,
    pos: usize,
}

impl Scanner {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn skip_whitespace(&mut self) -> bool {
        let start = self.pos;
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
        self.pos > start
    }

    /// A selector list, ending at the end of input or, when `nested`, at `)`.
    fn list(&mut self, nested: bool) -> Result<SelectorList, String> {
        let mut list = Vec::new();
        let mut complex: Vec<Vec<Simple>> = Vec::new();
        let mut compound: Vec<Simple> = Vec::new();
        loop {
            if self.skip_whitespace() && !compound.is_empty() {
                complex.push(std::mem::take(&mut compound));
            }
            let Some(c) = self.peek() else {
                if nested {
                    return Err("unterminated `(`".to_owned());
                }
                break;
            };
            match c {
                ')' if nested => {
                    self.pos += 1;
                    break;
                }
                ',' => {
                    self.pos += 1;
                    if !compound.is_empty() {
                        complex.push(std::mem::take(&mut compound));
                    }
                    if complex.is_empty() {
                        return Err("empty selector in a list".to_owned());
                    }
                    list.push(std::mem::take(&mut complex));
                }
                '>' | '+' | '~' => {
                    self.pos += 1;
                    if !compound.is_empty() {
                        complex.push(std::mem::take(&mut compound));
                    }
                }
                '.' => {
                    self.pos += 1;
                    compound.push(Simple::Class(self.ident()?));
                }
                '#' => {
                    self.pos += 1;
                    self.name()?;
                    compound.push(Simple::Other);
                }
                '[' => {
                    self.pos += 1;
                    compound.push(Simple::Attribute(self.attribute()?));
                }
                ':' => {
                    self.pos += 1;
                    if self.peek() == Some(':') {
                        self.pos += 1;
                    }
                    let name = self.ident()?.to_ascii_lowercase();
                    if self.peek() == Some('(') {
                        self.pos += 1;
                        if SELECTOR_PSEUDOS.contains(&name.as_str()) {
                            for inner in self.list(true)? {
                                compound.extend(inner.into_iter().flatten());
                            }
                        } else {
                            self.plain_arguments()?;
                        }
                    }
                    compound.push(Simple::Pseudo(name));
                }
                '&' => {
                    self.pos += 1;
                    compound.push(Simple::Other);
                }
                '*' | '|' => {
                    self.namespace_prefix()?;
                    if self.peek() == Some('*') {
                        self.pos += 1;
                        compound.push(Simple::Other);
                    } else {
                        compound.push(Simple::Type(self.ident()?.to_ascii_lowercase()));
                    }
                }
                _ if starts_ident(c, self.peek_at(1)) => {
                    let name = self.ident()?;
                    if self.peek() == Some('|') && self.peek_at(1) != Some('=') {
                        self.pos += 1;
                        if self.peek() == Some('*') {
                            self.pos += 1;
                            compound.push(Simple::Other);
                        } else {
                            compound.push(Simple::Type(self.ident()?.to_ascii_lowercase()));
                        }
                    } else {
                        compound.push(Simple::Type(name.to_ascii_lowercase()));
                    }
                }
                other => return Err(format!("unexpected `{other}`")),
            }
        }
        if !compound.is_empty() {
            complex.push(compound);
        }
        if complex.is_empty() {
            return Err("empty selector".to_owned());
        }
        list.push(complex);
        Ok(list)
    }

    /// Consume `*|` or `|` when it prefixes a type selector.
    fn namespace_prefix(&mut self) -> Result<(), String> {
        match (self.peek(), self.peek_at(1)) {
            (Some('*'), Some('|')) if self.peek_at(2) != Some('=') => self.pos += 2,
            (Some('|'), next) if next != Some('=') && next != Some('|') => self.pos += 1,
            (Some('|'), _) => return Err("unexpected `|`".to_owned()),
            _ => {}
        }
        Ok(())
    }

    /// `[name]` or `[name op value flag]`, after the `[`; returns the name.
    fn attribute(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        self.namespace_prefix()?;
        let first = self.ident()?;
        let name = if self.peek() == Some('|') && self.peek_at(1) != Some('=') {
            self.pos += 1;
            self.ident()?
        } else {
            first
        };
        self.skip_whitespace();
        match self.peek() {
            Some(']') => {
                self.pos += 1;
                return Ok(name.to_ascii_lowercase());
            }
            Some('=') => self.pos += 1,
            Some('~' | '|' | '^' | '$' | '*') if self.peek_at(1) == Some('=') => self.pos += 2,
            _ => return Err(format!("malformed attribute selector [{name}")),
        }
        self.skip_whitespace();
        match self.peek() {
            Some(quote @ ('"' | '\'')) => {
                self.pos += 1;
                self.string(quote)?;
            }
            Some(c) if starts_ident(c, self.peek_at(1)) => {
                self.ident()?;
            }
            _ => return Err(format!("attribute selector [{name}] has no value")),
        }
        self.skip_whitespace();
        if self
            .peek()
            .is_some_and(|c| starts_ident(c, self.peek_at(1)))
        {
            self.ident()?;
            self.skip_whitespace();
        }
        if self.peek() != Some(']') {
            return Err(format!("unterminated attribute selector [{name}"));
        }
        self.pos += 1;
        Ok(name.to_ascii_lowercase())
    }

    /// Skip a quoted string after its opening quote.
    fn string(&mut self, quote: char) -> Result<(), String> {
        loop {
            match self.peek() {
                None | Some('\n') => return Err("unterminated string".to_owned()),
                Some('\\') => self.pos += 2,
                Some(c) => {
                    self.pos += 1;
                    if c == quote {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Skip the arguments of a functional pseudo-class that takes no selector,
    /// such as `:lang(en)` or `:nth-child(2n + 1)`. An argument that could hold a
    /// selector (for example `:nth-child(2n of .dark)`) is rejected.
    fn plain_arguments(&mut self) -> Result<(), String> {
        let mut depth = 1usize;
        while depth > 0 {
            match self.peek() {
                None => return Err("unterminated `(`".to_owned()),
                Some('(') => depth += 1,
                Some(')') => depth -= 1,
                Some(quote @ ('"' | '\'')) => {
                    self.pos += 1;
                    self.string(quote)?;
                    continue;
                }
                Some(c @ ('.' | '#' | '[' | ':' | '\\')) => {
                    return Err(format!(
                        "`{c}` in the arguments of a pseudo-class this check does not classify"
                    ));
                }
                Some(c) if c.is_ascii_alphabetic() => {
                    let word = self.ident()?;
                    if word.eq_ignore_ascii_case("of") {
                        return Err(
                            "a selector argument (`of`) this check does not classify".to_owned()
                        );
                    }
                    continue;
                }
                Some(_) => {}
            }
            self.pos += 1;
        }
        Ok(())
    }

    /// An identifier with its escapes decoded.
    fn ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(c) if starts_ident(c, self.peek_at(1)) => self.name(),
            Some(c) => Err(format!("expected a name, found `{c}`")),
            None => Err("expected a name".to_owned()),
        }
    }

    /// A run of name code points with escapes decoded.
    fn name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c == '\\' {
                self.pos += 1;
                name.push(self.escape()?);
            } else if is_name_char(c) {
                self.pos += 1;
                name.push(c);
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err("expected a name".to_owned());
        }
        Ok(name)
    }

    /// The code point of an escape, after its backslash.
    fn escape(&mut self) -> Result<char, String> {
        match self.peek() {
            None | Some('\n' | '\r' | '\u{c}') => Err("invalid escape".to_owned()),
            Some(c) if c.is_ascii_hexdigit() => {
                let mut value = 0u32;
                let mut digits = 0;
                while digits < 6 && self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                    value = value * 16 + self.peek().and_then(|c| c.to_digit(16)).unwrap_or(0);
                    self.pos += 1;
                    digits += 1;
                }
                if self.peek().is_some_and(char::is_whitespace) {
                    self.pos += 1;
                }
                Ok(char::from_u32(value)
                    .filter(|&c| c != '\0')
                    .unwrap_or('\u{fffd}'))
            }
            Some(c) => {
                self.pos += 1;
                Ok(c)
            }
        }
    }
}

/// Functional pseudo-classes whose arguments are selectors.
const SELECTOR_PSEUDOS: [&str; 9] = [
    "is",
    "where",
    "not",
    "has",
    "matches",
    "-webkit-any",
    "-moz-any",
    "host",
    "host-context",
];

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

fn starts_ident(c: char, next: Option<char>) -> bool {
    c.is_ascii_alphabetic()
        || c == '_'
        || c == '\\'
        || !c.is_ascii()
        || (c == '-' && next.is_some_and(|n| is_name_char(n) || n == '\\'))
}

/// The theme hook a selector uses other than the public one, if any: a `.dark`
/// class, a Luna `data-theme` or `data-color-scheme` attribute, or another
/// `data-ds-*` attribute.
fn alias_hook(selector: &str, manifest: &Manifest) -> Result<Option<String>, String> {
    for simple in scan_selector(selector)?.iter().flatten().flatten() {
        match simple {
            Simple::Class(name) if name.eq_ignore_ascii_case("dark") => {
                return Ok(Some(".dark".to_owned()));
            }
            Simple::Attribute(name)
                if ALIAS_ATTRIBUTES.contains(&name.as_str())
                    || (name.starts_with(HOOK_PREFIX) && *name != manifest.attribute) =>
            {
                return Ok(Some(format!("[{name}]")));
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Whether any selector in the list targets the root element: its subject
/// compound holds `:root` or `html`, including inside `:is()` or `:where()`.
fn targets_root(selector: &str) -> Result<bool, String> {
    Ok(scan_selector(selector)?.iter().any(|complex| {
        complex.last().is_some_and(|subject| {
            subject.iter().any(|simple| match simple {
                Simple::Pseudo(name) => name == "root",
                Simple::Type(name) => name == "html",
                _ => false,
            })
        })
    }))
}

/// What a validated theme contract covers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub stylesheets: usize,
    pub hook_values: usize,
}

/// Validate the reached Design System stylesheets against the theme inventory.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_stylesheets(
    stylesheets: &[(String, String)],
    manifest: &Manifest,
) -> Result<Summary, String> {
    let mut theme_seen = false;
    for (file, source) in stylesheets {
        let is_theme = file == THEME_FILE;
        theme_seen |= is_theme;
        let found = rules(file, source)?;
        for rule in &found {
            if let Some(alias) =
                alias_hook(&rule.selector, manifest).map_err(|error| format!("{file}: {error}"))?
            {
                return Err(format!(
                    "{file} selects `{}` with theme hook `{alias}`; the only public theme hook is [{}] on :root",
                    rule.selector, manifest.attribute
                ));
            }
            if !is_theme
                && rule
                    .declarations
                    .iter()
                    .any(|(name, _)| is_color_scheme(name))
            {
                return Err(format!(
                    "{file} sets {COLOR_SCHEME} under `{}`; only {THEME_FILE} selects the color scheme",
                    rule.selector
                ));
            }
        }
        if is_theme {
            validate_theme_rules(file, &found, manifest)?;
        }
    }
    if !theme_seen {
        return Err(format!(
            "{THEME_FILE} is not reached from any declared export; the theme contract must enter the cascade"
        ));
    }
    Ok(Summary {
        stylesheets: stylesheets.len(),
        hook_values: manifest.values.len(),
    })
}

/// The theme stylesheet holds exactly one unconditional rule per manifest row,
/// each setting only `color-scheme` to the row's value.
fn validate_theme_rules(file: &str, found: &[Rule], manifest: &Manifest) -> Result<(), String> {
    let mut expected: Vec<(String, String)> = vec![(":root".to_owned(), manifest.default.clone())];
    for value in &manifest.values {
        expected.push((
            format!(":root[{}=\"{value}\"]", manifest.attribute),
            value.clone(),
        ));
    }

    let mut seen = BTreeSet::new();
    for rule in found {
        if rule.conditional {
            return Err(format!(
                "{file} places `{}` inside a conditional group rule; the theme contract is unconditional and `light dark` already follows the preference",
                rule.selector
            ));
        }
        let Some((_, value)) = expected
            .iter()
            .find(|(selector, _)| *selector == rule.selector)
        else {
            return Err(format!(
                "{file} has selector `{}`, which theme.tsv does not classify; expected only {}",
                rule.selector,
                expected
                    .iter()
                    .map(|(selector, _)| format!("`{selector}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        if !seen.insert(rule.selector.clone()) {
            return Err(format!(
                "{file} repeats selector `{}`; each theme state has one rule",
                rule.selector
            ));
        }
        match rule.declarations.as_slice() {
            [(name, declared)] if is_color_scheme(name) && declared == value => {}
            [(name, _)] if name.starts_with("--") => {
                return Err(format!(
                    "{file} declares {name} under `{}`; the theme selects {COLOR_SCHEME} and assigns no token",
                    rule.selector
                ));
            }
            _ => {
                return Err(format!(
                    "{file} `{}` must contain exactly `{COLOR_SCHEME}: {value};`",
                    rule.selector
                ));
            }
        }
    }
    if let Some((selector, _)) = expected
        .iter()
        .find(|(selector, _)| !seen.contains(selector))
    {
        return Err(format!(
            "{file} does not implement `{selector}`, which theme.tsv classifies"
        ));
    }
    Ok(())
}

/// A consumer stylesheet may style under the public hook, but must not use an
/// alias hook or set `color-scheme` on the root: a consumer layer outranks
/// `ds`, so a root `color-scheme` there silently disables the public hook.
pub fn validate_consumer(file: &str, source: &str, manifest: &Manifest) -> Result<(), String> {
    for rule in rules(file, source)? {
        if let Some(alias) = alias_hook(&rule.selector, manifest)
            .map_err(|error| format!("consumer stylesheet {file}: {error}"))?
        {
            return Err(format!(
                "consumer stylesheet {file} selects `{}` with theme hook `{alias}`; use [{}] on :root",
                rule.selector, manifest.attribute
            ));
        }
        if targets_root(&rule.selector)
            .map_err(|error| format!("consumer stylesheet {file}: {error}"))?
            && rule
                .declarations
                .iter()
                .any(|(name, _)| is_color_scheme(name))
        {
            return Err(format!(
                "consumer stylesheet {file} sets {COLOR_SCHEME} on `{}`; a consumer layer outranks the Design System, so this disables [{}]",
                rule.selector, manifest.attribute
            ));
        }
    }
    Ok(())
}

/// Heading of the theme document section that shows the default rule.
const DEFAULT_SECTION: &str = "## Default";
/// Heading of the theme document section whose table states the hook behavior.
const HOOK_SECTION: &str = "## Explicit light or dark";

/// The theme document states the contract, not just its names: the default
/// section shows exactly the default rule, the hook section's behavior table
/// has exactly one row per theme state with the scheme `theme.tsv` classifies,
/// and the document names no hook value, `data-ds-*` attribute, or alias
/// outside the contract.
pub fn validate_document(document: &str, manifest: &Manifest) -> Result<(), String> {
    validate_default_section(section(document, DEFAULT_SECTION)?, manifest)?;
    validate_hook_table(section(document, HOOK_SECTION)?, manifest)?;

    let lower = document.to_ascii_lowercase();
    for alias in ALIAS_ATTRIBUTES {
        if lower.contains(alias) {
            return Err(format!(
                "the theme document names `{alias}`; document only the public hook {}",
                manifest.attribute
            ));
        }
    }
    for (index, _) in lower.match_indices(HOOK_PREFIX) {
        let name: String = lower[index..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if name != manifest.attribute {
            return Err(format!(
                "the theme document names `{name}`; the only public hook is {}",
                manifest.attribute
            ));
        }
        let rest = &lower[index + name.len()..];
        if let Some(quoted) = rest.strip_prefix("=\"") {
            let value = quoted.split('"').next().unwrap_or_default();
            if !manifest.values.iter().any(|known| known == value) {
                return Err(format!(
                    "the theme document shows {name}=\"{value}\", which theme.tsv does not classify"
                ));
            }
        } else if rest.starts_with('=') {
            return Err(format!(
                "the theme document shows an unquoted {name} value; quote hook values as theme.tsv classifies them"
            ));
        }
    }
    Ok(())
}

/// The body of the one `heading` section, up to the next `#` or `##` heading.
fn section<'a>(document: &'a str, heading: &str) -> Result<&'a str, String> {
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
            "the theme document must have exactly one `{heading}` section, found {}",
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

/// The default section holds exactly one `css` block: `:root` with only the
/// default `color-scheme`.
fn validate_default_section(body: &str, manifest: &Manifest) -> Result<(), String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in body.lines() {
        let fence = line.trim();
        match current.as_mut() {
            None if fence.starts_with("```") => {
                if fence != "```css" {
                    return Err(format!(
                        "the theme document `{DEFAULT_SECTION}` section has a non-css code block `{fence}`"
                    ));
                }
                current = Some(String::new());
            }
            None => {}
            Some(_) if fence == "```" => blocks.extend(current.take()),
            Some(text) => {
                text.push_str(line);
                text.push('\n');
            }
        }
    }
    if current.is_some() {
        return Err(format!(
            "the theme document `{DEFAULT_SECTION}` section has an unterminated code block"
        ));
    }
    let [css_text] = blocks.as_slice() else {
        return Err(format!(
            "the theme document `{DEFAULT_SECTION}` section must show exactly one css block, found {}",
            blocks.len()
        ));
    };
    let expected = format!(":root {{ {COLOR_SCHEME}: {}; }}", manifest.default);
    let wrong =
        || format!("the theme document `{DEFAULT_SECTION}` block must be exactly `{expected}`");
    let nodes = css::parse(css_text).map_err(|error| format!("{}: {error}", wrong()))?;
    let [Node::Style { prelude, body }] = nodes.as_slice() else {
        return Err(wrong());
    };
    let declarations = tokens::split_declarations("theme document", body)?;
    let [(name, value)] = declarations.as_slice() else {
        return Err(wrong());
    };
    if normalize_selector(prelude) != ":root" || name != COLOR_SCHEME || *value != manifest.default
    {
        return Err(wrong());
    }
    Ok(())
}

/// The hook section's behavior table has exactly the rows `no attribute`, one
/// per hook value, and `any other value`, each with the `color-scheme` that
/// `theme.tsv` gives it and a matching resolution.
fn validate_hook_table(body: &str, manifest: &Manifest) -> Result<(), String> {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .collect();
    let context = format!("the theme document `{HOOK_SECTION}` table");
    let [_header, separator, rows @ ..] = lines.as_slice() else {
        return Err(format!("{context} is missing"));
    };
    if !cells(separator)
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'))
    {
        return Err(format!("{context} has no header separator row"));
    }

    let default_resolution = if manifest.default == "light dark" {
        "the user's preference".to_owned()
    } else {
        manifest.default.clone()
    };
    let mut expected: Vec<(String, String, String)> = vec![(
        "no attribute".to_owned(),
        format!("`{}`", manifest.default),
        default_resolution.clone(),
    )];
    for value in &manifest.values {
        expected.push((
            format!("`{}=\"{value}\"`", manifest.attribute),
            format!("`{value}`"),
            value.clone(),
        ));
    }
    expected.push((
        "any other value".to_owned(),
        format!("`{}`", manifest.default),
        default_resolution,
    ));

    let mut seen = BTreeSet::new();
    for row in rows {
        let row_cells = cells(row);
        let [markup, scheme, resolves] = row_cells.as_slice() else {
            return Err(format!(
                "{context} row `{row}` must have markup, color-scheme, and resolution cells"
            ));
        };
        let Some((_, want_scheme, want_resolution)) =
            expected.iter().find(|(want, _, _)| want == markup)
        else {
            return Err(format!(
                "{context} has row `{markup}`, which theme.tsv does not classify"
            ));
        };
        if !seen.insert(markup.clone()) {
            return Err(format!("{context} repeats row `{markup}`"));
        }
        if scheme != want_scheme {
            return Err(format!(
                "{context} gives `{markup}` color-scheme {scheme}; theme.tsv gives {want_scheme}"
            ));
        }
        if !resolves.starts_with(want_resolution.as_str()) {
            return Err(format!(
                "{context} says `{markup}` resolves `{resolves}`; it resolves {want_resolution}"
            ));
        }
    }
    if let Some((markup, _, _)) = expected
        .iter()
        .find(|(markup, _, _)| !seen.contains(markup))
    {
        return Err(format!("{context} has no row for `{markup}`"));
    }
    Ok(())
}

/// The trimmed cells of one Markdown table row.
fn cells(row: &str) -> Vec<String> {
    let inner = row.trim().trim_start_matches('|');
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
# comment
public-preview\tdefault\tcolor-scheme\tlight dark
public-preview\tattribute\tdata-ds-scheme\tlight dark
";

    const THEME: &str = "\
:root { color-scheme: light dark; }
:root[data-ds-scheme=\"light\"] { color-scheme: light; }
:root[data-ds-scheme=\"dark\"] { color-scheme: dark; }
";

    const SEMANTIC: &str =
        ":root { --ds-color-text: light-dark(var(--ds-ref-a), var(--ds-ref-b)); }";

    fn manifest() -> Manifest {
        parse_manifest(MANIFEST).expect("manifest")
    }

    fn sheets(theme: &str, semantic: &str) -> Vec<(String, String)> {
        vec![
            (
                "packages/styles/tokens/semantic.css".to_owned(),
                semantic.to_owned(),
            ),
            (THEME_FILE.to_owned(), theme.to_owned()),
        ]
    }

    fn reject(theme: &str, semantic: &str) -> String {
        validate_stylesheets(&sheets(theme, semantic), &manifest())
            .expect_err("theme contract must be rejected")
    }

    #[test]
    fn manifest_parses_one_default_and_one_hook() {
        assert_eq!(
            manifest(),
            Manifest {
                default: "light dark".to_owned(),
                attribute: "data-ds-scheme".to_owned(),
                values: vec!["light".to_owned(), "dark".to_owned()],
            }
        );
    }

    #[test]
    fn manifest_rejects_aliases_classes_and_unknown_values() {
        let second = format!("{MANIFEST}public-preview\tattribute\tdata-ds-theme\tdark\n");
        assert!(parse_manifest(&second).unwrap_err().contains("second hook"));
        let stable = MANIFEST.replace("public-preview\tattribute", "public-stable\tattribute");
        assert!(
            parse_manifest(&stable)
                .unwrap_err()
                .contains("public-stable")
        );
        let blue = MANIFEST.replace("\tlight dark\n", "\tlight blue\n");
        assert!(parse_manifest(&blue).is_err());
        let luna = MANIFEST.replace("data-ds-scheme", "data-theme");
        assert!(parse_manifest(&luna).unwrap_err().contains("data-ds-"));
        let repeated = MANIFEST.replace("data-ds-scheme\tlight dark", "data-ds-scheme\tdark dark");
        assert!(parse_manifest(&repeated).unwrap_err().contains("repeats"));
        let missing = "public-preview\tdefault\tcolor-scheme\tlight dark\n";
        assert!(
            parse_manifest(missing)
                .unwrap_err()
                .contains("no attribute hook")
        );
    }

    #[test]
    fn repository_shaped_theme_passes() {
        let summary = validate_stylesheets(&sheets(THEME, SEMANTIC), &manifest()).expect("theme");
        assert_eq!(summary.hook_values, 2);
    }

    #[test]
    fn theme_must_not_assign_tokens() {
        let forked = THEME.replace("{ color-scheme: dark; }", "{ --ds-color-text: red; }");
        assert!(reject(&forked, SEMANTIC).contains("assigns no token"));
        let extra = THEME.replace(
            "{ color-scheme: dark; }",
            "{ color-scheme: dark; --ds-color-text: red; }",
        );
        assert!(reject(&extra, SEMANTIC).contains("exactly"));
    }

    #[test]
    fn theme_rejects_unclassified_missing_or_repeated_states() {
        let blue = format!("{THEME}:root[data-ds-scheme=\"blue\"] {{ color-scheme: dark; }}\n");
        assert!(reject(&blue, SEMANTIC).contains("does not classify"));
        let missing = THEME.replace(
            ":root[data-ds-scheme=\"dark\"] { color-scheme: dark; }\n",
            "",
        );
        assert!(reject(&missing, SEMANTIC).contains("does not implement"));
        let repeated = format!("{THEME}:root[data-ds-scheme=\"dark\"] {{ color-scheme: dark; }}\n");
        assert!(reject(&repeated, SEMANTIC).contains("repeats"));
        let wrong = THEME.replace("{ color-scheme: dark; }", "{ color-scheme: light; }");
        assert!(reject(&wrong, SEMANTIC).contains("exactly"));
        let descendant = THEME.replace(
            ":root[data-ds-scheme=\"dark\"]",
            "[data-ds-scheme=\"dark\"]",
        );
        assert!(reject(&descendant, SEMANTIC).contains("does not classify"));
    }

    #[test]
    fn theme_rejects_conditional_rules() {
        let media = format!(
            "{THEME}@media (prefers-color-scheme: dark) {{ :root {{ color-scheme: dark; }} }}\n"
        );
        assert!(reject(&media, SEMANTIC).contains("conditional"));
    }

    #[test]
    fn only_the_theme_sets_color_scheme() {
        let semantic = format!("{SEMANTIC}\n:root {{ color-scheme: light dark; }}");
        assert!(reject(THEME, &semantic).contains("only packages/styles/tokens/theme.css"));
        let upper = format!("{SEMANTIC}\nmain {{ Color-Scheme: dark; }}");
        assert!(reject(THEME, &upper).contains("only"));
    }

    #[test]
    fn alias_hooks_are_rejected_in_design_system_stylesheets() {
        for alias in [
            ".dark { color: red; }",
            ":root.dark { color: red; }",
            "[data-theme=\"dark\"] { color: red; }",
            "[data-ds-theme=\"dark\"] { color: red; }",
            "[data-color-scheme=dark] { color: red; }",
        ] {
            let semantic = format!("{SEMANTIC}\n{alias}");
            assert!(reject(THEME, &semantic).contains("theme hook"), "{alias}");
        }
        // Class names that merely start with `dark` are not the alias.
        let semantic = format!("{SEMANTIC}\n.darkroom {{ color: red; }}");
        validate_stylesheets(&sheets(THEME, &semantic), &manifest()).expect("not an alias");
    }

    #[test]
    fn theme_must_be_reached() {
        let only = vec![(
            "packages/styles/tokens/semantic.css".to_owned(),
            SEMANTIC.to_owned(),
        )];
        assert!(
            validate_stylesheets(&only, &manifest())
                .unwrap_err()
                .contains("not reached")
        );
    }

    #[test]
    fn consumers_may_style_under_the_hook_but_not_override_it() {
        let manifest = manifest();
        validate_consumer(
            "c.css",
            "@layer app { :root[data-ds-scheme=\"dark\"] img { opacity: 0.9; } main { color-scheme: dark; } }",
            &manifest,
        )
        .expect("styling under the hook and a non-root scheme are allowed");
        for source in [
            "@layer app { :root { color-scheme: light dark; } }",
            "@layer app { html { color-scheme: dark; } }",
            "@layer app { body, :root { color-scheme: dark; } }",
            "@layer app { [data-theme=dark] { color: red; } }",
            "@layer app { .dark main { color: red; } }",
        ] {
            assert!(
                validate_consumer("c.css", source, &manifest).is_err(),
                "{source}"
            );
        }
    }

    const DOCUMENT: &str = "\
# Theme contract

## Default

```css
:root {
  color-scheme: light dark;
}
```

## Explicit light or dark

| Markup | `color-scheme` on the root | `light-dark()` roles resolve |
|---|---|---|
| no attribute | `light dark` | the user's preference |
| `data-ds-scheme=\"light\"` | `light` | light, whatever the preference |
| `data-ds-scheme=\"dark\"` | `dark` | dark, whatever the preference |
| any other value | `light dark` | the user's preference |

```html
<html lang=\"en\" data-ds-scheme=\"dark\">
```

## Classification and checks

Rejects a `.dark` class.
";

    #[test]
    fn document_states_the_contract() {
        let manifest = manifest();
        validate_document(DOCUMENT, &manifest).expect("document");
        for (from, to, expected) in [
            // The behavior table must give each state its classified scheme.
            (
                "`data-ds-scheme=\"dark\"` | `dark` | dark",
                "`data-ds-scheme=\"dark\"` | `light` | dark",
                "theme.tsv gives `dark`",
            ),
            (
                "`data-ds-scheme=\"dark\"` | `dark` | dark,",
                "`data-ds-scheme=\"dark\"` | `dark` | light,",
                "resolves",
            ),
            (
                "| any other value | `light dark` |",
                "| any other value | `dark` |",
                "theme.tsv gives",
            ),
            (
                "| `data-ds-scheme=\"dark\"` | `dark` | dark, whatever the preference |\n",
                "",
                "no row for",
            ),
            (
                "| no attribute |",
                "| `data-ds-scheme=\"system\"` | `light dark` | the user's preference |\n| no attribute |",
                "does not classify",
            ),
            (
                "| no attribute | `light dark` | the user's preference |\n",
                "| no attribute | `light dark` | the user's preference |\n| no attribute | `light dark` | the user's preference |\n",
                "repeats",
            ),
            // The default block must be exactly the default rule.
            (
                "color-scheme: light dark;\n}",
                "color-scheme: light;\n}",
                "exactly",
            ),
            (":root {\n  color", "html {\n  color", "exactly"),
            ("```css\n:root", "```scss\n:root", "non-css"),
            // No undocumented value, hook, or alias anywhere.
            (
                "data-ds-scheme=\"dark\">",
                "data-ds-scheme=\"system\">",
                "does not classify",
            ),
            (
                "Rejects a `.dark` class.",
                "Set `data-ds-theme`.",
                "data-ds-theme",
            ),
            (
                "Rejects a `.dark` class.",
                "Luna used `data-theme`.",
                "data-theme",
            ),
            ("## Default\n", "## Defaults\n", "`## Default` section"),
        ] {
            assert!(DOCUMENT.contains(from), "{from}");
            let changed = DOCUMENT.replacen(from, to, 1);
            let error = validate_document(&changed, &manifest).expect_err(to);
            assert!(error.contains(expected), "{to}: {error}");
        }
    }

    #[test]
    fn spelled_alias_hooks_are_rejected() {
        let manifest = manifest();
        for selector in [
            "[ data-theme=\"dark\"] p",
            "[data\\-theme=\"dark\"] p",
            "[data-\\74 heme] p",
            ".d\\61rk p",
            ".\\64 ark p",
            ".DARK p",
            "[*|data-theme] p",
            "[svg|data-color-scheme] p",
            "[DATA-DS-THEME] p",
            "[ data-ds-mode ] p",
            ":is(main, :where(.dark)) p",
            ":not([data-theme]) p",
        ] {
            let source = format!("@layer app {{ {selector} {{ color: red; }} }}");
            let error = validate_consumer("c.css", &source, &manifest).expect_err(selector);
            assert!(error.contains("theme hook"), "{selector}: {error}");
        }
    }

    #[test]
    fn hook_names_inside_values_or_longer_names_are_not_aliases() {
        let manifest = manifest();
        for selector in [
            "[aria-label=\".dark\"] p",
            "a[href=\"[data-theme]\"] p",
            "a[title='data-color-scheme'] p",
            ".darkroom p",
            ".dark-mode-toggle p",
            "[data-themes] p",
            "#dark p",
            ":root[data-ds-scheme=\"dark\"] img",
            ":root[ data-ds-scheme = dark i ] img",
            "li:nth-child(2n + 1):lang(en)",
            "svg|rect, *|*",
        ] {
            let source = format!("@layer app {{ {selector} {{ color: red; }} }}");
            validate_consumer("c.css", &source, &manifest).expect(selector);
        }
    }

    #[test]
    fn unclassifiable_selectors_fail_closed() {
        let manifest = manifest();
        for selector in [
            "[data-theme",
            "li:nth-child(2n of .dark)",
            "li:nth-child(2n + \\31)",
            "[=x] p",
            "a, , b",
            "p ::",
            ".d/* comment */ark p",
            ":r/* comment */oot",
            ".x\\\", .d/* comment */ark p",
            ".x\\', :r/* comment */oot",
            ".x\\\" { } .y { content: \"a\"; } .d/* comment */ark p",
        ] {
            let source = format!("@layer app {{ {selector} {{ color: red; }} }}");
            let error = validate_consumer("c.css", &source, &manifest).expect_err(selector);
            assert!(error.contains("cannot classify"), "{selector}: {error}");
        }
    }

    #[test]
    fn spelled_root_color_scheme_overrides_are_rejected() {
        let manifest = manifest();
        for selector in [
            ":where(:root)",
            ":is(html)",
            "html:root",
            "HTML",
            ":ROOT",
            "main, :root",
            "body > p, html",
        ] {
            let source = format!("@layer app {{ {selector} {{ color-scheme: dark; }} }}");
            let error = validate_consumer("c.css", &source, &manifest).expect_err(selector);
            assert!(error.contains("disables"), "{selector}: {error}");
        }
        for selector in [":where(:root) main", ":root > body", "html body", "main"] {
            let source = format!("@layer app {{ {selector} {{ color-scheme: dark; }} }}");
            validate_consumer("c.css", &source, &manifest).expect(selector);
        }
    }
}
