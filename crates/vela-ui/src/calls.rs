//! Bounded function-call checks for locally declared functions: argument
//! counts, plus literal argument kinds against primitive parameter annotations.
//!
//! Only top-level function declarations with a one-line parameter list are
//! indexed. Calls are matched in comment-masked source so nested commas and
//! string contents do not affect the count.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct CallSite {
    pub name: String,
    pub name_offset: usize,
    pub open: usize,
    pub close: usize,
    pub arguments: Vec<Range<usize>>,
    pub params: Vec<Param>,
    pub min: usize,
    pub max: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Param {
    /// Primitive annotation when the parameter declares one.
    pub annotation: Option<String>,
    pub rest: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CallIssue {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Clone)]
struct Spec {
    min: usize,
    max: Option<usize>,
    declaration: usize,
    params: Vec<Param>,
}

fn split_parameters(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
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
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if depth > 0 => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn function_spec(text: &str, line: &str, line_start: usize) -> Option<(String, Spec)> {
    let mut trimmed = line.trim_start();
    if trimmed.len() != line.len() {
        return None;
    }
    let mut base = 0;
    loop {
        let stripped = ["export ", "default ", "async "]
            .into_iter()
            .find_map(|prefix| trimmed.strip_prefix(prefix).map(|tail| (prefix, tail)));
        let Some((prefix, tail)) = stripped else {
            break;
        };
        base += prefix.len();
        trimmed = tail;
    }
    let after = trimmed.strip_prefix("function")?;
    if after.starts_with('*') || !after.contains('{') {
        return None;
    }
    let after = after.trim_start();
    let name_len = after
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &after[..name_len];
    if !crate::rename::valid_identifier(name) {
        return None;
    }
    let name_offset = line_start + base + trimmed.len() - after.len();
    let open = line[name_offset - line_start + name_len..].find('(')? + name_offset - line_start
        + name_len;
    let close = line[open + 1..].find(')')? + open + 1;
    let mut required = 0;
    let mut total = 0;
    let mut rest = false;
    let mut params = Vec::new();
    for parameter in split_parameters(&line[open + 1..close]) {
        let parameter = parameter.trim();
        if parameter.is_empty() {
            continue;
        }
        total += 1;
        let is_rest = parameter.starts_with("...");
        if is_rest {
            rest = true;
        } else if !parameter.contains('?') && !parameter.contains('=') {
            required += 1;
        }
        params.push(Param {
            annotation: parameter_annotation(parameter, is_rest),
            rest: is_rest,
        });
    }
    let _ = text;
    Some((
        name.to_owned(),
        Spec {
            min: required,
            max: (!rest).then_some(total),
            declaration: name_offset,
            params,
        },
    ))
}

/// Only primitive annotations are comparable against literal arguments; named,
/// literal, array, generic and function types stay unreported.
fn parameter_annotation(parameter: &str, rest: bool) -> Option<String> {
    if rest {
        return None;
    }
    let parameter = parameter.trim_start_matches("...");
    let (head, tail) = parameter.split_once(':')?;
    if head.contains(['{', '[']) || tail.is_empty() {
        return None;
    }
    let annotation = tail.split('=').next().unwrap_or(tail).trim();
    if annotation.is_empty() {
        return None;
    }
    let primitive = annotation.split('|').all(|part| {
        matches!(
            part.trim(),
            "string" | "number" | "boolean" | "null" | "any" | "unknown"
        )
    });
    primitive.then(|| annotation.to_owned())
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in text[open..].char_indices() {
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
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn arguments(text: &str, open: usize, close: usize) -> Vec<Range<usize>> {
    let inner = &text[open + 1..close];
    if inner.trim().is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
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
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                let start_offset = open + 1 + start;
                let end_offset = open + 1 + index;
                let trimmed = inner[start..index].trim();
                if !trimmed.is_empty() {
                    let leading =
                        inner[start..index].len() - inner[start..index].trim_start().len();
                    ranges.push(start_offset + leading..start_offset + leading + trimmed.len());
                }
                start = index + 1;
                let _ = end_offset;
            }
            _ => {}
        }
    }
    let tail = inner[start..].trim();
    if !tail.is_empty() {
        let raw = &inner[start..];
        let leading = raw.len() - raw.trim_start().len();
        ranges.push(open + 1 + start + leading..open + 1 + start + leading + tail.len());
    }
    ranges
}

fn specs(text: &str) -> HashMap<String, Spec> {
    let masked = crate::semantic::mask_comments(text);
    let mut specs = HashMap::new();
    let mut duplicates = HashSet::new();
    let mut line_start = 0;
    for line in masked.split('\n') {
        if let Some((name, spec)) = function_spec(&masked, line, line_start)
            && specs.insert(name.clone(), spec).is_some()
        {
            duplicates.insert(name);
        }
        line_start += line.len() + 1;
    }
    for name in duplicates {
        specs.remove(&name);
    }
    specs
}

pub(super) fn calls(text: &str) -> Vec<CallSite> {
    let specs = specs(text);
    if specs.is_empty() {
        return Vec::new();
    }
    let masked = crate::semantic::mask_comments(text);
    let chars = masked.char_indices().collect::<Vec<_>>();
    let mut sites = Vec::new();
    let mut index = 0;
    while index < chars.len() && sites.len() < 500 {
        let (offset, ch) = chars[index];
        if !(ch.is_alphabetic() || matches!(ch, '_' | '$')) {
            index += 1;
            continue;
        }
        let start = offset;
        let mut end = offset + ch.len_utf8();
        let mut cursor = index + 1;
        while cursor < chars.len() {
            let (next, ch) = chars[cursor];
            if ch.is_alphanumeric() || matches!(ch, '_' | '$') {
                end = next + ch.len_utf8();
                cursor += 1;
            } else {
                break;
            }
        }
        index = cursor;
        let name = &masked[start..end];
        let Some(spec) = specs.get(name) else {
            continue;
        };
        if start == spec.declaration || masked[..start].ends_with('.') {
            continue;
        }
        let rest = &masked[end..];
        let Some(open_offset) = rest.char_indices().find_map(|(position, ch)| match ch {
            '(' => Some(position),
            ch if ch.is_whitespace() => None,
            _ => Some(usize::MAX),
        }) else {
            continue;
        };
        if open_offset == usize::MAX {
            continue;
        }
        let open = end + open_offset;
        let Some(close) = matching_paren(&masked, open) else {
            continue;
        };
        let arguments = arguments(&masked, open, close);
        sites.push(CallSite {
            name: name.to_owned(),
            name_offset: start,
            open,
            close,
            min: spec.min,
            max: spec.max,
            params: spec.params.clone(),
            arguments,
        });
    }
    sites
}

pub(super) fn issues(text: &str) -> Vec<CallIssue> {
    let masked = crate::semantic::mask_comments(text);
    let mut issues = Vec::new();
    for site in calls(text) {
        let count = site.arguments.len();
        let message = if count < site.min {
            Some(format!(
                "Function '{}' expects {} argument{} but got {count}.",
                site.name,
                site.min,
                if site.min == 1 { "" } else { "s" }
            ))
        } else if let Some(max) = site.max.filter(|max| count > *max) {
            Some(format!(
                "Function '{}' expects at most {max} argument{} but got {count}.",
                site.name,
                if max == 1 { "" } else { "s" }
            ))
        } else {
            None
        };
        if let Some(message) = message {
            let line = masked[..site.name_offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = masked[..site.name_offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            issues.push(CallIssue {
                line,
                column: site.name_offset - line_start + 1,
                message,
            });
            continue;
        }
        if count < site.min || site.max.is_some_and(|max| count > max) {
            continue;
        }
        for (index, argument) in site.arguments.iter().enumerate() {
            let Some(param) = site.params.get(index) else {
                break;
            };
            if param.rest {
                break;
            }
            let Some(annotation) = param.annotation.as_deref() else {
                continue;
            };
            let value = text[argument.clone()].trim();
            let Some(kind) = crate::semantic::literal_kind(value) else {
                continue;
            };
            if !matches!(kind, "string" | "number" | "boolean" | "null") {
                continue;
            }
            if crate::semantic::annotation_accepts(annotation, kind) {
                continue;
            }
            let offset = argument.start;
            let line = masked[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
            let line_start = masked[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
            issues.push(CallIssue {
                line,
                column: offset - line_start + 1,
                message: format!(
                    "Argument of type '{kind}' is not assignable to parameter of type '{annotation}'."
                ),
            });
        }
    }
    issues
}

pub(super) fn site_at(text: &str, line: usize, column: usize) -> Option<CallSite> {
    let masked = crate::semantic::mask_comments(text);
    calls(text).into_iter().find(|site| {
        let site_line = masked[..site.name_offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let line_start = masked[..site.name_offset]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        site_line == line && site.name_offset - line_start + 1 == column
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_arguments_are_checked_against_primitive_annotations() {
        let mismatch = issues("function greet(name: string) {}\ngreet(42);\n");
        assert!(
            mismatch
                .iter()
                .any(|issue| issue.message
                    == "Argument of type 'number' is not assignable to parameter of type 'string'."),
            "{mismatch:?}"
        );
        assert!(
            issues("function greet(name: string) {}\ngreet(\"ada\");\n").is_empty(),
            "matching literals stay clean"
        );

        let union = issues("function show(value: string | number) {}\nshow(true);\n");
        assert!(
            union
                .iter()
                .any(|issue| issue.message.contains("not assignable to parameter of type 'string | number'")),
            "{union:?}"
        );
        assert!(
            issues("function show(value: string | number) {}\nshow(7);\n").is_empty()
        );

        let optional = issues("function setAge(age?: number) {}\nsetAge(\"old\");\n");
        assert!(optional.iter().any(|issue| issue.message.contains("'number'")), "{optional:?}");

        let default = issues("function pad(width: number = 0) {}\npad(false);\n");
        assert!(default.iter().any(|issue| issue.message.contains("'number'")), "{default:?}");

        // Anything that is not a literal, and annotations that are not
        // primitive unions, stay unreported.
        for clean in [
            "function f(x: string) {}\nconst value = 1;\nf(value);\n",
            "function f(mode: \"light\" | \"dark\") {}\nf(\"dim\");\n",
            "function f(p: Point) {}\nf({ x: 1 });\n",
            "function f(x: string[]) {}\nf([\"a\"]);\n",
            "function f(...rest: number[]) {}\nf(\"a\");\n",
            "function f(x: number) {}\nf(1 + 2);\n",
        ] {
            let found = issues(clean);
            assert!(found.is_empty(), "{clean:?} -> {found:?}");
        }

        // A count mismatch still reports the count rather than a type error.
        let count = issues("function f(x: string) {}\nf(1, 2);\n");
        assert_eq!(count.len(), 1, "{count:?}");
        assert!(count[0].message.contains("expects at most"), "{count:?}");
    }

    #[test]
    fn argument_counts_handle_optional_default_rest_and_nesting() {
        let text =
            "function f(a, b) { return a + b; }\nf(1);\nf(1, 2, 3);\nf({x: 1, y: 2}, 'a,b');\n";
        let found = issues(text);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].line, 2);
        assert_eq!(found[0].column, 1);
        assert!(found[0].message.contains("expects 2 arguments but got 1"));
        assert_eq!(found[1].line, 3);
        assert!(found[1].message.contains("at most 2 arguments but got 3"));

        assert!(issues("function f(a, b = 1, ...rest) {}\nf(1);").is_empty());
        assert!(issues("function f(a, b = 1, ...rest) {}\nf(1, 2, 3);").is_empty());
        assert!(issues("function f(a, b) {}\nf(1);\n// f(1, 2);").len() == 1);
        assert!(issues("function f(a, b) {}\nf(1, 2);").is_empty());
        assert!(issues("function f(a, b) {}\nobj.f(1);").is_empty());
        assert!(issues("function f(a, b) {}\nf(1, 2,);").is_empty());
        assert!(issues("function f(a, b) {}\nf('a,b', 2);").is_empty());
        assert!(issues("function f(a) {}\n  function g(b) {}\nf();").len() == 1);
    }
}

