//! Bounded duplicate-literal checks for TypeScript union type aliases.
//!
//! Only string/number/boolean/null/undefined literals are compared. Removing a
//! duplicated union literal is always safe, so the quick fix edits inline unions
//! or whole literal lines in multiline unions.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateUnion {
    /// Original literal text (quotes included).
    pub label: String,
    /// Byte offset of the later literal.
    pub offset: usize,
    pub first_line: usize,
    pub removal: Option<Range<usize>>,
}

#[derive(Clone)]
struct Part {
    start: usize,
    end: usize,
}

fn literal_key(raw: &str) -> Option<String> {
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
        return (!inner.is_empty()).then(|| format!("str:{inner}"));
    }
    if matches!(raw, "true" | "false" | "null" | "undefined") {
        return Some(format!("id:{raw}"));
    }
    let number = raw.replace('_', "");
    if number.parse::<f64>().is_ok_and(|value| value.is_finite()) {
        return Some(format!("num:{number}"));
    }
    None
}

fn alias_end(structural: &str, start: usize) -> usize {
    let limit = structural.len().min(start + 16 * 1024);
    let mut depth = 0i32;
    for (index, ch) in structural[start..limit].char_indices() {
        let at = start + index;
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if depth > 0 => depth -= 1,
            ';' if depth == 0 => return at,
            _ => {}
        }
    }
    // No terminating semicolon: continue through leading-pipe union lines.
    let mut end = structural[start..]
        .find('\n')
        .map(|index| start + index)
        .unwrap_or(structural.len());
    let mut line_start = end + 1;
    while line_start < structural.len() {
        let line_end = structural[line_start..]
            .find('\n')
            .map(|index| line_start + index)
            .unwrap_or(structural.len());
        if structural[line_start..line_end]
            .trim_start()
            .starts_with('|')
        {
            end = line_end;
            line_start = line_end + 1;
        } else {
            break;
        }
    }
    end
}

fn split_union(structural: &str, start: usize, end: usize) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut segment = start;
    for (index, ch) in structural[start..end].char_indices() {
        let at = start + index;
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if depth > 0 => depth -= 1,
            '|' if depth == 0 => {
                parts.push(Part {
                    start: segment,
                    end: at,
                });
                segment = at + 1;
            }
            _ => {}
        }
    }
    parts.push(Part {
        start: segment,
        end,
    });
    parts
}

fn inline_removal(text: &str, parts: &[Part], index: usize) -> Option<Range<usize>> {
    let part = &parts[index];
    let trimmed_start = part.start
        + (text[part.start..part.end].len() - text[part.start..part.end].trim_start().len());
    let trimmed = text[part.start..part.end].trim();
    let literal_start = trimmed_start;
    let literal_end = literal_start + trimmed.len();
    let line_start = text[..literal_start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[literal_end..]
        .find('\n')
        .map(|index| literal_end + index)
        .unwrap_or(text.len());
    let prefix = text[line_start..literal_start].trim();
    let suffix = text[literal_end..line_end].trim();
    let own_line = (prefix.is_empty() || prefix == "|") && (suffix.is_empty() || suffix == ";");
    if own_line {
        if suffix == ";" {
            return Some(line_start..literal_end);
        }
        let end = if text[line_end..].starts_with('\n') {
            line_end + 1
        } else {
            line_end
        };
        return Some(line_start..end);
    }
    if index + 1 < parts.len() {
        let pipe = text[literal_end..parts[index + 1].start].find('|')? + literal_end;
        let mut end = pipe + 1;
        while text[end..].starts_with(' ') {
            end += 1;
        }
        return Some(literal_start..end);
    }
    let previous = &parts[index - 1];
    let previous_end = previous.start + text[previous.start..previous.end].trim_end().len();
    Some(previous_end..literal_end)
}

/// Duplicate literal members in `type Name = A | B | ...` aliases.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateUnion> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("type") {
        let at = search + relative;
        let before = structural[..at].trim_end();
        let declaration = before.is_empty()
            || before.ends_with("export")
            || before.ends_with("declare")
            || before.ends_with("default");
        let boundary = structural[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
        let after = &structural[at + "type".len()..];
        if !declaration || !boundary || !after.chars().next().is_some_and(char::is_whitespace) {
            search = at + "type".len();
            continue;
        }
        let rest = after.trim_start();
        let name_len = rest
            .char_indices()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);
        let after_name = rest[name_len..].trim_start();
        if !crate::rename::valid_identifier(&rest[..name_len]) || !after_name.starts_with('=') {
            search = at + "type".len();
            continue;
        }
        let Some(equals) = structural[at..].find('=').map(|index| at + index) else {
            search = at + "type".len();
            continue;
        };
        let end = alias_end(&structural, equals + 1);
        if end - equals > 16 * 1024 || structural[equals + 1..end].matches('\n').count() > 50 {
            search = at + "type".len();
            continue;
        }
        let parts = split_union(&structural, equals + 1, end);
        let mut seen = HashMap::<String, (usize, usize)>::new();
        for (index, part) in parts.iter().enumerate() {
            let raw = masked[part.start..part.end].trim();
            let Some(key) = literal_key(raw) else {
                continue;
            };
            let offset = part.start
                + (masked[part.start..part.end].len()
                    - masked[part.start..part.end].trim_start().len());
            let line = masked[..offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            if let Some((first_line, _)) = seen.get(&key) {
                found.push(DuplicateUnion {
                    label: raw.to_owned(),
                    offset,
                    first_line: *first_line,
                    removal: inline_removal(&masked, &parts, index),
                });
            } else {
                seen.insert(key, (line, offset));
            }
        }
        search = at + "type".len();
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remove(text: &str) -> String {
        let removal = duplicates(text)[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        edited
    }

    #[test]
    fn inline_literal_duplicates_remove_the_later_member() {
        let text = "type Mode = \"a\" | \"a\";\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "\"a\"");
        assert_eq!(remove(text), "type Mode = \"a\";\n");

        let middle = "type M = \"a\" | \"a\" | \"b\";\n";
        assert_eq!(remove(middle), "type M = \"a\" | \"b\";\n");
        let last = "type M = \"a\" | \"b\" | \"a\";\n";
        assert_eq!(remove(last), "type M = \"a\" | \"b\";\n");
        let mixed_quotes = "type M = \"a\" | 'a';\n";
        assert_eq!(remove(mixed_quotes), "type M = \"a\";\n");
    }

    #[test]
    fn numbers_booleans_and_null_literals_compare_by_value() {
        assert_eq!(remove("type N = 1 | 1;\n"), "type N = 1;\n");
        assert_eq!(remove("type B = true | true;\n"), "type B = true;\n");
        assert_eq!(remove("type Z = null | null;\n"), "type Z = null;\n");
        assert_eq!(
            remove("type U = undefined | undefined;\n"),
            "type U = undefined;\n"
        );
        assert!(duplicates("type N = 1 | 2;\n").is_empty());
        assert!(duplicates("type T = true | false;\n").is_empty());
    }

    #[test]
    fn multiline_unions_remove_whole_literal_lines() {
        let text = "type Mode =\n  | \"read\"\n  | \"read\";\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].first_line, 2);
        let edited = remove(text);
        assert_eq!(edited, "type Mode =\n  | \"read\"\n;\n");

        let without_semicolon = "type Mode =\n  | \"read\"\n  | \"read\"\n";
        assert_eq!(remove(without_semicolon), "type Mode =\n  | \"read\"\n");
        assert!(duplicates("type Mode =\n  | \"read\"\n  | \"write\"\n").is_empty());
    }

    #[test]
    fn non_literal_types_prefixes_and_comments_are_bounded() {
        assert!(duplicates("type Ref = Foo | Foo;\n").is_empty());
        assert!(duplicates("type Obj = { a: 1 } | { a: 1 };\n").is_empty());
        assert!(duplicates("type Arr = string[] | string[];\n").is_empty());
        assert_eq!(duplicates("export type M = \"a\" | \"a\";\n").len(), 1);
        assert_eq!(duplicates("declare type M = \"a\" | \"a\";\n").len(), 1);
        assert!(duplicates("const type = \"a\" | \"a\";\n").is_empty());
        assert!(duplicates("// type M = \"a\" | \"a\";\n").is_empty());
        assert!(duplicates("const text = \"type M = \\\"a\\\" | \\\"a\\\"\";\n").is_empty());
        assert!(duplicates("type M = \"a\" | \"a\" | Foo;\n").len() == 1);
        assert!(duplicates("type Fn = (a: string) => void | (a: string) => void;\n").is_empty());
    }
}
