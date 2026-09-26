//! Lexical helpers shared by the checks that read stylesheets, fixture markup, and
//! archive content. They close the escape and inert-content forms that the
//! release verifier must not mis-accept, without becoming a general CSS or HTML
//! parser: each helper either normalizes to a form the existing substring checks
//! already understand or rejects the construct outright.

/// Lowercased CSS with comments replaced by one space and every escape decoded,
/// so `!\69mportant`, `@\69mport`, and `--ds-\72ef-` read as their plain spelling.
///
/// A comment becomes a space rather than nothing because CSS treats it as a token
/// separator: `!/**/important` is `!important`, while `imp/**/ortant` is not.
/// Quoted strings are copied without comment stripping, because `/*` inside a
/// string is text.
pub fn css_normalize(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i += 2;
            out.push(' ');
            continue;
        }
        if c == '\\' {
            i += 1;
            let Some(&next) = chars.get(i) else {
                break;
            };
            if next.is_ascii_hexdigit() {
                let mut digits = String::new();
                while digits.len() < 6 && chars.get(i).is_some_and(char::is_ascii_hexdigit) {
                    digits.push(chars[i]);
                    i += 1;
                }
                if chars.get(i).is_some_and(|w| w.is_ascii_whitespace()) {
                    i += 1;
                }
                let decoded = u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .filter(|d| *d != '\0')
                    .unwrap_or('\u{fffd}');
                out.push(decoded);
                continue;
            }
            if next == '\n' {
                i += 1;
                continue;
            }
            out.push(next);
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out.to_ascii_lowercase()
}

/// Whether normalized CSS holds an `!important` flag, however it is spaced.
pub fn has_important(normalized_css: &str) -> bool {
    normalized_css.match_indices('!').any(|(index, _)| {
        normalized_css[index + 1..]
            .trim_start()
            .starts_with("important")
    })
}

/// Whether `content` holds `marker` at a token boundary: not embedded in a longer
/// alphanumeric word. `../` additionally must not follow another `.`, so a run of
/// dots or an ellipsis before a slash is not a parent-directory path. A private
/// workspace path still matches, because a separator precedes the marker.
pub fn contains_marker(content: &str, marker: &str) -> bool {
    content.match_indices(marker).any(|(index, _)| {
        let before = content[..index].chars().next_back();
        let starts_word = marker.chars().next().is_some_and(char::is_alphanumeric);
        match before {
            None => true,
            Some(c) if starts_word => !c.is_alphanumeric(),
            Some(c) if marker.starts_with('.') => c != '.' && !c.is_alphanumeric(),
            Some(_) => true,
        }
    })
}

/// Reject fixture markup the shared start-tag reader cannot account for.
///
/// The contents of `template`, `script`, and `noscript` are not ordinary rendered
/// page content, yet a start-tag reader would count their tags as fixture
/// coverage. A character reference inside a `class` value would spell a hook the
/// reader never sees. Both are rejected instead of parsed. `html` is lowercase.
pub fn reject_inert_or_escaped_markup(html: &str, label: &str) -> Result<(), String> {
    let visible = strip_html_comments(html);
    for name in ["template", "script", "noscript"] {
        if crate::base::has_element(&visible, name) {
            return Err(format!(
                "the {label} contains <{name}>; its contents are not ordinary page content, so they cannot count as fixture coverage"
            ));
        }
    }
    let mut rest = visible.as_str();
    while let Some(index) = rest.find("class") {
        let before = rest[..index].chars().next_back();
        let after = rest[index + "class".len()..].trim_start();
        rest = &rest[index + "class".len()..];
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let Some(value) = after.strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start();
        let end = match value.chars().next() {
            Some(quote @ ('"' | '\'')) => value[1..].find(quote).map(|end| &value[1..1 + end]),
            _ => value.split(|c: char| c.is_whitespace() || c == '>').next(),
        };
        if end.is_some_and(|text| text.contains('&')) {
            return Err(format!(
                "the {label} has a character reference in a class value; spell each class directly"
            ));
        }
    }
    Ok(())
}

fn strip_html_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start + 4..]
            .split_once("-->")
            .map_or("", |(_, tail)| tail);
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_escapes_and_comments() {
        assert_eq!(css_normalize("@\\69mport"), "@import");
        assert_eq!(css_normalize("@\\000069 mport"), "@import");
        assert_eq!(css_normalize("--ds-\\72 ef-gray"), "--ds-ref-gray");
        assert_eq!(css_normalize("--ds-\\ref-gray"), "--ds-ref-gray");
        assert_eq!(css_normalize("a/* x */b"), "a b");
        assert_eq!(css_normalize("\"/* kept */\""), "\"/* kept */\"");
        assert_eq!(css_normalize("A\\\nB"), "ab");
    }

    #[test]
    fn important_is_found_however_it_is_written() {
        for css in [
            "a{color:red !important}",
            "a{color:red ! important}",
            "a{color:red!important}",
            "a{color:red !/**/important}",
            "a{color:red !\\69mportant}",
            "a{color:red !IMPORTANT}",
            "a{color:red !\n\timportant}",
        ] {
            assert!(has_important(&css_normalize(css)), "{css}");
        }
        for css in [
            "a{color:red}",
            "a{content:\"important\"}",
            "a{color:red !imp/**/ortant}",
        ] {
            assert!(!has_important(&css_normalize(css)), "{css}");
        }
    }

    #[test]
    fn markers_match_at_token_boundaries_only() {
        // Marker text is assembled so this file does not trip its own scan.
        let build = ["Build", "/bin/"].concat();
        let remote = ["git@github", ".com:luckgrid/"].concat();
        assert!(contains_marker(&format!("/home/x/{build}y"), &build));
        assert!(contains_marker(&format!("{build}y"), &build));
        assert!(!contains_marker(&format!("my{build}y"), &build));
        assert!(contains_marker("href=\"../x\"", "../"));
        assert!(contains_marker("src/../x", "../"));
        assert!(!contains_marker("wait.../now", "../"));
        assert!(!contains_marker("a...../b", "../"));
        assert!(contains_marker(&format!("{remote}x"), &remote));
    }

    #[test]
    fn inert_containers_and_class_references_are_rejected() {
        assert!(
            reject_inert_or_escaped_markup("<main><p class=\"a\"></p></main>", "fixture").is_ok()
        );
        assert!(
            reject_inert_or_escaped_markup("<!-- <template> --><main></main>", "fixture").is_ok()
        );
        for html in [
            "<template><p class=\"ds-stack\"></p></template>",
            "<noscript><p class=\"ds-stack\"></p></noscript>",
            "<script>x</script>",
            "<p class=\"ds-&#97;ction\"></p>",
            "<p class='ds-&amp;'></p>",
            "<p class=ds-&#97;ction></p>",
        ] {
            assert!(
                reject_inert_or_escaped_markup(html, "fixture").is_err(),
                "{html}"
            );
        }
    }
}
