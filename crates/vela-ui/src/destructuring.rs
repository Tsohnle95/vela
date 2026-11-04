//! Bounded duplicate binding checks for object-destructuring declarations.
//!
//! Only single-line `const`/`let`/`var` patterns with simple or aliased keys are
//! compared. Nested patterns, computed keys, rest elements and defaults are
//! either skipped or reported without an automatic edit.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateBinding {
    pub name: String,
    /// Byte offset of the later local binding.
    pub offset: usize,
    /// Document range removing the redundant later property, when safe.
    pub removal: Option<Range<usize>>,
}

#[derive(Clone)]
struct Binding {
    name: String,
    canonical: String,
    start: usize,
    end: usize,
    offset: usize,
}

fn matching_brace(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in line[open..].char_indices() {
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
            '{' => depth += 1,
            '}' => {
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

fn split_top_level(inner: &str, start: usize) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    let mut segment = 0usize;
    for (index, ch) in inner.char_indices() {
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
            '"' | '\'' | '`' | '{' | '[' | '(' => {
                if matches!(ch, '"' | '\'' | '`') {
                    quote = Some(ch);
                } else {
                    depth += 1;
                }
            }
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                spans.push((start + segment, start + index));
                segment = index + 1;
            }
            _ => {}
        }
    }
    spans.push((start + segment, start + inner.len()));
    spans
}

fn binding_from(part: &str, start: usize) -> Option<Binding> {
    let trimmed = part.trim();
    let leading = part.len() - part.trim_start().len();
    if trimmed.is_empty()
        || trimmed.starts_with("...")
        || trimmed.starts_with('[')
        || trimmed.starts_with('{')
    {
        return None;
    }
    let base = start + leading;
    let (key, local, local_offset, has_default) = match trimmed.chars().next()? {
        quote @ ('"' | '\'') => {
            let close = trimmed[1..].find(quote)? + 1;
            let key = &trimmed[1..close];
            let rest = trimmed[close + 1..].trim_start();
            let after = rest.strip_prefix(':')?.trim_start();
            let name_len = after
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            let local = &after[..name_len];
            if local.is_empty() || after[name_len..].trim_start().starts_with(['{', '[']) {
                return None;
            }
            let offset = trimmed.find(after).unwrap_or(0);
            let has_default = after[name_len..].trim_start().starts_with('=');
            (key, local, offset, has_default)
        }
        ch if ch.is_alphabetic() || matches!(ch, '_' | '$') => {
            let name_len = trimmed
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            let key = &trimmed[..name_len];
            let rest = trimmed[name_len..].trim_start();
            if let Some(after) = rest.strip_prefix(':') {
                let after = after.trim_start();
                if after.starts_with(['{', '[']) {
                    return None;
                }
                let local_len = after
                    .char_indices()
                    .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                    .last()
                    .map(|(index, ch)| index + ch.len_utf8())
                    .unwrap_or(0);
                let local = &after[..local_len];
                if local.is_empty() {
                    return None;
                }
                let offset = trimmed.find(after).unwrap_or(0);
                let has_default = after[local_len..].trim_start().starts_with('=');
                (key, local, offset, has_default)
            } else {
                let has_default = rest.starts_with('=');
                (key, key, 0, has_default)
            }
        }
        _ => return None,
    };
    if !crate::rename::valid_identifier(key) || !crate::rename::valid_identifier(local) {
        return None;
    }
    Some(Binding {
        name: local.to_owned(),
        canonical: format!(
            "{}|{}|{}",
            key,
            local,
            if has_default { "default" } else { "" }
        ),
        start: base,
        end: base + trimmed.len(),
        offset: base + local_offset,
    })
}

fn pattern_bindings(line: &str) -> Vec<Binding> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let mut rest = trimmed;
    let mut offset = leading;
    if let Some(after) = rest.strip_prefix("export ") {
        rest = after;
        offset += "export ".len();
    }
    let keyword_len = ["const ", "let ", "var "]
        .into_iter()
        .find(|keyword| rest.starts_with(keyword))
        .map(str::len)
        .unwrap_or(0);
    if keyword_len == 0 {
        return Vec::new();
    }
    let after_keyword = rest[keyword_len..].trim_start();
    let after_leading = rest[keyword_len..].len() - after_keyword.len();
    if !after_keyword.starts_with('{') {
        return Vec::new();
    }
    let open = offset + keyword_len + after_leading;
    let Some(close) = matching_brace(line, open) else {
        return Vec::new();
    };
    let tail = line[close + 1..].trim_start();
    if !tail.starts_with('=') {
        return Vec::new();
    }
    let inner = &line[open + 1..close];
    let mut bindings = Vec::new();
    for (start, end) in split_top_level(inner, open + 1) {
        if let Some(binding) = binding_from(&line[start..end], start) {
            bindings.push(binding);
        }
    }
    bindings
}

fn named_removal(line: &str, later: &Binding) -> Option<Range<usize>> {
    if line[later.end..].starts_with(',') {
        let mut end = later.end + 1;
        while line[end..].starts_with(' ') {
            end += 1;
        }
        return Some(later.start..end);
    }
    let comma = line[..later.start].rfind(',')?;
    Some(comma..later.end)
}

/// Duplicate local names within one single-line object-destructuring pattern.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateBinding> {
    let masked = crate::semantic::mask_comments(text);
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in masked.split('\n') {
        let mut seen = HashMap::<String, Binding>::new();
        for binding in pattern_bindings(line) {
            if let Some(first) = seen.get(&binding.name) {
                let removal = (first.canonical == binding.canonical)
                    .then(|| named_removal(line, &binding))
                    .flatten()
                    .map(|range| line_start + range.start..line_start + range.end);
                found.push(DuplicateBinding {
                    name: binding.name,
                    offset: line_start + binding.offset,
                    removal,
                });
            } else {
                seen.insert(binding.name.clone(), binding);
            }
        }
        line_start += line.len() + 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_and_aliased_duplicates_report_and_remove() {
        let text = "const { read, read } = source;\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const { read } = source;\n");

        let middle = "let { a, a, b } = source;\n";
        let removal = duplicates(middle)[0].removal.clone().unwrap();
        let mut edited = middle.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "let { a, b } = source;\n");

        let last = "var { a, b, a } = source;\n";
        let removal = duplicates(last)[0].removal.clone().unwrap();
        let mut edited = last.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "var { a, b } = source;\n");

        let identical_alias = "const { a: x, a: x } = source;\n";
        assert!(duplicates(identical_alias)[0].removal.is_some());
        let exported = "export const { a, a } = source;\n";
        assert!(duplicates(exported)[0].removal.is_some());
    }

    #[test]
    fn different_keys_or_defaults_are_reported_without_an_action() {
        let aliased = "const { a: x, b: x } = source;\n";
        assert_eq!(duplicates(aliased).len(), 1);
        assert!(duplicates(aliased)[0].removal.is_none());

        let default = "const { a, a = 1 } = source;\n";
        assert_eq!(duplicates(default).len(), 1);
        assert!(duplicates(default)[0].removal.is_none());

        let quoted = "const { 'a': x, \"a\": x } = source;\n";
        assert_eq!(duplicates(quoted).len(), 1);
        assert!(duplicates(quoted)[0].removal.is_some());
    }

    #[test]
    fn nested_patterns_distinct_names_and_other_braces_are_bounded() {
        assert!(duplicates("const { a, b } = source;\n").is_empty());
        assert!(duplicates("const { a: { a } } = source;\n").is_empty());
        assert!(duplicates("const { [a]: a } = source;\n").is_empty());
        assert!(duplicates("const { a } = source;\nconst { a } = other;\n").is_empty());
        assert!(duplicates("// const { a, a } = source;\n").is_empty());
        assert!(duplicates("const { a /* a */, b } = source;\n").is_empty());
        assert!(duplicates("const { a,\na } = source;\n").is_empty());
        assert!(duplicates("const value = { a: 1, a: 2 };\n").is_empty());
        assert!(duplicates("function f({ a, a }) {}\n").is_empty());
        assert!(duplicates("const { a, a };\n").is_empty());
        let rest = "const { ...rest, rest } = source;\n";
        let _ = rest;
        let unicode = "const { café, café } = source;\n";
        assert!(duplicates(unicode)[0].removal.is_some());
    }
}
