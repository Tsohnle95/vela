//! Bounded duplicate switch-label checks for literal case keys.
//!
//! Only simple literal labels (numbers, quoted strings, booleans, null,
//! undefined and identifiers) are compared. A safe quick fix removes a later
//! duplicate label only when its clause is empty, so fallthrough and statement
//! execution are unchanged.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateCase {
    pub label: String,
    /// Byte offset of the later case expression, or the `default` keyword.
    pub offset: usize,
    /// Document range removing the empty later label, when safe.
    pub removal: Option<Range<usize>>,
}

fn word_at(text: &str, at: usize, word: &str) -> bool {
    if !text[at..].starts_with(word) {
        return false;
    }
    let before_ok = text[..at]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
    let after_ok = text[at + word.len()..]
        .chars()
        .next()
        .is_none_or(|ch| ch.is_whitespace() || matches!(ch, ':' | '('));
    before_ok && after_ok
}

fn matching(text: &str, open: usize, opening: char, closing: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text[open..].char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            _ if ch == opening => depth += 1,
            _ if ch == closing => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

fn label_key(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let first = raw.chars().next()?;
    if matches!(first, '"' | '\'') {
        let close = raw[1..].find(first)? + 1;
        if close + 1 != raw.len() {
            return None;
        }
        let inner = &raw[1..close];
        if inner.is_empty() {
            return None;
        }
        return Some(format!("str:{inner}"));
    }
    if matches!(raw, "true" | "false" | "null" | "undefined") {
        return Some(format!("id:{raw}"));
    }
    let number = raw.replace('_', "");
    if number.parse::<f64>().is_ok_and(|value| value.is_finite()) {
        return Some(format!("num:{number}"));
    }
    crate::rename::valid_identifier(raw).then(|| format!("id:{raw}"))
}

/// True when only whitespace separates `colon` from the next label or the end
/// of the switch body.
fn empty_clause(structural: &str, colon: usize, body_end: usize) -> bool {
    let mut next = colon + 1;
    while next < body_end
        && structural[next..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        next += structural[next..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    next >= body_end || word_at(structural, next, "case") || word_at(structural, next, "default")
}

fn label_removal(text: &str, start: usize, colon: usize) -> Option<Range<usize>> {
    let line_start = text[..start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[colon..]
        .find('\n')
        .map(|index| colon + index)
        .unwrap_or(text.len());
    if text[line_start..start].trim().is_empty() && text[colon + 1..line_end].trim().is_empty() {
        let end = if text[line_end..].starts_with('\n') {
            line_end + 1
        } else {
            line_end
        };
        return Some(line_start..end);
    }
    let mut end = colon + 1;
    while text[end..].starts_with(' ') {
        end += 1;
    }
    Some(start..end)
}

/// Duplicate literal labels within each switch body.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateCase> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("switch") {
        let at = search + relative;
        if !word_at(&structural, at, "switch") {
            search = at + "switch".len();
            continue;
        }
        let Some(open_paren) = structural[at + "switch".len()..]
            .find('(')
            .map(|i| at + "switch".len() + i)
        else {
            search = at + "switch".len();
            continue;
        };
        let Some(close_paren) = matching(&structural, open_paren, '(', ')') else {
            break;
        };
        let Some(open) = structural[close_paren + 1..]
            .find('{')
            .map(|i| close_paren + 1 + i)
        else {
            search = close_paren + 1;
            continue;
        };
        let Some(close) = matching(&structural, open, '{', '}') else {
            break;
        };
        let body_end = close;
        if body_end - open > 16 * 1024 {
            search = close + 1;
            continue;
        }
        let mut seen = HashMap::<String, ()>::new();
        let mut pos = open + 1;
        let mut depth = 0i32;
        while pos < body_end {
            if depth == 0 && word_at(&structural, pos, "case") {
                let mut expr_start = pos + "case".len();
                while expr_start < body_end
                    && masked[expr_start..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    expr_start += masked[expr_start..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
                }
                let Some(colon) = structural[expr_start..body_end]
                    .find(':')
                    .map(|index| expr_start + index)
                else {
                    break;
                };
                let raw = masked[expr_start..colon].trim();
                if let Some(key) = label_key(raw)
                    && seen.insert(key, ()).is_some()
                {
                    let empty = empty_clause(&structural, colon, body_end);
                    found.push(DuplicateCase {
                        label: raw.to_owned(),
                        offset: expr_start,
                        removal: empty.then(|| label_removal(&masked, pos, colon)).flatten(),
                    });
                }
                pos = colon + 1;
                continue;
            }
            if depth == 0 && word_at(&structural, pos, "default") {
                let mut colon = pos + "default".len();
                while colon < body_end
                    && structural[colon..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    colon += structural[colon..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
                }
                if structural[colon..].starts_with(':') {
                    let key = "default".to_owned();
                    if seen.insert(key, ()).is_some() {
                        let empty = empty_clause(&structural, colon, body_end);
                        found.push(DuplicateCase {
                            label: "default".into(),
                            offset: pos,
                            removal: empty.then(|| label_removal(&masked, pos, colon)).flatten(),
                        });
                    }
                    pos = colon + 1;
                    continue;
                }
            }
            let ch = structural[pos..].chars().next().unwrap_or(' ');
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            pos += ch.len_utf8();
        }
        search = at + "switch".len();
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_duplicate_cases_are_flagged_and_removed() {
        let text = "switch (x) {\n  case 1:\n  case 1:\n  case 2:\n    work();\n}\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "1");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "switch (x) {\n  case 1:\n  case 2:\n    work();\n}\n"
        );

        let inline = "switch (x) { case 1: case 1: case 2: work(); }\n";
        let removal = duplicates(inline)[0].removal.clone().unwrap();
        let mut edited = inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "switch (x) { case 1: case 2: work(); }\n");
    }

    #[test]
    fn non_empty_duplicate_cases_are_flagged_without_removal() {
        let text = "switch (x) {\n  case 1:\n    first();\n  case 1:\n    second();\n}\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert!(found[0].removal.is_none());

        let inline = "switch (x) { case 1: first(); break; case 1: second(); break; }\n";
        assert_eq!(duplicates(inline).len(), 1);
        assert!(duplicates(inline)[0].removal.is_none());

        let fallthrough = "switch (x) { case 1: first(); case 1: second(); }\n";
        assert_eq!(duplicates(fallthrough).len(), 1);
        assert!(duplicates(fallthrough)[0].removal.is_none());
    }

    #[test]
    fn literal_strings_numbers_and_distinct_labels_are_compared() {
        let strings = "switch (x) { case \"a\": break; case 'a': break; }\n";
        let found = duplicates(strings);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "'a'");

        assert!(duplicates("switch (x) { case 1: break; case 1.5: break; }\n").is_empty());
        assert!(duplicates("switch (x) { case 1: break; case 2: break; }\n").is_empty());
        assert!(duplicates("switch (x) { case true: break; case true: break; }\n").len() == 1);
        assert!(duplicates("switch (x) { case A: break; case A: break; }\n").len() == 1);
        assert!(duplicates("switch (x) { case a + b: break; case a + b: break; }\n").is_empty());
    }

    #[test]
    fn duplicate_defaults_and_nested_switches_are_bounded() {
        let inline = "switch (x) { default: break; default: case 2: work(); }\n";
        let found = duplicates(inline);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "default");
        assert!(found[0].removal.is_some());
        let removal = found[0].removal.clone().unwrap();
        let mut edited = inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "switch (x) { default: break; case 2: work(); }\n");

        let nonempty = "switch (x) { default: first(); default: second(); }\n";
        assert_eq!(duplicates(nonempty).len(), 1);
        assert!(duplicates(nonempty)[0].removal.is_none());

        let nested = "switch (x) { case 1: switch (y) { case 2: break; case 2: break; } break; }\n";
        let found = duplicates(nested);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "2");
        assert!(found[0].offset > nested.find("switch (y)").unwrap());
    }

    #[test]
    fn strings_comments_and_other_keywords_are_bounded() {
        assert!(duplicates("const text = \"switch (x) { case 1: case 1: }\";\n").is_empty());
        assert!(duplicates("// switch (x) { case 1: case 1: }\n").is_empty());
        assert!(duplicates("switch (x) { case 1: break; }\n").is_empty());
        assert!(duplicates("if (x) { case 1: case 1: }\n").is_empty());
        assert!(duplicates("switch (x) { case 1: { case 1: break; } break; }\n").is_empty());
    }
}
