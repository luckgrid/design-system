//! Token authority checks: the custom properties declared by Design System
//! stylesheets must match the `tokens.tsv` compatibility inventory, keep the
//! reference and semantic tiers distinct, and depend on nothing but each other.
//!
//! Like `css`, this recognizes only the shapes the token contract uses and
//! rejects anything else rather than guessing.

use std::collections::{BTreeMap, BTreeSet};

use crate::css::{self, Node};

/// Repository-relative stylesheet that owns every reference value.
pub const REFERENCE_FILE: &str = "packages/styles/tokens/reference.css";
/// Repository-relative stylesheet that owns every semantic role.
pub const SEMANTIC_FILE: &str = "packages/styles/tokens/semantic.css";

const REFERENCE_PREFIX: &str = "--ds-ref-";
const TOKEN_PREFIX: &str = "--ds-";
const INTERNAL: &str = "internal";
const PUBLIC_PREVIEW: &str = "public-preview";
const VALUE_TYPES: [&str; 4] = ["color", "font-family", "length", "number"];

/// Fluid reference values interpolate between these viewport widths, in rem at
/// the default 16px root size (480px and 2560px).
const FLUID_MIN_VIEWPORT_REM: f64 = 30.0;
const FLUID_MAX_VIEWPORT_REM: f64 = 160.0;
/// A fluid text maximum may be at most this multiple of its minimum, so text
/// still reaches 200% of its size under browser zoom (WCAG 1.4.4).
const TEXT_MAX_RATIO: f64 = 2.5;
/// Allowed rounding error when a clamp() formula is checked against its bounds.
const FORMULA_TOLERANCE_REM: f64 = 0.001;

/// One `tokens.tsv` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub class: String,
    pub name: String,
    pub value_type: String,
}

/// One custom-property declaration found in a stylesheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub file: String,
    pub selector: String,
    pub name: String,
    pub value: String,
}

/// Parse and validate the inventory. Each property appears once, its class
/// follows its tier, and its value type is one the contract names.
pub fn parse_inventory(text: &str) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = Vec::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [class, name, value_type] = fields.as_slice() else {
            return Err(format!(
                "tokens line {line_number} must be <class><tab><custom-property><tab><value-type>"
            ));
        };
        if !is_token_name(name) {
            return Err(format!(
                "tokens line {line_number} property '{name}' must be a lowercase {TOKEN_PREFIX}* name"
            ));
        }
        let expected = if name.starts_with(REFERENCE_PREFIX) {
            INTERNAL
        } else {
            PUBLIC_PREVIEW
        };
        if *class != expected {
            return Err(format!(
                "tokens line {line_number} classifies {name} as '{class}'; {REFERENCE_PREFIX}* reference values are {INTERNAL} and semantic roles are {PUBLIC_PREVIEW} until a reviewed contract earns public-stable"
            ));
        }
        if !VALUE_TYPES.contains(value_type) {
            return Err(format!(
                "tokens line {line_number} value type '{value_type}' is not one of {}",
                VALUE_TYPES.join(", ")
            ));
        }
        if rows.iter().any(|row| row.name == *name) {
            return Err(format!(
                "tokens line {line_number} classifies {name} more than once"
            ));
        }
        rows.push(Row {
            class: (*class).to_owned(),
            name: (*name).to_owned(),
            value_type: (*value_type).to_owned(),
        });
    }
    if rows.is_empty() {
        return Err("tokens inventory declares no properties".to_owned());
    }
    Ok(rows)
}

/// Collect every custom-property declaration in a stylesheet.
///
/// Declarations inside style rules nested in group rules are included. Nested
/// style rules and escaped code points inside a declaration block are rejected
/// because they could hide a declaration from this scan.
pub fn declarations(file: &str, source: &str) -> Result<Vec<Declaration>, String> {
    let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
    let mut found = Vec::new();
    collect(file, &nodes, &mut found)?;
    Ok(found)
}

fn collect(file: &str, nodes: &[Node], found: &mut Vec<Declaration>) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Group { children, .. } => collect(file, children, found)?,
            Node::Style { prelude, body } => {
                for (name, value) in split_declarations(file, body)? {
                    if name.starts_with("--") {
                        found.push(Declaration {
                            file: file.to_owned(),
                            selector: prelude.trim().to_owned(),
                            name,
                            value,
                        });
                    }
                }
            }
            Node::Statement { .. } | Node::Opaque { .. } => {}
        }
    }
    Ok(())
}

/// Split a declaration block into `(name, value)` pairs.
fn split_declarations(file: &str, body: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    for c in body.chars() {
        if let Some(open) = quote {
            if c == '\\' {
                return Err(format!(
                    "stylesheet {file} uses an escape inside a declaration string; the token checker does not support escapes"
                ));
            }
            if c == open {
                quote = None;
            }
            current.push(c);
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '\\' => {
                return Err(format!(
                    "stylesheet {file} uses an escaped code point in a declaration block; write declarations literally"
                ));
            }
            '{' | '}' => {
                return Err(format!(
                    "stylesheet {file} nests a rule inside a style rule; the token checker requires flat declaration blocks"
                ));
            }
            '(' => depth += 1,
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| format!("stylesheet {file} has an unbalanced ')'"))?;
            }
            ';' if depth == 0 => {
                push_declaration(file, &current, &mut pairs)?;
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if depth != 0 {
        return Err(format!("stylesheet {file} has an unbalanced '('"));
    }
    push_declaration(file, &current, &mut pairs)?;
    Ok(pairs)
}

fn push_declaration(
    file: &str,
    text: &str,
    pairs: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let (name, value) = text
        .split_once(':')
        .ok_or_else(|| format!("stylesheet {file} has a declaration without ':' `{text}`"))?;
    pairs.push((name.trim().to_owned(), normalize_space(value)));
    Ok(())
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_token_name(name: &str) -> bool {
    name.strip_prefix(TOKEN_PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && !rest.starts_with('-')
            && !rest.ends_with('-')
            && rest
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

/// What a validated token authority contains.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub reference: usize,
    pub semantic: usize,
    pub fluid: usize,
    pub scheme_pairs: usize,
}

/// Validate every custom property declared by the reached Design System
/// stylesheets against the inventory and the tier rules.
///
/// `stylesheets` pairs each repository-relative path with its source.
pub fn validate_authority(
    stylesheets: &[(String, String)],
    rows: &[Row],
) -> Result<Summary, String> {
    let inventory: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.name.as_str(), row)).collect();

    let mut declared: BTreeMap<String, Declaration> = BTreeMap::new();
    let mut all = Vec::new();
    for (file, source) in stylesheets {
        for declaration in declarations(file, source)? {
            if let Some(previous) = declared.get(&declaration.name) {
                return Err(format!(
                    "{} is declared more than once ({} and {}); each token has one authored value",
                    declaration.name, previous.file, declaration.file
                ));
            }
            declared.insert(declaration.name.clone(), declaration.clone());
            all.push(declaration);
        }
    }

    for declaration in &all {
        let name = declaration.name.as_str();
        let Some(row) = inventory.get(name) else {
            return Err(format!(
                "{} declares {name}, which is not classified in tokens.tsv",
                declaration.file
            ));
        };
        let owner = if row.class == INTERNAL {
            REFERENCE_FILE
        } else {
            SEMANTIC_FILE
        };
        if declaration.file != owner {
            return Err(format!(
                "{name} is declared in {}; {} tokens belong only in {owner}",
                declaration.file, row.class
            ));
        }
        if declaration.selector != ":root" {
            return Err(format!(
                "{name} is declared under `{}`; tokens are declared on :root only",
                declaration.selector
            ));
        }
    }

    for row in rows {
        if !declared.contains_key(&row.name) {
            return Err(format!(
                "tokens.tsv classifies {}, but no Design System stylesheet declares it",
                row.name
            ));
        }
    }

    // Every var() reference in every reached stylesheet must resolve inside the
    // authority, so no token or rule depends on a provider or consumer namespace.
    for (file, source) in stylesheets {
        for (name, value) in all_declarations(file, source)? {
            for reference in var_references(file, &name, &value)? {
                if !declared.contains_key(&reference) {
                    return Err(format!(
                        "{file} {name} references var({reference}), which the Design System token authority does not declare"
                    ));
                }
            }
        }
    }

    let mut summary = Summary::default();
    for declaration in &all {
        let row = inventory[declaration.name.as_str()];
        if row.class == INTERNAL {
            summary.reference += 1;
            if validate_reference(declaration, row)? {
                summary.fluid += 1;
            }
        } else {
            summary.semantic += 1;
            if validate_semantic(declaration, row, &inventory)? {
                summary.scheme_pairs += 1;
            }
        }
    }
    Ok(summary)
}

/// Every declaration (custom or not) in a stylesheet, for var() tracing.
fn all_declarations(file: &str, source: &str) -> Result<Vec<(String, String)>, String> {
    let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
    let mut pairs = Vec::new();
    let mut stack: Vec<&Node> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        match node {
            Node::Group { children, .. } => stack.extend(children.iter()),
            Node::Style { body, .. } => pairs.extend(split_declarations(file, body)?),
            Node::Statement { .. } | Node::Opaque { .. } => {}
        }
    }
    Ok(pairs)
}

/// Names referenced through `var()` in one value. Function names are matched
/// ASCII case-insensitively, as browsers do.
fn var_references(file: &str, name: &str, value: &str) -> Result<Vec<String>, String> {
    let lower = value.to_ascii_lowercase();
    let mut references = Vec::new();
    for (index, _) in lower.match_indices("var(") {
        let preceded_by_ident = lower[..index]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if preceded_by_ident {
            continue;
        }
        let rest = value[index + 4..].trim_start();
        let end = rest
            .find(|c: char| c == ',' || c == ')' || c.is_whitespace())
            .ok_or_else(|| format!("{file} {name} has an unterminated var()"))?;
        let reference = &rest[..end];
        if !reference.starts_with("--") {
            return Err(format!("{file} {name} has a malformed var() `{value}`"));
        }
        references.push(reference.to_owned());
    }
    Ok(references)
}

/// Reference values are raw and scheme-neutral. Returns whether the value is a
/// fluid clamp().
fn validate_reference(declaration: &Declaration, row: &Row) -> Result<bool, String> {
    let name = &declaration.name;
    let value = declaration.value.as_str();
    let lower = value.to_ascii_lowercase();
    if lower.contains("var(") {
        return Err(format!(
            "reference {name} uses var(); reference values are raw literals"
        ));
    }
    if lower.contains("light-dark(") {
        return Err(format!(
            "reference {name} uses light-dark(); reference values are scheme-neutral and the semantic tier pairs them"
        ));
    }
    match row.value_type.as_str() {
        "color" => {
            let is_oklch = lower.starts_with("oklch(")
                && lower.ends_with(')')
                && !lower["oklch(".len()..].contains('(')
                && !lower.contains(" from ");
            if !is_oklch {
                return Err(format!(
                    "reference color {name} must be one plain oklch() value; relative color syntax and other functions are not used in the required floor"
                ));
            }
            Ok(false)
        }
        "length" if lower.contains("clamp(") => {
            validate_clamp(name, value)?;
            Ok(true)
        }
        _ if lower.contains('(') => Err(format!(
            "reference {name} uses a function; only oklch() colors and clamp() lengths are authored as reference functions"
        )),
        _ => Ok(false),
    }
}

/// A fluid reference must be `clamp(<rem>, <rem> + <vw>, <rem>)`, interpolate
/// exactly between its bounds across the fluid viewport range, and match the
/// pixel bounds its name records.
fn validate_clamp(name: &str, value: &str) -> Result<(), String> {
    let malformed = || {
        format!(
            "fluid reference {name} must be `clamp(<min>rem, <intercept>rem + <slope>vw, <max>rem)`; found `{value}`"
        )
    };
    let inner = value
        .strip_prefix("clamp(")
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(malformed)?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let [min, preferred, max] = parts.as_slice() else {
        return Err(malformed());
    };
    let (intercept, slope) = preferred.split_once(" + ").ok_or_else(malformed)?;
    let unit = |text: &str, unit: &str| -> Result<f64, String> {
        text.trim()
            .strip_suffix(unit)
            .and_then(|number| number.parse::<f64>().ok())
            .filter(|number| number.is_finite() && *number >= 0.0)
            .ok_or_else(malformed)
    };
    let min = unit(min, "rem")?;
    let max = unit(max, "rem")?;
    let intercept = unit(intercept, "rem")?;
    let slope = unit(slope, "vw")?;
    if min <= 0.0 || min >= max || slope <= 0.0 {
        return Err(format!(
            "fluid reference {name} must grow from a positive minimum to a larger maximum"
        ));
    }

    // 1vw is 1% of the viewport width.
    let at = |viewport_rem: f64| intercept + slope * viewport_rem / 100.0;
    if (at(FLUID_MIN_VIEWPORT_REM) - min).abs() > FORMULA_TOLERANCE_REM
        || (at(FLUID_MAX_VIEWPORT_REM) - max).abs() > FORMULA_TOLERANCE_REM
    {
        return Err(format!(
            "fluid reference {name} does not interpolate from {min}rem at a {FLUID_MIN_VIEWPORT_REM}rem viewport to {max}rem at a {FLUID_MAX_VIEWPORT_REM}rem viewport"
        ));
    }

    let (family, bounds) = name
        .strip_prefix(REFERENCE_PREFIX)
        .and_then(|rest| rest.split_once('-'))
        .ok_or_else(malformed)?;
    let named: Vec<f64> = bounds
        .split('-')
        .filter_map(|number| number.parse::<f64>().ok())
        .collect();
    let [named_min, named_max] = named.as_slice() else {
        return Err(format!(
            "fluid reference {name} must be named --ds-ref-<family>-<min px>-<max px>"
        ));
    };
    if (named_min - min * 16.0).abs() > f64::EPSILON
        || (named_max - max * 16.0).abs() > f64::EPSILON
    {
        return Err(format!(
            "fluid reference {name} is named for {named_min}px-{named_max}px but its bounds are {}px-{}px",
            min * 16.0,
            max * 16.0
        ));
    }
    if family == "text" && max / min > TEXT_MAX_RATIO {
        return Err(format!(
            "fluid text reference {name} grows more than {TEXT_MAX_RATIO}x; text must still reach 200% under zoom"
        ));
    }
    Ok(())
}

/// Semantic roles are assigned from the reference tier, from another semantic
/// role, or from a fixed literal. Returns whether the role is a light/dark pair.
fn validate_semantic(
    declaration: &Declaration,
    row: &Row,
    inventory: &BTreeMap<&str, &Row>,
) -> Result<bool, String> {
    let name = &declaration.name;
    let value = declaration.value.as_str();
    let lower = value.to_ascii_lowercase();

    if let Some(target) = sole_var(value) {
        let Some(target_row) = inventory.get(target.as_str()) else {
            return Err(format!("semantic {name} aliases unclassified {target}"));
        };
        if target_row.value_type != row.value_type {
            return Err(format!(
                "semantic {name} ({}) aliases {target} ({}); an alias keeps its value type",
                row.value_type, target_row.value_type
            ));
        }
        return Ok(false);
    }

    if let Some(arguments) = lower
        .strip_prefix("light-dark(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        if row.value_type != "color" {
            return Err(format!(
                "semantic {name} uses light-dark() but is typed {}",
                row.value_type
            ));
        }
        let original = &value["light-dark(".len()..value.len() - 1];
        let branches: Vec<&str> = split_top_level(original);
        let valid = branches.len() == 2
            && arguments.matches('(').count() == 2
            && branches.iter().all(|branch| {
                sole_var(branch).is_some_and(|target| {
                    target.starts_with(REFERENCE_PREFIX)
                        && inventory
                            .get(target.as_str())
                            .is_some_and(|target_row| target_row.value_type == "color")
                })
            });
        if !valid {
            return Err(format!(
                "semantic {name} must pair two reference colors as light-dark(var(--ds-ref-*), var(--ds-ref-*)); found `{value}`"
            ));
        }
        return Ok(true);
    }

    let literal = !lower.contains('(') && !lower.contains('#');
    match row.value_type.as_str() {
        "color" | "font-family" => Err(format!(
            "semantic {} {name} must alias a token or pair reference colors; literal values belong in the reference tier",
            row.value_type
        )),
        "length" if literal && is_fixed_length(&lower) => Ok(false),
        "number" if literal && lower.parse::<f64>().is_ok_and(f64::is_finite) => Ok(false),
        _ => Err(format!(
            "semantic {name} `{value}` must alias a token or be one fixed {} literal; fluid formulas belong in the reference tier",
            row.value_type
        )),
    }
}

/// The custom property named by a value that is exactly `var(--name)`.
fn sole_var(value: &str) -> Option<String> {
    let value = value.trim();
    let inner = value
        .get(..4)
        .filter(|prefix| prefix.eq_ignore_ascii_case("var("))
        .and(value.strip_suffix(')'))
        .map(|rest| rest[4..].trim())?;
    (inner.starts_with("--") && !inner.contains([',', '(', ')', ' '])).then(|| inner.to_owned())
}

fn split_top_level(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

/// A single non-negative length in an absolute or font-relative unit.
fn is_fixed_length(value: &str) -> bool {
    ["px", "rem", "em", "ch"].iter().any(|unit| {
        value
            .strip_suffix(unit)
            .and_then(|number| number.parse::<f64>().ok())
            .is_some_and(|number| number.is_finite() && number >= 0.0)
    })
}

/// A consumer stylesheet may assign public semantic roles, but must not
/// override reference or other internal values or invent `--ds-*` names.
pub fn validate_consumer(file: &str, source: &str, rows: &[Row]) -> Result<usize, String> {
    let public: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.class == PUBLIC_PREVIEW)
        .map(|row| row.name.as_str())
        .collect();
    let mut mapped = 0;
    for declaration in declarations(file, source)? {
        let name = declaration.name.as_str();
        if !name.starts_with(TOKEN_PREFIX) {
            continue;
        }
        if !public.contains(name) {
            return Err(format!(
                "consumer stylesheet {file} declares {name}; consumers may assign only public-preview semantic roles, never reference or internal values"
            ));
        }
        mapped += 1;
    }
    Ok(mapped)
}

/// The public token document must name every public role and must not present
/// any internal property as something to override.
pub fn validate_document(document: &str, rows: &[Row]) -> Result<(), String> {
    for row in rows {
        let mentioned = document.match_indices(&row.name).any(|(index, _)| {
            document[index + row.name.len()..]
                .chars()
                .next()
                .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '-'))
        });
        if row.class == PUBLIC_PREVIEW && !mentioned {
            return Err(format!(
                "the token document does not document public role {}",
                row.name
            ));
        }
        if row.class == INTERNAL && mentioned {
            return Err(format!(
                "the token document names internal property {}; document only public roles as override seams",
                row.name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVENTORY: &str = "\
internal\t--ds-ref-gray-100\tcolor
internal\t--ds-ref-gray-12\tcolor
internal\t--ds-ref-font-sans\tfont-family
internal\t--ds-ref-space-16-32\tlength
internal\t--ds-ref-text-16-22\tlength
public-preview\t--ds-color-canvas\tcolor
public-preview\t--ds-color-focus\tcolor
public-preview\t--ds-font-body\tfont-family
public-preview\t--ds-space-flow\tlength
public-preview\t--ds-text-body\tlength
public-preview\t--ds-border-width\tlength
public-preview\t--ds-leading-body\tnumber
";

    const REFERENCE: &str = ":root {
  --ds-ref-gray-100: oklch(100% 0 0);
  --ds-ref-gray-12: oklch(12% 0 0);
  --ds-ref-font-sans: ui-sans-serif, \"Segoe UI Emoji\", sans-serif;
  --ds-ref-space-16-32: clamp(1rem, 0.7692rem + 0.7692vw, 2rem);
  --ds-ref-text-16-22: clamp(1rem, 0.9135rem + 0.2885vw, 1.375rem);
}";

    const SEMANTIC: &str = ":root {
  --ds-color-canvas: light-dark(var(--ds-ref-gray-100), var(--ds-ref-gray-12));
  --ds-color-focus: var(--ds-color-canvas);
  --ds-font-body: var(--ds-ref-font-sans);
  --ds-space-flow: var(--ds-ref-space-16-32);
  --ds-text-body: var(--ds-ref-text-16-22);
  --ds-border-width: 1px;
  --ds-leading-body: 1.5;
}";

    fn rows() -> Vec<Row> {
        parse_inventory(INVENTORY).expect("inventory")
    }

    fn check(reference: &str, semantic: &str) -> Result<Summary, String> {
        check_with(reference, semantic, &rows())
    }

    fn check_with(reference: &str, semantic: &str, rows: &[Row]) -> Result<Summary, String> {
        validate_authority(
            &[
                (REFERENCE_FILE.to_owned(), reference.to_owned()),
                (SEMANTIC_FILE.to_owned(), semantic.to_owned()),
            ],
            rows,
        )
    }

    fn rejected(reference: &str, semantic: &str, expected: &str) {
        let error = check(reference, semantic).expect_err("must be rejected");
        assert!(error.contains(expected), "{error}");
    }

    #[test]
    fn accepted_authority_passes() {
        let summary = check(REFERENCE, SEMANTIC).expect("accepted authority");
        assert_eq!(
            summary,
            Summary {
                reference: 5,
                semantic: 7,
                fluid: 2,
                scheme_pairs: 1
            }
        );
    }

    #[test]
    fn inventory_rejects_wrong_class_duplicates_and_types() {
        let public_reference = parse_inventory("public-preview\t--ds-ref-gray-12\tcolor\n")
            .expect_err("public reference");
        assert!(public_reference.contains("internal"), "{public_reference}");
        let internal_role =
            parse_inventory("internal\t--ds-color-canvas\tcolor\n").expect_err("internal role");
        assert!(internal_role.contains("public-preview"), "{internal_role}");
        let stable = parse_inventory("public-stable\t--ds-color-canvas\tcolor\n")
            .expect_err("public-stable");
        assert!(stable.contains("public-stable"), "{stable}");
        let duplicate = parse_inventory(
            "public-preview\t--ds-color-canvas\tcolor\npublic-preview\t--ds-color-canvas\tcolor\n",
        )
        .expect_err("duplicate");
        assert!(duplicate.contains("more than once"), "{duplicate}");
        let value_type =
            parse_inventory("public-preview\t--ds-color-canvas\tpaint\n").expect_err("value type");
        assert!(value_type.contains("value type"), "{value_type}");
        let name = parse_inventory("public-preview\t--brand-accent\tcolor\n").expect_err("name");
        assert!(name.contains("--ds-"), "{name}");
    }

    #[test]
    fn unclassified_and_undeclared_tokens_are_rejected() {
        rejected(
            REFERENCE,
            &SEMANTIC.replace("}", "  --ds-color-extra: var(--ds-color-canvas);\n}"),
            "not classified in tokens.tsv",
        );
        let mut extra = rows();
        extra.push(Row {
            class: PUBLIC_PREVIEW.to_owned(),
            name: "--ds-color-missing".to_owned(),
            value_type: "color".to_owned(),
        });
        let error = check_with(REFERENCE, SEMANTIC, &extra).expect_err("missing declaration");
        assert!(
            error.contains("no Design System stylesheet declares it"),
            "{error}"
        );
    }

    #[test]
    fn tiers_stay_in_their_own_stylesheets() {
        rejected(
            &REFERENCE.replace("}", "  --ds-leading-body: 1.5;\n}"),
            &SEMANTIC.replace("  --ds-leading-body: 1.5;\n", ""),
            "belong only in",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "--ds-border-width: 1px;",
                "--ds-border-width: 1px; --ds-border-width: 2px;",
            ),
            "declared more than once",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(":root", "[data-theme=\"dark\"]"),
            "declared on :root only",
        );
    }

    #[test]
    fn undeclared_and_provider_variables_are_rejected() {
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "--ds-leading-body: 1.5;",
                "--ds-leading-body: var(--leading-normal);",
            ),
            "var(--leading-normal)",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace("var(--ds-ref-font-sans)", "VAR(--font-sans)"),
            "var(--font-sans)",
        );
    }

    #[test]
    fn scheme_pairs_live_only_in_the_semantic_tier() {
        rejected(
            &REFERENCE.replace(
                "oklch(100% 0 0)",
                "light-dark(oklch(100% 0 0), oklch(0% 0 0))",
            ),
            SEMANTIC,
            "scheme-neutral",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "light-dark(var(--ds-ref-gray-100), var(--ds-ref-gray-12))",
                "light-dark(oklch(100% 0 0), oklch(12% 0 0))",
            ),
            "must pair two reference colors",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "light-dark(var(--ds-ref-gray-100), var(--ds-ref-gray-12))",
                "light-dark(var(--ds-ref-gray-100), var(--ds-color-focus))",
            ),
            "must pair two reference colors",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "light-dark(var(--ds-ref-gray-100), var(--ds-ref-gray-12))",
                "oklch(100% 0 0)",
            ),
            "literal values belong in the reference tier",
        );
    }

    #[test]
    fn reference_colors_are_plain_oklch() {
        rejected(
            &REFERENCE.replace("oklch(12% 0 0)", "oklch(from white calc(l - 0.1) c h)"),
            SEMANTIC,
            "one plain oklch()",
        );
        rejected(
            &REFERENCE.replace("oklch(12% 0 0)", "#1f1f1f"),
            SEMANTIC,
            "one plain oklch()",
        );
        rejected(
            &REFERENCE.replace("oklch(12% 0 0)", "var(--ds-ref-gray-100)"),
            SEMANTIC,
            "raw literals",
        );
    }

    #[test]
    fn fluid_references_are_bounded_and_named_for_their_bounds() {
        let space = "clamp(1rem, 0.7692rem + 0.7692vw, 2rem)";
        for (replacement, expected) in [
            ("clamp(2rem, 0.7692rem + 0.7692vw, 1rem)", "larger maximum"),
            (
                "clamp(1rem, 0.5rem + 0.7692vw, 2rem)",
                "does not interpolate",
            ),
            ("clamp(1rem, 0.7692rem + 0.7692vw)", "must be `clamp("),
            ("clamp(16px, 0.7692rem + 0.7692vw, 2rem)", "must be `clamp("),
            (
                "clamp(1rem, 0.7692rem + 0.7692cqi, 2rem)",
                "must be `clamp(",
            ),
            ("calc(1rem + 1vw)", "uses a function"),
            (
                "clamp(1rem, 0.6923rem + 1.0256vw, 2.3333rem)",
                "is named for 16px-32px",
            ),
        ] {
            rejected(&REFERENCE.replace(space, replacement), SEMANTIC, expected);
        }
    }

    #[test]
    fn fluid_text_is_resize_bounded() {
        let rows = parse_inventory(&INVENTORY.replace("--ds-ref-text-16-22", "--ds-ref-text-8-24"))
            .expect("inventory");
        let error = check_with(
            &REFERENCE.replace(
                "--ds-ref-text-16-22: clamp(1rem, 0.9135rem + 0.2885vw, 1.375rem)",
                "--ds-ref-text-8-24: clamp(0.5rem, 0.2692rem + 0.7692vw, 1.5rem)",
            ),
            &SEMANTIC.replace("--ds-ref-text-16-22", "--ds-ref-text-8-24"),
            &rows,
        )
        .expect_err("unbounded text growth");
        assert!(error.contains("200%"), "{error}");
    }

    #[test]
    fn semantic_values_are_aliases_pairs_or_fixed_literals() {
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "var(--ds-ref-space-16-32)",
                "clamp(1rem, 0.7692rem + 0.7692vw, 2rem)",
            ),
            "fluid formulas belong in the reference tier",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "--ds-font-body: var(--ds-ref-font-sans);",
                "--ds-font-body: serif;",
            ),
            "literal values belong in the reference tier",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace("var(--ds-ref-space-16-32)", "var(--ds-ref-gray-12)"),
            "keeps its value type",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace("--ds-leading-body: 1.5;", "--ds-leading-body: 150%;"),
            "fixed number literal",
        );
    }

    #[test]
    fn hidden_declarations_are_rejected() {
        rejected(
            REFERENCE,
            &SEMANTIC.replace(
                "--ds-border-width: 1px;",
                "--ds-border-width: 1px; & p { --x: 1; }",
            ),
            "nests a rule",
        );
        rejected(
            REFERENCE,
            &SEMANTIC.replace("--ds-leading-body: 1.5;", "--ds-leading-body: v\\61r(--x);"),
            "escaped code point",
        );
    }

    #[test]
    fn consumers_map_only_public_roles() {
        let rows = rows();
        let mapping = "@layer app {\n  :root {\n    --brand-blue: oklch(50% 0.2 300);\n    --ds-color-focus: var(--brand-blue);\n  }\n}";
        assert_eq!(validate_consumer("consumer.css", mapping, &rows), Ok(1));
        for (declaration, expected) in [
            ("--ds-ref-gray-12: black;", "--ds-ref-gray-12"),
            ("--ds-ref-new: black;", "--ds-ref-new"),
            ("--ds-brand-accent: black;", "--ds-brand-accent"),
        ] {
            let source = format!("@layer app {{ :root {{ {declaration} }} }}");
            let error = validate_consumer("consumer.css", &source, &rows).expect_err("override");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn document_names_public_roles_and_no_internal_value() {
        let rows = rows();
        let public: Vec<String> = rows
            .iter()
            .filter(|row| row.class == PUBLIC_PREVIEW)
            .map(|row| format!("`{}`", row.name))
            .collect();
        let document = format!(
            "Roles: {}. Reference values use `--ds-ref-*`.",
            public.join(", ")
        );
        assert_eq!(validate_document(&document, &rows), Ok(()));
        let missing = document.replace("`--ds-space-flow`", "");
        assert!(validate_document(&missing, &rows).is_err());
        let internal = format!("{document} Override `--ds-ref-gray-12`.");
        let error = validate_document(&internal, &rows).expect_err("internal named");
        assert!(error.contains("--ds-ref-gray-12"), "{error}");
    }
}
