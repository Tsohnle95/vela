//! Bounded duplicate checks for TypeScript generic type parameters.
//!
//! Lists on `function`, `type`, `interface` and `class` declarations are
//! inspected. A safe quick fix removes the later duplicate only when both
//! entries are plain identifiers (no constraint or default), preserving all
//! existing name resolution.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateTypeParameter {
    pub name: String,
    pub offset: usize,
    pub first_line: usize,
    pub removal: Option<Range<usize>>,
}

#[derive(Clone, Debug)]
struct Parameter {
    name: String,
    start: usize,
    end: usize,
    comma: Option<usize>,
    plain: bool,
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (index, ch) in text[open..].char_indices() {
        match ch {
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

fn matching_angle(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (index, ch) in text[open..].char_indices() {
        let at = open + index;
        match ch {
            '<' => depth += 1,
            '>' => {
                if text[..at].ends_with('=') {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_params(text: &str, start: usize, end: usize) -> Vec<(usize, usize, Option<usize>)> {
    let structural = crate::object_keys::mask_strings(text);
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut segment = start;
    for (index, ch) in structural[start..end].char_indices() {
        let at = start + index;
        match ch {
            '<' => depth += 1,
            '>' => {
                if structural[..at].ends_with('=') {
                    continue;
                }
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => {
                spans.push((segment, at, Some(at)));
                segment = at + 1;
            }
            _ => {}
        }
    }
    spans.push((segment, end, None));
    spans
}

fn parameter_from(text: &str, start: usize, end: usize, comma: Option<usize>) -> Option<Parameter> {
    let trimmed = text[start..end].trim();
    let leading = text[start..end].len() - text[start..end].trim_start().len();
    if trimmed.is_empty() {
        return None;
    }
    let rest = trimmed.strip_prefix("const ").unwrap_or(trimmed);
    let name_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if !crate::rename::valid_identifier(&rest[..name_len]) {
        return None;
    }
    let plain = rest[name_len..].trim().is_empty();
    Some(Parameter {
        name: rest[..name_len].to_owned(),
        start: start + leading,
        end: start + text[start..end].trim_end().len(),
        comma,
        plain,
    })
}

fn parameter_removal(text: &str, parameter: &Parameter) -> Option<Range<usize>> {
    if !parameter.plain {
        return None;
    }
    let after = parameter
        .comma
        .map(|comma| comma + 1)
        .unwrap_or(parameter.end);
    let line_start = text[..parameter.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[after..]
        .find('\n')
        .map(|index| after + index)
        .unwrap_or(text.len());
    let own_line = text[line_start..parameter.start].trim().is_empty()
        && text[after..line_end].trim().is_empty();
    if own_line {
        let end = if text[line_end..].starts_with('\n') {
            line_end + 1
        } else {
            line_end
        };
        return Some(line_start..end);
    }
    if parameter.comma.is_some() {
        let mut end = after;
        while text[end..].starts_with(' ') {
            end += 1;
        }
        return Some(parameter.start..end);
    }
    let comma = text[..parameter.start].rfind(',')?;
    Some(comma..parameter.end)
}

fn type_parameter_list(text: &str, keyword_at: usize, keyword: &str) -> Option<(usize, usize)> {
    let after = &text[keyword_at + keyword.len()..];
    let with_star = after.strip_prefix('*').unwrap_or(after);
    let rest = with_star.trim_start();
    let name_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if name_len == 0 && keyword != "function" {
        return None;
    }
    let after_name = rest[name_len..].trim_start();
    if !after_name.starts_with('<') {
        return None;
    }
    let name_prefix = with_star.len() - rest.len();
    let open = keyword_at
        + keyword.len()
        + (after.len() - with_star.len())
        + name_prefix
        + name_len
        + (rest[name_len..].len() - after_name.len());
    let close = matching_angle(text, open)?;
    let body = &text[open + 1..close];
    if body.len() > 4 * 1024 || body.matches('\n').count() > 50 {
        return None;
    }
    Some((open + 1, close))
}

fn collect_list(masked: &str, start: usize, end: usize, found: &mut Vec<DuplicateTypeParameter>) {
    let mut parameters = Vec::new();
    for (part_start, part_end, comma) in split_params(masked, start, end) {
        if let Some(parameter) = parameter_from(masked, part_start, part_end, comma) {
            parameters.push(parameter);
        }
    }
    let mut seen = HashMap::<String, (usize, Parameter)>::new();
    for parameter in parameters {
        let line = masked[..parameter.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        if let Some((first_line, first)) = seen.get(&parameter.name) {
            let removal = (first.plain && parameter.plain)
                .then(|| parameter_removal(masked, &parameter))
                .flatten();
            found.push(DuplicateTypeParameter {
                name: parameter.name.clone(),
                offset: parameter.start,
                first_line: *first_line,
                removal,
            });
            continue;
        }
        seen.insert(parameter.name.clone(), (line, parameter));
    }
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

fn close_paren_before(text: &str, at: usize) -> Option<usize> {
    let mut braces = 0i32;
    let mut brackets = 0i32;
    for (index, ch) in text[..at].char_indices().rev() {
        match ch {
            '}' => braces += 1,
            '{' => braces -= 1,
            ']' => brackets += 1,
            '[' => brackets -= 1,
            ')' if braces == 0 && brackets == 0 => return Some(index),
            _ => {}
        }
    }
    None
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

fn matching_open_angle(text: &str, close: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (index, ch) in text[..=close].char_indices().rev() {
        match ch {
            '>' => {
                if text[..index].ends_with('=') {
                    continue;
                }
                depth += 1;
            }
            '<' => {
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

fn declaration_prefix(text: &str, open_angle: usize) -> bool {
    let before = text[..open_angle].trim_end();
    if before.ends_with("function") {
        return true;
    }
    let name_start = before
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, _)| index)
        .unwrap_or(before.len());
    if name_start == before.len() {
        return false;
    }
    let prior = text[..name_start].trim_end();
    ["function", "class", "interface", "type", "new"]
        .iter()
        .any(|keyword| prior.ends_with(keyword))
}

fn arrow_type_parameters(masked: &str, structural: &str) -> Vec<DuplicateTypeParameter> {
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("=>") {
        let at = search + relative;
        search = at + 2;
        let Some(close) = close_paren_before(structural, at) else {
            continue;
        };
        let Some(open) = matching_open_paren(structural, close) else {
            continue;
        };
        let Some((angle_close, '>')) = prev_non_ws(structural, open) else {
            continue;
        };
        let Some(angle_open) = matching_open_angle(structural, angle_close) else {
            continue;
        };
        let body = &structural[angle_open + 1..angle_close];
        if body.len() <= 4 * 1024 && body.matches('\n').count() <= 50 {
            collect_list(masked, angle_open + 1, angle_close, &mut found);
        }
    }
    found
}

fn method_type_parameters(masked: &str, structural: &str) -> Vec<DuplicateTypeParameter> {
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find('<') {
        let at = search + relative;
        search = at + 1;
        let Some((_, previous)) = prev_non_ws(structural, at) else {
            continue;
        };
        if !(previous.is_alphanumeric() || matches!(previous, '_' | '$'))
            || declaration_prefix(structural, at)
        {
            continue;
        }
        let Some(close) = matching_angle(structural, at) else {
            continue;
        };
        let Some(open) = structural[skip_ws(structural, close + 1, structural.len())..]
            .starts_with('(')
            .then(|| skip_ws(structural, close + 1, structural.len()))
        else {
            continue;
        };
        let Some(param_close) = matching_paren(structural, open) else {
            continue;
        };
        let after = skip_ws(structural, param_close + 1, structural.len());
        let body_ok = if structural[after..].starts_with('{') {
            true
        } else if structural[after..].starts_with(':') {
            let mut pos = skip_ws(structural, after + 1, structural.len());
            let limit = structural.len().min(pos + 1024);
            let mut found = false;
            while pos < limit {
                let ch = structural[pos..].chars().next().unwrap_or(' ');
                if ch == '{' {
                    found = true;
                    break;
                }
                if matches!(ch, ';' | ',') {
                    break;
                }
                pos += ch.len_utf8();
            }
            found
        } else {
            false
        };
        if !body_ok {
            continue;
        }
        let body = &structural[at + 1..close];
        if body.len() <= 4 * 1024 && body.matches('\n').count() <= 50 {
            collect_list(masked, at + 1, close, &mut found);
        }
        search = param_close + 1;
    }
    found
}

/// Duplicate generic type parameters on declarations.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateTypeParameter> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    for keyword in ["function", "type", "interface", "class"] {
        let mut search = 0;
        while let Some(relative) = structural[search..].find(keyword) {
            let at = search + relative;
            search = at + keyword.len();
            let boundary = structural[..at]
                .chars()
                .next_back()
                .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
            let after_ok = structural[at + keyword.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
            if !boundary || !after_ok {
                continue;
            }
            let Some((start, end)) = type_parameter_list(&structural, at, keyword) else {
                continue;
            };
            collect_list(&masked, start, end, &mut found);
            search = end + 1;
        }
    }
    found.extend(arrow_type_parameters(&masked, &structural));
    found.extend(method_type_parameters(&masked, &structural));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_duplicates_on_declarations_are_removed() {
        let function = "function f<T, T>(value: T) { return value; }\n";
        let found = duplicates(function);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "T");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = function.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "function f<T>(value: T) { return value; }\n");

        let alias = "type Pair<T, T> = [T, T];\n";
        assert!(duplicates(alias)[0].removal.is_some());
        let interface = "interface Box<T, T> { value: T }\n";
        assert!(duplicates(interface)[0].removal.is_some());
        let class = "class Box<T, T> { value!: T; }\n";
        assert!(duplicates(class)[0].removal.is_some());
        let exported = "export class Box<T, T> {}\n";
        assert!(duplicates(exported)[0].removal.is_some());
    }

    #[test]
    fn multiline_and_shared_line_removal_keep_syntax_valid() {
        let multiline = "function f<\n  T,\n  T,\n>(value: T) { return value; }\n";
        let removal = duplicates(multiline)[0].removal.clone().unwrap();
        let mut edited = multiline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "function f<\n  T,\n>(value: T) { return value; }\n");

        let shared = "function f<T, T, U>() {}\n";
        let removal = duplicates(shared)[0].removal.clone().unwrap();
        let mut edited = shared.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "function f<T, U>() {}\n");

        let last = "function f<U, T, T>() {}\n";
        let removal = duplicates(last)[0].removal.clone().unwrap();
        let mut edited = last.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "function f<U, T>() {}\n");
    }

    #[test]
    fn constraints_defaults_and_distinct_parameters_are_conservative() {
        let constraint = "function f<T extends string, T extends string>() {}\n";
        assert_eq!(duplicates(constraint).len(), 1);
        assert!(duplicates(constraint)[0].removal.is_none());

        let default = "type A<T = string, T = string> = T;\n";
        assert_eq!(duplicates(default).len(), 1);
        assert!(duplicates(default)[0].removal.is_none());

        let mixed = "type A<T extends object, T> = T;\n";
        assert!(duplicates(mixed)[0].removal.is_none());
        assert!(duplicates("function f<T, U>(a: T, b: U) {}\n").is_empty());
        assert!(duplicates("type A<T> = T;\n").is_empty());
        assert!(duplicates("const value = a < b > c;\n").is_empty());
    }

    #[test]
    fn nested_generics_comments_and_strings_are_bounded() {
        let nested =
            "function f<T extends Map<string, number>, T extends Map<string, number>>() {}\n";
        assert_eq!(duplicates(nested).len(), 1);
        assert!(duplicates(nested)[0].removal.is_none());

        let arrow_constraint = "function f<T, U extends (value: string) => void, T>() {}\n";
        assert_eq!(duplicates(arrow_constraint).len(), 1);

        assert!(duplicates("// function f<T, T>() {}\n").is_empty());
        assert!(duplicates("const text = \"function f<T, T>() {}\";\n").is_empty());
        assert!(duplicates("function f<T>(value: T) {}\n").is_empty());
        let arrow = "const f = <T, T>(value: T) => value;\n";
        let found = duplicates(arrow);
        assert_eq!(found.len(), 1);
        let removal = found[0].removal.clone().unwrap();
        let mut edited = arrow.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const f = <T>(value: T) => value;\n");
    }

    #[test]
    fn arrow_and_method_generics_are_checked() {
        let multiline = "const f = <\n  T,\n  T,\n>(value: T) => value;\n";
        let removal = duplicates(multiline)[0].removal.clone().unwrap();
        let mut edited = multiline.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "const f = <\n  T,\n>(value: T) => value;\n");

        let async_arrow = "const f = async <T, T>(value: T) => value;\n";
        assert_eq!(duplicates(async_arrow).len(), 1);

        let constrained = "const f = <T extends string, T extends string>(value: T) => value;\n";
        assert_eq!(duplicates(constrained).len(), 1);
        assert!(duplicates(constrained)[0].removal.is_none());

        let class_method = "class A { read<T, T>(value: T) { return value; } }\n";
        let found = duplicates(class_method);
        assert_eq!(found.len(), 1);
        let removal = found[0].removal.clone().unwrap();
        let mut edited = class_method.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "class A { read<T>(value: T) { return value; } }\n");

        let object_method = "const o = { read<T, T>(value: T) { return value; } };\n";
        assert!(duplicates(object_method)[0].removal.is_some());

        let static_method = "class A { static read<T, T>(value: T) { return value; } }\n";
        assert_eq!(duplicates(static_method).len(), 1);

        let returned = "class A { read<T, T>(value: T): T { return value; } }\n";
        assert_eq!(duplicates(returned).len(), 1);
        let object_return = "class A { read<T, T>(value: T): { x: T } { return { x: value }; } }\n";
        assert_eq!(duplicates(object_return).len(), 1);
        let object_method_return = "const o = { read<T, T>(value: T): T { return value; } };\n";
        assert_eq!(duplicates(object_method_return).len(), 1);

        let anonymous = "const f = function <T, T>(value: T) { return value; };\n";
        let found = duplicates(anonymous);
        assert_eq!(found.len(), 1);
        let removal = found[0].removal.clone().unwrap();
        let mut edited = anonymous.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "const f = function <T>(value: T) { return value; };\n"
        );

        assert!(duplicates("class A { read<T, U>(value: T) { return value; } }\n").is_empty());
        assert!(duplicates("function f<T, T>(value: T) { return value; }\n").len() == 1);
    }
}
