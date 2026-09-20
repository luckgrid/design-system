//! Optional Tailwind adapter checks. The adapter is intentionally outside the
//! portable stylesheet graph, so this validates its own public projection and
//! generated provider output without teaching the core checker Tailwind syntax.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::tokens;

const ADAPTER_ENTRY: &str = "index.css";

pub fn run(
    root: &Path,
    projection: &Path,
    inventory: &Path,
    adapter_exports: &Path,
    manifest: &Path,
    adapter: &Path,
    generated: &Path,
) -> Result<String, String> {
    let token_rows = tokens::parse_inventory(&read(root, inventory)?)?;
    let semantic: BTreeSet<String> = token_rows
        .iter()
        .filter(|row| row.class == "public-preview")
        .map(|row| row.name.clone())
        .collect();
    let ledger = parse_ledger(&read(root, projection)?, &semantic)?;

    let exports = read(root, adapter_exports)?;
    let declared_exports: Vec<&str> = exports
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if declared_exports.as_slice() != ["public-preview\ttailwind.css\tadapters/tailwind/index.css"]
    {
        return Err("adapter exports must declare exactly public-preview tailwind.css at adapters/tailwind/index.css".to_owned());
    }
    let surfaces = read(root, manifest)?;
    if !surfaces.lines().any(|line| {
        line.starts_with("public-preview\tadapters/tailwind/index.css\t")
            && line.ends_with("scan-exempt:adapter-entrypoint-imports-declared-core-export")
    }) {
        return Err(
            "adapter entrypoint is not public-preview in bootstrap-surfaces.tsv".to_owned(),
        );
    }

    let entry = read(root, &adapter.join(ADAPTER_ENTRY))?;
    if entry.contains("--ds-ref-") {
        return Err("Tailwind adapter projects an internal --ds-ref-* value".to_owned());
    }
    if entry
        .lines()
        .any(|line| line.trim_start().starts_with("@source"))
    {
        return Err("reusable Tailwind adapter must not embed consumer source scanning".to_owned());
    }
    ensure_no_shared_extensions(&entry)?;
    if entry.contains("@import \"tailwindcss\";") || entry.contains("preflight") {
        return Err(
            "supported Tailwind adapter must omit aggregate Tailwind import and Preflight"
                .to_owned(),
        );
    }
    for required in [
        "@import \"../../packages/styles/index.css\";",
        "@import \"tailwindcss/theme\" layer(tailwind.theme);",
        "@import \"tailwindcss/utilities\" layer(tailwind.utilities);",
        "@theme inline",
    ] {
        if !entry.contains(required) {
            return Err(format!(
                "Tailwind adapter is missing required composition `{required}`"
            ));
        }
    }
    for (semantic, tailwind) in &ledger {
        let expected = format!("{tailwind}: var({semantic});");
        if !entry.contains(&expected) {
            return Err(format!(
                "Tailwind projection `{tailwind}` must be the direct semantic alias `{expected}`"
            ));
        }
    }

    let output = read(root, generated)?;
    if output.contains("preflight") || output.contains("box-sizing:border-box") {
        return Err("generated Tailwind output includes Preflight-like base output".to_owned());
    }
    for (semantic, tailwind) in &ledger {
        if output.contains(tailwind) && !output.contains(semantic) {
            return Err(format!(
                "generated Tailwind output mentions `{tailwind}` without its semantic source `{semantic}`"
            ));
        }
    }
    if !output.contains("--ds-color-canvas") || !output.contains(".bg-ds-canvas") {
        return Err(
            "generated fixture output does not prove a projected Tailwind utility".to_owned(),
        );
    }
    Ok(format!(
        "validated Tailwind adapter: {} public semantic roles project through @theme inline, internal references excluded, no Preflight, and generated output {} resolves semantic variables",
        ledger.len(),
        generated.display()
    ))
}

fn parse_ledger(text: &str, semantic: &BTreeSet<String>) -> Result<Vec<(String, String)>, String> {
    let mut mapped = Vec::new();
    let mut names = BTreeSet::new();
    let mut adapter_names = BTreeSet::new();
    let mut excluded = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(format!(
                "projection line {} must have five tab-separated fields",
                index + 1
            ));
        }
        if fields[0] == "internal"
            && fields[1] == "--ds-ref-*"
            && fields[4] == "excluded-internal-reference-tier"
        {
            excluded = true;
            continue;
        }
        if fields[0] != "public-preview"
            || fields[4] != "projected"
            || !semantic.contains(fields[1])
        {
            return Err(format!(
                "projection line {} is not a public semantic projection",
                index + 1
            ));
        }
        if !fields[2].starts_with("--")
            || !names.insert(fields[1].to_owned())
            || !adapter_names.insert(fields[2].to_owned())
        {
            return Err(format!(
                "projection line {} duplicates or invalidly names a projection",
                index + 1
            ));
        }
        mapped.push((fields[1].to_owned(), fields[2].to_owned()));
    }
    if !excluded {
        return Err("projection ledger does not explicitly exclude --ds-ref-*".to_owned());
    }
    if &names != semantic {
        return Err("projection ledger must map every and only public semantic token".to_owned());
    }
    Ok(mapped)
}

fn read(root: &Path, path: &Path) -> Result<String, String> {
    fs::read_to_string(root.join(path)).map_err(|error| format!("read {}: {error}", path.display()))
}

fn ensure_no_shared_extensions(entry: &str) -> Result<(), String> {
    for directive in ["@utility", "@custom-variant", "@variant", "@slot"] {
        if entry.contains(directive) {
            return Err(format!(
                "reusable Tailwind adapter must not publish shared extension directive `{directive}`"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ensure_no_shared_extensions, parse_ledger};
    use std::collections::BTreeSet;

    #[test]
    fn rejects_missing_or_internal_projection() {
        let semantic = BTreeSet::from(["--ds-color-canvas".to_owned()]);
        assert!(
            parse_ledger(
                "public-preview\t--ds-ref-x\t--color-x\tcolor\tprojected",
                &semantic
            )
            .is_err()
        );
        assert!(
            parse_ledger(
                "internal\t--ds-ref-*\t-\t-\texcluded-internal-reference-tier",
                &semantic
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_shared_extension_directives() {
        for directive in ["@utility", "@custom-variant", "@variant", "@slot"] {
            assert!(ensure_no_shared_extensions(directive).is_err());
        }
        assert!(ensure_no_shared_extensions("@theme inline { }").is_ok());
    }
}
