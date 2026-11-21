//! Bounded excess/missing property checks for object literals.
//!
//! Source's TypeScript worker reports an excess-property error when a literal
//! assigned to an annotated binding names a property the local type does not
//! declare, and a missing-property error when a required property is absent.
//! This provider reproduces those two checks for object types declared in the
//! same document (`interface Name { … }` and `type Name = { … }`). Generic,
//! extending and index-signature types are skipped, as are literals that spread
//! another value, so the provider never guesses.

use std::collections::HashMap;

const DIAGNOSTIC_LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObjectTypeKind {
    UnknownProperty,
    MissingProperty,
}

#[derive(Debug)]
pub(super) struct ObjectTypeDiagnostic {
    pub offset: usize,
    pub kind: ObjectTypeKind,
    pub message: String,
}

struct TypeShape {
    properties: Vec<(String, bool)>,
    open: bool,
    extends: bool,
    generic: bool,
}

pub(super) fn diagnostics(text: &str) -> Vec<ObjectTypeDiagnostic> {
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    if masked.len() != text.len() {
        return Vec::new();
    }
    let types = local_types(text, &masked);
    if types.is_empty() {
        return Vec::new();
    }
    let bytes = masked.as_bytes();
    let mut found = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'=' {
            index += 1;
            continue;
        }
        if matches!(bytes.get(index + 1), Some(b'>') | Some(b'='))
            || matches!(
                index.checked_sub(1).and_then(|previous| bytes.get(previous)),
                Some(b'=') | Some(b'!') | Some(b'<') | Some(b'>')
            )
        {
            index += 1;
            continue;
        }
        let Some((name, name_offset, open, close)) = annotated_literal(text, &masked, &types, index)
        else {
            index += 1;
            continue;
        };
        let shape = &types[&name];
        index = close + 1;
        if shape.open || shape.extends || shape.generic {
            continue;
        }
        let literal = parse_literal(text, &masked, open, close);
        if literal.spread || literal.computed {
            continue;
        }
        for (key, offset) in &literal.keys {
            if !shape
                .properties
                .iter()
                .any(|(property, _)| property == key)
            {
                found.push(ObjectTypeDiagnostic {
                    offset: *offset,
                    kind: ObjectTypeKind::UnknownProperty,
                    message: format!(
                        "Object literal may only specify known properties, and '{key}' does not exist in type '{name}'."
                    ),
                });
            }
        }
        for (property, optional) in &shape.properties {
            if *optional {
                continue;
            }
            if !literal.keys.iter().any(|(key, _)| key == property) {
                found.push(ObjectTypeDiagnostic {
                    offset: name_offset,
                    kind: ObjectTypeKind::MissingProperty,
                    message: format!("Property '{property}' is missing in type '{name}'."),
                });
            }
        }
        if found.len() >= DIAGNOSTIC_LIMIT {
            break;
        }
    }
    found.truncate(DIAGNOSTIC_LIMIT);
    found
}

fn local_types(text: &str, masked: &str) -> HashMap<String, TypeShape> {
    let bytes = masked.as_bytes();
    let mut types = HashMap::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if !masked.is_char_boundary(index) || !at_word_start(bytes, index) {
            index += 1;
            continue;
        }
        if let Some(rest) = masked[index..].strip_prefix("interface")
            && rest.starts_with([' ', '\t', '\n'])
        {
            let name_start = index + "interface".len();
            let (name, after) = read_word(masked, name_start);
            if !name.is_empty()
                && let Some((open, close)) = type_body(masked, after)
            {
                let generic = masked[after..open].contains('<');
                let extends = masked[after..open].contains("extends");
                let mut shape = parse_shape(text, masked, open, close);
                shape.generic = generic;
                shape.extends = extends;
                types.insert(name, shape);
            }
            index = after;
            continue;
        }
        if let Some(rest) = masked[index..].strip_prefix("type")
            && rest.starts_with([' ', '\t', '\n'])
        {
            let name_start = index + "type".len();
            let (name, after) = read_word(masked, name_start);
            if !name.is_empty()
                && let Some(equals) = alias_equals(masked, after)
                && let Some((open, close)) = type_body(masked, equals + 1)
            {
                let generic = masked[after..open].contains('<');
                let mut shape = parse_shape(text, masked, open, close);
                shape.generic = generic;
                types.insert(name, shape);
            }
            index = after;
            continue;
        }
        index += 1;
    }
    types
}

/// An alias must be `type Name = …`; an imported `type Name` inside braces is
/// not an alias and must not capture a later body.
fn alias_equals(masked: &str, from: usize) -> Option<usize> {
    let bytes = masked.as_bytes();
    let mut index = from;
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
        index += 1;
    }
    if bytes.get(index) == Some(&b'<') {
        let mut depth = 0usize;
        while index < bytes.len() {
            match bytes[index] {
                b'<' => depth += 1,
                b'>' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                b';' | b'}' => return None,
                _ => {}
            }
            index += 1;
        }
        while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
            index += 1;
        }
    }
    (bytes.get(index) == Some(&b'=')).then_some(index)
}

fn parse_shape(text: &str, masked: &str, open: usize, close: usize) -> TypeShape {
    let bytes = masked.as_bytes();
    let mut properties = Vec::new();
    let mut open_index = false;
    let mut depth = 0usize;
    let mut segment = open + 1;
    let mut index = open + 1;
    while index <= close {
        let ch = *bytes.get(index).unwrap_or(&b'}');
        match ch {
            b'{' | b'[' | b'(' | b'<' => depth += 1,
            b'}' | b']' | b')' | b'>' => depth = depth.saturating_sub(1),
            b';' | b',' | b'\n' if depth == 0 => {
                collect_property(text, masked, segment, index, &mut properties, &mut open_index);
                segment = index + 1;
            }
            _ => {}
        }
        if index == close {
            if depth == 0 {
                collect_property(text, masked, segment, close, &mut properties, &mut open_index);
            }
            break;
        }
        index += 1;
    }
    TypeShape {
        properties,
        open: open_index,
        extends: false,
        generic: false,
    }
}

fn collect_property(
    text: &str,
    masked: &str,
    start: usize,
    end: usize,
    properties: &mut Vec<(String, bool)>,
    open_index: &mut bool,
) {
    let segment = masked[start..end].trim();
    if segment.is_empty() {
        return;
    }
    if segment.starts_with('[') {
        *open_index = true;
        return;
    }
    // Skip method overloads, generics and anything that is not a plain member.
    let lead = masked[start..end]
        .find(|ch: char| !ch.is_whitespace())
        .map(|offset| start + offset)
        .unwrap_or(start);
    let (name, after) = read_member_name(text, masked, lead, end);
    if name.is_empty() {
        return;
    }
    let rest = masked[after..end].trim_start();
    let optional = rest.starts_with('?');
    if rest.starts_with([':', '?', '(']) {
        properties.push((name, optional));
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
    read_word(masked, start)
}

fn read_word(text: &str, start: usize) -> (String, usize) {
    let bytes = text.as_bytes();
    let mut index = start;
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
        index += 1;
    }
    let word_start = index;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'$'))
    {
        index += 1;
    }
    (text[word_start..index].to_owned(), index)
}

/// Finds the `Name = { … }` object type opening after `=`.
fn type_body(masked: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = masked.as_bytes();
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                let close = matching_brace(bytes, index)?;
                return Some((index, close));
            }
            b';' | b'}' | b')' => return None,
            _ => index += 1,
        }
    }
    None
}

struct Literal {
    keys: Vec<(String, usize)>,
    spread: bool,
    computed: bool,
}

fn parse_literal(text: &str, masked: &str, open: usize, close: usize) -> Literal {
    let bytes = masked.as_bytes();
    let mut keys = Vec::new();
    let mut spread = false;
    let mut computed = false;
    let mut depth = 0usize;
    let mut segment = open + 1;
    let mut index = open + 1;
    while index <= close {
        let ch = *bytes.get(index).unwrap_or(&b'}');
        match ch {
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                collect_key(text, masked, segment, index, &mut keys, &mut spread, &mut computed);
                segment = index + 1;
            }
            _ => {}
        }
        if index == close {
            collect_key(text, masked, segment, close, &mut keys, &mut spread, &mut computed);
            break;
        }
        index += 1;
    }
    Literal {
        keys,
        spread,
        computed,
    }
}

fn collect_key(
    text: &str,
    masked: &str,
    start: usize,
    end: usize,
    keys: &mut Vec<(String, usize)>,
    spread: &mut bool,
    computed: &mut bool,
) {
    let segment = masked[start..end].trim();
    if segment.is_empty() {
        return;
    }
    let lead = masked[start..end]
        .find(|ch: char| !ch.is_whitespace())
        .map(|offset| start + offset)
        .unwrap_or(start);
    if segment.starts_with("...") {
        *spread = true;
        return;
    }
    if segment.starts_with('[') {
        *computed = true;
        return;
    }
    if segment.starts_with(['(', '<']) {
        return;
    }
    let (name, after) = read_member_name(text, masked, lead, end);
    if name.is_empty() {
        return;
    }
    // A real property ends at `:`, a method call, a shorthand terminator or the
    // segment end. Anything else is a fragment of a value that contained a
    // top-level comma (for example `new Map<string, FileBaseline>()`).
    let next = masked[after..end]
        .chars()
        .find(|ch| !ch.is_whitespace());
    if matches!(next, Some(':') | Some('(') | Some(',') | Some('}') | Some('?') | Some('=') | None)
    {
        keys.push((name, lead));
    }
}

/// Validates `= {` after an annotation that names a local object type.
fn annotated_literal(
    text: &str,
    masked: &str,
    types: &HashMap<String, TypeShape>,
    equals: usize,
) -> Option<(String, usize, usize, usize)> {
    let bytes = masked.as_bytes();
    let mut index = equals;
    while index > 0 && matches!(bytes[index - 1], b' ' | b'\t' | b'\n' | b'\r') {
        index -= 1;
    }
    let name_end = index;
    while index > 0
        && (bytes[index - 1].is_ascii_alphanumeric() || matches!(bytes[index - 1], b'_' | b'$'))
    {
        index -= 1;
    }
    if index == name_end {
        return None;
    }
    let name = text[index..name_end].to_owned();
    if !types.contains_key(&name) {
        return None;
    }
    let name_offset = index;
    let mut before = index;
    while before > 0 && matches!(bytes[before - 1], b' ' | b'\t' | b'\n' | b'\r') {
        before -= 1;
    }
    if before == 0 || bytes[before - 1] != b':' {
        return None;
    }
    let mut after = equals + 1;
    while after < bytes.len() && matches!(bytes[after], b' ' | b'\t' | b'\n' | b'\r') {
        after += 1;
    }
    // `=>` or a destructured pattern never reaches here.
    if bytes.get(after) != Some(&b'{') {
        return None;
    }
    let close = matching_brace(bytes, after)?;
    Some((name, name_offset, after, close))
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

    fn messages(text: &str) -> Vec<String> {
        diagnostics(text)
            .into_iter()
            .map(|found| found.message)
            .collect()
    }

    #[test]
    fn excess_and_missing_properties_follow_local_interfaces() {
        let excess = messages(
            "interface Point {\n  x: number;\n  y: number;\n}\nconst p: Point = { x: 1, y: 2, z: 3 };\n",
        );
        assert_eq!(
            excess,
            vec![
                "Object literal may only specify known properties, and 'z' does not exist in type 'Point'."
                    .to_string()
            ]
        );

        let missing = messages("interface Point {\n  x: number;\n  y: number;\n}\nconst p: Point = { x: 1 };\n");
        assert_eq!(
            missing,
            vec!["Property 'y' is missing in type 'Point'.".to_string()]
        );

        let alias = messages("type Point = { x: number; y?: number };\nconst p: Point = { x: 1 };\n");
        assert!(alias.is_empty(), "{alias:?}");
    }

    #[test]
    fn optional_open_extension_and_generic_types_are_bounded() {
        assert!(
            messages("interface P { x: number; y?: number }\nconst p: P = { x: 1 };\n").is_empty()
        );
        assert!(
            messages("interface P { [key: string]: number }\nconst p: P = { a: 1 };\n").is_empty()
        );
        assert!(
            messages("interface Base { x: number }\ninterface P extends Base { y: number }\nconst p: P = { y: 1 };\n")
                .is_empty()
        );
        assert!(
            messages("interface P<T> { x: T }\nconst p: P<number> = { y: 1 };\n").is_empty()
        );
    }

    #[test]
    fn spread_computed_nested_and_unknown_types_are_bounded() {
        assert!(
            messages("interface P { x: number }\nconst extra = { y: 1 };\nconst p: P = { x: 1, ...extra };\n")
                .is_empty(),
            "spread literals are never reported"
        );
        assert!(
            messages("interface P { x: number }\nconst key = 'x';\nconst p: P = { [key]: 1 };\n")
                .is_empty()
        );
        assert!(
            messages("interface P { inner: { a: number } }\nconst p: P = { inner: { a: 1, b: 2 } };\n")
                .is_empty(),
            "only top-level keys are compared"
        );
        assert!(messages("const p: Unknown = { z: 1 };\n").is_empty());
        assert!(
            messages(
                "interface P { m: Map<string, number> }\nconst p: P = { m: new Map<string, number>() };\n"
            )
            .is_empty(),
            "commas inside generic arguments never invent keys"
        );
        assert!(messages("const plain = { z: 1 };\n").is_empty());
        assert!(
            messages("interface P { x: number }\nconst fn = () => ({ x: 1 } as P);\n").is_empty()
        );
    }

    #[test]
    fn methods_quoted_keys_shorthand_and_multiple_bindings_work() {
        let text = "interface Handler { run: () => void; 'on-click': () => void; label?: string }\nconst h: Handler = { run() {}, 'on-click': () => {}, label };\n";
        let found = messages(text);
        assert!(found.is_empty(), "{found:?}");

        let two = messages(
            "interface A { x: number }\ninterface B { y: number }\nconst a: A = { x: 1 };\nconst b: B = { y: 2, z: 3 };\n",
        );
        assert_eq!(
            two,
            vec![
                "Object literal may only specify known properties, and 'z' does not exist in type 'B'."
                    .to_string()
            ]
        );
    }

    #[test]
    fn offsets_point_at_the_offending_key() {
        let text = "interface P { x: number }\nconst p: P = { y: 1 };\n";
        let found = diagnostics(text);
        // The literal names an undeclared `y` and omits the required `x`.
        assert_eq!(found.len(), 2, "{found:?}");
        let unknown = found
            .iter()
            .find(|found| found.kind == ObjectTypeKind::UnknownProperty)
            .expect("excess property");
        assert_eq!(&text[unknown.offset..unknown.offset + 1], "y");
        let missing = found
            .iter()
            .find(|found| found.kind == ObjectTypeKind::MissingProperty)
            .expect("missing property");
        assert!(text[missing.offset..].starts_with('P'), "{missing:?}");
    }
}


