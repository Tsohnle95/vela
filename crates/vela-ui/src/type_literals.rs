//! Bounded duplicate-property checks for inline object type literals.
//!
//! Only `type Name = { ... }` aliases are inspected. Members use the shared
//! type-member grammar; method overloads stay legal and only identical duplicate
//! properties are auto-removable.

use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateTypeLiteral {
    pub name: String,
    /// Byte offset of the later property name.
    pub offset: usize,
    pub first_line: usize,
    pub removal: Option<Range<usize>>,
}

fn matching(text: &str, open: usize) -> Option<usize> {
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

/// Duplicate property signatures in `type Name = { ... }` aliases.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateTypeLiteral> {
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
        let rhs_start = structural[equals + 1..]
            .find(|ch: char| !ch.is_whitespace())
            .map(|index| equals + 1 + index)
            .unwrap_or(structural.len());
        if !structural[rhs_start..].starts_with('{') {
            search = at + "type".len();
            continue;
        }
        let Some(close) = matching(&structural, rhs_start) else {
            search = at + "type".len();
            continue;
        };
        let body = &structural[rhs_start + 1..close];
        if body.len() <= 8 * 1024 && body.matches('\n').count() <= 200 {
            found.extend(
                crate::type_members::duplicates_in(&masked, rhs_start + 1, close)
                    .into_iter()
                    .map(|property| DuplicateTypeLiteral {
                        name: property.name,
                        offset: property.offset,
                        first_line: property.first_line,
                        removal: property.removal,
                    }),
            );
        }
        search = at + "type".len();
    }
    found.extend(annotation_properties(&masked, &structural));
    found.extend(signature_properties(&masked, &structural));
    found
}

fn annotation_properties(masked: &str, structural: &str) -> Vec<DuplicateTypeLiteral> {
    let mut found = Vec::new();
    for keyword in ["const", "let", "var"] {
        let mut search = 0;
        while let Some(relative) = structural[search..].find(keyword) {
            let at = search + relative;
            search = at + keyword.len();
            let before_ok = structural[..at]
                .chars()
                .next_back()
                .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
            let after = &structural[at + keyword.len()..];
            if !before_ok || !after.chars().next().is_some_and(char::is_whitespace) {
                continue;
            }
            let rest = after.trim_start();
            let name_len = rest
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            if name_len == 0 || !crate::rename::valid_identifier(&rest[..name_len]) {
                continue;
            }
            let mut cursor = name_len;
            while rest[cursor..].starts_with(' ') {
                cursor += 1;
            }
            if !rest[cursor..].starts_with(':') {
                continue;
            }
            cursor += 1;
            while rest[cursor..].starts_with(' ') || rest[cursor..].starts_with('\n') {
                cursor += 1;
            }
            if !rest[cursor..].starts_with('{') {
                continue;
            }
            let rest_offset = at + keyword.len() + (after.len() - rest.len());
            let open = rest_offset + cursor;
            let Some(close) = matching(structural, open) else {
                continue;
            };
            let tail = structural[close + 1..].trim_start();
            if !(tail.starts_with('=') || tail.starts_with(';') || tail.starts_with(',')) {
                continue;
            }
            let body = &structural[open + 1..close];
            if body.len() <= 8 * 1024 && body.matches('\n').count() <= 200 {
                found.extend(
                    crate::type_members::duplicates_in(masked, open + 1, close)
                        .into_iter()
                        .map(|property| DuplicateTypeLiteral {
                            name: property.name,
                            offset: property.offset,
                            first_line: property.first_line,
                            removal: property.removal,
                        }),
                );
            }
            search = close + 1;
        }
    }
    found
}

fn close_paren_before(text: &str, at: usize) -> Option<usize> {
    let mut braces = 0i32;
    let mut brackets = 0i32;
    let mut angles = 0i32;
    for (index, ch) in text[..at].char_indices().rev() {
        match ch {
            '}' => braces += 1,
            '{' => braces -= 1,
            ']' => brackets += 1,
            '[' => brackets -= 1,
            '>' => angles += 1,
            '<' => angles -= 1,
            ')' if braces == 0 && brackets == 0 && angles == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn prev_non_ws(text: &str, mut pos: usize) -> Option<(usize, char)> {
    while pos > 0 {
        let (index, ch) = text[..pos].char_indices().next_back()?;
        if !ch.is_whitespace() {
            return Some((index, ch));
        }
        pos = index;
    }
    None
}

fn skip_ws(text: &str, mut pos: usize, end: usize) -> usize {
    while pos < end
        && text[pos..end]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        pos += text[pos..end]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    pos
}

fn matching_open_paren(text: &str, close: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (index, ch) in text[..=close].char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
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
            '(' => depth += 1,
            ')' => {
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

fn collect_body(
    masked: &str,
    structural: &str,
    open: usize,
    found: &mut Vec<DuplicateTypeLiteral>,
) -> Option<usize> {
    let close = matching(structural, open)?;
    let body = &structural[open + 1..close];
    if body.len() <= 8 * 1024 && body.matches('\n').count() <= 200 {
        found.extend(
            crate::type_members::duplicates_in(masked, open + 1, close)
                .into_iter()
                .map(|property| DuplicateTypeLiteral {
                    name: property.name,
                    offset: property.offset,
                    first_line: property.first_line,
                    removal: property.removal,
                }),
        );
    }
    Some(close)
}

fn scan_params(
    masked: &str,
    structural: &str,
    start: usize,
    end: usize,
    found: &mut Vec<DuplicateTypeLiteral>,
) {
    let mut pos = start;
    let mut depth = 0i32;
    let mut angle = 0i32;
    let mut segment_has_default = false;
    while pos < end {
        let ch = structural[pos..end].chars().next().unwrap_or(' ');
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '<' => angle += 1,
            '>' if angle > 0 => angle -= 1,
            ',' if depth == 0 && angle == 0 => segment_has_default = false,
            '=' if depth == 0 && angle == 0 => segment_has_default = true,
            ':' if depth == 0 && angle == 0 && !segment_has_default => {
                if let Some((_, previous)) = prev_non_ws(structural, pos)
                    && (previous.is_alphanumeric() || matches!(previous, '_' | '$' | '?'))
                {
                    let next = skip_ws(structural, pos + 1, end);
                    if structural[next..end].starts_with('{')
                        && let Some(close) = collect_body(masked, structural, next, found)
                    {
                        pos = close + 1;
                        continue;
                    }
                }
            }
            _ => {}
        }
        pos += ch.len_utf8();
    }
}

/// Duplicate properties in function/arrow parameter and return type literals.
fn signature_properties(masked: &str, structural: &str) -> Vec<DuplicateTypeLiteral> {
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("function") {
        let at = search + relative;
        search = at + "function".len();
        let boundary = structural[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
        if !boundary {
            continue;
        }
        let Some(open) = structural[search..].find('(').map(|index| search + index) else {
            continue;
        };
        let Some(close) = matching_paren(structural, open) else {
            continue;
        };
        scan_params(masked, structural, open + 1, close, &mut found);
        let mut pos = skip_ws(structural, close + 1, structural.len());
        if structural[pos..].starts_with(':') {
            pos = skip_ws(structural, pos + 1, structural.len());
            if structural[pos..].starts_with('{') {
                collect_body(masked, structural, pos, &mut found);
            }
        }
        search = close + 1;
    }
    search = 0;
    while let Some(relative) = structural[search..].find("=>") {
        let at = search + relative;
        search = at + 2;
        let Some(close) = close_paren_before(structural, at) else {
            continue;
        };
        let Some(open) = matching_open_paren(structural, close) else {
            continue;
        };
        scan_params(masked, structural, open + 1, close, &mut found);
        let mut pos = skip_ws(structural, close + 1, at);
        if structural[pos..at].starts_with(':') {
            pos = skip_ws(structural, pos + 1, at);
            if structural[pos..at].starts_with('{') {
                collect_body(masked, structural, pos, &mut found);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_properties_are_flagged_and_removed() {
        let text = "type Options = {\n  read: string;\n  write: string;\n  read: string;\n};\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        assert_eq!(found[0].first_line, 2);
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "type Options = {\n  read: string;\n  write: string;\n};\n"
        );

        let inline = "type A = { read: string; write: string; read: string };\n";
        let removal = duplicates(inline)[0].removal.clone().unwrap();
        let mut edited = inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "type A = { read: string; write: string };\n");

        let exported = "export type A = { read: string; read: string };\n";
        assert!(duplicates(exported)[0].removal.is_some());
        let quoted = "type A = { \"read\": string; read: string };\n";
        assert!(duplicates(quoted)[0].removal.is_some());
    }

    #[test]
    fn differing_types_modifiers_and_property_method_conflicts_have_no_action() {
        let differing = "type A = { read: string; read: number };\n";
        assert_eq!(duplicates(differing).len(), 1);
        assert!(duplicates(differing)[0].removal.is_none());

        let readonly = "type A = { readonly read: string; read: string };\n";
        assert_eq!(duplicates(readonly).len(), 1);
        assert!(duplicates(readonly)[0].removal.is_none());

        let optional = "type A = { read?: string; read: string };\n";
        assert_eq!(duplicates(optional).len(), 1);
        assert!(duplicates(optional)[0].removal.is_none());

        let conflict = "type A = { read: string; read(): void };\n";
        assert_eq!(duplicates(conflict).len(), 1);
        assert!(duplicates(conflict)[0].removal.is_none());
    }

    #[test]
    fn overloads_nested_types_and_distinct_names_are_bounded() {
        assert!(duplicates("type A = { read(): void; read(x: string): void };\n").is_empty());
        assert!(duplicates("type A = { read: string; write: number };\n").is_empty());
        assert!(duplicates("type A = { [key: string]: string };\n").is_empty());
        assert!(duplicates("type A = { new (): A };\n").is_empty());

        let nested = "type A = { read: { x: string }; read: { x: string } };\n";
        assert_eq!(duplicates(nested).len(), 1);
        assert!(duplicates(nested)[0].removal.is_some());
    }

    #[test]
    fn annotation_object_types_are_checked_in_declarations() {
        let inline = "const options: { read: string; read: string } = value;\n";
        let found = duplicates(inline);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = inline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const options: { read: string } = value;\n");

        let multiline =
            "const options: {\n  read: string;\n  write: string;\n  read: string;\n} = value;\n";
        let removal = duplicates(multiline)[0].removal.clone().unwrap();
        let mut edited = multiline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "const options: {\n  read: string;\n  write: string;\n} = value;\n"
        );

        let exported = "export const options: { read: string; read: string } = value;\n";
        assert!(duplicates(exported)[0].removal.is_some());

        let differing = "let options: { read: string; read: number } = value;\n";
        assert_eq!(duplicates(differing).len(), 1);
        assert!(duplicates(differing)[0].removal.is_none());

        assert!(duplicates("const plain = { read: 1, read: 2 };\n").is_empty());
        assert!(duplicates("const o: A = { read: 1, read: 2 };\n").is_empty());
        assert!(duplicates("const o: string = cond ? a : { read: 1, read: 2 };\n").is_empty());
        assert!(duplicates("const { a }: T = value;\n").is_empty());
        assert!(duplicates("for (const x: { a: string; a: string } of list) {}\n").is_empty());
    }

    #[test]
    fn nested_object_types_are_checked_recursively() {
        let nested = "type A = { outer: { read: string; read: string } };\n";
        let found = duplicates(nested);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = nested.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "type A = { outer: { read: string } };\n");

        let deep = "type A = { outer: { middle: { read: string; read: string } } };\n";
        assert_eq!(duplicates(deep).len(), 1);

        let annotation = "const options: { outer: { read: string; read: string } } = value;\n";
        assert_eq!(duplicates(annotation).len(), 1);

        let multiline = "type A = {\n  outer: {\n    read: string;\n    read: string;\n  };\n};\n";
        let removal = duplicates(multiline)[0].removal.clone().unwrap();
        let mut edited = multiline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "type A = {\n  outer: {\n    read: string;\n  };\n};\n"
        );

        let differing = "type A = { outer: { read: string; read: number } };\n";
        assert_eq!(duplicates(differing).len(), 1);
        assert!(duplicates(differing)[0].removal.is_none());

        let unique = "type A = { outer: { read: string; write: string } };\n";
        assert!(duplicates(unique).is_empty());
    }

    #[test]
    fn function_and_arrow_signature_types_are_checked() {
        let parameter = "function f(options: { read: string; read: string }) {}\n";
        let found = duplicates(parameter);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = parameter.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "function f(options: { read: string }) {}\n");

        let returned =
            "function f(): { read: string; write: string; read: string } { return value; }\n";
        let removal = duplicates(returned)[0].removal.clone().unwrap();
        let mut edited = returned.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "function f(): { read: string; write: string } { return value; }\n"
        );

        let arrow = "const f = (options: { read: string; read: string }) => options;\n";
        assert!(duplicates(arrow)[0].removal.is_some());
        let arrow_return =
            "const f = (value: string): { read: string; read: string } => value as never;\n";
        assert_eq!(duplicates(arrow_return).len(), 1);
        let expression =
            "const f = function (options: { read: string; read: string }) { return options; };\n";
        assert!(duplicates(expression)[0].removal.is_some());
    }

    #[test]
    fn signature_defaults_ternaries_and_destructuring_are_excluded() {
        assert!(duplicates("function f(x = { read: 1, read: 2 }) {}\n").is_empty());
        assert!(duplicates("function f(x = cond ? a : { read: 1, read: 2 }) {}\n").is_empty());
        assert!(duplicates("function f({ read }: { read: string; read: string }) {}\n").is_empty());
        assert!(
            duplicates("function f(x: { read: string; read: number }) {}\n")[0]
                .removal
                .is_none()
        );
        assert!(duplicates("function f(x: { read: string; write: string }) {}\n").is_empty());
        assert!(duplicates("function f(x: string) {}\n").is_empty());
    }

    #[test]
    fn non_alias_objects_comments_strings_and_other_braces_are_excluded() {
        assert!(duplicates("const value = { read: 1, read: 2 };\n").is_empty());
        assert!(duplicates("interface A { read: string; read: string }\n").is_empty());
        assert!(duplicates("type Fn<U> = { read: U; read: U };\n").is_empty());
        assert!(duplicates("// type A = { read: string; read: string };\n").is_empty());
        assert!(
            duplicates("const text = \"type A = { read: string; read: string }\";\n").is_empty()
        );
        assert!(duplicates("type A = string;\n").is_empty());
        assert!(duplicates("type A = ({ read: string; read: string });\n").is_empty());
    }
}
