//! Primitive checks: the stylesheets under `packages/styles/primitives/`
//! implement exactly the primitives `primitives.tsv` promotes. Each one sits
//! behind one zero-specificity class hook, and its rules are derived from the
//! inventory in a fixed order. A base primitive owns no state and no layout
//! property, so it composes with the layouts. A UI primitive styles its state
//! through native and ARIA selectors only. The primitives document and the
//! primitives fixture must cover the same inventory, and the document must
//! record every candidate with the same disposition and reason as the inventory.
//!
//! Like the other checks, this recognizes only the shapes the primitives use
//! and rejects anything else rather than guessing.

use std::collections::{BTreeMap, BTreeSet};

use crate::base::{self, Vocabulary};
use crate::css::{self, Node};
use crate::layout;
use crate::theme;
use crate::tokens;

/// Repository-relative stylesheet that imports the primitive modules.
pub const PRIMITIVES_ENTRY: &str = "packages/styles/primitives.css";
/// Directory that holds one module per promoted primitive.
const PRIMITIVES_DIR: &str = "packages/styles/primitives/";

const PUBLIC_PREVIEW: &str = "public-preview";
/// Prefix of every primitive class hook.
const HOOK_PREFIX: &str = "ds-";

/// The kinds a `public-preview` row may carry.
const BASE_PRIMITIVE: &str = "base-primitive";
const UI_PRIMITIVE: &str = "ui-primitive";
const VARIANT: &str = "variant";
const STATE: &str = "state";

/// The reason codes each non-promoted disposition may use.
const REASONS: [(&str, &[&str]); 3] = [
    ("consumer", &["product-composition", "product-identity"]),
    (
        "deferred",
        &["native-pattern-refinement", "no-repeat-evidence"],
    ),
    (
        "rejected",
        &[
            "covered-by-promoted",
            "covered-by-base",
            "duplicates-native-state",
            "conflicts-with-native-semantics",
        ],
    ),
];

/// The native and ARIA states a UI primitive may style. Each is a state name,
/// the element its selector requires (empty for any element), the selector
/// suffix after the hook, and the token the document must name. An empty
/// `aria-current` is the ARIA default, `false`, and `false` matches in any
/// case, so the current state excludes both.
const STATES: [(&str, &str, &str, &str); 4] = [
    ("hover", "", ":hover", ":hover"),
    (
        "current",
        "",
        "[aria-current]:not([aria-current=\"\"], [aria-current=\"false\" i])",
        "[aria-current]",
    ),
    ("disabled", "", ":disabled", ":disabled"),
    ("unlinked", "a", ":not(:any-link)", ":not(:any-link)"),
];

/// Native elements a UI primitive hook may sit on.
const UI_ROOTS: [&str; 2] = ["button", "a"];

/// Properties a base primitive may declare: its box edges, colors, and inner
/// spacing. Layout properties belong to `ds.layouts`, so a base primitive and a
/// layout on the same element never set the same property.
const BASE_PROPERTIES: [&str; 8] = [
    "padding-block",
    "padding-inline",
    "border-width",
    "border-style",
    "border-color",
    "border-radius",
    "background-color",
    "color",
];

/// Properties a UI primitive may declare: its own inline content box, target
/// size, edges, colors, emphasis, and pointer cursor. Outline, visibility,
/// pointer-events, opacity, and positioning are absent: focus stays with the
/// base and state stays native.
const UI_PROPERTIES: [&str; 18] = [
    "display",
    "align-items",
    "justify-content",
    "gap",
    "min-block-size",
    "min-inline-size",
    "padding-block",
    "padding-inline",
    "border-width",
    "border-style",
    "border-color",
    "border-radius",
    "background-color",
    "color",
    "font-weight",
    "text-decoration-line",
    "cursor",
    "aspect-ratio",
];

const PRIMITIVE_VALUES: Vocabulary = Vocabulary {
    label: "a primitive",
    functions: &["var", "calc"],
    units: &[],
    keywords: &[
        "inline-flex",
        "center",
        "solid",
        "dashed",
        "transparent",
        "none",
        "pointer",
        "not-allowed",
        "currentcolor",
    ],
};

/// Whether a primitive is a base primitive or a UI primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Base,
    Ui,
}

impl Kind {
    fn code(self) -> &'static str {
        match self {
            Kind::Base => BASE_PRIMITIVE,
            Kind::Ui => UI_PRIMITIVE,
        }
    }
}

/// One promoted primitive with its variants and states, in inventory order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Primitive {
    pub name: String,
    pub kind: Kind,
    /// Variant suffixes; the hook of `primary` on `action` is `.ds-action-primary`.
    pub variants: Vec<String>,
    /// State names from the native state table.
    pub states: Vec<String>,
}

impl Primitive {
    pub(crate) fn hook(&self) -> String {
        format!("{HOOK_PREFIX}{}", self.name)
    }

    pub(crate) fn variant_hook(&self, variant: &str) -> String {
        format!("{HOOK_PREFIX}{}-{variant}", self.name)
    }

    /// Every class hook this primitive publishes: its own, then its variants'.
    fn hooks(&self) -> Vec<String> {
        let mut hooks = vec![self.hook()];
        hooks.extend(self.variants.iter().map(|v| self.variant_hook(v)));
        hooks
    }

    /// The selector of every rule the module holds, in the required order.
    fn selectors(&self) -> Vec<String> {
        let hook = self.hook();
        let mut selectors = vec![format!(":where(.{hook})")];
        for variant in &self.variants {
            selectors.push(format!(":where(.{hook}.{})", self.variant_hook(variant)));
        }
        for state in &self.states {
            let (_, element, suffix, _) = state_entry(state).expect("validated state");
            selectors.push(format!(":where({element}.{hook}{suffix})"));
        }
        selectors
    }
}

fn state_entry(name: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    STATES.iter().copied().find(|(state, ..)| *state == name)
}

/// The validated `primitives.tsv` inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Promoted primitives, in sub-layer order.
    pub primitives: Vec<Primitive>,
    /// Candidates that were not promoted: disposition, name, and reason.
    pub candidates: Vec<(String, String, String)>,
}

impl Manifest {
    fn hooks(&self) -> BTreeSet<String> {
        self.primitives.iter().flat_map(Primitive::hooks).collect()
    }

    /// Promoted variant and state rows.
    pub fn members(&self) -> usize {
        self.primitives
            .iter()
            .map(|primitive| primitive.variants.len() + primitive.states.len())
            .sum()
    }

    pub fn names(&self) -> String {
        self.primitives
            .iter()
            .map(|primitive| format!("{} ({})", primitive.name, primitive.kind.code()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && !text.ends_with('-')
        && !text.contains("--")
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse and validate the primitives inventory.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut primitives: Vec<Primitive> = Vec::new();
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [class, name, subject] = fields.as_slice() else {
            return Err(format!(
                "primitives line {line_number} must be <class><tab><primitive, variant, state, or candidate><tab><kind or reason>"
            ));
        };
        if !is_name(name) {
            return Err(format!(
                "primitives line {line_number} name '{name}' is not a lowercase hyphenated name"
            ));
        }
        if !seen.insert((*name).to_owned()) {
            return Err(format!(
                "primitives line {line_number} repeats '{name}'; each name is classified exactly once"
            ));
        }
        if *class == PUBLIC_PREVIEW {
            match *subject {
                BASE_PRIMITIVE | UI_PRIMITIVE => primitives.push(Primitive {
                    name: (*name).to_owned(),
                    kind: if *subject == BASE_PRIMITIVE {
                        Kind::Base
                    } else {
                        Kind::Ui
                    },
                    variants: Vec::new(),
                    states: Vec::new(),
                }),
                VARIANT | STATE => {
                    let Some(parent) = primitives.last_mut() else {
                        return Err(format!(
                            "primitives line {line_number} lists {subject} '{name}' before any primitive"
                        ));
                    };
                    let Some(suffix) = name
                        .strip_prefix(parent.name.as_str())
                        .and_then(|rest| rest.strip_prefix('-'))
                    else {
                        return Err(format!(
                            "primitives line {line_number} lists {subject} '{name}' under '{}'; it is named '{}-<{subject}>'",
                            parent.name, parent.name
                        ));
                    };
                    if parent.kind == Kind::Base {
                        return Err(format!(
                            "primitives line {line_number} gives base primitive '{}' the {subject} '{name}'; \
a base primitive owns no variant or state",
                            parent.name
                        ));
                    }
                    if *subject == VARIANT {
                        if !parent.states.is_empty() {
                            return Err(format!(
                                "primitives line {line_number} lists variant '{name}' after a state; variants come first"
                            ));
                        }
                        parent.variants.push(suffix.to_owned());
                    } else {
                        if state_entry(suffix).is_none() {
                            return Err(format!(
                                "primitives line {line_number} lists state '{name}'; the native states are {}",
                                STATES
                                    .iter()
                                    .map(|(state, ..)| *state)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                        parent.states.push(suffix.to_owned());
                    }
                }
                _ => {
                    return Err(format!(
                        "primitives line {line_number} gives '{name}' the kind '{subject}'; the kinds are \
{BASE_PRIMITIVE}, {UI_PRIMITIVE}, {VARIANT}, and {STATE}"
                    ));
                }
            }
        } else if let Some((_, reasons)) =
            REASONS.iter().find(|(disposition, _)| disposition == class)
        {
            if !reasons.contains(subject) {
                return Err(format!(
                    "primitives line {line_number} gives {class} candidate '{name}' the reason '{subject}'; \
a {class} reason is one of {}",
                    reasons.join(", ")
                ));
            }
            candidates.push((
                (*class).to_owned(),
                (*name).to_owned(),
                (*subject).to_owned(),
            ));
        } else {
            return Err(format!(
                "primitives line {line_number} classifies '{name}' as '{class}'; rows are {PUBLIC_PREVIEW} or one of {}",
                REASONS
                    .iter()
                    .map(|(disposition, _)| *disposition)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if primitives.is_empty() {
        return Err("primitives inventory promotes no primitive".to_owned());
    }
    Ok(Manifest {
        primitives,
        candidates,
    })
}

/// What validated primitives cover.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub modules: usize,
    pub rules: usize,
    pub declarations: usize,
}

/// Validate the primitive stylesheets among the reached Design System
/// stylesheets.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_stylesheets(
    stylesheets: &[(String, String)],
    manifest: &Manifest,
) -> Result<Summary, String> {
    let entry = stylesheets
        .iter()
        .find(|(file, _)| file == PRIMITIVES_ENTRY)
        .ok_or_else(|| {
            format!("{PRIMITIVES_ENTRY} is not reached from any declared export; the primitives must enter the cascade")
        })?;
    validate_entry(&entry.0, &entry.1, manifest)?;

    let mut modules = BTreeMap::new();
    for (file, source) in stylesheets {
        if let Some(name) = file.strip_prefix(PRIMITIVES_DIR) {
            let primitive = name.strip_suffix(".css").unwrap_or(name);
            if !manifest.primitives.iter().any(|p| p.name == primitive) {
                return Err(format!(
                    "{file} is not a promoted primitive module; primitives.tsv promotes {}",
                    manifest.names()
                ));
            }
            modules.insert(primitive.to_owned(), (file.clone(), source.clone()));
        }
    }

    let mut summary = Summary::default();
    for primitive in &manifest.primitives {
        let Some((file, source)) = modules.get(&primitive.name) else {
            return Err(format!(
                "{PRIMITIVES_DIR}{}.css is not reached; every promoted primitive enters the cascade",
                primitive.name
            ));
        };
        theme::reject_selector_comments(file, source)?;
        let expected = primitive.selectors();
        let mut position = 0;
        let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
        let rules = css::split_print(nodes).map_err(|_| {
            format!(
                "{file} holds an at-rule; primitives hold unconditional style rules and `@media print` rules only"
            )
        })?;
        for css::FlatRule {
            prelude,
            body,
            print,
        } in rules
        {
            let selector = prelude.split_whitespace().collect::<Vec<_>>().join(" ");
            if selector.contains("data-") {
                return Err(format!(
                    "{file} `{selector}` tests a data-* attribute; primitive state comes from native and ARIA state"
                ));
            }
            if print {
                // A print rule adapts a rule that already ran: it names a
                // classified selector and follows that selector's own rule, so
                // every later state rule still outranks it.
                match expected.iter().position(|candidate| *candidate == selector) {
                    Some(found) if found < position => {}
                    Some(_) => {
                        return Err(format!(
                            "{file} `{selector}` in `@media print` comes before its own rule; a print rule follows the rule it adapts"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "{file} `{selector}` in `@media print`: the {} rules select only {}, so each has zero specificity and styles no other hook or state",
                            primitive.name,
                            expected.join(", ")
                        ));
                    }
                }
                let declarations = tokens::split_declarations(file, &body)?;
                if declarations.is_empty() {
                    return Err(format!("{file} `{selector}` declares nothing"));
                }
                for (name, value) in &declarations {
                    check_declaration(primitive.kind, name, value).map_err(|error| {
                        format!("{file} `{selector}` in `@media print` {name}: {error}")
                    })?;
                }
                summary.declarations += declarations.len();
                continue;
            }
            match expected.iter().position(|candidate| *candidate == selector) {
                Some(found) if found == position => {}
                Some(found) if found < position => {
                    return Err(format!(
                        "{file} `{selector}` appears twice or out of order; the rules run {}",
                        expected.join(", ")
                    ));
                }
                Some(_) => {
                    return Err(format!(
                        "{file} has no `{}` rule before `{selector}`; the rules run {}",
                        expected[position],
                        expected.join(", ")
                    ));
                }
                None => {
                    return Err(format!(
                        "{file} `{selector}`: the {} rules select only {}, so each has zero specificity \
and styles no other hook or state",
                        primitive.name,
                        expected.join(", ")
                    ));
                }
            }
            position += 1;
            let declarations = tokens::split_declarations(file, &body)?;
            if declarations.is_empty() {
                return Err(format!("{file} `{selector}` declares nothing"));
            }
            for (name, value) in &declarations {
                check_declaration(primitive.kind, name, value)
                    .map_err(|error| format!("{file} `{selector}` {name}: {error}"))?;
            }
            summary.rules += 1;
            summary.declarations += declarations.len();
        }
        if let Some(missing) = expected.get(position) {
            return Err(format!(
                "{file} has no `{missing}` rule; every classified hook and state has one rule"
            ));
        }
        summary.modules += 1;
    }
    Ok(summary)
}

/// `primitives.css` holds only its sub-layer order and one import per promoted
/// primitive, both in inventory order.
fn validate_entry(file: &str, source: &str, manifest: &Manifest) -> Result<(), String> {
    let mut order = Vec::new();
    let mut imported = Vec::new();
    for node in css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))? {
        match node {
            Node::Statement { name, prelude } if name == "layer" => {
                order.push(prelude.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            Node::Statement { name, prelude } if name == "import" => {
                imported.push(prelude.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            _ => {
                return Err(format!(
                    "{file} holds a rule; it only orders and imports the primitive modules"
                ));
            }
        }
    }
    let names: Vec<&str> = manifest
        .primitives
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let expected_order = names.join(", ");
    if order != [expected_order.clone()] {
        return Err(format!(
            "{file} must declare exactly one `@layer {expected_order};` in inventory order"
        ));
    }
    let expected: Vec<String> = names
        .iter()
        .map(|name| format!("\"./primitives/{name}.css\" layer({name})"))
        .collect();
    if imported != expected {
        return Err(format!(
            "{file} must import exactly {} in that order",
            expected.join(", ")
        ));
    }
    Ok(())
}

/// Check one declaration's property and value.
fn check_declaration(kind: Kind, name: &str, value: &str) -> Result<(), String> {
    let property = name.to_ascii_lowercase();
    if property.starts_with("--") {
        return Err(
            "declares a custom property; primitives bind existing roles and consumers override the property itself"
                .to_owned(),
        );
    }
    if property.starts_with("outline") {
        return Err("restyles the outline; the base owns the one focus indicator".to_owned());
    }
    if kind == Kind::Base && layout::PROPERTIES.contains(&property.as_str()) {
        return Err(
            "is a layout property; a base primitive leaves arrangement to ds.layouts so both compose on one element"
                .to_owned(),
        );
    }
    let allowed: &[&str] = match kind {
        Kind::Base => &BASE_PROPERTIES,
        Kind::Ui => &UI_PROPERTIES,
    };
    if !allowed.contains(&property.as_str()) {
        return Err(format!(
            "is not a {} property; it declares only {}",
            kind.code(),
            allowed.join(", ")
        ));
    }
    base::check_value(&value.to_ascii_lowercase(), &PRIMITIVE_VALUES)
}

/// Heading of the primitives document section that states each contract.
const CONTRACT_SECTION: &str = "## Contract";
/// Heading of the primitives document section that lists the other candidates.
const CANDIDATES_SECTION: &str = "## Rejected and deferred";

/// The primitives document states the inventory. The contract section has one
/// `###` heading per promoted primitive; that subsection names the primitive's
/// kind code, every hook, and every state's native selector. The contract names
/// no hook other than a promoted primitive or layout hook. The candidates
/// section has one table row per candidate whose disposition and reason match
/// the inventory, and no other row.
pub fn validate_document(
    document: &str,
    manifest: &Manifest,
    layout_hooks: &BTreeSet<String>,
) -> Result<(), String> {
    let contract = base::section(document, CONTRACT_SECTION)?;
    let hooks = manifest.hooks();
    for item in base::backticked(contract) {
        if let Some(class) = item.strip_prefix('.')
            && class.starts_with(HOOK_PREFIX)
            && !hooks.contains(class)
            && !layout_hooks.contains(class)
        {
            return Err(format!(
                "the primitives document names hook `{item}`, which neither primitives.tsv nor layouts.tsv promotes"
            ));
        }
    }
    for primitive in &manifest.primitives {
        let heading = format!("### {}", primitive.name);
        let body = subsection(contract, &heading).ok_or_else(|| {
            format!(
                "the primitives document `{CONTRACT_SECTION}` section has no `{heading}` heading"
            )
        })?;
        let named: BTreeSet<String> = base::backticked(body).into_iter().collect();
        let mut required = vec![primitive.kind.code().to_owned()];
        required.extend(primitive.hooks().iter().map(|hook| format!(".{hook}")));
        for state in &primitive.states {
            let (_, _, _, token) = state_entry(state).expect("validated state");
            required.push(token.to_owned());
        }
        if let Some(missing) = required.iter().find(|item| !named.contains(*item)) {
            return Err(format!(
                "the primitives document `{heading}` subsection does not name `{missing}`"
            ));
        }
    }

    let candidates = base::section(document, CANDIDATES_SECTION)?;
    let mut rows = BTreeMap::new();
    for line in candidates.lines().filter(|line| line.starts_with('|')) {
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        let Some(name) = cells.first().and_then(|cell| backticked_one(cell)) else {
            continue;
        };
        let disposition = cells.get(1).copied().unwrap_or("");
        let reason = cells
            .get(2)
            .and_then(|cell| backticked_one(cell))
            .unwrap_or_default();
        if rows
            .insert(name.clone(), (disposition.to_owned(), reason))
            .is_some()
        {
            return Err(format!(
                "the primitives document `{CANDIDATES_SECTION}` section lists `{name}` twice"
            ));
        }
    }
    for (disposition, name, reason) in &manifest.candidates {
        match rows.remove(name) {
            None => {
                return Err(format!(
                    "the primitives document `{CANDIDATES_SECTION}` section has no row for candidate `{name}`"
                ));
            }
            Some((listed, listed_reason)) if listed != *disposition || listed_reason != *reason => {
                return Err(format!(
                    "the primitives document lists `{name}` as {listed} `{listed_reason}`; primitives.tsv records {disposition} `{reason}`"
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(extra) = rows.keys().next() {
        return Err(format!(
            "the primitives document lists `{extra}`, which primitives.tsv does not record as a candidate"
        ));
    }
    Ok(())
}

/// The body of the one `heading` subsection, up to the next heading.
fn subsection<'a>(section: &'a str, heading: &str) -> Option<&'a str> {
    let mut offset = 0;
    let mut start = None;
    for line in section.split_inclusive('\n') {
        if start.is_some() && line.starts_with('#') {
            return start.map(|begin| &section[begin..offset]);
        }
        offset += line.len();
        if start.is_none() && line.trim_end() == heading {
            start = Some(offset);
        }
    }
    start.map(|begin| &section[begin..])
}

/// The cell's content when it is exactly one backticked item.
fn backticked_one(cell: &str) -> Option<String> {
    let inner = cell.strip_prefix('`')?.strip_suffix('`')?;
    (!inner.contains('`')).then(|| inner.to_owned())
}

/// One start tag: its lowercase name and attributes.
#[derive(Debug)]
pub(crate) struct Tag {
    pub(crate) name: String,
    attributes: Vec<(String, Option<String>)>,
}

impl Tag {
    pub(crate) fn attribute(&self, name: &str) -> Option<Option<&str>> {
        self.attributes
            .iter()
            .find(|(attribute, _)| attribute == name)
            .map(|(_, value)| value.as_deref())
    }

    pub(crate) fn classes(&self) -> Vec<&str> {
        match self.attribute("class") {
            Some(Some(value)) => value.split_whitespace().collect(),
            _ => Vec::new(),
        }
    }
}

/// Every start tag in `html`, which is lowercase. Comments, doctypes, and end
/// tags are skipped. A `class` value must be quoted.
pub(crate) fn start_tags(html: &str) -> Result<Vec<Tag>, String> {
    let bytes = html.as_bytes();
    let mut tags = Vec::new();
    let mut index = 0;
    while let Some(offset) = html[index..].find('<') {
        index += offset + 1;
        if html[index..].starts_with("!--") {
            let end = html[index..]
                .find("-->")
                .ok_or("the fixture has an unterminated comment")?;
            index += end + 3;
            continue;
        }
        if !bytes.get(index).is_some_and(u8::is_ascii_lowercase) {
            continue;
        }
        let start = index;
        while bytes
            .get(index)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
        {
            index += 1;
        }
        let mut tag = Tag {
            name: html[start..index].to_owned(),
            attributes: Vec::new(),
        };
        loop {
            while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            match bytes.get(index) {
                None => {
                    return Err(format!("the fixture has an unterminated <{}>", tag.name));
                }
                Some(b'>') => {
                    index += 1;
                    break;
                }
                Some(b'/') => {
                    index += 1;
                    continue;
                }
                _ => {}
            }
            let start = index;
            while bytes
                .get(index)
                .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b'=' | b'>' | b'/'))
            {
                index += 1;
            }
            let name = html[start..index].to_owned();
            while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            let value = if bytes.get(index) == Some(&b'=') {
                index += 1;
                while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                    index += 1;
                }
                match bytes.get(index) {
                    Some(quote @ (b'"' | b'\'')) => {
                        let close = html[index + 1..].find(*quote as char).ok_or_else(|| {
                            format!("the fixture has an unterminated `{name}` value")
                        })?;
                        let value = html[index + 1..index + 1 + close].to_owned();
                        index += close + 2;
                        Some(value)
                    }
                    _ => {
                        if name == "class" {
                            return Err("the fixture has an unquoted class attribute".to_owned());
                        }
                        let start = index;
                        while bytes
                            .get(index)
                            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'>')
                        {
                            index += 1;
                        }
                        Some(html[start..index].to_owned())
                    }
                }
            } else {
                None
            };
            tag.attributes.push((name, value));
        }
        tags.push(tag);
    }
    Ok(tags)
}

/// The primitives fixture uses every promoted hook and exercises every state
/// it can express in markup. It uses no unpromoted `ds-` class, puts a UI
/// primitive only on its native roots, gives a variant only with its
/// primitive's hook, never duplicates state in `data-*` or overrides a role,
/// and composes each base primitive with a layout on one element. Returns the
/// number of hooked elements.
pub fn validate_fixture(
    html: &str,
    manifest: &Manifest,
    layout_hooks: &BTreeSet<String>,
) -> Result<usize, String> {
    let lower = html.to_ascii_lowercase();
    crate::lexical::reject_inert_or_escaped_markup(&lower, "primitives fixture")?;
    if !base::has_element(&lower, "main") {
        return Err(
            "the primitives fixture has no `main`; it places the primitives inside a consumer-owned page shell"
                .to_owned(),
        );
    }
    let hooks = manifest.hooks();
    let mut used = BTreeSet::new();
    let mut states = BTreeSet::new();
    let mut composed = BTreeSet::new();
    let mut hooked = 0;
    for tag in start_tags(&lower)? {
        let classes = tag.classes();
        let mut carries_primitive = false;
        let mut carries_layout = false;
        for class in classes
            .iter()
            .filter(|class| class.starts_with(HOOK_PREFIX))
        {
            if layout_hooks.contains(*class) {
                carries_layout = true;
            } else if hooks.contains(*class) {
                carries_primitive = true;
                used.insert((*class).to_owned());
            } else {
                return Err(format!(
                    "the primitives fixture uses class `{class}`, which neither primitives.tsv nor layouts.tsv promotes"
                ));
            }
        }
        if !carries_primitive {
            continue;
        }
        hooked += 1;
        let mut roots = Vec::new();
        for primitive in &manifest.primitives {
            let root = primitive.hook();
            let has_root = classes.contains(&root.as_str());
            for variant in &primitive.variants {
                let hook = primitive.variant_hook(variant);
                if classes.contains(&hook.as_str()) && !has_root {
                    return Err(format!(
                        "the primitives fixture uses `{hook}` without `{root}`; a variant refines its primitive"
                    ));
                }
            }
            if !has_root {
                continue;
            }
            roots.push(primitive);
            if primitive.kind == Kind::Ui && !UI_ROOTS.contains(&tag.name.as_str()) {
                return Err(format!(
                    "the primitives fixture puts `{root}` on <{}>; the {} UI primitive sits on {}",
                    tag.name,
                    primitive.name,
                    UI_ROOTS.map(|name| format!("<{name}>")).join(" or ")
                ));
            }
            if primitive.kind == Kind::Base && carries_layout {
                composed.insert(primitive.name.clone());
            }
            if primitive.kind == Kind::Ui && carries_layout {
                return Err(format!(
                    "the primitives fixture puts a layout hook on `{root}`; a UI primitive owns its display, \
so put the layout on a parent element"
                ));
            }
            for state in &primitive.states {
                let present = match state.as_str() {
                    "current" => tag.attribute("aria-current").is_some_and(is_current),
                    "disabled" => tag.name == "button" && tag.attribute("disabled").is_some(),
                    "unlinked" => tag.name == "a" && tag.attribute("href").is_none(),
                    _ => false,
                };
                if present {
                    states.insert(format!("{}-{state}", primitive.name));
                }
            }
        }
        if roots.len() > 1 {
            return Err(format!(
                "the primitives fixture puts two primitives on one <{}>; compose a base primitive with a layout, not with a UI primitive",
                tag.name
            ));
        }
        if let Some((attribute, _)) = tag
            .attributes
            .iter()
            .find(|(name, _)| name.starts_with("data-") || name == "role")
        {
            return Err(format!(
                "the primitives fixture gives a primitive <{}> the `{attribute}` attribute; \
primitives use native elements and ARIA state, not data-* state or a replaced role",
                tag.name
            ));
        }
    }
    for hook in &hooks {
        if !used.contains(hook) {
            return Err(format!(
                "the primitives fixture never uses `.{hook}`; every promoted hook needs fixture coverage"
            ));
        }
    }
    for primitive in &manifest.primitives {
        if primitive.kind == Kind::Base && !composed.contains(&primitive.name) {
            return Err(format!(
                "the primitives fixture never composes `.{}` with a layout hook on one element",
                primitive.hook()
            ));
        }
        for state in &primitive.states {
            // Hover is a pointer state that markup cannot express; the browser
            // suite covers it.
            if state != "hover" && !states.contains(&format!("{}-{state}", primitive.name)) {
                return Err(format!(
                    "the primitives fixture never shows the {} state `{state}`; every markup-expressible state needs fixture coverage",
                    primitive.name
                ));
            }
        }
    }
    Ok(hooked)
}

/// Whether an `aria-current` value (lowercased, `None` when the attribute has
/// no value) marks the current item: anything but an empty value or `false`.
pub(crate) fn is_current(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "false")
}

/// The class hooks `layouts.tsv` promotes, for the checks that let a
/// primitive compose with a layout.
pub fn layout_hooks(manifest: &layout::Manifest) -> BTreeSet<String> {
    manifest
        .layouts
        .iter()
        .map(|name| format!("{HOOK_PREFIX}{name}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
# comment
public-preview\tsurface\tbase-primitive
public-preview\taction\tui-primitive
public-preview\taction-primary\tvariant
public-preview\taction-current\tstate
public-preview\taction-disabled\tstate
public-preview\taction-unlinked\tstate
consumer\tbreadcrumbs\tproduct-composition
deferred\tdialog\tnative-pattern-refinement
rejected\tcard\tcovered-by-promoted
";

    const ENTRY: &str = "@layer surface, action;
@import \"./primitives/surface.css\" layer(surface);
@import \"./primitives/action.css\" layer(action);
";

    const SURFACE: &str = ":where(.ds-surface) { padding-block: var(--ds-space-flow); border-style: solid; background-color: var(--ds-color-surface); }";
    const ACTION: &str = ":where(.ds-action) { display: inline-flex; min-block-size: var(--ds-size-target-min); cursor: pointer; }
:where(.ds-action.ds-action-primary) { background-color: var(--ds-color-accent); }
:where(.ds-action[aria-current]:not([aria-current=\"\"], [aria-current=\"false\" i])) { font-weight: var(--ds-weight-strong); }
:where(.ds-action:disabled) { cursor: not-allowed; }
:where(a.ds-action:not(:any-link)) { cursor: not-allowed; }";

    fn manifest() -> Manifest {
        parse_manifest(MANIFEST).expect("manifest")
    }

    fn layouts() -> BTreeSet<String> {
        ["ds-stack".to_owned(), "ds-cluster".to_owned()].into()
    }

    fn sheets(surface: &str, action: &str) -> Vec<(String, String)> {
        vec![
            (PRIMITIVES_ENTRY.to_owned(), ENTRY.to_owned()),
            (
                "packages/styles/primitives/surface.css".to_owned(),
                surface.to_owned(),
            ),
            (
                "packages/styles/primitives/action.css".to_owned(),
                action.to_owned(),
            ),
        ]
    }

    fn with_action(action: &str) -> Result<Summary, String> {
        validate_stylesheets(&sheets(SURFACE, action), &manifest())
    }

    fn with_surface_declaration(declaration: &str) -> Result<Summary, String> {
        validate_stylesheets(
            &sheets(
                &SURFACE.replace("border-style: solid;", declaration),
                ACTION,
            ),
            &manifest(),
        )
    }

    fn with_action_declaration(declaration: &str) -> Result<Summary, String> {
        with_action(&ACTION.replace("cursor: pointer;", declaration))
    }

    #[test]
    fn accepts_conforming_primitives() {
        let summary = validate_stylesheets(&sheets(SURFACE, ACTION), &manifest()).expect("ok");
        assert_eq!(summary.modules, 2);
        assert_eq!(summary.rules, 6);
        assert_eq!(summary.declarations, 10);
        assert_eq!(manifest().members(), 4);
    }

    #[test]
    fn manifest_rejects_bad_rows() {
        let promoted =
            "public-preview\tsurface\tbase-primitive\npublic-preview\taction\tui-primitive\n";
        for (text, needle) in [
            (
                "internal\tsurface\tbase-primitive".to_owned(),
                "rows are public-preview",
            ),
            (
                "public-preview\tsurface\tprimitive".to_owned(),
                "the kinds are",
            ),
            (
                "public-preview\tSurface\tbase-primitive".to_owned(),
                "not a lowercase",
            ),
            ("public-preview\tsurface".to_owned(), "must be <class>"),
            (
                "public-preview\taction-primary\tvariant".to_owned(),
                "before any primitive",
            ),
            (
                format!("{promoted}public-preview\tprimary\tvariant"),
                "is named 'action-<variant>'",
            ),
            (
                "public-preview\tsurface\tbase-primitive\npublic-preview\tsurface-raised\tvariant"
                    .to_owned(),
                "owns no variant or state",
            ),
            (
                format!("{promoted}public-preview\taction-pressed\tstate"),
                "the native states are",
            ),
            (
                format!(
                    "{promoted}public-preview\taction-hover\tstate\npublic-preview\taction-primary\tvariant"
                ),
                "variants come first",
            ),
            (
                format!("{promoted}deferred\tcard\tlater"),
                "a deferred reason is one of",
            ),
            (
                format!("{promoted}consumer\tcard\tno-repeat-evidence"),
                "a consumer reason is one of",
            ),
            (
                format!("{promoted}rejected\tsurface\tcovered-by-base"),
                "repeats 'surface'",
            ),
            (
                "deferred\tdialog\tnative-pattern-refinement".to_owned(),
                "promotes no primitive",
            ),
        ] {
            let error = parse_manifest(&text).expect_err(&text);
            assert!(error.contains(needle), "{text}: {error}");
        }
    }

    #[test]
    fn rejects_selectors_other_than_the_derived_ones() {
        for rule in [
            ".ds-action { cursor: pointer; }",
            ":where(.ds-action-primary) { cursor: pointer; }",
            ":where(.ds-action.ds-action-quiet) { cursor: pointer; }",
            ":where(.ds-action:focus-visible) { cursor: pointer; }",
            ":where(.ds-action[aria-pressed=\"true\"]) { cursor: pointer; }",
            ":where(.ds-action[aria-current]:not([aria-current=\"false\"])) { cursor: pointer; }",
            ":where(.ds-action:active) { cursor: pointer; }",
            ":where(button.ds-action) { cursor: pointer; }",
            ":where(.ds-action) > :where(*) { cursor: pointer; }",
            ":where(.ds-surface) { cursor: pointer; }",
            ":WHERE(.ds-action) { cursor: pointer; }",
            ":where(.ds-act\\69 on) { cursor: pointer; }",
        ] {
            let error = with_action(&format!("{ACTION}\n{rule}")).expect_err(rule);
            assert!(
                error.contains("select only") || error.contains("twice or out of order"),
                "{rule}: {error}"
            );
        }
        let error = validate_stylesheets(
            &sheets(
                ":where(.ds-surface:hover) { color: var(--ds-color-text); }",
                ACTION,
            ),
            &manifest(),
        )
        .expect_err("base state");
        assert!(error.contains("select only :where(.ds-surface)"), "{error}");
    }

    #[test]
    fn rejects_data_state_hooks() {
        for rule in [
            ":where(.ds-action[data-state=\"open\"]) { cursor: pointer; }",
            ":where(.ds-action[data-disabled]) { cursor: pointer; }",
            ":where([data-action]) { cursor: pointer; }",
        ] {
            let error = with_action(&format!("{ACTION}\n{rule}")).expect_err(rule);
            assert!(error.contains("data-* attribute"), "{rule}: {error}");
        }
    }

    #[test]
    fn requires_every_rule_in_order() {
        let error = with_action(
            &ACTION.replace(":where(.ds-action:disabled) { cursor: not-allowed; }\n", ""),
        )
        .expect_err("missing disabled");
        assert!(
            error.contains("no `:where(.ds-action:disabled)` rule"),
            "{error}"
        );
        let error = with_action(&ACTION.replace(
            ":where(a.ds-action:not(:any-link)) { cursor: not-allowed; }",
            "",
        ))
        .expect_err("missing unlinked");
        assert!(
            error.contains("no `:where(a.ds-action:not(:any-link))` rule"),
            "{error}"
        );
        let reordered = ":where(.ds-action) { cursor: pointer; }
:where(.ds-action:disabled) { cursor: not-allowed; }
:where(.ds-action.ds-action-primary) { background-color: var(--ds-color-accent); }";
        let error = with_action(reordered).expect_err("order");
        assert!(
            error.contains("before `:where(.ds-action:disabled)`"),
            "{error}"
        );
        let error = with_action(&format!(
            "{ACTION}\n:where(.ds-action) {{ cursor: pointer; }}"
        ))
        .expect_err("twice");
        assert!(error.contains("twice or out of order"), "{error}");
        let error = with_action(&ACTION.replace(
            ":where(.ds-action:disabled) { cursor: not-allowed; }",
            ":where(.ds-action:disabled) { }",
        ))
        .expect_err("empty");
        assert!(error.contains("declares nothing"), "{error}");
    }

    #[test]
    fn base_primitives_declare_no_layout_or_state() {
        for declaration in [
            "display: flex;",
            "gap: var(--ds-space-flow);",
            "flex-direction: column;",
            "margin-block: 0;",
            "min-inline-size: 0;",
        ] {
            let error = with_surface_declaration(declaration).expect_err(declaration);
            assert!(error.contains("layout property"), "{declaration}: {error}");
        }
        for declaration in [
            "cursor: pointer;",
            "font-weight: var(--ds-weight-strong);",
            "margin-inline: 0;",
        ] {
            let error = with_surface_declaration(declaration).expect_err(declaration);
            assert!(
                error.contains("not a base-primitive property"),
                "{declaration}: {error}"
            );
        }
    }

    #[test]
    fn ui_primitives_keep_focus_and_native_state() {
        for (declaration, needle) in [
            ("outline: none;", "outline"),
            ("outline-color: transparent;", "outline"),
            ("pointer-events: none;", "not a ui-primitive property"),
            ("opacity: 0.6;", "not a ui-primitive property"),
            ("visibility: hidden;", "not a ui-primitive property"),
            ("position: absolute;", "not a ui-primitive property"),
            ("padding: 0;", "not a ui-primitive property"),
            ("text-decoration: none;", "not a ui-primitive property"),
            ("--ds-action-bg: red;", "custom property"),
        ] {
            let error = with_action_declaration(declaration).expect_err(declaration);
            assert!(error.contains(needle), "{declaration}: {error}");
        }
    }

    #[test]
    fn values_bind_to_public_roles() {
        for (declaration, needle) in [
            ("min-inline-size: 44px;", "unit `px`"),
            ("gap: 1rem;", "unit `rem`"),
            ("color: red;", "`red`"),
            ("color: #fff;", "hex color"),
            ("color: var(--ds-ref-gray-12);", "internal reference"),
            ("color: var(--brand);", "var(--brand)"),
            ("color: var(--ds-color-text, black);", "fallback"),
            ("gap: clamp(0, 1, 2);", "`clamp()`"),
            ("display: flex;", "`flex`"),
        ] {
            let error = with_action_declaration(declaration).expect_err(declaration);
            assert!(error.contains(needle), "{declaration}: {error}");
        }
    }

    fn action_with_print(rule: &str) -> String {
        ACTION.replace(
            ":where(.ds-action.ds-action-primary) { background-color: var(--ds-color-accent); }",
            &format!(
                ":where(.ds-action.ds-action-primary) {{ background-color: var(--ds-color-accent); }}\n{rule}"
            ),
        )
    }

    #[test]
    fn a_print_rule_follows_the_rule_it_adapts() {
        with_action(&action_with_print(
            "@media print { :where(.ds-action.ds-action-primary) { color: var(--ds-color-text); } }",
        ))
        .expect("print rule after its own rule");

        let early = ACTION.replace(
            ":where(.ds-action) {",
            "@media print { :where(.ds-action.ds-action-primary) { color: var(--ds-color-text); } }\n:where(.ds-action) {",
        );
        let error = with_action(&early).expect_err("print rule before its own rule");
        assert!(error.contains("comes before its own rule"), "{error}");
    }

    #[test]
    fn a_print_rule_styles_only_classified_selectors_and_values() {
        let unknown = with_action(&action_with_print(
            "@media print { :where(.ds-action.ds-action-loud) { color: var(--ds-color-text); } }",
        ))
        .expect_err("unclassified selector");
        assert!(unknown.contains("`@media print`"), "{unknown}");
        let value = with_action(&action_with_print(
            "@media print { :where(.ds-action.ds-action-primary) { color: #000; } }",
        ))
        .expect_err("hex color");
        assert!(value.contains("`@media print`"), "{value}");
        let nested = with_action(&action_with_print(
            "@media print { @supports (display: grid) { :where(.ds-action.ds-action-primary) { color: var(--ds-color-text); } } }",
        ))
        .expect_err("nested at-rule");
        assert!(nested.contains("at-rule"), "{nested}");
    }

    #[test]
    fn rejects_at_rules() {
        for rule in [
            "@media (hover: hover) { :where(.ds-action:hover) { cursor: pointer; } }",
            "@container (min-width: 30rem) { :where(.ds-action) { cursor: pointer; } }",
            "@supports (display: grid) { :where(.ds-action) { cursor: pointer; } }",
            "@scope (.ds-action) { :scope { cursor: pointer; } }",
        ] {
            let error = with_action(&format!("{ACTION}\n{rule}")).expect_err(rule);
            assert!(error.contains("at-rule"), "{rule}: {error}");
        }
    }

    #[test]
    fn requires_the_inventory_to_match_the_modules() {
        let mut missing = sheets(SURFACE, ACTION);
        missing.remove(0);
        let error = validate_stylesheets(&missing, &manifest()).expect_err("entry");
        assert!(error.contains("not reached"), "{error}");
        let mut module = sheets(SURFACE, ACTION);
        module.remove(2);
        let error = validate_stylesheets(&module, &manifest()).expect_err("module");
        assert!(error.contains("action.css is not reached"), "{error}");
        let mut extra = sheets(SURFACE, ACTION);
        extra.push((
            "packages/styles/primitives/card.css".to_owned(),
            ":where(.ds-card) { color: var(--ds-color-text); }".to_owned(),
        ));
        let error = validate_stylesheets(&extra, &manifest()).expect_err("extra");
        assert!(error.contains("not a promoted primitive module"), "{error}");
        for (entry, needle) in [
            (
                ENTRY.replace("surface, action", "action, surface"),
                "@layer surface, action;",
            ),
            (
                ENTRY.replace("layer(action)", "layer(surface)"),
                "must import exactly",
            ),
            (
                format!("{ENTRY}:where(.ds-action) {{ cursor: pointer; }}"),
                "holds a rule",
            ),
        ] {
            let mut entries = sheets(SURFACE, ACTION);
            entries[0].1 = entry.clone();
            let error = validate_stylesheets(&entries, &manifest()).expect_err(&entry);
            assert!(error.contains(needle), "{entry}: {error}");
        }
    }

    const DOC: &str = "\
# Primitives

## Contract

Composes with `.ds-stack`.

### surface

Kind: `base-primitive`. Hook: `.ds-surface`.

### action

Kind: `ui-primitive`. Hooks: `.ds-action`, `.ds-action-primary`.
States: `[aria-current]`, `:disabled`, `:not(:any-link)`.

## Rejected and deferred

| Candidate | Disposition | Reason | Why |
|---|---|---|---|
| `breadcrumbs` | consumer | `product-composition` | page |
| `dialog` | deferred | `native-pattern-refinement` | DS-E01.S3.T5 — Native Platform Pattern Refinement |
| `card` | rejected | `covered-by-promoted` | surface |
";

    #[test]
    fn document_must_state_the_inventory() {
        validate_document(DOC, &manifest(), &layouts()).expect("document");
        for (doc, needle) in [
            (DOC.replace("### surface\n", ""), "no `### surface` heading"),
            (
                DOC.replace("`ui-primitive`", "`base-primitive`"),
                "does not name `ui-primitive`",
            ),
            (
                DOC.replace(", `.ds-action-primary`", ""),
                "does not name `.ds-action-primary`",
            ),
            (
                DOC.replace("`:disabled`, ", ""),
                "does not name `:disabled`",
            ),
            (DOC.replace("`.ds-stack`", "`.ds-center`"), "`.ds-center`"),
            (
                DOC.replace(
                    "| `dialog` | deferred | `native-pattern-refinement` | DS-E01.S3.T5 — Native Platform Pattern Refinement |\n",
                    "",
                ),
                "no row for candidate `dialog`",
            ),
            (
                DOC.replace("| `card` | rejected |", "| `card` | deferred |"),
                "records rejected",
            ),
            (
                DOC.replace("`covered-by-promoted`", "`covered-by-base`"),
                "records rejected `covered-by-promoted`",
            ),
            (
                format!("{DOC}| `tag` | deferred | `no-repeat-evidence` | none |\n"),
                "`tag`, which primitives.tsv",
            ),
            (
                format!("{DOC}| `card` | rejected | `covered-by-promoted` | again |\n"),
                "twice",
            ),
            (DOC.replace("## Contract\n", ""), "## Contract"),
        ] {
            let error = validate_document(&doc, &manifest(), &layouts()).expect_err(needle);
            assert!(error.contains(needle), "{needle}: {error}");
        }
    }

    const FIXTURE: &str = "<!doctype html><main>
<!-- <div class=\"ds-bogus\"> -->
<article class=\"ds-surface ds-stack\"><p>a</p></article>
<p class='ds-cluster'>
<button class=\"ds-action ds-action-primary\" type=button>Go</button>
<button class=\"ds-action\" disabled>Off</button>
<a class=\"ds-action\" href=\"#x\" aria-current=\"page\">Here</a>
<a class=\"ds-action\">Nowhere</a>
</p></main>";

    #[test]
    fn fixture_must_cover_hooks_states_and_composition() {
        assert_eq!(validate_fixture(FIXTURE, &manifest(), &layouts()), Ok(5));
        for (bad, needle) in [
            (FIXTURE.replace("<main>", "<div>"), "no `main`"),
            (FIXTURE.replace(" ds-action-primary", ""), "never uses `.ds-action-primary`"),
            (FIXTURE.replace("ds-surface ds-stack", "ds-surface"), "never composes `.ds-surface`"),
            (FIXTURE.replace(" disabled>", ">"), "state `disabled`"),
            (FIXTURE.replace("aria-current=\"page\"", "aria-current=\"false\""), "state `current`"),
            (FIXTURE.replace("aria-current=\"page\"", "aria-current=\"FALSE\""), "state `current`"),
            (FIXTURE.replace("aria-current=\"page\"", "aria-current=\"\""), "state `current`"),
            (FIXTURE.replace("aria-current=\"page\"", "aria-current"), "state `current`"),
            (FIXTURE.replace("class=\"ds-action\" disabled", "class=\"ds-action ds-stack\" disabled"), "layout hook on `ds-action`"),
            (FIXTURE.replace("class=\"ds-action\" disabled", "class=\"ds-cluster ds-action\" disabled"), "layout hook on `ds-action`"),
            (FIXTURE.replace("<a class=\"ds-action\">", "<a class=\"ds-action\" href=\"#y\">"), "state `unlinked`"),
            (FIXTURE.replace("ds-surface ds-stack", "ds-surface ds-card"), "`ds-card`"),
            (FIXTURE.replace("class=\"ds-action ds-action-primary\"", "class=\"ds-action-primary\""), "without `ds-action`"),
            (FIXTURE.replace("<button class=\"ds-action\" disabled>Off</button>", "<span class=\"ds-action\">Off</span><button class=\"ds-action\" disabled>Off</button>"), "on <span>"),
            (FIXTURE.replace("class=\"ds-action\" disabled", "class=\"ds-action ds-surface\" disabled"), "two primitives"),
            (FIXTURE.replace("disabled>", "disabled data-state=\"off\">"), "`data-state`"),
            (FIXTURE.replace("<a class=\"ds-action\">", "<a class=\"ds-action\" role=\"button\">"), "`role`"),
            (FIXTURE.replace("class='ds-cluster'", "class=ds-cluster"), "unquoted"),
        ] {
            let error = validate_fixture(&bad, &manifest(), &layouts()).expect_err(&bad);
            assert!(error.contains(needle), "{needle}: {error}");
        }
    }
}
