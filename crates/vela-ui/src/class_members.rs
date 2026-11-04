//! Bounded duplicate-member checks for class declarations and expressions.
//!
//! Accessor pairs, static/instance pairs and different member kinds are treated
//! as distinct. A safe quick fix removes the earlier duplicate method/accessor,
//! or an earlier field whose initializer is a primitive literal; removing a
//! shadowed member cannot change the effective definition. Multi-line class
//! bodies are supported; multiline fields must be semicolon-terminated.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateMember {
    pub name: String,
    /// Byte offset of the later member name.
    pub offset: usize,
    /// Document range removing the earlier shadowed member, when safe.
    pub removal: Option<Range<usize>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Method,
    Get,
    Set,
    Field,
}

impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Get => "get",
            Self::Set => "set",
            Self::Field => "field",
        }
    }
}

#[derive(Clone)]
struct Member {
    name: String,
    offset: usize,
    start: usize,
    end: usize,
    kind: Kind,
    static_: bool,
    safe: bool,
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

fn parse_member(text: &str, body_end: usize, mut pos: usize) -> Option<(Member, usize)> {
    while pos < body_end
        && (text[pos..].chars().next().is_some_and(char::is_whitespace)
            || text[pos..].starts_with(';'))
    {
        pos += text[pos..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    if pos >= body_end {
        return None;
    }
    let start = pos;
    if text[pos..].starts_with('@') {
        return None;
    }
    let mut static_ = false;
    let mut kind = Kind::Method;
    let mut private = false;
    loop {
        let rest = &text[pos..body_end];
        if let Some(after) = rest.strip_prefix("static ") {
            if after.trim_start().starts_with('{') {
                return None;
            }
            static_ = true;
            pos += "static ".len();
        } else if rest.starts_with("async ") {
            pos += "async ".len();
        } else if rest.starts_with("get ") {
            kind = Kind::Get;
            pos += "get ".len();
        } else if rest.starts_with("set ") {
            kind = Kind::Set;
            pos += "set ".len();
        } else if let Some((prefix, _)) = [
            "public ",
            "private ",
            "protected ",
            "readonly ",
            "override ",
            "declare ",
            "abstract ",
        ]
        .into_iter()
        .find_map(|prefix| rest.strip_prefix(prefix).map(|tail| (prefix, tail)))
        {
            pos += prefix.len();
        } else if rest.starts_with('*') {
            pos += 1;
        } else if rest.starts_with('#') {
            private = true;
            pos += 1;
        } else {
            break;
        }
    }
    if pos >= body_end {
        return None;
    }
    let (name, name_offset, name_end) = match text[pos..].chars().next()? {
        quote @ ('"' | '\'') => {
            let close = text[pos + 1..body_end].find(quote)? + pos + 1;
            (&text[pos + 1..close], pos, close + 1)
        }
        ch if ch.is_alphabetic() || matches!(ch, '_' | '$') => {
            let len = text[pos..body_end]
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            (&text[pos..pos + len], pos, pos + len)
        }
        _ => return None,
    };
    if !crate::rename::valid_identifier(name) {
        return None;
    }
    let name = name.to_owned();
    pos = name_end;
    while pos < body_end
        && text[pos..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == '?' || ch == '!')
    {
        pos += text[pos..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    let (end, safe) = if text[pos..body_end].starts_with('(') {
        let close = matching(text, pos, '(', ')')?;
        let open = text[close + 1..body_end].find('{')? + close + 1;
        let end = matching(text, open, '{', '}')? + 1;
        (end, name != "constructor" && !private)
    } else if text[pos..body_end].starts_with('=') {
        kind = Kind::Field;
        let eq = pos;
        let mut depth = 0i32;
        let mut quote = None;
        let mut escaped = false;
        let mut end = None;
        for (index, ch) in text[eq + 1..body_end].char_indices() {
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
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ';' if depth == 0 => {
                    end = Some(eq + 1 + index + 1);
                    break;
                }
                _ => {}
            }
        }
        let end = end.unwrap_or(body_end);
        let initializer = text[eq + 1..end].trim().trim_end_matches(';').trim();
        (
            end,
            crate::object_keys::literal_value(initializer) && !private,
        )
    } else if text[pos..body_end].starts_with(';') {
        kind = Kind::Field;
        (pos + 1, !private)
    } else if pos >= body_end {
        kind = Kind::Field;
        (body_end, !private)
    } else {
        return None;
    };
    Some((
        Member {
            name,
            offset: name_offset,
            start,
            end,
            kind,
            static_,
            safe,
        },
        end,
    ))
}

fn class_body(text: &str, class_at: usize) -> Option<(usize, usize)> {
    let open = text[class_at + "class".len()..].find('{')? + class_at + "class".len();
    let close = matching(text, open, '{', '}')?;
    let body = &text[open + 1..close];
    if body.len() > 16 * 1024 || body.matches('\n').count() > 200 {
        return None;
    }
    Some((open + 1, close))
}

fn member_removal(text: &str, member: &Member) -> Option<Range<usize>> {
    if !member.safe {
        return None;
    }
    let line_start = text[..member.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[member.end..]
        .find('\n')
        .map(|index| member.end + index)
        .unwrap_or(text.len());
    let own_line = text[line_start..member.start].trim().is_empty()
        && text[member.end..line_end].trim().is_empty();
    if own_line {
        let end = if text[line_end..].starts_with('\n') {
            line_end + 1
        } else {
            line_end
        };
        return Some(line_start..end);
    }
    let mut end = member.end;
    while text[end..].starts_with(' ') {
        end += 1;
    }
    Some(member.start..end)
}

/// Duplicate members within class declarations or single-line/multi-line
/// class expressions.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateMember> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("class") {
        let at = search + relative;
        let before = structural[..at].trim_end();
        let after = &structural[at + "class".len()..];
        let boundary = structural[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
        let valid_after = after
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == '{');
        let declaration = before.is_empty()
            || before.ends_with("export")
            || before.ends_with("default")
            || before.ends_with("abstract");
        let expression = before
            .chars()
            .next_back()
            .is_some_and(|ch| matches!(ch, '=' | '(' | '[' | ',' | ':' | '?' | '>'))
            || before.ends_with("return");
        if !boundary || !valid_after || (!declaration && !expression) {
            search = at + "class".len();
            continue;
        }
        let Some((body_start, body_end)) = class_body(&masked, at) else {
            search = at + "class".len();
            continue;
        };
        let mut seen = HashMap::<String, Member>::new();
        let mut pos = body_start;
        while pos < body_end {
            let Some((member, next)) = parse_member(&masked, body_end, pos) else {
                break;
            };
            if next <= pos {
                break;
            }
            pos = next;
            let key = format!("{}|{}|{}", member.static_, member.kind.key(), member.name);
            if let Some(first) = seen.get(&key) {
                found.push(DuplicateMember {
                    name: member.name.clone(),
                    offset: member.offset,
                    removal: member_removal(&masked, first),
                });
            } else {
                seen.insert(key, member);
            }
        }
        search = body_end + 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_methods_fields_and_accessors_remove_the_earlier_member() {
        let method = "class Reader { read() { return 1; } read() { return 2; } }\n";
        let found = duplicates(method);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = method.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class Reader { read() { return 2; } }\n");

        let field = "class A { value = 1; value = 2; }\n";
        let removal = duplicates(field)[0].removal.clone().unwrap();
        let mut edited = field.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class A { value = 2; }\n");

        let accessor = "class A { get size() { return 1; } get size() { return 2; } }\n";
        let removal = duplicates(accessor)[0].removal.clone().unwrap();
        let mut edited = accessor.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class A { get size() { return 2; } }\n");

        let exported = "export class A { read() {} read() {} }\n";
        assert!(duplicates(exported)[0].removal.is_some());
        let string_name = "class A { 'read'() {} read() {} }\n";
        assert_eq!(duplicates(string_name).len(), 1);

        let expression = "const A = class { read() {} read() {} };\n";
        let found = duplicates(expression);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = expression.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const A = class { read() {} };\n");

        let named = "const A = class Named { read() {} read() {} };\n";
        assert!(duplicates(named)[0].removal.is_some());
        let exported = "export const A = class { read() {} read() {} };\n";
        assert!(duplicates(exported)[0].removal.is_some());
        let parenthesized = "const A = (class { read() {} read() {} });\n";
        assert!(duplicates(parenthesized)[0].removal.is_some());
    }

    #[test]
    fn multiline_classes_and_expressions_remove_the_earlier_line() {
        let text = "class Reader {\n  read() { return 1; }\n  read() { return 2; }\n}\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class Reader {\n  read() { return 2; }\n}\n");

        let fields = "class A {\n  value = 1;\n  value = 2;\n}\n";
        let removal = duplicates(fields)[0].removal.clone().unwrap();
        let mut edited = fields.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class A {\n  value = 2;\n}\n");

        let expression = "const A = class {\n  read() {}\n  read() {}\n};\n";
        let removal = duplicates(expression)[0].removal.clone().unwrap();
        let mut edited = expression.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const A = class {\n  read() {}\n};\n");

        let shared = "class A {\n  read() {} read() {}\n}\n";
        assert_eq!(duplicates(shared).len(), 1);
        assert!(duplicates(shared)[0].removal.is_some());
    }

    #[test]
    fn pairs_static_instance_and_different_kinds_are_distinct() {
        assert!(duplicates("class A { get size() {} set size(v) {} }\n").is_empty());
        assert!(duplicates("class A { static read() {} read() {} }\n").is_empty());
        assert!(duplicates("class A { value = 1; value() {} }\n").is_empty());
        assert!(duplicates("class A { read() {} write() {} }\n").is_empty());
        assert!(duplicates("class A { async read() {} read() {} }\n").len() == 1);
        assert!(duplicates("class A { static read() {} static read() {} }\n").len() == 1);
        assert!(duplicates("class A {\n  get size() {}\n  set size(v) {}\n}\n").is_empty());
        assert!(duplicates("class A {\n  static read() {}\n  read() {}\n}\n").is_empty());
    }

    #[test]
    fn unsafe_duplicates_are_reported_without_removal() {
        let constructor = "class A { constructor() {} constructor() {} }\n";
        assert_eq!(duplicates(constructor).len(), 1);
        assert!(duplicates(constructor)[0].removal.is_none());

        let private = "class A { #value = 1; #value = 2; }\n";
        assert_eq!(duplicates(private).len(), 1);
        assert!(duplicates(private)[0].removal.is_none());

        let calls = "class A { value = compute(); value = 2; }\n";
        assert_eq!(duplicates(calls).len(), 1);
        assert!(duplicates(calls)[0].removal.is_none());

        let multiline_calls = "class A {\n  value = compute();\n  value = 2;\n}\n";
        assert_eq!(duplicates(multiline_calls).len(), 1);
        assert!(duplicates(multiline_calls)[0].removal.is_none());
    }

    #[test]
    fn blocks_strings_decorators_and_computed_names_are_bounded() {
        assert!(duplicates("if (ok) { read(); read(); }\n").is_empty());
        assert!(duplicates("const text = \"class { read() {} read() {} }\";\n").is_empty());
        assert!(duplicates("// class A { read() {} read() {} }\n").is_empty());
        assert!(duplicates("class A { static {} static {} }\n").is_empty());
        assert!(duplicates("class A { [read]() {} [read]() {} }\n").is_empty());
        assert!(duplicates("class A { @dec read() {} @dec read() {} }\n").is_empty());
        let arrow = "const fn = () => class { read() {} read() {} };\n";
        assert!(duplicates(arrow)[0].removal.is_some());
    }
}
