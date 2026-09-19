//! Hook checks: the published CSS exposes exactly the public hook vocabulary
//! that `layouts.tsv`, `primitives.tsv`, and `theme.tsv` classify, and every
//! rule that selects a hook stays inside the element that carries it.
//!
//! The scoping boundary is the hook itself: every hooked selector is anchored
//! at one element, and only a layout reaches one step further, to its direct
//! children. No rule depends on an ancestor, a sibling, or a descendant, and no
//! stylesheet uses `@scope`, which is only progressive at the accepted browser
//! floor. Class hooks are the vocabulary; the only attribute hook is the theme
//! attribute, and every other `data-*` name is reserved.
//!
//! Like the other checks, this recognizes only the shapes the stylesheets use
//! and rejects anything else rather than guessing.

use std::collections::{BTreeMap, BTreeSet};

use crate::base;
use crate::css::{self, Node};
use crate::layout;
use crate::primitive::{self, Kind};
use crate::theme::{self, Simple};

const PUBLIC_PREVIEW: &str = "public-preview";
/// Prefix every Design System class hook carries.
const HOOK_PREFIX: &str = "ds-";

/// The kinds of public hook, as the hooks document names them.
const LAYOUT: &str = "layout";
const BASE_PRIMITIVE: &str = "base-primitive";
const UI_PRIMITIVE: &str = "ui-primitive";
const VARIANT: &str = "variant";
const THEME_ATTRIBUTE: &str = "theme-attribute";

/// Heading of the hooks document section that lists every public hook.
const HOOKS_SECTION: &str = "## Public hooks";

/// One public hook: its kind, the module or contract that owns it, and its
/// compatibility class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    pub kind: &'static str,
    pub owner: String,
    pub class: &'static str,
}

/// The public hook vocabulary, derived from the three inventories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    /// Class hooks, without the leading `.`.
    pub classes: BTreeMap<String, Hook>,
    /// Attribute hooks.
    pub attributes: BTreeMap<String, Hook>,
}

impl Vocabulary {
    fn kind(&self, class: &str) -> Option<&'static str> {
        self.classes.get(class).map(|hook| hook.kind)
    }

    /// Every hook as the document spells it: `.class` or the attribute name.
    fn spelled(&self) -> BTreeMap<String, &Hook> {
        self.classes
            .iter()
            .map(|(class, hook)| (format!(".{class}"), hook))
            .chain(
                self.attributes
                    .iter()
                    .map(|(name, hook)| (name.clone(), hook)),
            )
            .collect()
    }

    pub fn len(&self) -> usize {
        self.classes.len() + self.attributes.len()
    }
}

/// Build the vocabulary from the validated inventories.
pub fn vocabulary(
    layouts: &layout::Manifest,
    primitives: &primitive::Manifest,
    theme: &theme::Manifest,
) -> Vocabulary {
    let mut classes = BTreeMap::new();
    for name in &layouts.layouts {
        classes.insert(
            format!("{HOOK_PREFIX}{name}"),
            Hook {
                kind: LAYOUT,
                owner: name.clone(),
                class: PUBLIC_PREVIEW,
            },
        );
    }
    for item in &primitives.primitives {
        let kind = match item.kind {
            Kind::Base => BASE_PRIMITIVE,
            Kind::Ui => UI_PRIMITIVE,
        };
        classes.insert(
            item.hook(),
            Hook {
                kind,
                owner: item.name.clone(),
                class: PUBLIC_PREVIEW,
            },
        );
        for variant in &item.variants {
            classes.insert(
                item.variant_hook(variant),
                Hook {
                    kind: VARIANT,
                    owner: item.name.clone(),
                    class: PUBLIC_PREVIEW,
                },
            );
        }
    }
    let mut attributes = BTreeMap::new();
    attributes.insert(
        theme.attribute.clone(),
        Hook {
            kind: THEME_ATTRIBUTE,
            owner: "theme".to_owned(),
            class: PUBLIC_PREVIEW,
        },
    );
    Vocabulary {
        classes,
        attributes,
    }
}

/// What the stylesheet pass covered.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub stylesheets: usize,
    pub rules: usize,
    pub hooked: usize,
}

/// Validate every reached Design System stylesheet against the vocabulary.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_stylesheets(
    stylesheets: &[(String, String)],
    vocabulary: &Vocabulary,
) -> Result<Summary, String> {
    let mut summary = Summary::default();
    for (file, source) in stylesheets {
        theme::reject_selector_comments(file, source)?;
        let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
        check_nodes(file, &nodes, vocabulary, &mut summary)?;
        summary.stylesheets += 1;
    }
    Ok(summary)
}

fn check_nodes(
    file: &str,
    nodes: &[Node],
    vocabulary: &Vocabulary,
    summary: &mut Summary,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Statement { name, .. } | Node::Opaque { name, .. } => check_at_rule(file, name)?,
            Node::Group { name, children, .. } => {
                check_at_rule(file, name)?;
                check_nodes(file, children, vocabulary, summary)?;
            }
            Node::Style { prelude, body } => {
                // Messages show the selector on one line, but the check reads the
                // raw text: the whitespace after a hex escape is part of the
                // escape, so collapsing it first could hide a combinator.
                let selector = prelude.split_whitespace().collect::<Vec<_>>().join(" ");
                if body.contains('{') {
                    return Err(format!(
                        "{file} `{selector}` nests a rule; a nested selector's reach cannot be bounded by its hook"
                    ));
                }
                if check_selector(prelude.trim(), vocabulary)
                    .map_err(|error| format!("{file} `{selector}`: {error}"))?
                {
                    summary.hooked += 1;
                }
                summary.rules += 1;
            }
        }
    }
    Ok(())
}

/// `@scope` is progressive at the browser floor, so no baseline rule may sit
/// behind it; an escaped at-keyword cannot hide it.
fn check_at_rule(file: &str, name: &str) -> Result<(), String> {
    if name.contains('\\') {
        return Err(format!(
            "{file} spells an at-rule with an escape (`@{name}`); write at-rules literally"
        ));
    }
    if name.eq_ignore_ascii_case("scope") {
        return Err(format!(
            "{file} uses `@scope`; it is only progressive at the browser floor, so a hook's own \
selector is the scoping boundary"
        ));
    }
    Ok(())
}

/// Pseudo-classes a hooked selector may use. Each tests the hooked element's
/// own state or position as the document root; none depends on an ancestor,
/// a sibling, a descendant, or a shadow tree. Anything else fails closed.
const HOOKED_PSEUDOS: [&str; 7] = [
    "where", "is", "not", "hover", "disabled", "any-link", "root",
];

/// Selector pseudo-classes whose argument is a list of alternatives.
const ALTERNATIVE_PSEUDOS: [&str; 5] = ["is", "where", "matches", "-webkit-any", "-moz-any"];

/// Check one selector list, as written. Returns whether it selects a public
/// hook.
///
/// Every class is a declared hook and every `data-*` attribute is a declared
/// attribute hook. A hooked complex selector holds its hooks in its first
/// compound, which is the element that carries them:
///
/// - no hook is negated, and no alternative list offers a branch without it;
/// - no combinator hides inside a pseudo-class argument;
/// - it uses only the state pseudo-classes in `HOOKED_PSEUDOS`, so no `:has()`,
///   structural, focus-within, or shadow-tree selector;
/// - the theme attribute sits on `:root`;
/// - it has no second compound, except that a layout reaches its direct
///   children with `> :where(*)`.
fn check_selector(selector: &str, vocabulary: &Vocabulary) -> Result<bool, String> {
    let list = theme::scan_selector(selector)?;
    let combinators = top_level_combinators(selector)?;
    if combinators.len() != list.len() {
        return Err("cannot pair its combinators with its selectors".to_owned());
    }
    let arguments = pseudo_arguments(selector)?;
    let mut nested = false;
    let mut alternatives = false;
    let mut negated = false;
    for (name, argument) in &arguments {
        let branches = top_level_combinators(argument)?;
        nested |= branches.iter().any(|combinators| !combinators.is_empty());
        if ALTERNATIVE_PSEUDOS.contains(&name.as_str()) {
            alternatives |= branches.len() > 1;
        }
        if name == "not" {
            negated |= theme::scan_selector(argument)?
                .iter()
                .flatten()
                .flatten()
                .any(|simple| match simple {
                    Simple::Class(_) => true,
                    Simple::Attribute(attribute) => attribute.starts_with("data-"),
                    _ => false,
                });
        }
    }
    let mut hooked = false;
    for (complex, combinators) in list.iter().zip(&combinators) {
        let mut hooks = Vec::new();
        for (index, compound) in complex.iter().enumerate() {
            for simple in compound {
                match simple {
                    Simple::Class(name) => {
                        let Some(kind) = vocabulary.kind(name) else {
                            return Err(format!(
                                "selects class `.{name}`, which is not a public hook; \
the hooks are the promoted layouts and primitives"
                            ));
                        };
                        if index > 0 {
                            return Err(format!(
                                "reaches hook `.{name}` through another element; a hooked rule is anchored \
at the element that carries the hook"
                            ));
                        }
                        hooks.push(kind);
                    }
                    Simple::Attribute(name) if name.starts_with("data-") => {
                        let Some(hook) = vocabulary.attributes.get(name) else {
                            return Err(format!(
                                "tests `{name}`; every `data-*` name other than the theme attribute is reserved"
                            ));
                        };
                        if index > 0 {
                            return Err(format!(
                                "reaches attribute hook `{name}` through another element; a hooked rule is \
anchored at the element that carries the hook"
                            ));
                        }
                        hooks.push(hook.kind);
                    }
                    _ => {}
                }
            }
        }
        if hooks.is_empty() {
            continue;
        }
        hooked = true;
        if negated {
            return Err(
                "negates a hook inside `:not()`, which selects every element without it".to_owned(),
            );
        }
        if alternatives {
            return Err(
                "offers alternatives in `:is()` or `:where()` beside a hook; a branch without the hook \
styles elements that do not carry it"
                    .to_owned(),
            );
        }
        if nested {
            return Err(
                "hides a combinator inside a pseudo-class argument, which reaches past the hooked element"
                    .to_owned(),
            );
        }
        if complex[0]
            .iter()
            .any(|simple| *simple == Simple::Pseudo("has".to_owned()))
        {
            return Err(
                "puts a hook beside `:has()`, which reaches past the hooked element".to_owned(),
            );
        }
        if let Some(pseudo) = complex.iter().flatten().find_map(|simple| match simple {
            Simple::Pseudo(name) if !HOOKED_PSEUDOS.contains(&name.as_str()) => Some(name),
            _ => None,
        }) {
            return Err(format!(
                "uses `:{pseudo}` beside a hook; a hooked rule tests only the element's own state \
({})",
                HOOKED_PSEUDOS.map(|name| format!(":{name}")).join(", ")
            ));
        }
        if hooks.contains(&THEME_ATTRIBUTE)
            && !complex[0].contains(&Simple::Pseudo("root".to_owned()))
        {
            return Err("tests the theme attribute away from `:root`".to_owned());
        }
        match complex.as_slice() {
            [_] => {}
            [_, child] => {
                let only_layouts = hooks.iter().all(|kind| *kind == LAYOUT);
                let any_child = child.iter().all(|simple| {
                    matches!(simple, Simple::Other) || *simple == Simple::Pseudo("where".to_owned())
                });
                if !(only_layouts && combinators.as_slice() == ['>'] && any_child) {
                    return Err(
                        "reaches past the hooked element; only a layout reaches its direct children, \
with `> :where(*)`"
                            .to_owned(),
                    );
                }
            }
            _ => {
                return Err(
                    "reaches past the hooked element through more than one combinator".to_owned(),
                );
            }
        }
    }
    Ok(hooked)
}

/// Pseudo-classes whose arguments are selectors.
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

/// The argument of every functional selector pseudo-class, at any depth, with
/// its lowercase name: `:where(nav .x)` gives `("where", "nav .x")`.
fn pseudo_arguments(selector: &str) -> Result<Vec<(String, String)>, String> {
    let chars: Vec<char> = selector.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    let mut quote: Option<char> = None;
    let mut bracket = 0usize;
    while index < chars.len() {
        let c = chars[index];
        if let Some(open) = quote {
            if c == '\\' {
                index += 1;
            } else if c == open {
                quote = None;
            }
            index += 1;
            continue;
        }
        match c {
            '\\' => index = escape_last(&chars, index),
            '"' | '\'' => quote = Some(c),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '(' if bracket == 0 => {
                let name: String = chars[..index]
                    .iter()
                    .rev()
                    .take_while(|c| c.is_ascii_alphanumeric() || **c == '-')
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let close = matching_paren(&chars, index)?;
                let name = name.to_ascii_lowercase();
                if SELECTOR_PSEUDOS.contains(&name.as_str()) {
                    let argument: String = chars[index + 1..close].iter().collect();
                    found.extend(pseudo_arguments(&argument)?);
                    found.push((name, argument));
                }
                index = close;
            }
            _ => {}
        }
        index += 1;
    }
    Ok(found)
}

/// Whether any selector argument of a functional pseudo-class, at any depth,
/// holds a combinator: `:where(nav .x)` or `:is(a > b)`.
#[cfg(test)]
fn nested_combinators(selector: &str) -> Result<bool, String> {
    for (_, argument) in pseudo_arguments(selector)? {
        if top_level_combinators(&argument)?
            .iter()
            .any(|combinators| !combinators.is_empty())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The index of the last character of the escape whose backslash is at
/// `backslash`: up to six hex digits and one following whitespace character,
/// or one other character. The whitespace ends the escape; it is not a
/// descendant combinator.
fn escape_last(chars: &[char], backslash: usize) -> usize {
    let start = backslash + 1;
    let mut end = start;
    while end < chars.len() && end - start < 6 && chars[end].is_ascii_hexdigit() {
        end += 1;
    }
    if end == start {
        return start.min(chars.len().saturating_sub(1));
    }
    if end < chars.len() && chars[end].is_whitespace() {
        end += 1;
    }
    end - 1
}

/// The index of the `)` that closes the `(` at `open`.
fn matching_paren(chars: &[char], open: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut index = open;
    while index < chars.len() {
        let c = chars[index];
        if let Some(q) = quote {
            if c == '\\' {
                index += 1;
            } else if c == q {
                quote = None;
            }
        } else {
            match c {
                '\\' => index = escape_last(chars, index),
                '"' | '\'' => quote = Some(c),
                '(' => depth += 1,
                ')' => {
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
    Err("has an unterminated `(`".to_owned())
}

/// The combinators between the compounds of each top-level complex selector:
/// `' '` for a descendant, or `>`, `+`, `~`. Brackets, parentheses, strings,
/// and escapes are skipped, so only top-level combinators count.
fn top_level_combinators(selector: &str) -> Result<Vec<Vec<char>>, String> {
    let chars: Vec<char> = selector.trim().chars().collect();
    let mut list = vec![Vec::new()];
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut pending: Option<char> = None;
    let mut started = false;
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        index += 1;
        if let Some(open) = quote {
            if c == '\\' {
                index += 1;
            } else if c == open {
                quote = None;
            }
            continue;
        }
        if depth > 0 {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                '"' | '\'' => quote = Some(c),
                '\\' => index = escape_last(&chars, index - 1) + 1,
                _ => {}
            }
            continue;
        }
        match c {
            ',' => {
                list.push(Vec::new());
                pending = None;
                started = false;
            }
            '>' | '+' | '~' => pending = Some(c),
            c if c.is_whitespace() => {
                if started && pending.is_none() {
                    pending = Some(' ');
                }
            }
            _ => {
                if let Some(combinator) = pending.take() {
                    list.last_mut()
                        .ok_or("empty selector list")?
                        .push(combinator);
                }
                started = true;
                match c {
                    '(' | '[' => depth += 1,
                    '"' | '\'' => quote = Some(c),
                    '\\' => index = escape_last(&chars, index - 1) + 1,
                    _ => {}
                }
            }
        }
    }
    if depth > 0 || quote.is_some() {
        return Err("has an unterminated bracket or string".to_owned());
    }
    Ok(list)
}

/// The hooks document lists every public hook exactly once in its
/// `## Public hooks` table, with its kind, owner, and compatibility class.
pub fn validate_document(document: &str, vocabulary: &Vocabulary) -> Result<(), String> {
    let section = base::section(document, HOOKS_SECTION)?;
    let mut expected = vocabulary.spelled();
    let mut listed = BTreeSet::new();
    for line in section.lines().filter(|line| line.starts_with('|')) {
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        let Some(hook) = cells.first().and_then(|cell| backticked_one(cell)) else {
            continue;
        };
        if !listed.insert(hook.clone()) {
            return Err(format!(
                "the hooks document `{HOOKS_SECTION}` table lists `{hook}` twice"
            ));
        }
        let Some(entry) = expected.remove(&hook) else {
            return Err(format!(
                "the hooks document lists `{hook}`, which no inventory classifies as a public hook"
            ));
        };
        let kind = cells
            .get(1)
            .and_then(|cell| backticked_one(cell))
            .unwrap_or_default();
        let owner = cells
            .get(2)
            .and_then(|cell| backticked_one(cell))
            .unwrap_or_default();
        let class = cells
            .get(3)
            .and_then(|cell| backticked_one(cell))
            .unwrap_or_default();
        if kind != entry.kind || owner != entry.owner || class != entry.class {
            return Err(format!(
                "the hooks document lists `{hook}` as `{kind}` of `{owner}`, `{class}`; the inventories \
classify it as `{}` of `{}`, `{}`",
                entry.kind, entry.owner, entry.class
            ));
        }
    }
    if let Some(missing) = expected.keys().next() {
        return Err(format!(
            "the hooks document `{HOOKS_SECTION}` table has no row for `{missing}`"
        ));
    }
    Ok(())
}

/// The cell's content when it is exactly one backticked item.
fn backticked_one(cell: &str) -> Option<String> {
    let inner = cell.strip_prefix('`')?.strip_suffix('`')?;
    (!inner.contains('`')).then(|| inner.to_owned())
}

/// The scoping fixture uses every class hook, no other `ds-` class, never puts
/// two primitives on one element, and never puts a layout hook on a UI
/// primitive. Returns the number of hooked elements.
pub fn validate_fixture(html: &str, vocabulary: &Vocabulary) -> Result<usize, String> {
    let lower = html.to_ascii_lowercase();
    if !base::has_element(&lower, "main") {
        return Err(
            "the scoping fixture has no `main`; it places the hooks inside a consumer-owned page shell"
                .to_owned(),
        );
    }
    let mut used = BTreeSet::new();
    let mut hooked = 0;
    for tag in primitive::start_tags(&lower)? {
        let mut kinds = Vec::new();
        for class in tag.classes() {
            if !class.starts_with(HOOK_PREFIX) {
                continue;
            }
            let Some(kind) = vocabulary.kind(class) else {
                return Err(format!(
                    "the scoping fixture uses class `{class}`, which is not a public hook"
                ));
            };
            used.insert(class.to_owned());
            kinds.push(kind);
        }
        if kinds.is_empty() {
            continue;
        }
        hooked += 1;
        if kinds
            .iter()
            .filter(|kind| **kind == BASE_PRIMITIVE || **kind == UI_PRIMITIVE)
            .count()
            > 1
        {
            return Err(format!(
                "the scoping fixture puts two primitives on one <{}>; nest them instead",
                tag.name
            ));
        }
        if kinds.contains(&UI_PRIMITIVE) && kinds.contains(&LAYOUT) {
            return Err(format!(
                "the scoping fixture puts a layout hook on a UI primitive <{}>; a UI primitive owns its \
display, so put the layout on a parent element",
                tag.name
            ));
        }
    }
    if let Some(missing) = vocabulary
        .classes
        .keys()
        .find(|class| !used.contains(*class))
    {
        return Err(format!(
            "the scoping fixture never uses `.{missing}`; every class hook needs reach coverage"
        ));
    }
    Ok(hooked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocabulary() -> Vocabulary {
        let layouts = layout::parse_manifest(
            "public-preview\tstack\t.ds-stack\npublic-preview\tcluster\t.ds-cluster\n",
        )
        .expect("layouts");
        let primitives = primitive::parse_manifest(
            "public-preview\tsurface\tbase-primitive\npublic-preview\taction\tui-primitive\n\
public-preview\taction-quiet\tvariant\npublic-preview\taction-current\tstate\n",
        )
        .expect("primitives");
        let theme = theme::parse_manifest(
            "public-preview\tdefault\tcolor-scheme\tlight dark\n\
public-preview\tattribute\tdata-ds-scheme\tlight dark\n",
        )
        .expect("theme");
        super::vocabulary(&layouts, &primitives, &theme)
    }

    fn sheet(source: &str) -> Result<Summary, String> {
        validate_stylesheets(
            &[("packages/styles/x.css".to_owned(), source.to_owned())],
            &vocabulary(),
        )
    }

    #[test]
    fn derives_the_vocabulary_from_the_inventories() {
        let vocabulary = vocabulary();
        assert_eq!(
            vocabulary.classes.keys().collect::<Vec<_>>(),
            [
                "ds-action",
                "ds-action-quiet",
                "ds-cluster",
                "ds-stack",
                "ds-surface"
            ]
        );
        assert_eq!(vocabulary.kind("ds-action-quiet"), Some(VARIANT));
        assert_eq!(vocabulary.kind("ds-surface"), Some(BASE_PRIMITIVE));
        assert!(vocabulary.attributes.contains_key("data-ds-scheme"));
        assert_eq!(vocabulary.len(), 6);
    }

    #[test]
    fn accepts_root_anchored_hooks() {
        let summary = sheet(
            "@layer x {
:where(.ds-stack) { display: flex; }
:where(.ds-stack) > :where(*) { margin-block: 0; }
:where(.ds-action.ds-action-quiet) { color: red; }
:where(.ds-action[aria-current]:not([aria-current=\"\"], [aria-current=\"false\" i])) { color: red; }
:where(a.ds-action:not(:any-link)) { color: red; }
:root[data-ds-scheme~=\"dark\"] { color-scheme: dark; }
:where(a:any-link) > :where(img) { color: red; }
:where(h1 + p) { color: red; }
}",
        )
        .expect("ok");
        assert_eq!(summary.rules, 8);
        assert_eq!(summary.hooked, 6);
        // An escaped spelling of a hook is the same hook, not a combinator.
        let escaped = sheet(":where(.ds-\\73 urface) { color: red; }").expect("escaped hook");
        assert_eq!(escaped.hooked, 1);
    }

    #[test]
    fn rejects_undeclared_class_hooks() {
        for rule in [
            ":where(.ds-card) { color: red; }",
            ":where(.ds-act\\69 on-primary) { color: red; }",
            ":where(.dark) { color: red; }",
            ":where(.ds-action:not(.ds-bogus)) { color: red; }",
            ":is(.ds-surface, .card) { color: red; }",
        ] {
            let error = sheet(rule).expect_err(rule);
            assert!(error.contains("not a public hook"), "{rule}: {error}");
        }
    }

    #[test]
    fn reserves_every_other_data_attribute() {
        for rule in [
            ":where([data-component=\"action\"]) { color: red; }",
            ":where(.ds-action[data-state]) { color: red; }",
            ":where([data-ds-variant]) { color: red; }",
            ":where([DATA-SLOT]) { color: red; }",
            ":where([data-d\\73-layout]) { color: red; }",
        ] {
            let error = sheet(rule).expect_err(rule);
            assert!(error.contains("reserved"), "{rule}: {error}");
        }
    }

    #[test]
    fn rejects_hooks_that_reach_past_their_element() {
        for (rule, needle) in [
            (
                ":where(.ds-surface) :where(p) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-surface) > :where(*) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-stack) :where(*) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-stack) + :where(*) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-stack) ~ :where(*) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-stack) > :where(p) { color: red; }",
                "only a layout",
            ),
            (
                ":where(.ds-stack) > :where(.x) { color: red; }",
                "not a public hook",
            ),
            (
                ":where(.ds-stack) > :where(*) > :where(*) { color: red; }",
                "more than one",
            ),
            (
                ":where(nav) :where(.ds-action) { color: red; }",
                "through another element",
            ),
            (
                ":where(nav) > :where(.ds-action) { color: red; }",
                "through another element",
            ),
            (":where(.ds-surface:has(p)) { color: red; }", ":has()"),
            (":where(body:has(.ds-action)) { color: red; }", ":has()"),
            (
                ":where(.ds-stack)>:where(*)+:where(*) { color: red; }",
                "more than one",
            ),
            (
                ":where(nav .ds-action) { color: red; }",
                "pseudo-class argument",
            ),
            (
                ":where(.ds-surface :where(p)) { color: red; }",
                "pseudo-class argument",
            ),
            (
                ":is(:where(nav > .ds-action)) { color: red; }",
                "pseudo-class argument",
            ),
            (
                ":where(.ds-action:not(nav *)) { color: red; }",
                "pseudo-class argument",
            ),
        ] {
            let error = sheet(rule).expect_err(rule);
            assert!(error.contains(needle), "{rule}: {error}");
        }
    }

    /// S056-P2-1: each bypass goes through `validate_stylesheets`, raw
    /// whitespace included, as the command reads it.
    #[test]
    fn rejects_s056_bypasses_through_the_whole_pipeline() {
        for (rule, needle) in [
            (
                ":where(.ds-actio\\6e  :where(p)) { color: red; }",
                "pseudo-class argument",
            ),
            (
                ":where(.ds-actio\\6e \t:where(p)) { color: red; }",
                "pseudo-class argument",
            ),
            (
                ":where(.ds-actio\\6e\n\n:where(p)) { color: red; }",
                "pseudo-class argument",
            ),
            (".ds-actio\\6e  p { color: red; }", "only a layout"),
            (":where(:not(.ds-action)) { color: red; }", "negates a hook"),
            (
                ":where(p:not(.ds-surface)) { color: red; }",
                "negates a hook",
            ),
            (
                ":where(p:not(:is(.ds-surface))) { color: red; }",
                "negates a hook",
            ),
            (
                ":where(p:not([data-ds-scheme])) { color: red; }",
                "negates a hook",
            ),
            (":is(.ds-surface, p) { color: red; }", "alternatives"),
            (":where(.ds-surface, p) { color: red; }", "alternatives"),
            (
                ":where([data-ds-scheme]) :where(p) { color: red; }",
                "away from `:root`",
            ),
            (
                ":root[data-ds-scheme] :where(p) { color: red; }",
                "only a layout",
            ),
            (
                ":root[data-ds-scheme] > :where(*) { color: red; }",
                "only a layout",
            ),
            (
                ":where(p) :root[data-ds-scheme] { color: red; }",
                "attribute hook",
            ),
            (
                ":where(p[data-ds-scheme]) { color: red; }",
                "away from `:root`",
            ),
            (
                ":where(.ds-surface:first-child) { color: red; }",
                "`:first-child`",
            ),
            (
                ":where(.ds-surface:focus-within) { color: red; }",
                "`:focus-within`",
            ),
            (
                ":where(.ds-surface:host-context(nav)) { color: red; }",
                "`:host-context`",
            ),
            (":where(.ds-surface)::part(x) { color: red; }", "`:part`"),
            (
                ":where(.ds-surface)::slotted(p) { color: red; }",
                "`:slotted`",
            ),
        ] {
            let error = sheet(rule).expect_err(rule);
            assert!(error.contains(needle), "{rule}: {error}");
        }
        // Legitimate raw spellings still pass the whole pipeline.
        for rule in [
            ":where(.ds-\\73 urface) { color: red; }",
            ":where(.ds-\\73\turface) { color: red; }",
            ":where(.ds-\\000073urface) { color: red; }",
            ":where(.ds-stack)\n>\n:where(*) { color: red; }",
            ":root[data-ds-scheme~=\"dark\"] { color-scheme: dark; }",
            ":where(.ds-action[aria-current]:not([aria-current=\"\"], [aria-current=\"false\" i])) { color: red; }",
        ] {
            sheet(rule).expect(rule);
        }
    }

    #[test]
    fn rejects_scope_and_nesting() {
        for (rule, needle) in [
            ("@scope (.ds-action) { :scope { color: red; } }", "@scope"),
            (
                "@SCOPE (.ds-action) to (p) { :scope { color: red; } }",
                "@scope",
            ),
            ("@layer x { @scope { p { color: red; } } }", "@scope"),
            (
                "@sc\\6f pe (.ds-action) { :scope { color: red; } }",
                "escape",
            ),
            (
                ":where(.ds-surface) { color: red; & p { color: blue; } }",
                "nests a rule",
            ),
        ] {
            let error = sheet(rule).expect_err(rule);
            assert!(error.contains(needle), "{rule}: {error}");
        }
    }

    #[test]
    fn pairs_top_level_combinators() {
        assert_eq!(
            top_level_combinators(":where(.a) > :where(*), a b ~ c,d+e").expect("ok"),
            vec![vec!['>'], vec![' ', '~'], vec!['+']]
        );
        assert_eq!(
            top_level_combinators(":where(a > b, [x=\" > \"]) :not(.c)").expect("ok"),
            vec![vec![' ']]
        );
        assert!(top_level_combinators(":where(a").is_err());
        // The whitespace that ends a hex escape is part of the escape.
        assert_eq!(
            top_level_combinators(".ds-\\73 urface > :where(*), .a\\ b").expect("ok"),
            vec![vec!['>'], vec![]]
        );
        assert!(!nested_combinators(":where(.ds-\\73 urface)").expect("ok"));
        assert!(nested_combinators(":where(.ds-\\73  urface)").expect("ok"));
    }

    const DOC: &str = "\
# Hooks

## Public hooks

| Hook | Kind | Owner | Class | Meaning |
|---|---|---|---|---|
| `.ds-stack` | `layout` | `stack` | `public-preview` | a |
| `.ds-cluster` | `layout` | `cluster` | `public-preview` | a |
| `.ds-surface` | `base-primitive` | `surface` | `public-preview` | a |
| `.ds-action` | `ui-primitive` | `action` | `public-preview` | a |
| `.ds-action-quiet` | `variant` | `action` | `public-preview` | a |
| `data-ds-scheme` | `theme-attribute` | `theme` | `public-preview` | a |

## Reserved
";

    #[test]
    fn document_lists_every_hook_once() {
        validate_document(DOC, &vocabulary()).expect("document");
        for (doc, needle) in [
            (
                DOC.replace("| `.ds-cluster` | `layout` | `cluster` | `public-preview` | a |\n", ""),
                "no row for `.ds-cluster`",
            ),
            (
                DOC.replace("`.ds-action-quiet` | `variant`", "`.ds-action-quiet` | `ui-primitive`"),
                "classify it as `variant`",
            ),
            (
                DOC.replace("`data-ds-scheme` | `theme-attribute` | `theme` | `public-preview`", "`data-ds-scheme` | `theme-attribute` | `theme` | `public-stable`"),
                "`public-stable`",
            ),
            (
                DOC.replace("\n## Reserved", "| `data-component` | `ui-primitive` | `x` | `public-preview` | a |\n\n## Reserved"),
                "which no inventory",
            ),
            (
                DOC.replace("## Reserved\n", "| `.ds-stack` | `layout` | `stack` | `public-preview` | a |\n"),
                "twice",
            ),
            (DOC.replace("## Public hooks\n", ""), "## Public hooks"),
        ] {
            let error = validate_document(&doc, &vocabulary()).expect_err(needle);
            assert!(error.contains(needle), "{needle}: {error}");
        }
    }

    const FIXTURE: &str = "<!doctype html><main>
<article class=\"ds-surface ds-stack\"><p class=\"ds-cluster\">
<button class=\"ds-action ds-action-quiet\" type=button>Go</button>
<span class=\"x-ds-action\">Lookalike</span>
</p></article></main>";

    #[test]
    fn fixture_covers_every_class_hook() {
        assert_eq!(validate_fixture(FIXTURE, &vocabulary()), Ok(3));
        for (bad, needle) in [
            (FIXTURE.replace("<main>", "<div>"), "no `main`"),
            (
                FIXTURE.replace(" ds-action-quiet", ""),
                "never uses `.ds-action-quiet`",
            ),
            (FIXTURE.replace("x-ds-action", "ds-card"), "`ds-card`"),
            (
                FIXTURE.replace(
                    "class=\"ds-action ds-action-quiet\"",
                    "class=\"ds-action ds-action-quiet ds-stack\"",
                ),
                "layout hook on a UI primitive",
            ),
            (
                FIXTURE.replace(
                    "class=\"ds-action ds-action-quiet\"",
                    "class=\"ds-action ds-surface ds-action-quiet\"",
                ),
                "two primitives",
            ),
        ] {
            let error = validate_fixture(&bad, &vocabulary()).expect_err(&bad);
            assert!(error.contains(needle), "{needle}: {error}");
        }
    }
}
