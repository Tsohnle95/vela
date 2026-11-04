//! Shared duplicate-property analysis for TypeScript structural type bodies.
//!
//! Interfaces and object type literals use the same member grammar: property
//! signatures, method signatures/overloads, optional/readonly flags and quoted
//! names. Method overloads are legal and skipped; identical duplicate properties
//! are the only auto-removable case.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateProperty {
    pub name: String,
    /// Byte offset of the later member name.
    pub offset: usize,
    pub first_line: usize,
    /// Document range removing the redundant later property, when identical.
    pub removal: Option<Range<usize>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Property,
    Method,
}

#[derive(Clone)]
struct Member {
    name: String,
    offset: usize,
    start: usize,
    end: usize,
    sep: Option<usize>,
    kind: Kind,
    readonly: bool,
    optional: bool,
    type_text: Option<String>,
}

impl Member {
    fn identical(&self, other: &Self) -> bool {
        self.kind == Kind::Property
            && other.kind == Kind::Property
            && self.readonly == other.readonly
            && self.optional == other.optional
            && self.type_text.is_some()
            && self.type_text == other.type_text
    }
}

fn member_from(text: &str, start: usize, end: usize) -> Option<Member> {
    let trimmed = text[start..end].trim();
    let leading = text[start..end].len() - text[start..end].trim_start().len();
    if trimmed.is_empty() {
        return None;
    }
    let (readonly, rest) = match trimmed.strip_prefix("readonly ") {
        Some(rest) => (true, rest.trim_start()),
        None => (false, trimmed),
    };
    if rest.starts_with('[')
        || rest.starts_with("new ")
        || rest.starts_with("get ")
        || rest.starts_with("set ")
    {
        return None;
    }
    let rest_leading = trimmed.len() - rest.len();
    let (name, name_offset, name_end) = match rest.chars().next()? {
        quote @ ('"' | '\'') => {
            let close = rest[1..].find(quote)? + 1;
            let inner = &rest[1..close];
            if inner.is_empty() {
                return None;
            }
            (inner.to_owned(), rest_leading + 1, rest_leading + close + 1)
        }
        ch if ch.is_alphabetic() || matches!(ch, '_' | '$') => {
            let len = rest
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            if !crate::rename::valid_identifier(&rest[..len]) {
                return None;
            }
            (rest[..len].to_owned(), rest_leading, rest_leading + len)
        }
        _ => return None,
    };
    let mut after = name_end;
    while after < trimmed.len() && trimmed[after..].starts_with(' ') {
        after += 1;
    }
    let optional = trimmed[after..].starts_with('?');
    if optional {
        after += 1;
        while after < trimmed.len() && trimmed[after..].starts_with(' ') {
            after += 1;
        }
    }
    let (kind, type_text) = if trimmed[after..].starts_with('(') {
        (Kind::Method, None)
    } else if trimmed[after..].starts_with(':') {
        let type_text = trimmed[after + 1..].trim();
        if type_text.is_empty() {
            return None;
        }
        (Kind::Property, Some(type_text.to_owned()))
    } else {
        return None;
    };
    let member_start = start + leading;
    Some(Member {
        name,
        offset: member_start + name_offset,
        start: member_start,
        end: start + text[start..end].trim_end().len(),
        sep: None,
        kind,
        readonly,
        optional,
        type_text,
    })
}

fn member_removal(text: &str, member: &Member) -> Option<Range<usize>> {
    let after = member.sep.map(|sep| sep + 1).unwrap_or(member.end);
    let line_start = text[..member.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[after..]
        .find('\n')
        .map(|index| after + index)
        .unwrap_or(text.len());
    let own_line =
        text[line_start..member.start].trim().is_empty() && text[after..line_end].trim().is_empty();
    if own_line {
        let end = if text[line_end..].starts_with('\n') {
            line_end + 1
        } else {
            line_end
        };
        return Some(line_start..end);
    }
    if member.sep.is_some() {
        let mut end = after;
        while text[end..].starts_with(' ') {
            end += 1;
        }
        return Some(member.start..end);
    }
    let sep = text[..member.start].rfind([';', ','])?;
    Some(sep..member.end)
}

fn split_members(text: &str, start: usize, end: usize) -> Vec<(usize, usize, Option<usize>)> {
    let structural = crate::object_keys::mask_strings(text);
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut segment = start;
    for (index, ch) in structural[start..end].char_indices() {
        let at = start + index;
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if depth > 0 => depth -= 1,
            ';' | ',' if depth == 0 => {
                spans.push((segment, at, Some(at)));
                segment = at + 1;
            }
            _ => {}
        }
    }
    spans.push((segment, end, None));
    spans
}

fn matching_brace(text: &str, open: usize) -> Option<usize> {
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

fn nested_object_open(masked: &str, member: &Member) -> Option<usize> {
    if member.kind != Kind::Property || !member.type_text.as_deref()?.starts_with('{') {
        return None;
    }
    let colon = masked[member.offset..member.end].find(':')? + member.offset;
    Some(masked[colon + 1..member.end].find('{')? + colon + 1)
}

/// Duplicate property members within a brace-delimited type body.
pub(super) fn duplicates_in(
    masked: &str,
    body_start: usize,
    body_end: usize,
) -> Vec<DuplicateProperty> {
    duplicates_inner(masked, body_start, body_end, 0)
}

fn duplicates_inner(
    masked: &str,
    body_start: usize,
    body_end: usize,
    depth: usize,
) -> Vec<DuplicateProperty> {
    let mut members = Vec::new();
    for (start, end, sep) in split_members(masked, body_start, body_end) {
        if let Some(mut member) = member_from(masked, start, end) {
            member.sep = sep;
            members.push(member);
        }
    }
    let mut found = Vec::new();
    if depth < 8 {
        for member in &members {
            if let Some(open) = nested_object_open(masked, member)
                && let Some(close) = matching_brace(masked, open)
            {
                let body = &masked[open + 1..close];
                if body.len() <= 8 * 1024 && body.matches('\n').count() <= 200 {
                    found.extend(duplicates_inner(masked, open + 1, close, depth + 1));
                }
            }
        }
    }
    let mut seen = HashMap::<String, (usize, Member)>::new();
    for member in members {
        let line = masked[..member.offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        if let Some((first_line, first)) = seen.get(&member.name) {
            if first.kind == Kind::Method && member.kind == Kind::Method {
                continue;
            }
            let removal = member
                .identical(first)
                .then(|| member_removal(masked, &member))
                .flatten();
            found.push(DuplicateProperty {
                name: member.name.clone(),
                offset: member.offset,
                first_line: *first_line,
                removal,
            });
            continue;
        }
        seen.insert(member.name.clone(), (line, member));
    }
    found
}
