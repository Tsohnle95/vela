//! Bounded semantic checks for JavaScript/TypeScript documents.
//!
//! Primitive literal annotations and duplicate top-level declarations are
//! checked. This is deliberately syntax-light and never claims language-server
//! coverage.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticKind {
    TypeMismatch,
    DuplicateDeclaration,
    UnusedDeclaration,
    UnusedParameter,
    MissingReturn,
    UnusedImport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SemanticDiagnostic {
    pub kind: SemanticKind,
    /// One-based line.
    pub line: usize,
    /// One-based column of the literal.
    pub column: usize,
    pub message: String,
}

#[derive(Clone, Copy)]
enum MaskState {
    Code,
    String { quote: char, escaped: bool },
    Line,
    Block,
}

/// Blank comments while preserving string contents, newlines and byte offsets.
pub(super) fn mask_comments(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(text.len());
    let mut state = MaskState::Code;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        match state {
            MaskState::Code => match ch {
                '"' | '\'' | '`' => {
                    out.push(ch);
                    state = MaskState::String {
                        quote: ch,
                        escaped: false,
                    };
                    index += 1;
                }
                '/' if next == Some('/') => {
                    out.push(' ');
                    out.push(' ');
                    state = MaskState::Line;
                    index += 2;
                }
                '/' if next == Some('*') => {
                    out.push(' ');
                    out.push(' ');
                    state = MaskState::Block;
                    index += 2;
                }
                _ => {
                    out.push(ch);
                    index += 1;
                }
            },
            MaskState::String { quote, escaped } => {
                out.push(ch);
                state = if escaped {
                    MaskState::String {
                        quote,
                        escaped: false,
                    }
                } else if ch == '\\' {
                    MaskState::String {
                        quote,
                        escaped: true,
                    }
                } else if ch == quote {
                    MaskState::Code
                } else {
                    MaskState::String { quote, escaped }
                };
                index += 1;
            }
            MaskState::Line => {
                if ch == '\n' {
                    out.push('\n');
                    state = MaskState::Code;
                } else {
                    out.extend(std::iter::repeat_n(' ', ch.len_utf8()));
                }
                index += 1;
            }
            MaskState::Block => {
                if ch == '*' && next == Some('/') {
                    out.push(' ');
                    out.push(' ');
                    state = MaskState::Code;
                    index += 2;
                } else {
                    if ch == '\n' {
                        out.push('\n');
                    } else {
                        out.extend(std::iter::repeat_n(' ', ch.len_utf8()));
                    }
                    index += 1;
                }
            }
        }
    }
    out
}

fn literal_kind(value: &str) -> Option<&'static str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with('"') || value.starts_with('\'') || value.starts_with('`') {
        return Some("string");
    }
    if value == "true" || value == "false" {
        return Some("boolean");
    }
    if value == "null" {
        return Some("null");
    }
    if value.starts_with('[') {
        return Some("array");
    }
    if value.starts_with('{') {
        return Some("object");
    }
    if value.starts_with("function") || value.contains("=>") || value.starts_with("async function")
    {
        return Some("function");
    }
    let number = value.replace('_', "");
    if number.parse::<f64>().is_ok() {
        return Some("number");
    }
    None
}

fn annotation_accepts(annotation: &str, literal: &str) -> bool {
    let annotation = annotation.trim();
    if matches!(annotation, "any" | "unknown") {
        return true;
    }
    if let Some((left, right)) = annotation.split_once('|') {
        return annotation_accepts(left, literal) || annotation_accepts(right, literal);
    }
    match literal {
        "array" => false,
        "object" => {
            annotation == "object"
                || annotation.starts_with("Record<")
                || annotation.starts_with('{')
        }
        "function" => annotation.starts_with("Function") || annotation.contains("=>"),
        _ => annotation == literal,
    }
}

fn array_accepts(annotation: &str, value: &str) -> bool {
    if let Some((left, right)) = annotation.split_once('|') {
        return array_accepts(left, value) || array_accepts(right, value);
    }
    let annotation = annotation.trim();
    let base = annotation.strip_suffix("[]").or_else(|| {
        annotation
            .strip_prefix("Array<")
            .and_then(|rest| rest.strip_suffix('>'))
    });
    let Some(base) = base else {
        return false;
    };
    let Some(inner) = value
        .trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    if inner.trim().is_empty() {
        return true;
    }
    for element in inner.split(',') {
        let Some(kind) = literal_kind(element) else {
            return true;
        };
        if kind == "array" {
            return true;
        }
        if !annotation_accepts(base, kind) {
            return false;
        }
    }
    true
}

/// One diagnostic per annotated literal that cannot satisfy its annotation.
pub(super) fn type_diagnostics(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut found = Vec::new();
    for (line_index, line) in masked.split('\n').enumerate() {
        let trimmed = line.trim_start();
        let base = line.len() - trimmed.len();
        let trimmed = trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed)
            .trim_start();
        let Some(rest) = ["const ", "let ", "var "]
            .iter()
            .find_map(|keyword| trimmed.strip_prefix(keyword))
        else {
            continue;
        };
        let Some((declaration, value)) = rest.split_once('=') else {
            continue;
        };
        let Some((name, annotation)) = declaration.split_once(':') else {
            continue;
        };
        if name.trim().is_empty() || !crate::rename::valid_identifier(name.trim()) {
            continue;
        }
        let value = value.trim().trim_end_matches(';').trim();
        let Some(literal) = literal_kind(value) else {
            continue;
        };
        let annotation = annotation.trim();
        let compatible = if matches!(annotation, "any" | "unknown") {
            true
        } else if literal == "array" {
            array_accepts(annotation, value)
        } else {
            annotation_accepts(annotation, literal)
        };
        if annotation.is_empty() || compatible {
            continue;
        }
        let value_offset = line.find('=').map(|index| index + 1).unwrap_or(0);
        let column = base + value_offset + line[value_offset..].len()
            - line[value_offset..].trim_start().len()
            + 1;
        found.push(SemanticDiagnostic {
            kind: SemanticKind::TypeMismatch,
            line: line_index + 1,
            column,
            message: format!("Type '{literal}' is not assignable to type '{annotation}'."),
        });
    }
    found
}

fn top_level_declaration(line: &str) -> Option<(usize, &str)> {
    if line.starts_with([' ', '\t']) {
        return None;
    }
    let mut offset = 0;
    let mut rest = line;
    if let Some(tail) = rest.strip_prefix("export ") {
        offset += "export ".len();
        rest = tail;
    }
    if rest.starts_with("declare ") {
        return None;
    }
    let mut keyword = "";
    for candidate in ["const ", "let ", "class ", "type "] {
        if rest.starts_with(candidate) {
            keyword = candidate;
            break;
        }
    }
    if keyword.is_empty() {
        return None;
    }
    let tail = &rest[keyword.len()..];
    if keyword == "const " && tail.starts_with("enum ") {
        return None;
    }
    let name_len = tail
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &tail[..name_len];
    crate::rename::valid_identifier(name).then_some((offset + keyword.len(), name))
}

/// Duplicate top-level lexical, class and type-alias declarations. `var`,
/// functions (overloads merge), enums and interfaces are deliberately excluded
/// because their merging/redeclaration rules vary and would invite false
/// positives in a syntax-light checker.
pub(super) fn duplicate_declarations(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut seen = HashMap::<&str, usize>::new();
    let mut found = Vec::new();
    for (line_index, line) in masked.split('\n').enumerate() {
        let Some((column, name)) = top_level_declaration(line) else {
            continue;
        };
        if let Some(first) = seen.get(name) {
            found.push(SemanticDiagnostic {
                kind: SemanticKind::DuplicateDeclaration,
                line: line_index + 1,
                column: column + 1,
                message: format!(
                    "Duplicate top-level declaration '{name}' (first declared on line {first})."
                ),
            });
        } else {
            seen.insert(name, line_index + 1);
        }
    }
    found
}

fn has_top_level_comma(line: &str, from: usize) -> bool {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for ch in line[from..].chars() {
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
            ',' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn variable_declaration(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("export ") || trimmed.starts_with("declare ") {
        return None;
    }
    let base = line.len() - trimmed.len();
    let keyword = ["const ", "let "]
        .into_iter()
        .find(|keyword| trimmed.starts_with(keyword))?;
    let tail = &trimmed[keyword.len()..];
    if keyword == "const " && tail.starts_with("enum ") {
        return None;
    }
    if tail.starts_with(['{', '[']) {
        return None;
    }
    let name_len = tail
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &tail[..name_len];
    if !crate::rename::valid_identifier(name) {
        return None;
    }
    let value = line.find('=').unwrap_or(line.len());
    if has_top_level_comma(line, value) {
        return None;
    }
    Some((base + keyword.len(), name))
}

/// Unused local `const`/`let` declarations. Any whole-word occurrence outside
/// comments counts as a use, which is conservative: writes, strings and JSX
/// usages are all accepted rather than guessed at. Exported declarations,
/// underscore-prefixed names, destructuring and multi-declarator lines are
/// excluded.
pub(super) fn unused_declarations(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut found = Vec::new();
    for (line_index, line) in masked.split('\n').enumerate() {
        let Some((column, name)) = variable_declaration(line) else {
            continue;
        };
        if name.starts_with('_') {
            continue;
        }
        if crate::highlights::whole_word_ranges(&masked, name, 2).len() < 2 {
            found.push(SemanticDiagnostic {
                kind: SemanticKind::UnusedDeclaration,
                line: line_index + 1,
                column: column + 1,
                message: format!("Unused declaration '{name}'."),
            });
        }
    }
    found
}

fn parameter_span(line: &str) -> Option<(usize, usize)> {
    let arrow = line.find("=>");
    let function = line.find("function");
    if let Some(function) = function {
        let before = &line[..function];
        let declaration = before.trim().is_empty()
            || before.trim_end().ends_with('=')
            || before
                .split_whitespace()
                .last()
                .is_some_and(|word| matches!(word, "export" | "default" | "async"));
        if declaration {
            let open = line[function..].find('(')? + function;
            let close = line[open + 1..].find(')')? + open + 1;
            let after = line[close + 1..].trim_start();
            let has_body = after.contains('{') && !after.starts_with(';');
            if has_body {
                return Some((open + 1, close));
            }
        }
    }
    if let Some(arrow) = arrow {
        let before = &line[..arrow];
        if let Some(open) = before.rfind('(') {
            let close = before[open + 1..].find(')')? + open + 1;
            let between = before[close + 1..].trim();
            if between.is_empty() || between.starts_with(':') {
                return Some((open + 1, close));
            }
        } else {
            let end = before.trim_end().len();
            let start = before[..end]
                .char_indices()
                .rev()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, _)| index)?;
            let name = &before[start..end];
            if crate::rename::valid_identifier(name) {
                return Some((start, end));
            }
        }
    }
    // Class/object method parameter lists: `read(a, a) {}`.
    let mut search = 0;
    while let Some(relative) = line[search..].find('(') {
        let open = search + relative;
        search = open + 1;
        let before = line[..open].trim_end();
        let name_end = before.len();
        let name_start = before
            .char_indices()
            .rev()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            .last()
            .map(|(index, _)| index)
            .unwrap_or(name_end);
        if name_start == name_end {
            continue;
        }
        let name = &before[name_start..name_end];
        if matches!(
            name,
            "if" | "for" | "while" | "switch" | "catch" | "with" | "function" | "return"
        ) {
            continue;
        }
        let prior = line[..name_start].trim_end();
        let member_context = prior
            .chars()
            .next_back()
            .is_some_and(|ch| matches!(ch, '{' | ';' | '}' | ','))
            || prior.split_whitespace().last().is_some_and(|word| {
                matches!(
                    word,
                    "static"
                        | "async"
                        | "get"
                        | "set"
                        | "public"
                        | "private"
                        | "protected"
                        | "readonly"
                        | "override"
                )
            });
        if !member_context {
            continue;
        }
        let close = line[open + 1..].find(')')? + open + 1;
        let after = line[close + 1..].trim_start();
        if after.contains('{') && !after.starts_with(';') {
            return Some((open + 1, close));
        }
    }
    None
}

fn split_parameters(text: &str) -> Vec<(usize, &str)> {
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
                parts.push((start, &text[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push((start, &text[start..]));
    parts
}

#[derive(Clone, Debug)]
pub(super) struct DuplicateParameter {
    pub name: String,
    /// Byte offset of the earlier shadowed parameter.
    pub first_offset: usize,
    /// Byte offset of the later parameter that wins the binding.
    pub later_offset: usize,
}

/// Simple names in a one-line function/arrow parameter list with their byte
/// offsets. Destructured and `this` parameters are skipped.
fn parameter_names(line: &str, line_start: usize) -> Vec<(String, usize)> {
    let Some((start, end)) = parameter_span(line) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for (offset, raw) in split_parameters(&line[start..end]) {
        let trimmed = raw.trim_start();
        let leading = raw.len() - trimmed.len();
        let trimmed = trimmed.trim_end();
        if trimmed.is_empty() || trimmed.starts_with(['{', '[']) {
            continue;
        }
        let rest = trimmed.strip_prefix("...").unwrap_or(trimmed);
        let rest_offset = trimmed.len() - rest.len();
        let name_len = rest
            .char_indices()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);
        let name = &rest[..name_len];
        if !crate::rename::valid_identifier(name) || name == "this" {
            continue;
        }
        names.push((
            name.to_owned(),
            line_start + start + offset + leading + rest_offset,
        ));
    }
    names
}

/// Duplicate simple parameter names in one-line function/arrow declarations.
/// Destructured, `this` and distinct names are ignored. The later parameter wins
/// the binding, leaving the earlier one shadowed.
pub(super) fn duplicate_parameters(text: &str) -> Vec<DuplicateParameter> {
    let masked = mask_comments(text);
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in masked.split('\n') {
        let mut seen = HashMap::<String, usize>::new();
        for (name, offset) in parameter_names(line, line_start) {
            if let Some(first_offset) = seen.get(&name).copied() {
                found.push(DuplicateParameter {
                    name,
                    first_offset,
                    later_offset: offset,
                });
            } else {
                seen.insert(name, offset);
            }
        }
        line_start += line.len() + 1;
    }
    found
}

/// Unused simple function/arrow parameters. Destructured, `this`, and
/// underscore-prefixed parameters are skipped; any other whole-word use in the
/// document counts. Declaration files are excluded by the caller.
pub(super) fn unused_parameters(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut found = Vec::new();
    for (line_index, line) in masked.split('\n').enumerate() {
        let Some((start, end)) = parameter_span(line) else {
            continue;
        };
        let parameters = &line[start..end];
        for (offset, raw) in split_parameters(parameters) {
            let parameter = raw.trim_start();
            let leading = raw.len() - parameter.len();
            let parameter = parameter.trim_end();
            if parameter.is_empty() {
                continue;
            }
            let name_offset = leading + start + offset;
            if parameter.starts_with(['{', '[']) {
                continue;
            }
            let parameter = parameter.strip_prefix("...").unwrap_or(parameter);
            let name_len = parameter
                .char_indices()
                .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                .last()
                .map(|(index, ch)| index + ch.len_utf8())
                .unwrap_or(0);
            let name = &parameter[..name_len];
            if !crate::rename::valid_identifier(name) || name == "this" || name.starts_with('_') {
                continue;
            }
            if crate::highlights::whole_word_ranges(&masked, name, 2).len() < 2 {
                found.push(SemanticDiagnostic {
                    kind: SemanticKind::UnusedParameter,
                    line: line_index + 1,
                    column: name_offset + 1,
                    message: format!("Parameter '{name}' is declared but never used."),
                });
            }
        }
    }
    found
}

fn matching_delimiter(text: &str, open: usize, opening: char, closing: char) -> Option<usize> {
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
            _ if ch == opening => depth += 1,
            _ if ch == closing => {
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

fn void_like(annotation: &str) -> bool {
    let annotation = annotation.trim();
    if matches!(
        annotation,
        "void" | "undefined" | "never" | "any" | "unknown"
    ) {
        return true;
    }
    if let Some(inner) = annotation
        .strip_prefix("Promise<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return void_like(inner);
    }
    if let Some((left, right)) = annotation.split_once('|') {
        return void_like(left) || void_like(right);
    }
    false
}

fn typed_function(line: &str) -> Option<(usize, usize, String)> {
    let trimmed = line.trim_start();
    let base = line.len() - trimmed.len();
    let mut rest = trimmed;
    let mut offset = base;
    loop {
        let stripped = ["export ", "default ", "async "]
            .into_iter()
            .find_map(|prefix| rest.strip_prefix(prefix).map(|tail| (prefix, tail)));
        let Some((prefix, tail)) = stripped else {
            break;
        };
        offset += prefix.len();
        rest = tail;
    }
    let after = rest.strip_prefix("function")?;
    if after.starts_with('*') {
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
    let name_column = offset + rest[..rest.len() - after.len()].len();
    let open = line[name_column + name_len..].find('(')? + name_column + name_len;
    let close = matching_delimiter(line, open, '(', ')')?;
    let tail = &line[close + 1..];
    let tail_trimmed = tail.trim_start();
    if !tail_trimmed.starts_with(':') {
        return None;
    }
    let brace = tail_trimmed.find('{')?;
    let annotation = tail_trimmed[..brace].trim();
    let return_type = annotation.trim_start_matches(':').trim();
    if return_type.is_empty() || return_type.starts_with('{') || void_like(return_type) {
        return None;
    }
    let brace = close + 1 + tail.len() - tail_trimmed.len() + brace;
    Some((name_column, brace, return_type.to_owned()))
}

/// Typed function declarations whose body has no `return` or `throw`. Bodies
/// are matched with comments masked; nested returns count conservatively.
pub(super) fn missing_returns(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut found = Vec::new();
    let mut line_start = 0;
    for (line_index, line) in masked.split('\n').enumerate() {
        if let Some((name_column, brace, return_type)) = typed_function(line) {
            let open = line_start + brace;
            if let Some(close) = matching_delimiter(&masked, open, '{', '}') {
                let body = &masked[open + 1..close];
                if crate::highlights::whole_word_ranges(body, "return", 1).is_empty()
                    && crate::highlights::whole_word_ranges(body, "throw", 1).is_empty()
                {
                    found.push(SemanticDiagnostic {
                        kind: SemanticKind::MissingReturn,
                        line: line_index + 1,
                        column: name_column + 1,
                        message: format!(
                            "Function returns '{return_type}' but has no return statement."
                        ),
                    });
                }
            }
        }
        line_start += line.len() + 1;
    }
    found
}

pub(super) fn import_bindings(line: &str) -> Vec<(usize, String)> {
    let trimmed = line.trim_start();
    let base = line.len() - trimmed.len();
    let Some(mut rest) = trimmed.strip_prefix("import ") else {
        return Vec::new();
    };
    let mut offset = base + "import ".len();
    if let Some(tail) = rest.strip_prefix("type ") {
        offset += "type ".len();
        rest = tail;
    }
    if rest.starts_with(['"', '\'']) {
        return Vec::new();
    }
    let Some(from) = rest.rfind(" from ") else {
        return Vec::new();
    };
    let clause = &rest[..from];
    let clause_leading = clause.len() - clause.trim_start().len();
    let clause = clause.trim_start();
    let clause_offset = offset + clause_leading;
    let mut bindings = Vec::new();

    fn local_name(text: &str) -> Option<&str> {
        let text = text.trim();
        let name_len = text
            .char_indices()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);
        let name = &text[..name_len];
        (crate::rename::valid_identifier(name) && !name.starts_with('_')).then_some(name)
    }

    let mut collect = |start: usize, text: &str| {
        if let Some(name) = local_name(text) {
            bindings.push((start, name.to_owned()));
        }
    };

    let namespace_after_default = clause
        .find(',')
        .filter(|comma| clause[comma + 1..].trim_start().starts_with("* as "));
    if namespace_after_default.is_none()
        && let Some(open) = clause.find('{')
    {
        if open > 0 {
            let default = clause[..open].trim().trim_end_matches(',');
            let start = clause[..open].find(default).unwrap_or(0);
            collect(clause_offset + start, default);
        }
        let close = clause.rfind('}').unwrap_or(clause.len().saturating_sub(1));
        if close > open {
            let inner = &clause[open + 1..close];
            let mut cursor = 0;
            for part in inner.split(',') {
                let part_start = open + 1 + cursor;
                let leading = part.len() - part.trim_start().len();
                let specifier = part.trim();
                let (local, local_offset) =
                    if let Some((before, after)) = specifier.split_once(" as ") {
                        let after_leading = after.len() - after.trim_start().len();
                        (after, before.len() + " as ".len() + after_leading)
                    } else if let Some(after) = specifier.strip_prefix("type ") {
                        let after_leading = after.len() - after.trim_start().len();
                        (after, "type ".len() + after_leading)
                    } else {
                        (specifier, 0)
                    };
                collect(clause_offset + part_start + leading + local_offset, local);
                cursor += part.len() + 1;
            }
        }
    } else if let Some(comma) = namespace_after_default {
        let default = clause[..comma].trim();
        let default_start = clause[..comma].find(default).unwrap_or(0);
        collect(clause_offset + default_start, default);
        let after = clause[comma + 1..].trim_start();
        let after_leading = clause[comma + 1..].len() - after.len();
        if let Some(namespace) = after.strip_prefix("* as ") {
            collect(
                clause_offset + comma + 1 + after_leading + "* as ".len(),
                namespace,
            );
        }
    } else if let Some(namespace) = clause.strip_prefix("* as ") {
        collect(clause_offset + "* as ".len(), namespace);
    } else {
        collect(clause_offset, clause);
    }
    bindings
}

/// Unused local names from single-line ES imports. Side-effect, multi-line and
/// underscore-prefixed bindings are excluded; any whole-word use counts.
pub(super) fn unused_imports(text: &str) -> Vec<SemanticDiagnostic> {
    let masked = mask_comments(text);
    let mut found = Vec::new();

    fn location(text: &str, offset: usize) -> (usize, usize) {
        let line = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
        let start = text[..offset]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        (line, offset - start + 1)
    }
    fn specifier_local(specifier: &str) -> Option<(usize, &str)> {
        let (prefix, rest) = if let Some(after) = specifier.strip_prefix("type ") {
            ("type ", after)
        } else {
            ("", specifier)
        };
        let (local, offset) = if let Some((before, after)) = rest.split_once(" as ") {
            (
                after.trim(),
                prefix.len() + before.len() + " as ".len() + after.len() - after.trim_start().len(),
            )
        } else {
            (
                rest.trim(),
                prefix.len() + rest.len() - rest.trim_start().len(),
            )
        };
        let name_len = local
            .char_indices()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);
        let name = &local[..name_len];
        (crate::rename::valid_identifier(name) && !name.starts_with('_')).then_some((offset, name))
    }

    // Multi-line named clauses: `import { ... } from` / `import D, { ... } from`.
    let mut line_starts = vec![0];
    line_starts.extend(
        masked
            .bytes()
            .enumerate()
            .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
    );
    for (line_index, start) in line_starts.iter().enumerate() {
        let end = line_starts
            .get(line_index + 1)
            .map(|next| next.saturating_sub(1))
            .unwrap_or(masked.len());
        let line = &masked[*start..end];
        let trimmed = line.trim_start();
        let import_start = start + (line.len() - trimmed.len());
        let is_import = trimmed == "import"
            || trimmed.starts_with("import ")
            || trimmed.starts_with("import\t");
        if !is_import {
            continue;
        }
        let rest = &masked[import_start + "import".len()..];
        let Some(open_offset) = rest.find('{') else {
            continue;
        };
        let before_open = &rest[..open_offset];
        if before_open.contains(';')
            || before_open.contains("from")
            || before_open.matches('\n').count() > 20
        {
            continue;
        }
        let open = import_start + "import".len() + open_offset;
        let Some(close) = masked[open + 1..].find('}').map(|index| open + 1 + index) else {
            continue;
        };
        let after = &masked[close + 1..];
        let after_trimmed = after.trim_start();
        if !after_trimmed.starts_with("from")
            || !after_trimmed["from".len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            continue;
        }
        let from_offset = close + 1 + after.len() - after_trimmed.len();
        let import_line = masked[..import_start]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let open_line = masked[..open].rfind('\n').map_or(0, |index| index + 1);
        let close_line = masked[..close].rfind('\n').map_or(0, |index| index + 1);
        let from_line = masked[..from_offset]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if open_line == import_line && close_line == import_line && from_line == import_line {
            continue;
        }
        let mut bindings = Vec::new();
        let mut clause_start = import_start + "import".len();
        let mut raw = &masked[clause_start..open];
        let leading = raw.len() - raw.trim_start().len();
        clause_start += leading;
        raw = &raw[leading..];
        if raw.starts_with("type")
            && raw["type".len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            let after = raw["type".len()..].trim_start();
            clause_start += raw.len() - after.len();
            raw = after;
        }
        let default = raw.split(',').next().unwrap_or("").trim();
        if !default.is_empty()
            && let Some((offset, name)) = specifier_local(default)
        {
            let offset = clause_start + raw.find(default).unwrap_or(0) + offset;
            bindings.push((offset, name.to_owned()));
        }
        let inner = &masked[open + 1..close];
        let mut cursor = 0;
        for part in inner.split(',') {
            let part_start = open + 1 + cursor;
            let leading = part.len() - part.trim_start().len();
            if let Some((offset, name)) = specifier_local(part.trim()) {
                bindings.push((part_start + leading + offset, name.to_owned()));
            }
            cursor += part.len() + 1;
        }
        for (offset, name) in bindings {
            if crate::highlights::whole_word_ranges(&masked, &name, 2).len() < 2 {
                let (line, column) = location(&masked, offset);
                found.push(SemanticDiagnostic {
                    kind: SemanticKind::UnusedImport,
                    line,
                    column,
                    message: format!("Import '{name}' is declared but never used."),
                });
            }
        }
    }

    for (line_index, line) in masked.split('\n').enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("import ") || !trimmed.contains(" from ") || trimmed.contains('\n')
        {
            continue;
        }
        for (offset, name) in import_bindings(line) {
            if crate::highlights::whole_word_ranges(&masked, &name, 2).len() < 2 {
                found.push(SemanticDiagnostic {
                    kind: SemanticKind::UnusedImport,
                    line: line_index + 1,
                    column: offset + 1,
                    message: format!("Import '{name}' is declared but never used."),
                });
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_literal_annotations_report_mismatches() {
        let diagnostics = type_diagnostics(
            "const name: string = 42;\nlet ok: number = 1;\nexport const flag: boolean = true;\nconst text: string = \"hi\";",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 22);
        assert_eq!(
            diagnostics[0].message,
            "Type 'number' is not assignable to type 'string'."
        );
        assert!(type_diagnostics("let ok: number = 1;").is_empty());
    }

    #[test]
    fn unions_any_arrays_objects_and_comments_are_bounded() {
        assert!(type_diagnostics("const value: string | number = 1;").is_empty());
        assert!(type_diagnostics("const value: any = 1;").is_empty());
        assert!(type_diagnostics("const value: unknown = [1];").is_empty());
        assert!(type_diagnostics("const list: number[] = [1];").is_empty());
        assert!(type_diagnostics("const list: string[] = [1];").len() == 1);
        assert!(type_diagnostics("const record: Record<string, number> = {a: 1};").is_empty());
        assert!(type_diagnostics("const text: string = `hi`;").is_empty());
        assert!(type_diagnostics("const flag: boolean = false;").is_empty());
        assert!(type_diagnostics("const value: string = 42; // mismatch").len() == 1);
        assert!(type_diagnostics("const value: string = call();").is_empty());
        assert!(type_diagnostics("const value = 42;").is_empty());
        assert!(type_diagnostics("  const value: string = 42;").len() == 1);
        assert!(type_diagnostics("let value: null = null;").is_empty());
        assert!(type_diagnostics("let value: string = null;").len() == 1);
        assert!(type_diagnostics("/*\nconst value: string = 42;\n*/").is_empty());
        assert!(type_diagnostics("const value: string = \"// not a comment\";").is_empty());
    }

    #[test]
    fn duplicate_top_level_declarations_are_bounded() {
        let text = "const value = 1;\nlet other = 2;\nconst value = 3;\nexport let other = 4;\nclass Thing {}\nclass Thing {}\ntype Alias = string;\ntype Alias = number;";
        let found = duplicate_declarations(text);
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].line, 3);
        assert_eq!(found[0].column, 7);
        assert!(found[0].message.contains("first declared on line 1"));
        assert_eq!(found[1].line, 4);
        assert!(found[1].message.contains("'other'"));
        assert_eq!(found[2].line, 6);
        assert_eq!(found[3].line, 8);
        assert_eq!(found[3].column, 6);

        assert!(duplicate_declarations("var value = 1;\nvar value = 2;").is_empty());
        assert!(
            duplicate_declarations("function f(a: string): void;\nfunction f(a: any) {}")
                .is_empty()
        );
        assert!(duplicate_declarations("enum E { A }\nenum E { B }").is_empty());
        assert!(duplicate_declarations("interface I {}\ninterface I {}").is_empty());
        assert!(
            duplicate_declarations("declare const value: number;\ndeclare const value: number;")
                .is_empty()
        );
        assert!(
            duplicate_declarations("{\n  const value = 1;\n}\n{\n  const value = 2;\n}").is_empty()
        );
        assert!(duplicate_declarations("// const value = 1;\n/* const value = 2; */").is_empty());
        assert!(duplicate_declarations("const café = 1;\nconst café = 2;").len() == 1);
    }

    #[test]
    fn unused_local_declarations_are_conservative_and_bounded() {
        let found = unused_declarations(
            "const used = 1;\nconsole.log(used);\nlet unused = 2;\nexport const shared = 3;\nconst _ignored = 4;",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 3);
        assert_eq!(found[0].column, 5);
        assert_eq!(found[0].message, "Unused declaration 'unused'.");

        assert!(unused_declarations("const value = 1;\n// value").len() == 1);
        assert!(unused_declarations("const value = 1;\nconsole.log(\"value\");").is_empty());
        assert!(unused_declarations("const a = 1, b = 2;").is_empty());
        assert!(unused_declarations("for (const item of list) {}").is_empty());
        assert!(unused_declarations("const { a } = object;").is_empty());
        assert!(unused_declarations("let local = 1;\nlocal = 2;").is_empty());
        assert!(unused_declarations("const café = 1;").len() == 1);
        assert!(unused_declarations("const used = () => used;").is_empty());
        assert!(unused_declarations("function f() {\n  const inner = 1;\n}").len() == 1);
        assert!(unused_declarations("const value = [1, 2];\nconsole.log(value);").is_empty());
    }

    #[test]
    fn unused_parameters_cover_functions_arrows_defaults_rest_and_exclusions() {
        let found = unused_parameters("function greet(name, age) {\n  return name;\n}");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 1);
        assert_eq!(found[0].column, 22);
        assert_eq!(
            found[0].message,
            "Parameter 'age' is declared but never used."
        );

        assert!(unused_parameters("function ok(name) { return name; }").is_empty());
        assert!(unused_parameters("function skip(_name) {}").is_empty());
        assert!(unused_parameters("function destructured({ name }) { return 1; }").is_empty());
        assert!(unused_parameters("const pick = (left, right) => left;").len() == 1);
        assert!(unused_parameters("const identity = value => value;").is_empty());
        assert!(unused_parameters("const idle = value => 1;").len() == 1);
        assert!(unused_parameters("function f(a = 1, b = 2) { return a; }").len() == 1);
        assert!(unused_parameters("function f(...values) {}").len() == 1);
        assert!(
            unused_parameters("function f(map: Map<string, number>, key: string) { return key; }")
                .len()
                == 1
        );
        assert!(
            unused_parameters("function f(a: string): void;\nfunction f(a: any) {}").is_empty()
        );
        assert!(unused_parameters("items.map((item) => item.id);").is_empty());
        assert!(unused_parameters("function f(used) { return `x${used}`; }").is_empty());
        assert!(unused_parameters("function f(commented) {\n  // commented\n}").len() == 1);
        assert!(unused_parameters("function f(café) {}").len() == 1);
    }

    #[test]
    fn missing_returns_cover_typed_declarations_and_exclusions() {
        let found = missing_returns("function greet(): string {\n  console.log('hi');\n}");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, SemanticKind::MissingReturn);
        assert_eq!(found[0].line, 1);
        assert_eq!(found[0].column, 10);
        assert!(found[0].message.contains("returns 'string'"));

        assert!(missing_returns("function ok(): string { return \"x\"; }").is_empty());
        assert!(missing_returns("function ok(): string { throw new Error(); }").is_empty());
        assert!(missing_returns("function ok(): string {\n  return \"x\";\n}").is_empty());
        assert!(missing_returns("function skip(): void {}").is_empty());
        assert!(missing_returns("async function skip(): Promise<void> {}").is_empty());
        assert!(missing_returns("function skip(): string | undefined {}").is_empty());
        assert!(missing_returns("function skip(): never { throw new Error(); }").is_empty());
        assert!(missing_returns("function overload(): string;").is_empty());
        assert!(missing_returns("function* generator(): Generator<number> {}").is_empty());
        assert!(
            missing_returns("export async function work(): number {\n  const x = 1;\n}").len() == 1
        );
        assert!(
            missing_returns("function ok(): string {\n  // no return here\n  return \"x\";\n}")
                .is_empty()
        );
        assert!(
            missing_returns("function outer(): string {\n  function inner() { return \"\"; }\n}")
                .is_empty()
        );
        assert!(missing_returns("function obj(): { a: string } {}").is_empty());
        assert!(missing_returns("function unclosed(): string {").is_empty());
    }

    #[test]
    fn unused_imports_cover_defaults_named_aliases_and_namespaces() {
        let found = unused_imports("import { used, unused } from \"pkg\";\nconsole.log(used);");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, SemanticKind::UnusedImport);
        assert_eq!(found[0].line, 1);
        assert_eq!(
            found[0].message,
            "Import 'unused' is declared but never used."
        );

        assert!(
            unused_imports("import defaultUsed from \"pkg\";\nconsole.log(defaultUsed);")
                .is_empty()
        );
        assert!(unused_imports("import defaultUnused from \"pkg\";").len() == 1);
        assert!(unused_imports("import * as ns from \"pkg\";\nns.call();").is_empty());
        assert!(unused_imports("import * as ns from \"pkg\";").len() == 1);
        assert!(unused_imports("import { foo as bar } from \"pkg\";\nbar();").is_empty());
        assert!(unused_imports("import { foo as bar } from \"pkg\";").len() == 1);
        assert!(unused_imports("import \"side-effects\";").is_empty());
        assert!(unused_imports("import { _ignored } from \"pkg\";").is_empty());
        assert!(unused_imports("import type { Unused } from \"pkg\";").len() == 1);
        assert!(
            unused_imports("import { type Used } from \"pkg\";\nconst value: Used = 1;").is_empty()
        );
        assert!(unused_imports("import { used } from \"pkg\";\n// used").len() == 1);
        assert!(
            unused_imports("import { used } from \"pkg\";\nconst label = \"used\";").is_empty()
        );
        assert!(unused_imports("import { used } from \"pkg\";\nexport { used };").is_empty());
        assert!(unused_imports("import {\n  multi\n} from \"pkg\";\nmulti();").is_empty());
        assert!(unused_imports("import { café } from \"pkg\";").len() == 1);
        assert!(
            unused_imports("import Default, * as ns from \"pkg\";\nconsole.log(Default, ns);")
                .is_empty()
        );
        let found =
            unused_imports("import Default, * as unusedNs from \"pkg\";\nconsole.log(Default);");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].message,
            "Import 'unusedNs' is declared but never used."
        );
        let found = unused_imports("import Unused, * as ns from \"pkg\";\nconsole.log(ns);");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].message,
            "Import 'Unused' is declared but never used."
        );
        assert!(
            unused_imports("import type Unused, * as ns from \"pkg\";\nconsole.log(ns);").len()
                == 1
        );
        let multi = "import {\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(used);";
        let found = unused_imports(multi);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 3);
        assert_eq!(found[0].column, 3);
        assert_eq!(
            found[0].message,
            "Import 'unused' is declared but never used."
        );
        assert!(
            unused_imports(
                "import {\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(used, unused);"
            )
            .is_empty()
        );
        let multi_default = "import Used, {\n  unused,\n} from \"pkg\";\nconsole.log(Used);";
        let found = unused_imports(multi_default);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 2);
        assert_eq!(
            found[0].message,
            "Import 'unused' is declared but never used."
        );
        let unused_default = "import Unused, {\n  used,\n} from \"pkg\";\nconsole.log(used);";
        assert_eq!(
            unused_imports(unused_default)[0].message,
            "Import 'Unused' is declared but never used."
        );
        assert!(
            unused_imports("import type {\n  Used,\n} from \"pkg\";\nconst value: Used = 1;")
                .is_empty()
        );
        assert!(unused_imports("import {\n  _ignored,\n} from \"pkg\";").is_empty());
        assert!(unused_imports("import {\n  used\n} from \"pkg\";\nused();").is_empty());
        let next_line_brace = "import\n{\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(used);";
        let found = unused_imports(next_line_brace);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 4);
        assert_eq!(found[0].column, 3);
        let next_line_default =
            "import Default,\n{\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(Default, used);";
        let found = unused_imports(next_line_default);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 4);
        let unused_default = "import Unused,\n{\n  used,\n} from \"pkg\";\nconsole.log(used);";
        assert_eq!(
            unused_imports(unused_default)[0].message,
            "Import 'Unused' is declared but never used."
        );
        assert!(
            unused_imports("import type\n{\n  Used,\n} from \"pkg\";\nconst value: Used = 1;")
                .is_empty()
        );
        assert!(unused_imports("import\nvalue = 1;\n{ broken }").is_empty());
        let from_next_line = "import { used, unused }\nfrom \"pkg\";\nconsole.log(used);";
        let found = unused_imports(from_next_line);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 1);
        assert_eq!(found[0].column, 16);
        assert!(
            unused_imports("import { used, unused }\nfrom\n\"pkg\";\nconsole.log(used, unused);")
                .is_empty()
        );
        let brace_then_from =
            "import Default,\n{\n  used,\n  unused,\n}\nfrom \"pkg\";\nconsole.log(Default, used);";
        assert_eq!(unused_imports(brace_then_from)[0].line, 4);
        assert!(
            unused_imports("import { used, unused } from \"pkg\";\nconsole.log(used);").len() == 1
        );
    }

    #[test]
    fn duplicate_parameter_names_are_reported_with_offsets() {
        let text = "function pick(value, index, value) {\n  return value;\n}\n";
        let found = duplicate_parameters(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "value");
        assert_eq!(found[0].first_offset, text.find("value").unwrap());
        assert_eq!(found[0].later_offset, text.rfind("value)").unwrap());

        let arrows = "const f = (a, b, a, a) => a;";
        let found = duplicate_parameters(arrows);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "a");
        assert_eq!(found[1].later_offset, arrows.rfind("a)").unwrap());

        assert!(duplicate_parameters("function f(a, b, c) {}").is_empty());
        assert!(duplicate_parameters("function f({ a }, { a }) {}").is_empty());
        assert!(duplicate_parameters("function f(this, this) {}").is_empty());
        assert!(duplicate_parameters("function f(a = 1, a = 2) {}").len() == 1);
        assert!(duplicate_parameters("function f(...a, a) {}").len() == 1);
        assert!(duplicate_parameters("function f(café, café) {}").len() == 1);
        assert!(duplicate_parameters("const f = (a, a) => a;").len() == 1);
        assert!(duplicate_parameters("// function f(a, a) {}").is_empty());
        assert!(duplicate_parameters("function f(a,\n  a) {}").is_empty());

        let method = "class A { read(value, value) { return value; } }";
        assert_eq!(duplicate_parameters(method).len(), 1);
        let object_method = "const o = { read(value, value) { return value; } };";
        assert_eq!(duplicate_parameters(object_method).len(), 1);
        let static_method = "class A { static read(value, value) { return value; } }";
        assert_eq!(duplicate_parameters(static_method).len(), 1);
        let async_method = "class A { async read(value, value) { return value; } }";
        assert_eq!(duplicate_parameters(async_method).len(), 1);
        let returned = "class A { read(value, value): string { return value; } }";
        assert_eq!(duplicate_parameters(returned).len(), 1);
        let expression = "const f = function (value, value) { return value; };";
        assert_eq!(duplicate_parameters(expression).len(), 1);
        assert!(duplicate_parameters("class A { read(a, b) { return [a, b]; } }").is_empty());
        assert!(duplicate_parameters("class A { read() { return helper(a, a); } }").is_empty());
        assert!(duplicate_parameters("if (a, a) {}").is_empty());
        assert!(duplicate_parameters("class A { read({ a }, { a }) { return 1; } }").is_empty());
    }
}
