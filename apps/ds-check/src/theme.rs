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
/// Selector fragments of theme hooks that are not this contract's. Luna's
/// `.dark` class and `data-theme` attribute are evidence, not portable API.
const ALIAS_HOOKS: [&str; 3] = [".dark", "[data-theme", "[data-color-scheme"];

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
    let nodes = css::parse(source).map_err(|error| format!("stylesheet {file}: {error}"))?;
    let mut found = Vec::new();
    collect(file, &nodes, false, &mut found)?;
    Ok(found)
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

fn alias_hook(selector: &str, manifest: &Manifest) -> Option<String> {
    let lower = selector.to_ascii_lowercase();
    if let Some(alias) = ALIAS_HOOKS.iter().find(|alias| {
        lower.match_indices(*alias).any(|(index, _)| {
            lower[index + alias.len()..]
                .chars()
                .next()
                .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '-' || next == '_'))
        })
    }) {
        return Some((*alias).to_owned());
    }
    let own = format!("[{}", manifest.attribute);
    lower.match_indices("[data-ds-").find_map(|(index, _)| {
        let rest = &lower[index..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '[' || c == '_'))
            .unwrap_or(rest.len());
        (rest[..end] != own).then(|| rest[..end].to_owned())
    })
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
            if let Some(alias) = alias_hook(&rule.selector, manifest) {
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
        if let Some(alias) = alias_hook(&rule.selector, manifest) {
            return Err(format!(
                "consumer stylesheet {file} selects `{}` with theme hook `{alias}`; use [{}] on :root",
                rule.selector, manifest.attribute
            ));
        }
        let root = rule
            .selector
            .split(',')
            .map(str::trim)
            .any(|selector| selector == ":root" || selector.eq_ignore_ascii_case("html"));
        if root
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

/// The theme document names the default, the hook, and every hook value.
pub fn validate_document(document: &str, manifest: &Manifest) -> Result<(), String> {
    let mut required = vec![format!("{COLOR_SCHEME}: {}", manifest.default)];
    for value in &manifest.values {
        required.push(format!("{}=\"{value}\"", manifest.attribute));
    }
    for phrase in required {
        if !document.contains(&phrase) {
            return Err(format!("the theme document does not document `{phrase}`"));
        }
    }
    let lower = document.to_ascii_lowercase();
    for alias in ["data-theme", "data-color-scheme"] {
        if lower.contains(alias) {
            return Err(format!(
                "the theme document names `{alias}`; document only the public hook {}",
                manifest.attribute
            ));
        }
    }
    Ok(())
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

    #[test]
    fn document_names_default_hook_and_values_only() {
        let manifest = manifest();
        let document =
            "color-scheme: light dark\ndata-ds-scheme=\"light\"\ndata-ds-scheme=\"dark\"\n";
        validate_document(document, &manifest).expect("document");
        let missing = document.replace("data-ds-scheme=\"dark\"", "");
        assert!(validate_document(&missing, &manifest).is_err());
        let alias = format!("{document}data-theme\n");
        assert!(
            validate_document(&alias, &manifest)
                .unwrap_err()
                .contains("data-theme")
        );
    }
}
