//! Deterministic, dependency-free measurements for authored CSS.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
struct Metrics {
    files: usize,
    bytes: usize,
    lines: usize,
    imports: usize,
    at_rules: usize,
    custom_properties: usize,
}

pub fn run(root: &Path, styles_root: &Path) -> Result<String, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("resolve repository root: {error}"))?;
    let styles_root = root.join(styles_root);
    let mut files = Vec::new();
    collect_css(&styles_root, &mut files)?;
    files.sort();

    let mut metrics = Metrics::default();
    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("read {}: {error}", file.display()))?;
        metrics.files += 1;
        metrics.bytes += source.len();
        metrics.lines += source.lines().count();
        metrics.imports += source
            .lines()
            .filter(|line| line.trim_start().starts_with("@import"))
            .count();
        metrics.at_rules += source
            .lines()
            .filter(|line| line.trim_start().starts_with('@'))
            .count();
        metrics.custom_properties += source
            .lines()
            .filter(|line| line.contains("--ds-") && line.contains(':'))
            .count();
    }

    Ok(format!(
        "audited {} CSS files under {}: {} bytes, {} lines, {} imports, {} at-rules, {} custom-property declarations",
        metrics.files,
        styles_root
            .strip_prefix(&root)
            .unwrap_or(&styles_root)
            .display(),
        metrics.bytes,
        metrics.lines,
        metrics.imports,
        metrics.at_rules,
        metrics.custom_properties
    ))
}

fn collect_css(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("inspect {}: {error}", path.display()))?;
    if metadata.is_file() {
        if path.extension().is_some_and(|extension| extension == "css") {
            files.push(path.to_owned());
        }
        return Ok(());
    }

    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    entries.sort();
    for entry in entries {
        collect_css(&entry, files)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::collect_css;
    use std::fs;

    #[test]
    fn collects_only_css_in_sorted_tree() {
        let root = std::env::temp_dir().join(format!("ds-check-audit-{}", std::process::id()));
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("nested directory");
        fs::write(root.join("z.css"), "a { --ds-x: 1; }\n").expect("css");
        fs::write(nested.join("a.css"), "@import \"x.css\";\n").expect("nested css");
        fs::write(root.join("ignored.txt"), "not css").expect("text");

        let mut files = Vec::new();
        collect_css(&root, &mut files).expect("collect css");
        files.sort();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.css"));
        assert!(files[1].ends_with("z.css"));

        let _ = fs::remove_dir_all(root);
    }
}
