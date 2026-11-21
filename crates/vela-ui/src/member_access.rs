//! Bounded `this.<member>` existence checks for TypeScript classes.
//!
//! Source's TypeScript worker reports `Property 'x' does not exist on type
//! 'Name'.` when a class body reads a member the class never declares. This
//! provider reproduces that check for classes declared in the same document.
//! Classes that extend another class, declare computed members, or live in
//! JavaScript files are skipped so the provider never guesses.

use std::collections::HashSet;

const DIAGNOSTIC_LIMIT: usize = 50;

#[derive(Debug)]
pub(super) struct MemberDiagnostic {
    pub offset: usize,
    pub message: String,
}

pub(super) fn diagnostics(rel: &str, text: &str) -> Vec<MemberDiagnostic> {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(extension.as_str(), "ts" | "tsx" | "mts" | "cts") {
        return Vec::new();
    }
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    if masked.len() != text.len() {
        return Vec::new();
    }
    let bytes = masked.as_bytes();
    let mut found = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if !masked.is_char_boundary(index) || !at_word_start(bytes, index) {
            index += 1;
            continue;
        }
        let Some(rest) = masked[index..].strip_prefix("class") else {
            index += 1;
            continue;
        };
        if !rest.starts_with([' ', '\t', '\n']) {
            index += 1;
            continue;
        }
        let (name, after) = read_word(&masked, index + "class".len());
        if name.is_empty() {
            index = after;
            continue;
        }
        let Some(open) = find_char(&masked, after, b'{') else {
            index = after;
            continue;
        };
        let Some(close) = matching_brace(bytes, open) else {
            break;
        };
        index = close + 1;
        if masked[after..open].contains("extends") {
            continue;
        }
        let Some(members) = class_members(text, &masked, open, close) else {
            continue;
        };
        collect_unknown(&masked, open, close, &name, &members, &mut found);
        if found.len() >= DIAGNOSTIC_LIMIT {
            break;
        }
    }
    found.truncate(DIAGNOSTIC_LIMIT);
    found
}

/// Member names declared by the class body, or `None` when the body uses syntax
/// this bounded parser cannot attribute (computed members).
fn class_members(text: &str, masked: &str, open: usize, close: usize) -> Option<HashSet<String>> {
    let bytes = masked.as_bytes();
    let mut members = HashSet::new();
    let mut index = open + 1;
    let mut depth = 0usize;
    let mut at_member_start = true;
    while index < close {
        let ch = bytes[index];
        match ch {
            b'{' | b'(' | b'[' => {
                if ch == b'[' && depth == 0 && at_member_start {
                    return None;
                }
                depth += 1;
                index += 1;
            }
            b'}' | b')' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    at_member_start = true;
                }
                index += 1;
            }
            b';' | b',' if depth == 0 => {
                at_member_start = true;
                index += 1;
            }
            _ if depth > 0 || !at_member_start => index += 1,
            _ if ch.is_ascii_whitespace() => index += 1,
            _ => {
                if masked[index..].starts_with("constructor")
                    && let Some(paren) = find_char(masked, index + "constructor".len(), b'(')
                {
                    let end = matching_paren(bytes, paren).unwrap_or(paren);
                    collect_parameter_names(masked, paren + 1, end, &mut members);
                    index = end + 1;
                    at_member_start = false;
                    continue;
                }
                #[cfg(test)]
                eprintln!(
                    "candidate at {index}: {:?}",
                    &masked[index..(index + 40).min(close)]
                );
                let (name, after) = read_member_name(text, masked, index, close);
                if name.is_empty() {
                    at_member_start = false;
                    index += 1;
                    continue;
                }
                let next = masked[after..close].chars().find(|ch| !ch.is_whitespace());
                let member_shape = matches!(
                    next,
                    Some('(' | ':' | '=' | '?' | '!' | '<' | ';' | '}') | None
                );
                // `get size()` is an accessor while `get(id)` is a method named
                // `get`; a bare modifier is never followed by a member shape.
                if !member_shape
                    && matches!(
                        name.as_str(),
                        "static"
                            | "async"
                            | "get"
                            | "set"
                            | "public"
                            | "private"
                            | "protected"
                            | "readonly"
                            | "override"
                            | "declare"
                            | "abstract"
                            | "accessor"
                    )
                {
                    index = after;
                    continue;
                }
                if member_shape {
                    members.insert(name);
                }
                at_member_start = false;
                index = after;
            }
        }
    }
    Some(members)
}

/// Constructor parameters (including parameter properties) count as members so
/// a bounded parser never over-reports.
fn collect_parameter_names(masked: &str, start: usize, end: usize, members: &mut HashSet<String>) {
    for (part, _) in split_top_level(masked, start, end) {
        let mut part = part.trim_start();
        for modifier in [
            "public",
            "private",
            "protected",
            "readonly",
            "override",
        ] {
            if let Some(rest) = part.strip_prefix(modifier)
                && rest.starts_with([' ', '\t'])
            {
                part = rest.trim_start();
            }
        }
        let (name, _) = read_word(part, 0);
        if !name.is_empty() && name != "this" {
            members.insert(name);
        }
    }
}

fn collect_unknown(
    masked: &str,
    open: usize,
    close: usize,
    class: &str,
    members: &HashSet<String>,
    found: &mut Vec<MemberDiagnostic>,
) {
    let bytes = masked.as_bytes();
    let mut index = open + 1;
    while index < close {
        if masked[index..].starts_with("this") && at_word_start(bytes, index) {
            let after = index + "this".len();
            if bytes.get(after) == Some(&b'.') {
                let start = word_start(masked, after + 1);
                let (name, end) = read_word(masked, start);
                if !name.is_empty() {
                    if !members.contains(&name) {
                        found.push(MemberDiagnostic {
                            offset: start,
                            message: format!(
                                "Property '{name}' does not exist on type '{class}'."
                            ),
                        });
                        if found.len() >= DIAGNOSTIC_LIMIT {
                            return;
                        }
                    }
                    index = end;
                    continue;
                }
            }
        }
        index += 1;
    }
}

fn read_member_name(text: &str, masked: &str, start: usize, end: usize) -> (String, usize) {
    let bytes = masked.as_bytes();
    if matches!(bytes.get(start), Some(b'"') | Some(b'\'')) {
        let quote = bytes[start];
        let mut index = start + 1;
        while index < end {
            if bytes[index] == quote {
                return (text[start + 1..index].to_owned(), index + 1);
            }
            index += 1;
        }
        return (String::new(), start);
    }
    if bytes.get(start) == Some(&b'#') {
        let (name, after) = read_word(masked, start + 1);
        return (format!("#{name}"), after);
    }
    read_word(masked, start)
}

fn read_word(text: &str, start: usize) -> (String, usize) {
    let bytes = text.as_bytes();
    let index = word_start(text, start);
    let mut end = index;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'$'))
    {
        end += 1;
    }
    (text[index..end].to_owned(), end)
}

fn word_start(text: &str, start: usize) -> usize {
    text[start..]
        .find(|ch: char| !ch.is_whitespace())
        .map(|offset| start + offset)
        .unwrap_or(text.len())
}

/// Splits a parameter list at top-level commas, returning `(text, offset)`.
fn split_top_level(text: &str, start: usize, end: usize) -> Vec<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut segment = start;
    let mut index = start;
    while index < end {
        match bytes[index] {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                parts.push((&text[segment..index], segment));
                segment = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    parts.push((&text[segment..end], segment));
    parts
}

fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn matching_paren(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn find_char(text: &str, from: usize, needle: u8) -> Option<usize> {
    text.as_bytes()[from..]
        .iter()
        .position(|byte| *byte == needle)
        .map(|offset| from + offset)
}

fn at_word_start(bytes: &[u8], index: usize) -> bool {
    index == 0
        || !matches!(
            bytes[index - 1],
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(rel: &str, text: &str) -> Vec<String> {
        diagnostics(rel, text)
            .into_iter()
            .map(|found| found.message)
            .collect()
    }

    #[test]
    fn unknown_this_members_are_reported() {
        let text = "class Panel {\n  count = 0;\n  render() {\n    return this.count + this.missing;\n  }\n}\n";
        assert_eq!(
            messages("x.ts", text),
            vec!["Property 'missing' does not exist on type 'Panel'.".to_string()]
        );

        let clean = "class Panel {\n  count = 0;\n  label?: string;\n  static total = 0;\n  get size() { return this.count; }\n  render() { return this.count; }\n  constructor(private readonly id: string, extra: number) {}\n  labels() { return this.id + this.extra; }\n}\n";
        assert!(messages("x.ts", clean).is_empty(), "{:?}", messages("x.ts", clean));
    }

    #[test]
    fn extensions_and_computed_members_are_bounded() {
        for (rel, text) in [
            ("x.js", "class A {\n  run() { return this.anything; }\n}\n"),
            ("x.tsx", "class A extends B {\n  run() { return this.inherited; }\n}\n"),
            ("x.ts", "class A {\n  [key: string]: number;\n  run() { return this.other; }\n}\n"),
        ] {
            assert!(messages(rel, text).is_empty(), "{rel}: {text:?}");
        }
    }

    #[test]
    fn private_members_quoted_keys_and_nested_objects() {
        let text = "class A {\n  #secret = 1;\n  'on-click' = () => {};\n  run() { return this.#secret + this.on; }\n}\n";
        let found = messages("x.ts", text);
        assert_eq!(found, vec!["Property 'on' does not exist on type 'A'.".to_string()]);

        // `this` inside a nested callback still refers to the class instance.
        let callback = "class A {\n  value = 1;\n  run() { return [1].map(() => this.value); }\n}\n";
        assert!(messages("x.ts", callback).is_empty());
    }

    #[test]
    fn methods_named_get_or_set_are_members_not_modifiers() {
        let text = "class A {\n  private get(id: string): string { return id; }\n  private set(value: string): void {}\n  run() { return this.get(this.set.toString()); }\n}\n";
        let found = messages("x.ts", text);
        assert!(
            found.iter().all(|message| !message.contains("'get'") && !message.contains("'set'")),
            "{found:?}"
        );
    }

    #[test]
    fn offsets_point_at_the_member_name() {
        let text = "class A {\n  run() { return this.nope; }\n}\n";
        let found = diagnostics("x.ts", text);
        assert_eq!(found.len(), 1);
        assert!(text[found[0].offset..].starts_with("nope"), "{found:?}");
    }
}


