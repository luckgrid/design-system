//! Layout checks: the stylesheets under `packages/styles/layouts/` implement
//! exactly the layouts `layouts.tsv` promotes, each behind one zero-specificity
//! class hook, without reordering content, without breakpoints, and with values
//! bound to the public semantic roles. The layouts document and the layouts
//! fixture must cover the same inventory, including every candidate that was
//! not promoted.
//!
//! Like the other checks, this recognizes only the shapes the layouts use and
//! rejects anything else rather than guessing.

use std::collections::BTreeSet;

use crate::base::{self, Vocabulary};
use crate::css::{self, Node};
use crate::theme;
use crate::tokens;

/// Repository-relative stylesheet that imports the layout modules.
pub const LAYOUTS_ENTRY: &str = "packages/styles/layouts.css";
/// Directory that holds one module per promoted layout.
const LAYOUTS_DIR: &str = "packages/styles/layouts/";

const PUBLIC_PREVIEW: &str = "public-preview";
/// Dispositions of candidates that were not promoted.
const DISPOSITIONS: [&str; 3] = ["consumer", "deferred", "rejected"];
/// Prefix of every layout class hook.
const HOOK_PREFIX: &str = "ds-";

/// Properties a layout may declare: its display model, gaps, alignment, track
/// sizes, and the child margin and minimum-size resets. Every one is logical
/// or axis-neutral.
pub(crate) const PROPERTIES: [&str; 10] = [
    "display",
    "flex-direction",
    "flex-wrap",
    "align-items",
    "gap",
    "row-gap",
    "column-gap",
    "grid-template-columns",
    "margin-block",
    "min-inline-size",
];
/// Properties that move content away from its source order.
const REORDERING: [&str; 13] = [
    "order",
    "float",
    "flex-flow",
    "grid-auto-flow",
    "grid-area",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "grid-template-areas",
    "position",
];

const LAYOUT_VALUES: Vocabulary = Vocabulary {
    label: "a layout",
    functions: &["var", "calc", "min", "repeat", "minmax"],
    units: &["fr", "%"],
    keywords: &[
        "flex",
        "grid",
        "column",
        "row",
        "wrap",
        "nowrap",
        "center",
        "start",
        "end",
        "stretch",
        "baseline",
        "auto-fill",
        "auto-fit",
    ],
};

/// The validated `layouts.tsv` inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Promoted layout names, in sub-layer order.
    pub layouts: Vec<String>,
    /// Candidates that were not promoted: disposition, name, and reason.
    pub candidates: Vec<(String, String, String)>,
}

impl Manifest {
    fn hook(name: &str) -> String {
        format!("{HOOK_PREFIX}{name}")
    }

    fn is_hook(&self, class: &str) -> bool {
        self.layouts.iter().any(|name| Self::hook(name) == class)
    }
}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && !text.ends_with('-')
        && !text.contains("--")
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse and validate the layouts inventory.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut layouts = Vec::new();
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
                "layouts line {line_number} must be <class><tab><layout or candidate><tab><hook or reason>"
            ));
        };
        if !is_name(name) {
            return Err(format!(
                "layouts line {line_number} name '{name}' is not a lowercase hyphenated name"
            ));
        }
        if !seen.insert((*name).to_owned()) {
            return Err(format!(
                "layouts line {line_number} repeats '{name}'; a layout or candidate is classified exactly once"
            ));
        }
        if *class == PUBLIC_PREVIEW {
            let hook = format!(".{}", Manifest::hook(name));
            if *subject != hook {
                return Err(format!(
                    "layouts line {line_number} promotes '{name}' with hook '{subject}'; its hook is `{hook}`"
                ));
            }
            layouts.push((*name).to_owned());
        } else if DISPOSITIONS.contains(class) {
            if !is_name(subject) {
                return Err(format!(
                    "layouts line {line_number} gives '{name}' the reason '{subject}'; a reason is a lowercase hyphenated code"
                ));
            }
            candidates.push((
                (*class).to_owned(),
                (*name).to_owned(),
                (*subject).to_owned(),
            ));
        } else {
            return Err(format!(
                "layouts line {line_number} classifies '{name}' as '{class}'; layouts are {PUBLIC_PREVIEW} or one of {}",
                DISPOSITIONS.join(", ")
            ));
        }
    }
    if layouts.is_empty() {
        return Err("layouts inventory promotes no layout".to_owned());
    }
    Ok(Manifest {
        layouts,
        candidates,
    })
}

/// What validated layouts cover.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub modules: usize,
    pub rules: usize,
    pub declarations: usize,
}

/// Validate the layout stylesheets among the reached Design System stylesheets.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_stylesheets(
    stylesheets: &[(String, String)],
    manifest: &Manifest,
) -> Result<Summary, String> {
    let entry = stylesheets
        .iter()
        .find(|(file, _)| file == LAYOUTS_ENTRY)
        .ok_or_else(|| {
            format!("{LAYOUTS_ENTRY} is not reached from any declared export; the layouts must enter the cascade")
        })?;
    validate_entry(&entry.0, &entry.1, manifest)?;

    let mut modules = Vec::new();
    for (file, source) in stylesheets {
        if let Some(name) = file.strip_prefix(LAYOUTS_DIR) {
            let layout = name.strip_suffix(".css").unwrap_or(name);
            if !manifest.layouts.iter().any(|promoted| promoted == layout) {
                return Err(format!(
                    "{file} is not a promoted layout module; layouts.tsv promotes {}",
                    manifest.layouts.join(", ")
                ));
            }
            modules.push((layout.to_owned(), file.clone(), source.clone()));
        }
    }
    for layout in &manifest.layouts {
        if !modules.iter().any(|(name, _, _)| name == layout) {
            return Err(format!(
                "{LAYOUTS_DIR}{layout}.css is not reached; every promoted layout enters the cascade"
            ));
        }
    }

    let mut summary = Summary {
        modules: modules.len(),
        ..Summary::default()
    };
    for (layout, file, source) in &modules {
        let container = format!(":where(.{})", Manifest::hook(layout));
        let children = format!("{container} > :where(*)");
        theme::reject_selector_comments(file, source)?;
        let mut selectors = BTreeSet::new();
        for node in css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))? {
            let Node::Style { prelude, body } = node else {
                return Err(format!(
                    "{file} holds an at-rule; layouts are intrinsic and hold unconditional style rules only"
                ));
            };
            let selector = prelude.split_whitespace().collect::<Vec<_>>().join(" ");
            if selector != container && selector != children {
                return Err(format!(
                    "{file} `{selector}`: a {layout} rule selects `{container}` or `{children}` only, \
so it has zero specificity and styles no other hook"
                ));
            }
            if !selectors.insert(selector.clone()) {
                return Err(format!(
                    "{file} `{selector}` appears twice; each layout has one container rule and at most one child rule"
                ));
            }
            let declarations = tokens::split_declarations(file, &body)?;
            if declarations.is_empty() {
                return Err(format!("{file} `{selector}` declares nothing"));
            }
            for (name, value) in &declarations {
                check_declaration(name, value)
                    .map_err(|error| format!("{file} `{selector}` {name}: {error}"))?;
            }
            if selector == container && !declarations.iter().any(|(name, _)| name == "display") {
                return Err(format!(
                    "{file} `{selector}` sets no display; the container rule states the layout model"
                ));
            }
            summary.rules += 1;
            summary.declarations += declarations.len();
        }
        if !selectors.contains(&container) {
            return Err(format!("{file} has no `{container}` rule"));
        }
    }
    Ok(summary)
}

/// `layouts.css` holds only its sub-layer order and one import per promoted
/// layout, both in inventory order.
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
                    "{file} holds a rule; it only orders and imports the layout modules"
                ));
            }
        }
    }
    let expected_order = manifest.layouts.join(", ");
    if order != [expected_order.clone()] {
        return Err(format!(
            "{file} must declare exactly one `@layer {expected_order};` in inventory order"
        ));
    }
    let expected: Vec<String> = manifest
        .layouts
        .iter()
        .map(|layout| format!("\"./layouts/{layout}.css\" layer({layout})"))
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
fn check_declaration(name: &str, value: &str) -> Result<(), String> {
    let property = name.to_ascii_lowercase();
    if property.starts_with("--") {
        return Err(
            "declares a custom property; layouts bind existing roles and consumers override the property itself"
                .to_owned(),
        );
    }
    if REORDERING.contains(&property.as_str()) {
        return Err(
            "can move content away from source order; layouts keep visual order equal to source order"
                .to_owned(),
        );
    }
    if !PROPERTIES.contains(&property.as_str()) {
        return Err(format!(
            "is not a layout property; layouts declare only the logical properties {}",
            PROPERTIES.join(", ")
        ));
    }
    let lower = value.to_ascii_lowercase();
    if lower.contains("reverse") || lower.contains("dense") {
        return Err(
            "reverses or repacks content; layouts keep visual order equal to source order"
                .to_owned(),
        );
    }
    base::check_value(&lower, &LAYOUT_VALUES)
}

/// Heading of the layouts document section that states each contract.
const CONTRACT_SECTION: &str = "## Contract";
/// Heading of the layouts document section that lists the other candidates.
const CANDIDATES_SECTION: &str = "## Rejected and deferred";

/// The layouts document states the inventory: the contract section has one
/// `###` heading per promoted layout naming its hook and no other hook; the
/// candidates section names every candidate that was not promoted.
pub fn validate_document(document: &str, manifest: &Manifest) -> Result<(), String> {
    let contract = base::section(document, CONTRACT_SECTION)?;
    for layout in &manifest.layouts {
        let heading = format!("### {layout}");
        if !contract.lines().any(|line| line.trim_end() == heading) {
            return Err(format!(
                "the layouts document `{CONTRACT_SECTION}` section has no `{heading}` heading"
            ));
        }
    }
    let named: BTreeSet<String> = base::backticked(contract).into_iter().collect();
    for item in &named {
        if let Some(class) = item.strip_prefix('.')
            && class.starts_with(HOOK_PREFIX)
            && !manifest.is_hook(class)
        {
            return Err(format!(
                "the layouts document names hook `{item}`, which layouts.tsv does not promote"
            ));
        }
    }
    if let Some(layout) = manifest
        .layouts
        .iter()
        .find(|layout| !named.contains(&format!(".{}", Manifest::hook(layout))))
    {
        return Err(format!(
            "the layouts document `{CONTRACT_SECTION}` section does not name the `{layout}` hook"
        ));
    }

    let candidates = base::section(document, CANDIDATES_SECTION)?;
    let listed: BTreeSet<String> = base::backticked(candidates).into_iter().collect();
    if let Some((_, name, _)) = manifest
        .candidates
        .iter()
        .find(|(_, name, _)| !listed.contains(name))
    {
        return Err(format!(
            "the layouts document `{CANDIDATES_SECTION}` section does not name candidate `{name}`"
        ));
    }
    Ok(())
}

/// The layouts fixture uses every promoted hook and no unpromoted `ds-` class.
/// Returns the number of hooked elements.
pub fn validate_fixture(html: &str, manifest: &Manifest) -> Result<usize, String> {
    let lower = html.to_ascii_lowercase();
    crate::lexical::reject_inert_or_escaped_markup(&lower, "layouts fixture")?;
    if !base::has_element(&lower, "main") {
        return Err(
            "the layouts fixture has no `main`; it places the layouts inside a consumer-owned page shell"
                .to_owned(),
        );
    }
    let mut used = BTreeSet::new();
    let mut hooked = 0;
    for value in class_values(&lower)? {
        let mut hooks_here = 0;
        for class in value.split_whitespace() {
            if class.starts_with(HOOK_PREFIX) {
                if !manifest.is_hook(class) {
                    return Err(format!(
                        "the layouts fixture uses class `{class}`, which layouts.tsv does not promote"
                    ));
                }
                used.insert(class.to_owned());
                hooks_here += 1;
            }
        }
        if hooks_here > 1 {
            return Err(format!(
                "the layouts fixture puts two layout hooks on one element (`{value}`); a layout arranges its own children"
            ));
        }
        hooked += hooks_here;
    }
    if let Some(layout) = manifest
        .layouts
        .iter()
        .find(|layout| !used.contains(&Manifest::hook(layout)))
    {
        return Err(format!(
            "the layouts fixture never uses `.{}`; every promoted layout needs fixture coverage",
            Manifest::hook(layout)
        ));
    }
    Ok(hooked)
}

/// The value of every `class` attribute, which must be quoted.
fn class_values(html: &str) -> Result<Vec<&str>, String> {
    let mut values = Vec::new();
    for (index, _) in html.match_indices("class") {
        if !html[..index].ends_with(char::is_whitespace) {
            continue;
        }
        let rest = html[index + "class".len()..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''));
        let Some(quote) = quote else {
            return Err("the layouts fixture has an unquoted class attribute".to_owned());
        };
        let body = &rest[1..];
        let end = body
            .find(quote)
            .ok_or("the layouts fixture has an unterminated class attribute")?;
        values.push(&body[..end]);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
# comment
public-preview\tstack\t.ds-stack
public-preview\tgrid\t.ds-grid
consumer\tpage-shell\tpage-composition
deferred\tsidebar\tno-consumer-evidence
rejected\tfooter-reorder\tbreaks-source-order
";

    const ENTRY: &str = "@layer stack, grid;
@import \"./layouts/stack.css\" layer(stack);
@import \"./layouts/grid.css\" layer(grid);
";

    const STACK: &str =
        ":where(.ds-stack) { display: flex; flex-direction: column; gap: var(--ds-space-flow); }
:where(.ds-stack) > :where(*) { margin-block: 0; min-inline-size: 0; }";
    const GRID: &str = ":where(.ds-grid) { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, calc(var(--ds-size-measure) / 3)), 1fr)); gap: var(--ds-space-flow); }";

    fn manifest() -> Manifest {
        parse_manifest(MANIFEST).expect("manifest")
    }

    fn sheets(stack: &str) -> Vec<(String, String)> {
        vec![
            (LAYOUTS_ENTRY.to_owned(), ENTRY.to_owned()),
            (
                "packages/styles/layouts/stack.css".to_owned(),
                stack.to_owned(),
            ),
            (
                "packages/styles/layouts/grid.css".to_owned(),
                GRID.to_owned(),
            ),
        ]
    }

    fn with_rule(rule: &str) -> Result<Summary, String> {
        validate_stylesheets(&sheets(&format!("{STACK}\n{rule}")), &manifest())
    }

    fn with_declaration(declaration: &str) -> Result<Summary, String> {
        validate_stylesheets(
            &sheets(&STACK.replace("gap: var(--ds-space-flow);", declaration)),
            &manifest(),
        )
    }

    #[test]
    fn accepts_conforming_layouts() {
        let summary = validate_stylesheets(&sheets(STACK), &manifest()).expect("layouts");
        assert_eq!(summary.modules, 2);
        assert_eq!(summary.rules, 3);
        assert_eq!(summary.declarations, 8);
    }

    #[test]
    fn manifest_rejects_bad_rows() {
        for (text, needle) in [
            ("internal\tstack\t.ds-stack", "public-preview or one of"),
            ("public-preview\tstack\t.stack", "its hook is `.ds-stack`"),
            ("public-preview\tstack\t.ds-grid", "its hook is `.ds-stack`"),
            ("public-preview\tStack\t.ds-Stack", "not a lowercase"),
            ("public-preview\tstack-\t.ds-stack-", "not a lowercase"),
            ("public-preview\tstack", "must be <class>"),
            ("deferred\tsidebar\tNo evidence", "reason"),
            (
                "public-preview\tstack\t.ds-stack\ndeferred\tstack\tlater",
                "repeats 'stack'",
            ),
            ("deferred\tsidebar\tno-consumer", "promotes no layout"),
        ] {
            let error = parse_manifest(text).expect_err(text);
            assert!(error.contains(needle), "{text}: {error}");
        }
    }

    #[test]
    fn rejects_selectors_other_than_the_hook() {
        for rule in [
            ".ds-stack { gap: 0; }",
            ":is(.ds-stack) { gap: 0; }",
            ":where(.ds-stack, .ds-grid) { gap: 0; }",
            ":where(.ds-stack) > * { gap: 0; }",
            ":where(.ds-stack) :where(*) { gap: 0; }",
            ":where(.ds-stack) + :where(*) { gap: 0; }",
            ":where(.ds-stack):hover { gap: 0; }",
            ":where(.ds-grid) { gap: 0; }",
            ":where([data-ds-layout~=\"stack\"]) { gap: 0; }",
            ":where([data-stack]) { gap: 0; }",
            ":where(#stack) { gap: 0; }",
            ":where(main) { gap: 0; }",
            ":where(.ds-st\\61 ck) { gap: 0; }",
            ":WHERE(.ds-stack) { gap: 0; }",
        ] {
            let error = with_rule(rule).expect_err(rule);
            assert!(
                error.contains("selects `:where(.ds-stack)`"),
                "{rule}: {error}"
            );
        }
    }

    #[test]
    fn rejects_selector_comments_and_duplicates() {
        let error = with_rule(":where(.ds-/**/stack) { gap: 0; }").expect_err("comment");
        assert!(error.contains("comment"), "{error}");
        let error = with_rule(":where(.ds-stack) { display: block; }").expect_err("twice");
        assert!(error.contains("appears twice"), "{error}");
        let error = validate_stylesheets(
            &sheets(":where(.ds-stack) > :where(*) { margin-block: 0; }"),
            &manifest(),
        )
        .expect_err("no container");
        assert!(error.contains("has no `:where(.ds-stack)` rule"), "{error}");
        let error = validate_stylesheets(
            &sheets(":where(.ds-stack) { gap: var(--ds-space-flow); }"),
            &manifest(),
        )
        .expect_err("no display");
        assert!(error.contains("sets no display"), "{error}");
    }

    #[test]
    fn rejects_reordering() {
        for declaration in [
            "order: 999;",
            "ORDER: -1;",
            "float: inline-start;",
            "flex-flow: row wrap;",
            "grid-auto-flow: dense;",
            "grid-area: aside;",
            "grid-row: 1;",
            "grid-column-start: 2;",
            "grid-template-areas: \"a b\";",
            "position: absolute;",
        ] {
            let error = with_declaration(declaration).expect_err(declaration);
            assert!(error.contains("source order"), "{declaration}: {error}");
        }
        for declaration in [
            "flex-direction: column-reverse;",
            "flex-wrap: wrap-reverse;",
        ] {
            let error = with_declaration(declaration).expect_err(declaration);
            assert!(error.contains("reverses"), "{declaration}: {error}");
        }
    }

    #[test]
    fn rejects_physical_and_unclassified_properties() {
        for declaration in [
            "margin-top: 0;",
            "width: 100%;",
            "max-width: 40%;",
            "padding: 0;",
            "justify-content: center;",
            "container-type: inline-size;",
            "color: currentcolor;",
        ] {
            let error = with_declaration(declaration).expect_err(declaration);
            assert!(
                error.contains("not a layout property"),
                "{declaration}: {error}"
            );
        }
    }

    #[test]
    fn rejects_custom_properties_and_local_values() {
        for (declaration, needle) in [
            ("--ds-stack-gap: 1rem;", "custom property"),
            ("--gap: 1rem;", "custom property"),
            ("gap: 1rem;", "unit `rem`"),
            ("gap: 16px;", "unit `px`"),
            ("gap: var(--gap);", "var(--gap)"),
            ("gap: var(--ds-ref-space-16-32);", "internal reference"),
            ("gap: var(--ds-space-flow, 1rem);", "fallback"),
            ("gap: clamp(0%, 1%, 2%);", "`clamp()`"),
            ("display: contents;", "`contents`"),
        ] {
            let error = with_declaration(declaration).expect_err(declaration);
            assert!(error.contains(needle), "{declaration}: {error}");
        }
    }

    #[test]
    fn rejects_at_rules() {
        for rule in [
            "@media (min-width: 40em) { :where(.ds-stack) { gap: 0; } }",
            "@container (min-width: 30rem) { :where(.ds-stack) { gap: 0; } }",
            "@supports (display: grid) { :where(.ds-stack) { gap: 0; } }",
            "@scope (.ds-stack) { :scope { gap: 0; } }",
        ] {
            let error = with_rule(rule).expect_err(rule);
            assert!(error.contains("at-rule"), "{rule}: {error}");
        }
    }

    #[test]
    fn requires_the_inventory_to_match_the_modules() {
        let mut missing = sheets(STACK);
        missing.remove(0);
        let error = validate_stylesheets(&missing, &manifest()).expect_err("no entry");
        assert!(error.contains("not reached"), "{error}");
        let mut module = sheets(STACK);
        module.remove(2);
        let error = validate_stylesheets(&module, &manifest()).expect_err("no module");
        assert!(error.contains("grid.css is not reached"), "{error}");
        let mut extra = sheets(STACK);
        extra.push((
            "packages/styles/layouts/sidebar.css".to_owned(),
            ":where(.ds-sidebar) { display: flex; }".to_owned(),
        ));
        let error = validate_stylesheets(&extra, &manifest()).expect_err("extra module");
        assert!(error.contains("not a promoted layout module"), "{error}");
    }

    #[test]
    fn requires_an_exact_entry() {
        for (entry, needle) in [
            (
                ENTRY.replace("@layer stack, grid;", "@layer grid, stack;"),
                "@layer stack, grid;",
            ),
            (
                ENTRY.replace("@layer stack, grid;\n", ""),
                "@layer stack, grid;",
            ),
            (
                ENTRY.replace("layer(grid)", "layer(stack)"),
                "must import exactly",
            ),
            (
                format!("{ENTRY}:where(.ds-stack) {{ display: flex; }}"),
                "holds a rule",
            ),
        ] {
            let mut entries = sheets(STACK);
            entries[0].1 = entry.clone();
            let error = validate_stylesheets(&entries, &manifest()).expect_err(&entry);
            assert!(error.contains(needle), "{entry}: {error}");
        }
    }

    const DOC: &str = "\
# Layouts

## Contract

### stack

Hook: `.ds-stack`.

### grid

Hook: `.ds-grid`.

## Rejected and deferred

| `page-shell` | consumer |
| `sidebar` | deferred |
| `footer-reorder` | rejected |
";

    #[test]
    fn document_must_state_the_inventory() {
        validate_document(DOC, &manifest()).expect("document");
        let error =
            validate_document(&DOC.replace("### grid\n", ""), &manifest()).expect_err("heading");
        assert!(error.contains("### grid"), "{error}");
        let error = validate_document(&DOC.replace("Hook: `.ds-grid`.", ""), &manifest())
            .expect_err("hook");
        assert!(error.contains("`grid` hook"), "{error}");
        let error = validate_document(
            &DOC.replace("Hook: `.ds-grid`.", "Hook: `.ds-grid`, `.ds-center`."),
            &manifest(),
        )
        .expect_err("unpromoted hook");
        assert!(error.contains("`.ds-center`"), "{error}");
        let error = validate_document(&DOC.replace("| `sidebar` | deferred |\n", ""), &manifest())
            .expect_err("candidate");
        assert!(error.contains("candidate `sidebar`"), "{error}");
        let error =
            validate_document(&DOC.replace("## Contract\n", ""), &manifest()).expect_err("section");
        assert!(error.contains("## Contract"), "{error}");
    }

    #[test]
    fn fixture_must_cover_the_hooks() {
        let html = "<main><div class=\"ds-stack\"><p>a</p><ul class='ds-grid x'><li>b</li></ul></div></main>";
        assert_eq!(validate_fixture(html, &manifest()), Ok(2));
        for (bad, needle) in [
            (html.replace("<main>", "<div>"), "no `main`"),
            (
                html.replace(" class='ds-grid x'", ""),
                "never uses `.ds-grid`",
            ),
            (
                html.replace("ds-grid x", "ds-grid ds-center"),
                "`ds-center`",
            ),
            (
                html.replace("ds-grid x", "ds-grid ds-stack"),
                "two layout hooks",
            ),
            (
                html.replace("class='ds-grid x'", "class=ds-grid"),
                "unquoted",
            ),
        ] {
            let error = validate_fixture(&bad, &manifest()).expect_err(&bad);
            assert!(error.contains(needle), "{bad}: {error}");
        }
    }
}
