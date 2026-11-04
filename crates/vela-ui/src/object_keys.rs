//! Bounded duplicate-key checks inside object literals.
//!
//! Only plain identifier/string keys with `:` values or shorthand keys are
//! compared. Computed keys, spreads, methods and accessors are skipped so the
//! check stays conservative. A quick fix exists only when removing the earlier
//! property cannot change behavior: its value must be a primitive literal.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateKey {
    pub key: String,
    /// Byte offset of the later duplicate key.
    pub offset: usize,
    /// One-based line of the first key.
    pub first_line: usize,
    /// Removal range for the earlier property when it is safe to drop.
    pub removal: Option<Range<usize>>,
}

pub(super) fn mask_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in text.chars() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            if ch == '\n' {
                out.push('\n');
            } else {
                out.extend(std::iter::repeat_n(' ', ch.len_utf8()));
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1
}

pub(super) fn literal_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return !inner.contains('"');
    }
    if let Some(inner) = value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        return !inner.contains('\'');
    }
    if matches!(value, "true" | "false" | "null" | "undefined") {
        return true;
    }
    value
        .replace('_', "")
        .parse::<f64>()
        .is_ok_and(|number| number.is_finite())
}

fn key_from(segment: &str) -> Option<(String, usize, bool)> {
    let trimmed = segment.trim_start();
    let leading = segment.len() - trimmed.len();
    let rest = trimmed;
    if rest.starts_with("...") || rest.starts_with('[') {
        return None;
    }
    let (key, after) = match rest.chars().next()? {
        quote @ ('"' | '\'') => {
            let close = rest[1..].find(quote)? + 1;
            (&rest[1..close], &rest[close + 1..])
        }
        ch if ch.is_alphabetic() || matches!(ch, '_' | '$') => {
            let len = rest
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            let key = &rest[..len];
            let after = &rest[len..];
            if matches!(
                after.trim_start().chars().next(),
                Some('(') | Some('*') | Some('{')
            ) {
                return None;
            }
            // `get`/`set`/`async` prefixes are accessors/methods, not data keys.
            if matches!(key, "get" | "set" | "async" | "static") {
                return None;
            }
            (key, after)
        }
        _ => return None,
    };
    let after = after.trim_start();
    let colon = after.strip_prefix(':');
    if colon.is_none() && !after.is_empty() && !after.starts_with(',') {
        return None;
    }
    Some((key.to_owned(), leading, colon.is_some()))
}

fn split_properties(body: &str, start: usize) -> Vec<(usize, usize)> {
    let structural = mask_strings(body);
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut segment = 0usize;
    for (index, ch) in structural.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                spans.push((start + segment, start + index));
                segment = index + 1;
            }
            _ => {}
        }
    }
    if start + segment <= start + body.len() {
        spans.push((start + segment, start + body.len()));
    }
    spans
}

fn property_removal(
    text: &str,
    property_start: usize,
    property_end: usize,
    has_comma: bool,
    comma_offset: Option<usize>,
) -> Option<Range<usize>> {
    let line_start = text[..property_start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[property_end..]
        .find('\n')
        .map(|index| property_end + index)
        .unwrap_or(text.len());
    let after_property = if has_comma {
        comma_offset? + 1
    } else {
        property_end
    };
    let own_line = text[line_start..property_start].trim().is_empty()
        && text[after_property..line_end].trim().is_empty();
    if own_line {
        let mut end = after_property;
        if text[end..].starts_with("\r\n") {
            end += 2;
        } else if text[end..].starts_with('\n') {
            end += 1;
        }
        return Some(line_start..end);
    }
    if has_comma {
        let comma = comma_offset?;
        let mut end = comma + 1;
        while text[end..].starts_with(' ') {
            end += 1;
        }
        return Some(property_start..end);
    }
    None
}

/// Duplicate plain keys within brace-delimited object-literal candidates.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateKey> {
    let masked = crate::semantic::mask_comments(text);
    let structural = mask_strings(&masked);
    let bytes = structural.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'{' {
            index += 1;
            continue;
        }
        let open = index;
        let before = structural[..open].trim_end();
        let expression_context = before
            .chars()
            .next_back()
            .is_some_and(|ch| matches!(ch, '=' | '(' | '[' | ',' | ':' | '?' | '>'))
            || before.ends_with("return");
        if !expression_context {
            index += 1;
            continue;
        }
        let mut depth = 0i32;
        let mut close = None;
        for (offset, ch) in structural[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else {
            break;
        };
        index = close + 1;
        let body = &structural[open + 1..close];
        if body.len() > 8 * 1024 || body.matches('\n').count() > 200 {
            continue;
        }
        let mut seen = HashMap::<String, (usize, usize, bool, Option<usize>)>::new();
        for (start, end) in split_properties(body, open + 1) {
            let segment = &masked[start..end];
            let Some((key, leading, _)) = key_from(segment) else {
                continue;
            };
            let property_start = start + leading;
            let trimmed_end = start + segment.trim_end().len();
            let comma = if end < close && masked[end..].starts_with(',') {
                Some(end)
            } else {
                None
            };
            let entry = seen.get(&key).copied();
            if let Some((first_start, first_end, first_literal, first_comma)) = entry {
                let removal = if first_literal {
                    property_removal(
                        &masked,
                        first_start,
                        first_end,
                        first_comma.is_some(),
                        first_comma,
                    )
                } else {
                    None
                };
                found.push(DuplicateKey {
                    key,
                    offset: property_start,
                    first_line: line_of(&masked, first_start),
                    removal,
                });
                continue;
            }
            let value = segment
                .find(':')
                .map(|index| &masked[start + index + 1..trimmed_end]);
            let literal = value.is_some_and(literal_value);
            seen.insert(key, (property_start, trimmed_end, literal, comma));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_plain_keys_report_the_later_offset_and_first_line() {
        let text = "const options = {\n  size: 1,\n  color: 'red',\n  size: 2,\n};\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "size");
        assert_eq!(found[0].first_line, 2);
        assert_eq!(found[0].offset, text.rfind("size").unwrap());

        let inline = "const point = { x: 1, x: 2 };";
        let found = duplicates(inline);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].offset, inline.rfind('x').unwrap());
        assert!(found[0].removal.is_some());
    }

    #[test]
    fn computed_method_spread_shorthand_and_quoted_keys_are_bounded() {
        assert!(duplicates("const o = { ['a']: 1, a: 2 };").is_empty());
        assert!(duplicates("const o = { ...a, a: 1 };").is_empty());
        assert!(duplicates("const o = { a() {}, a() {} };").is_empty());
        assert!(duplicates("const o = { get a() {}, get a() {} };").is_empty());
        let shorthand = "const o = { a, a };";
        assert_eq!(duplicates(shorthand).len(), 1);
        assert!(duplicates("const o = { a, a };")[0].removal.is_none());

        let quoted = "const o = { 'a': 1, \"a\": 2 };";
        let found = duplicates(quoted);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "a");

        assert!(duplicates("const o = { a: 1, 'a-b': 2 };").is_empty());
        assert!(duplicates("const o = { a: 1, b: 2 };").is_empty());
    }

    #[test]
    fn removal_only_for_literal_earlier_values_and_keeps_syntax_valid() {
        let multi = "const o = {\n  a: 1,\n  a: 2,\n};\n";
        let removal = duplicates(multi)[0].removal.clone().unwrap();
        let mut edited = multi.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const o = {\n  a: 2,\n};\n");

        let inline = "const o = { b: 0, a: 1, a: 2 };";
        let removal = duplicates(inline)[0].removal.clone().unwrap();
        let mut edited = inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const o = { b: 0, a: 2 };");

        let last_inline = "const o = { a: 1, b: 2, a: 3 };";
        let removal = duplicates(last_inline)[0].removal.clone().unwrap();
        let mut edited = last_inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const o = { b: 2, a: 3 };");

        let side_effects = "const o = { a: compute(), a: 2 };";
        assert!(duplicates(side_effects)[0].removal.is_none());
        let reference = "const o = { a: other, a: 2 };";
        assert!(duplicates(reference)[0].removal.is_none());
        let array = "const o = { a: [1, 2], a: 2 };";
        assert!(duplicates(array)[0].removal.is_none());
    }

    #[test]
    fn block_function_and_class_braces_are_not_treated_as_objects() {
        assert!(duplicates("function f() { const a = 1; a; }").is_empty());
        assert!(duplicates("if (ok) { a(); }").is_empty());
        assert!(duplicates("class A { x = 1; }").is_empty());
        assert!(duplicates("const { a, a } = value;").is_empty());
        assert!(duplicates("switch (x) { case 1: break; }").is_empty());
        let returned = "function f() { return { a: 1, a: 2 }; }";
        assert_eq!(duplicates(returned).len(), 1);
        let arrow = "const f = () => ({ a: 1, a: 2 });";
        assert_eq!(duplicates(arrow).len(), 1);
    }

    #[test]
    fn comments_do_not_contribute_keys_and_unmatched_braces_are_ignored() {
        assert!(duplicates("const o = { a: 1 /* a: 2 */ };").is_empty());
        assert!(duplicates("const o = { a: 1, // a: 2\n};").is_empty());
        assert!(duplicates("const o = { a: '}' , a: 2 };").len() == 1);
        assert!(duplicates("const o = { a: 1, ").is_empty());
    }
}
