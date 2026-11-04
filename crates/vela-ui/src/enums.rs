//! Bounded duplicate checks for TypeScript enum members.
//!
//! Member names and numeric/string values are compared. Auto-numbered values
//! follow TypeScript's previous-value-plus-one rule. A safe quick fix removes a
//! later duplicate name only when both members carry the same explicit literal
//! initializer, which preserves the name, its value and subsequent numbering.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IssueKind {
    DuplicateName,
    DuplicateValue,
}

#[derive(Clone, Debug)]
pub(super) struct EnumIssue {
    pub kind: IssueKind,
    pub name: String,
    /// For value collisions, the first member using the same value.
    pub first_name: Option<String>,
    pub value_label: Option<String>,
    /// Byte offset of the later member name.
    pub offset: usize,
    pub first_line: usize,
    pub removal: Option<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
enum Literal {
    Number(f64),
    Text(String),
}

fn number_label(value: f64) -> String {
    if value.fract() == 0. && value.abs() < 9_007_199_254_740_992. {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn literal(text: &str) -> Option<Literal> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let first = text.chars().next()?;
    if matches!(first, '"' | '\'') {
        let close = text[1..].find(first)? + 1;
        if close + 1 != text.len() {
            return None;
        }
        let inner = &text[1..close];
        return (!inner.is_empty()).then(|| Literal::Text(inner.to_owned()));
    }
    let number = text.replace('_', "");
    if let Some(hex) = number
        .strip_prefix("0x")
        .or_else(|| number.strip_prefix("0X"))
    {
        return i64::from_str_radix(hex, 16)
            .ok()
            .map(|value| Literal::Number(value as f64));
    }
    number
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(Literal::Number)
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

#[derive(Clone, Debug)]
struct Member {
    name: String,
    offset: usize,
    start: usize,
    end: usize,
    comma: Option<usize>,
    has_initializer: bool,
    explicit: Option<Literal>,
}

fn member_from(text: &str, start: usize, end: usize) -> Option<Member> {
    let trimmed = text[start..end].trim();
    let leading = text[start..end].len() - text[start..end].trim_start().len();
    if trimmed.is_empty() {
        return None;
    }
    let (name, name_offset, name_end) = match trimmed.chars().next()? {
        quote @ ('"' | '\'') => {
            let close = trimmed[1..].find(quote)? + 1;
            let inner = &trimmed[1..close];
            if inner.is_empty() {
                return None;
            }
            (inner.to_owned(), 1, close + 1)
        }
        ch if ch.is_alphabetic() || matches!(ch, '_' | '$') => {
            let len = trimmed
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            if !crate::rename::valid_identifier(&trimmed[..len]) {
                return None;
            }
            (trimmed[..len].to_owned(), 0, len)
        }
        _ => return None,
    };
    let after_name = &trimmed[name_end..];
    let initializer = after_name.trim_start().strip_prefix('=');
    let member_start = start + leading;
    Some(Member {
        name,
        offset: member_start + name_offset,
        start: member_start,
        end: start + text[start..end].trim_end().len(),
        comma: None,
        has_initializer: initializer.is_some(),
        explicit: initializer.and_then(literal),
    })
}

fn member_removal(text: &str, member: &Member) -> Option<Range<usize>> {
    let after = member.comma.map(|comma| comma + 1).unwrap_or(member.end);
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
    if member.comma.is_some() {
        let mut end = after;
        while text[end..].starts_with(' ') {
            end += 1;
        }
        return Some(member.start..end);
    }
    let comma = text[..member.start].rfind(',')?;
    Some(comma..member.end)
}

fn enum_body(text: &str, at: usize) -> Option<(usize, usize)> {
    let before = text[..at].trim_end();
    let declaration = before.is_empty()
        || before.ends_with("export")
        || before.ends_with("const")
        || before.ends_with("declare")
        || before.ends_with("default");
    if !declaration {
        return None;
    }
    let open = text[at + "enum".len()..].find('{')? + at + "enum".len();
    let close = matching(text, open, '{', '}')?;
    let body = &text[open + 1..close];
    if body.len() > 8 * 1024 || body.matches('\n').count() > 200 {
        return None;
    }
    Some((open + 1, close))
}

fn split_members(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let structural = crate::object_keys::mask_strings(text);
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut segment = start;
    for (index, ch) in structural[start..end].char_indices() {
        let at = start + index;
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                spans.push((segment, at));
                segment = at + 1;
            }
            _ => {}
        }
    }
    spans.push((segment, end));
    spans
}

/// Duplicate enum member names and numeric/string values.
pub(super) fn issues(text: &str) -> Vec<EnumIssue> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("enum") {
        let at = search + relative;
        let before_ok = structural[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
        let after_ok = structural[at + "enum".len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace);
        if !before_ok || !after_ok {
            search = at + "enum".len();
            continue;
        }
        let Some((body_start, body_end)) = enum_body(&structural, at) else {
            search = at + "enum".len();
            continue;
        };
        let mut members = Vec::new();
        for (start, end) in split_members(&masked, body_start, body_end) {
            if let Some(mut member) = member_from(&masked, start, end) {
                if end < body_end && masked[end..].starts_with(',') {
                    member.comma = Some(end);
                }
                members.push(member);
            }
        }
        let mut names = HashMap::<String, (usize, usize, Option<Literal>)>::new();
        let mut values = HashMap::<String, (String, usize)>::new();
        let mut previous: Option<f64> = None;
        let mut started = false;
        for (index, member) in members.iter().enumerate() {
            let line = masked[..member.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let numeric = match &member.explicit {
                Some(Literal::Number(value)) => Some(*value),
                Some(Literal::Text(_)) => {
                    previous = None;
                    started = true;
                    None
                }
                None if member.has_initializer => {
                    previous = None;
                    started = true;
                    None
                }
                None if started => previous.map(|value| value + 1.),
                None => Some(0.),
            };
            if let Some(value) = numeric {
                previous = Some(value);
                started = true;
            }
            if let Some((first_line, _, first_literal)) = names.get(&member.name) {
                let removal = (first_literal.is_some() && first_literal == &member.explicit)
                    .then(|| member_removal(&masked, member))
                    .flatten();
                found.push(EnumIssue {
                    kind: IssueKind::DuplicateName,
                    name: member.name.clone(),
                    first_name: None,
                    value_label: None,
                    offset: member.offset,
                    first_line: *first_line,
                    removal,
                });
                let _ = index;
                continue;
            }
            names.insert(
                member.name.clone(),
                (line, member.offset, member.explicit.clone()),
            );
            if let Some(Literal::Number(value)) = numeric.map(Literal::Number) {
                let key = number_label(value);
                if let Some((first_name, _)) = values.get(&key) {
                    found.push(EnumIssue {
                        kind: IssueKind::DuplicateValue,
                        name: member.name.clone(),
                        first_name: Some(first_name.clone()),
                        value_label: Some(key),
                        offset: member.offset,
                        first_line: line,
                        removal: None,
                    });
                } else {
                    values.insert(key, (member.name.clone(), member.offset));
                }
            } else if let Some(Literal::Text(value)) = &member.explicit {
                if let Some((first_name, _)) = values.get(value) {
                    found.push(EnumIssue {
                        kind: IssueKind::DuplicateValue,
                        name: member.name.clone(),
                        first_name: Some(first_name.clone()),
                        value_label: Some(format!("\"{value}\"")),
                        offset: member.offset,
                        first_line: line,
                        removal: None,
                    });
                } else {
                    values.insert(value.clone(), (member.name.clone(), member.offset));
                }
            }
        }
        search = at + "enum".len();
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_names_with_identical_literals_are_removable() {
        let text = "enum E {\n  A = 1,\n  B = 2,\n  A = 1,\n}\n";
        let found = issues(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, IssueKind::DuplicateName);
        assert_eq!(found[0].name, "A");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "enum E {\n  A = 1,\n  B = 2,\n}\n");

        let string = "enum E { A = \"x\", A = \"x\" }\n";
        assert!(issues(string)[0].removal.is_some());

        let auto = "enum E { A, A }\n";
        assert_eq!(issues(auto).len(), 1);
        assert!(issues(auto)[0].removal.is_none());

        let different = "enum E { A = 1, A = 2 }\n";
        assert!(issues(different)[0].removal.is_none());
    }

    #[test]
    fn duplicate_numeric_and_string_values_are_reported() {
        let explicit = "enum E { A = 1, B = 1 }\n";
        let found = issues(explicit);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, IssueKind::DuplicateValue);
        assert_eq!(found[0].name, "B");
        assert_eq!(found[0].first_name.as_deref(), Some("A"));
        assert_eq!(found[0].value_label.as_deref(), Some("1"));

        let auto = "enum E { A = 0, B, C = 1 }\n";
        let found = issues(auto);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, IssueKind::DuplicateValue);
        assert_eq!(found[0].name, "C");
        assert_eq!(found[0].first_name.as_deref(), Some("B"));

        let strings = "enum E { A = \"x\", B = \"x\" }\n";
        let found = issues(strings);
        assert_eq!(found[0].kind, IssueKind::DuplicateValue);
        assert_eq!(found[0].name, "B");

        let unique = "enum E { A = 1, B = 2, C }\n";
        assert!(issues(unique).is_empty());
    }

    #[test]
    fn quoted_names_prefixes_hex_and_comments_are_handled() {
        let quoted = "enum E { \"A\" = 1, A = 1 }\n";
        assert_eq!(issues(quoted)[0].kind, IssueKind::DuplicateName);
        assert!(issues(quoted)[0].removal.is_some());

        assert_eq!(issues("export enum E { A, B, A }\n").len(), 1);
        assert_eq!(issues("const enum E { A = 1, A = 1 }\n").len(), 1);
        assert_eq!(issues("export const enum E { A = 1, A = 1 }\n").len(), 1);

        let hex = "enum E { A = 0x10, B = 16 }\n";
        assert_eq!(issues(hex)[0].kind, IssueKind::DuplicateValue);

        assert!(issues("enum E { A = 1 /* A = 2 */ }\n").is_empty());
        assert!(issues("// enum E { A = 1, A = 1 }\n").is_empty());
        assert!(issues("const text = \"enum E { A = 1, A = 1 }\";\n").is_empty());
    }

    #[test]
    fn multiline_shared_lines_and_other_braces_are_bounded() {
        let shared = "enum E { A = 1, B = 2, A = 1 }\n";
        let removal = issues(shared)[0].removal.clone().unwrap();
        let mut edited = shared.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "enum E { A = 1, B = 2 }\n");

        assert!(issues("const value = { enum: 1 };\n").is_empty());
        assert!(issues("function enumValue() {}\n").is_empty());
        assert!(
            issues("enum E { A = compute(), A = 1 }\n")[0]
                .removal
                .is_none()
        );
    }
}
