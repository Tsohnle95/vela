//! Bounded quick fixes derived from the editor's local diagnostics.
//!
//! Only edits that cannot change surrounding semantics are offered: removing a
//! single-line unused declaration and prefixing an unused parameter with `_`.

use std::ops::Range;

use crate::diagnostics::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CodeAction {
    pub title: String,
    pub range: Range<usize>,
    pub replacement: String,
}

fn line_bounds(text: &str, line: usize) -> Option<(usize, usize, &str)> {
    if line == 0 {
        return None;
    }
    let mut start = 0;
    for _ in 1..line {
        let newline = text[start..].find('\n')?;
        start += newline + 1;
    }
    let end = text[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    Some((start, end, &text[start..end]))
}

fn has_top_level_comma(line: &str) -> bool {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
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

fn unused_declaration_action(text: &str, line: usize) -> Option<CodeAction> {
    let (start, end, content) = line_bounds(text, line)?;
    let trimmed = content.trim();
    let declaration = trimmed.starts_with("const ") || trimmed.starts_with("let ");
    if !declaration || has_top_level_comma(content) || !trimmed.ends_with(';') {
        return None;
    }
    let full_end = text[end..]
        .strip_prefix('\n')
        .map(|_| end + 1)
        .unwrap_or(end);
    Some(CodeAction {
        title: "Remove unused declaration".into(),
        range: start..full_end,
        replacement: String::new(),
    })
}

fn is_import_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed == "import" || trimmed.starts_with("import ") || trimmed.starts_with("import\t")
}

fn import_clause(content: &str) -> Option<(usize, &str)> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("import")?;
    let mut clause_start = content.len() - rest.len();
    let mut clause = rest.trim_start();
    clause_start += rest.len() - clause.len();
    if clause.starts_with("type")
        && clause["type".len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        let after = clause["type".len()..].trim_start();
        clause_start += clause.len() - after.len();
        clause = after;
    }
    let clause = clause
        .rfind(" from ")
        .or_else(|| clause.rfind("\nfrom"))
        .map(|end| &clause[..end])
        .unwrap_or(clause);
    Some((clause_start, clause))
}

fn named_specifier_removal(content: &str, offset: usize) -> Option<Range<usize>> {
    let (clause_start, clause) = import_clause(content)?;
    if clause.contains("//") || clause.contains("/*") {
        return None;
    }
    let open = clause.find('{')? + clause_start;
    let close = clause.rfind('}')? + clause_start;
    if close <= open || offset <= open || offset >= close {
        return None;
    }
    let inner_start = open + 1;
    let inner_end = close;
    let inner = &content[inner_start..inner_end];
    let mut cursor = 0;
    let mut previous_comma = None;
    for part in inner.split(',') {
        let part_start = inner_start + cursor;
        let part_end = part_start + part.len();
        if offset >= part_start && offset <= part_end {
            let trimmed_end = part_start + part.trim_end().len();
            if part_end < inner_end && content[part_end..].starts_with(',') {
                return Some(part_start..part_end + 1);
            }
            if let Some(comma) = previous_comma {
                return Some(comma..trimmed_end);
            }
            if let Some(comma) = content[clause_start..open].rfind(',') {
                return Some(clause_start + comma..close + 1);
            }
            return None;
        }
        if content[part_end..inner_end].starts_with(',') {
            previous_comma = Some(part_end);
        }
        cursor += part.len() + 1;
    }
    None
}

fn default_import_removal(content: &str, offset: usize) -> Option<Range<usize>> {
    let (clause_start, clause) = import_clause(content)?;
    if clause.contains("//") || clause.contains("/*") {
        return None;
    }
    let comma = clause.find(',')?;
    if clause[..comma].trim().is_empty() || clause[..comma].trim_start().starts_with('{') {
        return None;
    }
    if offset < clause_start || offset >= clause_start + comma {
        return None;
    }
    let mut end = clause_start + comma + 1;
    while content[end..].starts_with(' ') {
        end += 1;
    }
    let mut start = clause_start;
    if content[end..].starts_with('\n') {
        while start > 0 && content[..start].ends_with(' ') {
            start -= 1;
        }
    }
    Some(start..end)
}

fn namespace_import_removal(content: &str, offset: usize) -> Option<Range<usize>> {
    let (clause_start, clause) = import_clause(content)?;
    if clause.contains("//") || clause.contains("/*") {
        return None;
    }
    let namespace = clause.find("* as ")?;
    if offset < clause_start + namespace {
        return None;
    }
    let comma = clause[..namespace].rfind(',')?;
    let name_start = namespace + "* as ".len();
    let name_end = name_start + clause[name_start..].trim_end().len();
    Some(clause_start + comma..clause_start + name_end)
}

fn single_line_import_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let (start, end, content) = line_bounds(text, line)?;
    if !content.trim_start().starts_with("import ") {
        return None;
    }
    let bindings = crate::semantic::import_bindings(content);
    let (offset, name) = bindings.iter().find(|(offset, _)| offset + 1 == column)?;
    let full_end = text[end..]
        .strip_prefix('\n')
        .map(|_| end + 1)
        .unwrap_or(end);
    let range = if bindings.len() == 1 {
        start..full_end
    } else {
        let local = named_specifier_removal(content, *offset)
            .or_else(|| default_import_removal(content, *offset))
            .or_else(|| namespace_import_removal(content, *offset))?;
        start + local.start..start + local.end
    };
    Some(CodeAction {
        title: format!("Remove unused import '{name}'"),
        range,
        replacement: String::new(),
    })
}

fn multi_line_import_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let (line_start, _line_end, content) = line_bounds(text, line)?;
    let offset = line_start + column.checked_sub(1)?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let mut import_start = line_start;
    loop {
        let import_end = text[import_start..]
            .find('\n')
            .map(|index| import_start + index)
            .unwrap_or(text.len());
        let import_line = &text[import_start..import_end];
        if is_import_start(import_line) {
            break;
        }
        if import_start == 0 {
            return None;
        }
        import_start = text[..import_start - 1]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
    }
    let open_offset = text[import_start..].find('{')?;
    let before_open = &text[import_start..import_start + open_offset];
    if before_open.contains(';')
        || before_open.contains("from")
        || before_open.matches('\n').count() > 20
    {
        return None;
    }
    let open = import_start + open_offset;
    let close = text[open + 1..].find('}')? + open + 1;
    let after = &text[close + 1..];
    let after_trimmed = after.trim_start();
    if !after_trimmed.starts_with("from")
        || !after_trimmed["from".len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let from_offset = close + 1 + after.len() - after_trimmed.len();
    let import_line = text[..import_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let open_line = text[..open].rfind('\n').map_or(0, |index| index + 1);
    let close_line = text[..close].rfind('\n').map_or(0, |index| index + 1);
    let from_line = text[..from_offset].rfind('\n').map_or(0, |index| index + 1);
    if open_line == import_line && close_line == import_line && from_line == import_line {
        return None;
    }
    let statement = &text[import_start..];
    if offset < open {
        let range = default_import_removal(statement, offset - import_start)?;
        return Some(CodeAction {
            title: "Remove unused import".into(),
            range: import_start + range.start..import_start + range.end,
            replacement: String::new(),
        });
    }
    if offset >= close {
        return None;
    }
    if is_import_start(content) {
        let range = named_specifier_removal(statement, offset - import_start)?;
        return Some(CodeAction {
            title: "Remove unused import".into(),
            range: import_start + range.start..import_start + range.end,
            replacement: String::new(),
        });
    }
    let entry_start = text[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let entry_end = text[entry_start..]
        .find('\n')
        .map(|index| entry_start + index)
        .unwrap_or(text.len());
    let entry = text[entry_start..entry_end]
        .trim()
        .trim_end_matches(',')
        .trim();
    if entry.is_empty() || entry.contains(['{', '}', ';']) {
        return None;
    }
    let name_len = text[offset..]
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &text[offset..offset + name_len];
    if name.is_empty() || name.starts_with('_') {
        return None;
    }
    let line = &text[entry_start..entry_end];
    if line.contains("//") || line.contains("/*") {
        return None;
    }
    if entry.contains(',') {
        let indent_end = line.len() - line.trim_start().len();
        let mut cursor = 0;
        let mut previous_comma = None;
        for part in line[indent_end..].split(',') {
            let start = entry_start + indent_end + cursor;
            let end = start + part.len();
            if offset >= start && offset <= end {
                let specifier = part.trim();
                let parsed = specifier.strip_prefix("type ").unwrap_or(specifier);
                let local = parsed
                    .split_once(" as ")
                    .map(|(_, local)| local)
                    .unwrap_or(parsed)
                    .trim();
                if local != name {
                    return None;
                }
                let trimmed_end = start + part.trim_end().len();
                let range = if end < entry_end && text[end..].starts_with(',') {
                    if start == entry_start + indent_end {
                        let mut after = end + 1;
                        while text[after..].starts_with(' ') {
                            after += 1;
                        }
                        start..after
                    } else {
                        previous_comma?..trimmed_end
                    }
                } else {
                    previous_comma.unwrap_or(entry_start + indent_end)..trimmed_end
                };
                return Some(CodeAction {
                    title: format!("Remove unused import '{name}'"),
                    range,
                    replacement: String::new(),
                });
            }
            if text[end..entry_end].starts_with(',') {
                previous_comma = Some(end);
            }
            cursor += part.len() + 1;
        }
        return None;
    }
    let parsed = entry.strip_prefix("type ").unwrap_or(entry);
    let local = parsed
        .split_once(" as ")
        .map(|(_, local)| local)
        .unwrap_or(parsed)
        .trim();
    if local != name {
        return None;
    }
    let full_end = entry_end + usize::from(text[entry_end..].starts_with('\n'));
    Some(CodeAction {
        title: format!("Remove unused import '{name}'"),
        range: entry_start..full_end,
        replacement: String::new(),
    })
}

fn unused_import_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    single_line_import_action(text, line, column)
        .or_else(|| multi_line_import_action(text, line, column))
}

fn simple_primitive_annotation(annotation: &str) -> bool {
    annotation.split('|').all(|part| {
        matches!(
            part.trim(),
            "string" | "number" | "boolean" | "null" | "undefined" | "any" | "unknown"
        )
    })
}

fn append_at_end(text: &str, token: &str) -> CodeAction {
    let replacement = if text.ends_with('\n') {
        format!("{token}\n")
    } else {
        format!("\n{token}")
    };
    CodeAction {
        title: format!("Add '{token}'"),
        range: text.len()..text.len(),
        replacement,
    }
}

fn argument_actions(text: &str, line: usize, column: usize) -> Vec<CodeAction> {
    let Some(site) = crate::calls::site_at(text, line, column) else {
        return Vec::new();
    };
    let count = site.arguments.len();
    if count < site.min {
        let missing = site.min - count;
        let filler = vec!["undefined"; missing].join(", ");
        let replacement = if count == 0 {
            filler
        } else {
            format!(", {filler}")
        };
        return vec![CodeAction {
            title: format!(
                "Add {missing} missing argument{}",
                if missing == 1 { "" } else { "s" }
            ),
            range: site.close..site.close,
            replacement,
        }];
    }
    if let Some(max) = site.max.filter(|max| count > *max) {
        let range = if max == 0 {
            site.open + 1..site.close
        } else {
            site.arguments[max - 1].end..site.close
        };
        return vec![CodeAction {
            title: format!(
                "Remove {} extra argument{}",
                count - max,
                if count - max == 1 { "" } else { "s" }
            ),
            range,
            replacement: String::new(),
        }];
    }
    Vec::new()
}

fn duplicate_type_parameter_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::type_parameters::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate type parameter '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_type_literal_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::type_literals::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate object type property '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_union_member_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::type_unions::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate union member '{}'", found.label),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_interface_member_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::interfaces::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate interface property '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_enum_member_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::enums::issues(text).into_iter().find(|issue| {
        let found_line = text[..issue.offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let line_start = text[..issue.offset]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        found_line == line && issue.offset - line_start + 1 == column
    })?;
    Some(CodeAction {
        title: format!("Remove later duplicate enum member '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_switch_case_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::switch_cases::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    let title = if found.label == "default" {
        "Remove duplicate default clause".to_owned()
    } else {
        format!("Remove duplicate switch case '{}'", found.label)
    };
    Some(CodeAction {
        title,
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_class_member_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::class_members::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove earlier duplicate class member '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_destructuring_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::destructuring::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate destructuring binding '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_export_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::export_duplicates::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate export specifier '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_import_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::import_duplicates::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove duplicate import specifier '{}'", found.name),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn duplicate_parameter_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::semantic::duplicate_parameters(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.later_offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.later_offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.later_offset - line_start + 1 == column
        })?;
    if found.name.starts_with('_') {
        return None;
    }
    let candidate = format!("_{}", found.name);
    if !crate::highlights::whole_word_ranges(text, &candidate, 1).is_empty() {
        return None;
    }
    Some(CodeAction {
        title: format!("Prefix earlier duplicate '{}' with underscore", found.name),
        range: found.first_offset..found.first_offset + found.name.len(),
        replacement: candidate,
    })
}

fn duplicate_object_key_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let found = crate::object_keys::duplicates(text)
        .into_iter()
        .find(|found| {
            let found_line = text[..found.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..found.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            found_line == line && found.offset - line_start + 1 == column
        })?;
    Some(CodeAction {
        title: format!("Remove earlier duplicate key '{}'", found.key),
        range: found.removal?,
        replacement: String::new(),
    })
}

fn syntax_actions(text: &str, line: usize, column: usize, message: &str) -> Vec<CodeAction> {
    if let Some(rest) = message.strip_prefix("Unexpected '")
        && let Some(closing) = rest.chars().next()
        && matches!(closing, '}' | ']' | ')')
    {
        let Some((line_start, _, content)) = line_bounds(text, line) else {
            return Vec::new();
        };
        let relative = column.saturating_sub(1);
        if relative >= content.len() || !content[relative..].starts_with(closing) {
            return Vec::new();
        }
        return vec![CodeAction {
            title: format!("Remove '{closing}'"),
            range: line_start + relative..line_start + relative + closing.len_utf8(),
            replacement: String::new(),
        }];
    }
    if let Some(rest) = message.strip_prefix("Unclosed '")
        && let Some(opening) = rest.chars().next()
    {
        let closing = match opening {
            '{' => "}",
            '[' => "]",
            '(' => ")",
            _ => return Vec::new(),
        };
        return vec![append_at_end(text, closing)];
    }
    if message == "Unterminated block comment" {
        return vec![append_at_end(text, "*/")];
    }
    if message == "Unterminated string" {
        let Some((line_start, line_end, content)) = line_bounds(text, line) else {
            return Vec::new();
        };
        let relative = column.saturating_sub(1);
        let Some(quote) = content[relative..]
            .chars()
            .next()
            .filter(|ch| matches!(ch, '"' | '\'' | '`'))
        else {
            return Vec::new();
        };
        let _ = line_start;
        return vec![CodeAction {
            title: "Add closing quote".into(),
            range: line_end..line_end,
            replacement: quote.to_string(),
        }];
    }
    Vec::new()
}

fn matching_paren(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in line[open..].char_indices() {
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

fn missing_return_actions(text: &str, line: usize, column: usize) -> Vec<CodeAction> {
    let Some((line_start, _, content)) = line_bounds(text, line) else {
        return Vec::new();
    };
    let relative = column.saturating_sub(1);
    if relative > content.len() || !content.is_char_boundary(relative) {
        return Vec::new();
    }
    let name_len = content[relative..]
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if name_len == 0 {
        return Vec::new();
    }
    let Some(function) = content[..relative].rfind("function") else {
        return Vec::new();
    };
    let async_function = content[..function].split_whitespace().last() == Some("async");
    let Some(open) = content[function..].find('(').map(|index| function + index) else {
        return Vec::new();
    };
    let Some(close) = matching_paren(content, open) else {
        return Vec::new();
    };
    let rest = &content[close + 1..];
    let Some(brace) = rest.find('{') else {
        return Vec::new();
    };
    let annotation = rest[..brace].trim();
    let Some(annotation) = annotation.strip_prefix(':').map(str::trim) else {
        return Vec::new();
    };
    if annotation.is_empty() {
        return Vec::new();
    }
    let annotation_leading = rest[..brace].len() - rest[..brace].trim_start().len();
    let colon = close + 1 + annotation_leading;
    let annotation_start = colon
        + 1
        + (content[colon + 1..close + 1 + brace].len()
            - content[colon + 1..close + 1 + brace].trim_start().len());
    let annotation_end = annotation_start + annotation.len();
    let replacement = if async_function {
        "Promise<void>"
    } else {
        "void"
    };
    vec![
        CodeAction {
            title: format!("Change return type to '{replacement}'"),
            range: line_start + annotation_start..line_start + annotation_end,
            replacement: replacement.into(),
        },
        CodeAction {
            title: "Remove return type annotation".into(),
            range: line_start + colon..line_start + annotation_end,
            replacement: String::new(),
        },
    ]
}

fn type_mismatch_actions(text: &str, line: usize, column: usize, message: &str) -> Vec<CodeAction> {
    let Some((line_start, _, content)) = line_bounds(text, line) else {
        return Vec::new();
    };
    let relative = column.saturating_sub(1);
    if relative > content.len() || !content.is_char_boundary(relative) {
        return Vec::new();
    }
    let Some(equals) = content[..relative].rfind('=') else {
        return Vec::new();
    };
    let mut base = content.len() - content.trim_start().len();
    let mut trimmed = content.trim_start();
    if let Some(after) = trimmed.strip_prefix("export ") {
        base += "export ".len();
        trimmed = after;
    }
    let keyword_len = ["const ", "let ", "var "]
        .into_iter()
        .find(|keyword| trimmed.starts_with(keyword))
        .map(str::len)
        .unwrap_or(0);
    if keyword_len == 0 {
        return Vec::new();
    }
    let name_start = base + keyword_len;
    let name_end = content[name_start..equals]
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| name_start + index + ch.len_utf8())
        .unwrap_or(name_start);
    let Some(colon) = content[name_end..equals]
        .find(':')
        .map(|index| name_end + index)
    else {
        return Vec::new();
    };
    let annotation_start = colon
        + 1
        + (content[colon + 1..equals].len() - content[colon + 1..equals].trim_start().len());
    let annotation = content[colon + 1..equals].trim();
    if annotation.is_empty() {
        return Vec::new();
    }
    let annotation_end = annotation_start + annotation.len();
    let literal = message
        .strip_prefix("Type '")
        .and_then(|rest| rest.split("' is not assignable").next())
        .unwrap_or("");
    let mut actions = Vec::new();
    if matches!(literal, "string" | "number" | "boolean" | "null")
        && simple_primitive_annotation(annotation)
    {
        actions.push(CodeAction {
            title: format!("Change type to '{literal}'"),
            range: line_start + annotation_start..line_start + annotation_end,
            replacement: literal.to_owned(),
        });
    }
    actions.push(CodeAction {
        title: "Remove type annotation".into(),
        range: line_start + colon..line_start + annotation_end,
        replacement: String::new(),
    });
    actions
}

fn duplicate_declaration_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let (line_start, _, content) = line_bounds(text, line)?;
    let offset = line_start + column.checked_sub(1)?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let name_len = text[offset..]
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &text[offset..offset + name_len];
    if name.is_empty() || name.starts_with('_') {
        return None;
    }
    let relative = offset - line_start;
    let before = content[..relative.min(content.len())].trim_end();
    let keyword = before.split_whitespace().last();
    if !matches!(keyword, Some("const" | "let" | "class" | "type")) {
        return None;
    }
    let after = text[offset + name_len..].chars().next();
    if !after.is_some_and(|ch| {
        ch.is_whitespace() || matches!(ch, '=' | ':' | '(' | '{' | ';' | '<' | '?')
    }) {
        return None;
    }
    for suffix in 2..=100 {
        let candidate = format!("{name}{suffix}");
        if crate::highlights::whole_word_ranges(text, &candidate, 1).is_empty() {
            return Some(CodeAction {
                title: format!("Rename duplicate '{name}' to '{candidate}'"),
                range: offset..offset + name_len,
                replacement: candidate,
            });
        }
    }
    None
}

fn unused_parameter_action(text: &str, line: usize, column: usize) -> Option<CodeAction> {
    let (start, _, content) = line_bounds(text, line)?;
    let offset = start + column.checked_sub(1)?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let tail = &text[offset..];
    let name_len = tail
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let name = &tail[..name_len];
    if name.is_empty() || name == "this" || name.starts_with('_') {
        return None;
    }
    let before = text[..offset].chars().next_back();
    let after = tail[name_len..].chars().next();
    let parameter_context = before.is_some_and(|ch| matches!(ch, '(' | ',' | ' ' | '\t' | '.'))
        && after.is_some_and(|ch| matches!(ch, ':' | ',' | ')' | '=' | '?'));
    if !parameter_context || !content.contains(name) {
        return None;
    }
    Some(CodeAction {
        title: format!("Prefix '{name}' with underscore"),
        range: offset..offset + name.len(),
        replacement: format!("_{name}"),
    })
}

#[cfg(test)]
pub(super) fn actions(text: &str, diagnostics: &[Diagnostic], line: usize) -> Vec<CodeAction> {
    let key = vela_core::document::DocumentKey::new(String::new(), String::new());
    actions_with_files(&key, text, diagnostics, line, &[])
}

pub(super) fn actions_with_files(
    current_key: &vela_core::document::DocumentKey,
    text: &str,
    diagnostics: &[Diagnostic],
    line: usize,
    files: &[(vela_core::document::DocumentKey, String)],
) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.line == line)
    {
        let derived = match diagnostic.kind {
            DiagnosticKind::UnusedDeclaration => unused_declaration_action(text, line)
                .into_iter()
                .collect::<Vec<_>>(),
            DiagnosticKind::UnusedImport => unused_import_action(text, line, diagnostic.column)
                .into_iter()
                .collect(),
            DiagnosticKind::DuplicateDeclaration => {
                duplicate_declaration_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::UnusedParameter => {
                unused_parameter_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::TypeMismatch => {
                type_mismatch_actions(text, line, diagnostic.column, &diagnostic.message)
            }
            DiagnosticKind::MissingReturn => missing_return_actions(text, line, diagnostic.column),
            DiagnosticKind::Syntax => {
                syntax_actions(text, line, diagnostic.column, &diagnostic.message)
            }
            DiagnosticKind::ArgumentCount => argument_actions(text, line, diagnostic.column),
            DiagnosticKind::DuplicateObjectKey => {
                duplicate_object_key_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateParameter => {
                duplicate_parameter_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateImportSpecifier => {
                duplicate_import_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateExportSpecifier => {
                duplicate_export_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateDestructuringBinding => {
                duplicate_destructuring_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateClassMember => {
                duplicate_class_member_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateSwitchCase => {
                duplicate_switch_case_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateEnumMember => {
                duplicate_enum_member_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateInterfaceMember => {
                duplicate_interface_member_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateUnionMember => {
                duplicate_union_member_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateTypeLiteralProperty => {
                duplicate_type_literal_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::DuplicateTypeParameter => {
                duplicate_type_parameter_action(text, line, diagnostic.column)
                    .into_iter()
                    .collect()
            }
            DiagnosticKind::Validation => Vec::new(),
            DiagnosticKind::UnresolvedIdentifier => {
                crate::auto_import::import_action(current_key, text, files, line, diagnostic.column)
                    .map(|(title, statement)| CodeAction {
                        title,
                        range: {
                            let offset = crate::auto_import::insert_offset(text);
                            offset..offset
                        },
                        replacement: statement,
                    })
                    .into_iter()
                    .collect()
            }
        };
        for action in derived {
            if !actions.iter().any(|existing: &CodeAction| {
                existing.range == action.range && existing.title == action.title
            }) {
                actions.push(action);
            }
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(kind: DiagnosticKind, line: usize, column: usize) -> Diagnostic {
        Diagnostic {
            source: "JavaScript",
            kind,
            line,
            column,
            message: String::new(),
        }
    }

    #[test]
    fn unused_declaration_removal_includes_the_newline() {
        let text = "const answer = 42;\nconsole.log('x');\n";
        let found = actions(
            text,
            &[diagnostic(DiagnosticKind::UnusedDeclaration, 1, 7)],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].range, 0..19);
        assert!(found[0].replacement.is_empty());

        let last = "const answer = 42;";
        let found = actions(
            last,
            &[diagnostic(DiagnosticKind::UnusedDeclaration, 1, 7)],
            1,
        );
        assert_eq!(found[0].range, 0..last.len());

        for text in [
            "const a = 1, b = 2;\n",
            "const value = {\n  a: 1,\n};\n",
            "export const value = 1;\n",
        ] {
            assert!(
                actions(
                    text,
                    &[diagnostic(DiagnosticKind::UnusedDeclaration, 1, 1)],
                    1
                )
                .is_empty(),
                "{text:?}"
            );
        }
    }

    #[test]
    fn single_binding_unused_imports_remove_the_whole_line() {
        let text = "import { unused } from \"pkg\";\nconsole.log('x');\n";
        let found = actions(text, &[diagnostic(DiagnosticKind::UnusedImport, 1, 10)], 1);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].range, 0..30);
        assert!(found[0].title.contains("Remove unused import"));

        let multiple = "import { used, unused, other } from \"pkg\";\n";
        let found = actions(
            multiple,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 16)],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(&multiple[found[0].range.clone()], " unused,");
        let mut edited = multiple.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used, other } from \"pkg\";\n");

        let last = "import { used, unused } from \"pkg\";\n";
        let found = actions(last, &[diagnostic(DiagnosticKind::UnusedImport, 1, 16)], 1);
        assert_eq!(&last[found[0].range.clone()], ", unused");
        let mut edited = last.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used } from \"pkg\";\n");

        let first = "import { unused, used } from \"pkg\";\n";
        let found = actions(first, &[diagnostic(DiagnosticKind::UnusedImport, 1, 10)], 1);
        let mut edited = first.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used } from \"pkg\";\n");

        let default_and_named = "import Default, { used, unused } from \"pkg\";\n";
        let found = actions(
            default_and_named,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 25)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = default_and_named.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import Default, { used } from \"pkg\";\n");

        let only_named = "import Default, { unused } from \"pkg\";\n";
        let found = actions(
            only_named,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 19)],
            1,
        );
        let mut edited = only_named.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import Default from \"pkg\";\n");

        let unused_default = "import Unused, { used } from \"pkg\";\n";
        let found = actions(
            unused_default,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 8)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = unused_default.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used } from \"pkg\";\n");

        let unused_default_type = "import type Unused, { Used } from \"pkg\";\n";
        let found = actions(
            unused_default_type,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 13)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = unused_default_type.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import type { Used } from \"pkg\";\n");

        let unused_namespace = "import Default, * as ns from \"pkg\";\n";
        let found = actions(
            unused_namespace,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 22)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = unused_namespace.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import Default from \"pkg\";\n");

        let unused_default_namespace = "import Unused, * as ns from \"pkg\";\n";
        let found = actions(
            unused_default_namespace,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 8)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = unused_default_namespace.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import * as ns from \"pkg\";\n");
        let commented = "import { used, unused /* keep */ } from \"pkg\";\n";
        assert!(
            actions(
                commented,
                &[diagnostic(DiagnosticKind::UnusedImport, 1, 16)],
                1
            )
            .is_empty(),
            "commented clauses are not rewritten"
        );
        let wrong_column = "import { unused } from \"pkg\";\n";
        assert!(
            actions(
                wrong_column,
                &[diagnostic(DiagnosticKind::UnusedImport, 1, 1)],
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn multi_line_imports_remove_whole_specifier_lines() {
        let text = "import {\n  used,\n  unused,\n} from \"pkg\";\n";
        let found = actions(text, &[diagnostic(DiagnosticKind::UnusedImport, 3, 3)], 3);
        assert_eq!(found.len(), 1);
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  used,\n} from \"pkg\";\n");

        let no_comma = "import {\n  used,\n  unused\n} from \"pkg\";\n";
        let found = actions(
            no_comma,
            &[diagnostic(DiagnosticKind::UnusedImport, 3, 3)],
            3,
        );
        let mut edited = no_comma.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  used,\n} from \"pkg\";\n");

        let shared = "import {\n  used, unused\n} from \"pkg\";\n";
        let found = actions(shared, &[diagnostic(DiagnosticKind::UnusedImport, 2, 9)], 2);
        assert_eq!(found.len(), 1);
        let mut edited = shared.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  used\n} from \"pkg\";\n");

        let first_shared = "import {\n  unused, used,\n} from \"pkg\";\n";
        let found = actions(
            first_shared,
            &[diagnostic(DiagnosticKind::UnusedImport, 2, 3)],
            2,
        );
        let mut edited = first_shared.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  used,\n} from \"pkg\";\n");

        let middle_shared = "import {\n  a, unused, c,\n} from \"pkg\";\n";
        let found = actions(
            middle_shared,
            &[diagnostic(DiagnosticKind::UnusedImport, 2, 6)],
            2,
        );
        let mut edited = middle_shared.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  a, c,\n} from \"pkg\";\n");

        let next_line = "import\n{\n  used,\n  unused,\n} from \"pkg\";\n";
        let found = actions(
            next_line,
            &[diagnostic(DiagnosticKind::UnusedImport, 4, 3)],
            4,
        );
        assert_eq!(found.len(), 1);
        let mut edited = next_line.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import\n{\n  used,\n} from \"pkg\";\n");

        let next_line_default = "import Unused,\n{\n  used,\n} from \"pkg\";\n";
        let found = actions(
            next_line_default,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 8)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = next_line_default.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import\n{\n  used,\n} from \"pkg\";\n");

        let default = "import Unused, {\n  used,\n} from \"pkg\";\n";
        let found = actions(
            default,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 8)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = default.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import {\n  used,\n} from \"pkg\";\n");
    }

    #[test]
    fn imports_with_from_on_later_lines_remove_specifiers() {
        let text = "import { used, unused }\nfrom \"pkg\";\n";
        let found = actions(text, &[diagnostic(DiagnosticKind::UnusedImport, 1, 16)], 1);
        assert_eq!(found.len(), 1);
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used }\nfrom \"pkg\";\n");

        let default = "import Unused, { used }\nfrom \"pkg\";\n";
        let found = actions(
            default,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 8)],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = default.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { used }\nfrom \"pkg\";\n");

        let later = "const before = 1;\nimport { used, unused }\nfrom \"pkg\";\n";
        let found = actions(later, &[diagnostic(DiagnosticKind::UnusedImport, 2, 16)], 2);
        assert_eq!(found.len(), 1);
        let mut edited = later.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(
            edited,
            "const before = 1;\nimport { used }\nfrom \"pkg\";\n"
        );

        let from_newline = "import { used, unused }\nfrom\n\"pkg\";\n";
        let found = actions(
            from_newline,
            &[diagnostic(DiagnosticKind::UnusedImport, 1, 16)],
            1,
        );
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn duplicate_type_parameters_remove_only_plain_duplicates() {
        let text = "function f<T, T>(value: T) { return value; }\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateTypeParameter,
                line: 1,
                column: 15,
                message: "Duplicate type parameter 'T' (first declared on line 1).".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate type parameter 'T'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "function f<T>(value: T) { return value; }\n");

        let constrained = "type A<T extends string, T extends string> = T;\n";
        assert!(
            actions(
                constrained,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateTypeParameter,
                    line: 1,
                    column: 29,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "constraints are not auto-removed"
        );

        let multiline = "function f<\n  T,\n  T,\n>(value: T) { return value; }\n";
        let found = actions(
            multiline,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateTypeParameter,
                line: 3,
                column: 3,
                message: String::new(),
            }],
            3,
        );
        assert_eq!(found.len(), 1);
        let mut edited = multiline.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "function f<\n  T,\n>(value: T) { return value; }\n");
    }

    #[test]
    fn duplicate_object_type_properties_remove_only_identical_members() {
        let text = "type Options = {\n  read: string;\n  write: string;\n  read: string;\n};\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateTypeLiteralProperty,
                line: 4,
                column: 3,
                message: "Duplicate object type property 'read' (first declared on line 2).".into(),
            }],
            4,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].title,
            "Remove duplicate object type property 'read'"
        );
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(
            edited,
            "type Options = {\n  read: string;\n  write: string;\n};\n"
        );

        let differing = "type A = { read: string; read: number };\n";
        assert!(
            actions(
                differing,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateTypeLiteralProperty,
                    line: 1,
                    column: 28,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_union_literals_remove_the_later_member() {
        let text = "type Mode = \"read\" | \"write\" | \"read\";\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateUnionMember,
                line: 1,
                column: 32,
                message: "Duplicate union member '\"read\"' (first declared on line 1).".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate union member '\"read\"'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "type Mode = \"read\" | \"write\";\n");

        let middle = "type M = \"a\" | \"a\" | \"b\";\n";
        let found = actions(
            middle,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateUnionMember,
                line: 1,
                column: 16,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = middle.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "type M = \"a\" | \"b\";\n");

        let multiline = "type Mode =\n  | \"read\"\n  | \"read\";\n";
        let found = actions(
            multiline,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateUnionMember,
                line: 3,
                column: 5,
                message: String::new(),
            }],
            3,
        );
        assert_eq!(found.len(), 1);
        let mut edited = multiline.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "type Mode =\n  | \"read\"\n;\n");
    }

    #[test]
    fn duplicate_interface_properties_remove_only_identical_members() {
        let text = "interface Reader {\n  read: string;\n  write: string;\n  read: string;\n}\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateInterfaceMember,
                line: 4,
                column: 3,
                message: "Duplicate interface property 'read' (first declared on line 2).".into(),
            }],
            4,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate interface property 'read'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(
            edited,
            "interface Reader {\n  read: string;\n  write: string;\n}\n"
        );

        let differing = "interface A { read: string; read: number; }\n";
        assert!(
            actions(
                differing,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateInterfaceMember,
                    line: 1,
                    column: 29,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );

        let methods = "interface A { read(): void; read(x: string): void; }\n";
        assert!(
            actions(
                methods,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateInterfaceMember,
                    line: 1,
                    column: 26,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "method overloads expose no diagnostic"
        );
    }

    #[test]
    fn duplicate_enum_members_remove_only_identical_literal_duplicates() {
        let text = "enum E { A = 1, B = 2, A = 1 }\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::DuplicateEnumMember,
                line: 1,
                column: 24,
                message: "Duplicate enum member 'A' (first declared on line 1).".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove later duplicate enum member 'A'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "enum E { A = 1, B = 2 }\n");

        let values = "enum E { A = 1, B = 1 }\n";
        assert!(
            actions(
                values,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateEnumMember,
                    line: 1,
                    column: 16,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "value aliases are not auto-removed"
        );

        let different = "enum E { A = 1, A = 2 }\n";
        assert!(
            actions(
                different,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::DuplicateEnumMember,
                    line: 1,
                    column: 16,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_switch_cases_remove_only_empty_labels() {
        let text = "switch (value) {\n  case 1:\n  case 1:\n  case 2:\n    work();\n}\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateSwitchCase,
                line: 3,
                column: 8,
                message: "Duplicate switch case label '1'.".into(),
            }],
            3,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate switch case '1'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(
            edited,
            "switch (value) {\n  case 1:\n  case 2:\n    work();\n}\n"
        );

        let populated = "switch (value) {\n  case 1:\n    break;\n  case 1:\n    work();\n}\n";
        assert!(
            actions(
                populated,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateSwitchCase,
                    line: 4,
                    column: 8,
                    message: String::new(),
                }],
                4
            )
            .is_empty(),
            "clauses with statements are never auto-removed"
        );

        let duplicate_default = "switch (value) { default: break; default: case 2: work(); }\n";
        let found = actions(
            duplicate_default,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateSwitchCase,
                line: 1,
                column: 34,
                message: "Duplicate switch default clause.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate default clause");
    }

    #[test]
    fn duplicate_class_members_remove_the_earlier_definition() {
        let text = "class Reader { read() { return 1; } read() { return 2; } }\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateClassMember,
                line: 1,
                column: 37,
                message: "Duplicate class member 'read'.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].title,
            "Remove earlier duplicate class member 'read'"
        );
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "class Reader { read() { return 2; } }\n");

        let field = "class A { value = 1; value = 2; }\n";
        let found = actions(
            field,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateClassMember,
                line: 1,
                column: 22,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = field.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "class A { value = 2; }\n");

        let constructor = "class A { constructor() {} constructor() {} }\n";
        assert!(
            actions(
                constructor,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateClassMember,
                    line: 1,
                    column: 29,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "constructors are never auto-removed"
        );
    }

    #[test]
    fn duplicate_destructuring_bindings_remove_the_redundant_property() {
        let text = "const { read, read } = source;\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateDestructuringBinding,
                line: 1,
                column: 15,
                message: "Duplicate destructuring binding 'read'.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].title,
            "Remove duplicate destructuring binding 'read'"
        );
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "const { read } = source;\n");

        let last = "const { a, b, a } = source;\n";
        let found = actions(
            last,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateDestructuringBinding,
                line: 1,
                column: 15,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = last.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "const { a, b } = source;\n");

        let defaulted = "const { a, a = 1 } = source;\n";
        assert!(
            actions(
                defaulted,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateDestructuringBinding,
                    line: 1,
                    column: 15,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "defaults are not redundantly removable"
        );
    }

    #[test]
    fn duplicate_export_specifiers_remove_the_redundant_binding() {
        let text = "export { read, read };\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateExportSpecifier,
                line: 1,
                column: 16,
                message: "Duplicate export name 'read'.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate export specifier 'read'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "export { read };\n");

        let last = "export { a, b, a };\n";
        let found = actions(
            last,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateExportSpecifier,
                line: 1,
                column: 16,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = last.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "export { a, b };\n");

        let aliased = "export { a as b, c as b };\n";
        assert!(
            actions(
                aliased,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateExportSpecifier,
                    line: 1,
                    column: 23,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "different locals are not safely removable"
        );
    }

    #[test]
    fn duplicate_import_specifiers_remove_the_redundant_binding() {
        let text = "import { read, read } from \"./io\";\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateImportSpecifier,
                line: 1,
                column: 16,
                message: "Duplicate import binding 'read'.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove duplicate import specifier 'read'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { read } from \"./io\";\n");

        let middle = "import { a, a, b } from \"./pkg\";\n";
        let found = actions(
            middle,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateImportSpecifier,
                line: 1,
                column: 13,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = middle.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "import { a, b } from \"./pkg\";\n");

        let aliased = "import { a, b as a } from \"./pkg\";\n";
        assert!(
            actions(
                aliased,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateImportSpecifier,
                    line: 1,
                    column: 17,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "different exports are not safely removable"
        );
    }

    #[test]
    fn duplicate_parameters_prefix_the_shadowed_parameter() {
        let text = "function pick(value, value) {\n  return value;\n}\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateParameter,
                line: 1,
                column: 22,
                message: "Duplicate parameter 'value' (the later parameter wins).".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].title,
            "Prefix earlier duplicate 'value' with underscore"
        );
        assert_eq!(found[0].range, 14..19);
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "_value");
        assert_eq!(
            edited,
            "function pick(_value, value) {\n  return value;\n}\n"
        );

        let collision = "function pick(value, value) {\n  const _value = 1;\n  return value;\n}\n";
        assert!(
            actions(
                collision,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateParameter,
                    line: 1,
                    column: 22,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "an existing _value binding blocks the rename"
        );

        let method = "class A { read(value, value) { return value; } }\n";
        let found = actions(
            method,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateParameter,
                line: 1,
                column: 23,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = method.to_owned();
        edited.replace_range(found[0].range.clone(), "_value");
        assert_eq!(
            edited,
            "class A { read(_value, value) { return value; } }\n"
        );

        let underscore = "function pick(_value, _value) {}\n";
        assert!(
            actions(
                underscore,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateParameter,
                    line: 1,
                    column: 23,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );

        assert!(
            actions(
                text,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateParameter,
                    line: 1,
                    column: 15,
                    message: String::new(),
                }],
                1
            )
            .is_empty(),
            "the first occurrence is not actionable"
        );
    }

    #[test]
    fn duplicate_object_keys_remove_the_earlier_literal_property() {
        let text = "const options = {\n  size: 1,\n  color: 'red',\n  size: 2,\n};\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateObjectKey,
                line: 4,
                column: 3,
                message: "Duplicate object key 'size' (first key on line 2).".into(),
            }],
            4,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove earlier duplicate key 'size'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(
            edited,
            "const options = {\n  color: 'red',\n  size: 2,\n};\n"
        );

        let inline = "const options = { size: 1, size: 2 };\n";
        let found = actions(
            inline,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::DuplicateObjectKey,
                line: 1,
                column: 28,
                message: String::new(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        let mut edited = inline.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "const options = { size: 2 };\n");

        let unsafe_value = "const options = { size: compute(), size: 2 };\n";
        assert!(
            actions(
                unsafe_value,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateObjectKey,
                    line: 1,
                    column: 36,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );
        assert!(
            actions(
                text,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::DuplicateObjectKey,
                    line: 2,
                    column: 3,
                    message: String::new(),
                }],
                2
            )
            .is_empty(),
            "first occurrences are not actionable"
        );
    }

    #[test]
    fn argument_count_actions_fill_or_trim_arguments() {
        let text = "function f(a, b) { return a + b; }\nf(1);\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::ArgumentCount,
                line: 2,
                column: 1,
                message: "Function 'f' expects 2 arguments but got 1.".into(),
            }],
            2,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Add 1 missing argument");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), &found[0].replacement);
        assert_eq!(
            edited,
            "function f(a, b) { return a + b; }\nf(1, undefined);\n"
        );

        let zero = "function f(a, b) {}\nf();\n";
        let found = actions(
            zero,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::ArgumentCount,
                line: 2,
                column: 1,
                message: "Function 'f' expects 2 arguments but got 0.".into(),
            }],
            2,
        );
        let mut edited = zero.to_owned();
        edited.replace_range(found[0].range.clone(), &found[0].replacement);
        assert_eq!(edited, "function f(a, b) {}\nf(undefined, undefined);\n");

        let extra = "function f(a, b) { return a + b; }\nf(1, 2, 3, 4);\n";
        let found = actions(
            extra,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::ArgumentCount,
                line: 2,
                column: 1,
                message: "Function 'f' expects at most 2 arguments but got 4.".into(),
            }],
            2,
        );
        assert_eq!(found[0].title, "Remove 2 extra arguments");
        let mut edited = extra.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "function f(a, b) { return a + b; }\nf(1, 2);\n");
    }

    #[test]
    fn syntax_actions_close_or_remove_delimiters_and_quotes() {
        let unclosed = "function f() {\n  work();\n";
        let found = actions(
            unclosed,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::Syntax,
                line: 1,
                column: 14,
                message: "Unclosed '{'".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Add '}'");
        let mut edited = unclosed.to_owned();
        edited.replace_range(found[0].range.clone(), &found[0].replacement);
        assert_eq!(edited, "function f() {\n  work();\n}\n");

        let unexpected = "export let x = 1;\n}\n";
        let found = actions(
            unexpected,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::Syntax,
                line: 2,
                column: 1,
                message: "Unexpected '}'".into(),
            }],
            2,
        );
        assert_eq!(found[0].title, "Remove '}'");
        let mut edited = unexpected.to_owned();
        edited.replace_range(found[0].range.clone(), "");
        assert_eq!(edited, "export let x = 1;\n\n");

        let string = "const a = 'x;\n";
        let found = actions(
            string,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::Syntax,
                line: 1,
                column: 11,
                message: "Unterminated string".into(),
            }],
            1,
        );
        assert_eq!(found[0].title, "Add closing quote");
        let mut edited = string.to_owned();
        edited.replace_range(found[0].range.clone(), &found[0].replacement);
        assert_eq!(edited, "const a = 'x;'\n");

        let comment = "/* open";
        let found = actions(
            comment,
            &[Diagnostic {
                source: "JavaScript",
                kind: DiagnosticKind::Syntax,
                line: 1,
                column: 1,
                message: "Unterminated block comment".into(),
            }],
            1,
        );
        assert_eq!(found[0].title, "Add '*/'");
        let mut edited = comment.to_owned();
        edited.replace_range(found[0].range.clone(), &found[0].replacement);
        assert_eq!(edited, "/* open\n*/");

        assert!(
            actions(
                unexpected,
                &[Diagnostic {
                    source: "JavaScript",
                    kind: DiagnosticKind::Syntax,
                    line: 2,
                    column: 2,
                    message: "Unexpected '}'".into(),
                }],
                2
            )
            .is_empty()
        );
    }

    #[test]
    fn missing_return_actions_change_or_remove_the_annotation() {
        let text = "export function greet(): string {\n  console.log('hi');\n}\n";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::MissingReturn,
                line: 1,
                column: 17,
                message: "Function returns 'string' but has no return statement.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].title, "Change return type to 'void'");
        assert_eq!(&text[found[0].range.clone()], "string");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "void");
        assert_eq!(
            edited,
            "export function greet(): void {\n  console.log('hi');\n}\n"
        );
        let mut edited = text.to_owned();
        edited.replace_range(found[1].range.clone(), "");
        assert_eq!(
            edited,
            "export function greet() {\n  console.log('hi');\n}\n"
        );

        let async_text = "async function load(): Promise<string> {\n  await work();\n}\n";
        let found = actions(
            async_text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::MissingReturn,
                line: 1,
                column: 16,
                message: "Function returns 'Promise<string>' but has no return statement.".into(),
            }],
            1,
        );
        assert_eq!(found[0].title, "Change return type to 'Promise<void>'");
        assert_eq!(found[0].replacement, "Promise<void>");

        assert!(
            actions(
                text,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::MissingReturn,
                    line: 1,
                    column: 1,
                    message: String::new(),
                }],
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn type_mismatch_actions_change_or_remove_the_annotation() {
        let text = "const name: string = 42;\n";
        let message = "Type 'number' is not assignable to type 'string'.";
        let found = actions(
            text,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::TypeMismatch,
                line: 1,
                column: 22,
                message: message.into(),
            }],
            1,
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].title, "Change type to 'number'");
        assert_eq!(&text[found[0].range.clone()], "string");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "number");
        assert_eq!(edited, "const name: number = 42;\n");
        assert_eq!(found[1].title, "Remove type annotation");
        let mut edited = text.to_owned();
        edited.replace_range(found[1].range.clone(), "");
        assert_eq!(edited, "const name = 42;\n");

        let union = "export let value: string | undefined = 42;\n";
        let found = actions(
            union,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::TypeMismatch,
                line: 1,
                column: 41,
                message: message.into(),
            }],
            1,
        );
        assert_eq!(found[0].replacement, "number");

        let complex = "const values: Array<string> = [1];\n";
        let found = actions(
            complex,
            &[Diagnostic {
                source: "TypeScript",
                kind: DiagnosticKind::TypeMismatch,
                line: 1,
                column: 31,
                message: "Type 'array' is not assignable to type 'Array<string>'.".into(),
            }],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Remove type annotation");

        assert!(
            actions(
                text,
                &[Diagnostic {
                    source: "TypeScript",
                    kind: DiagnosticKind::TypeMismatch,
                    line: 1,
                    column: 1,
                    message: message.into(),
                }],
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_declarations_rename_only_the_second_declaration() {
        let text = "const value = 1;\nconst value = 2;\n";
        let found = actions(
            text,
            &[diagnostic(DiagnosticKind::DuplicateDeclaration, 2, 7)],
            2,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].range, 23..28);
        assert_eq!(found[0].replacement, "value2");
        assert_eq!(found[0].title, "Rename duplicate 'value' to 'value2'");
        let mut edited = text.to_owned();
        edited.replace_range(found[0].range.clone(), "value2");
        assert_eq!(edited, "const value = 1;\nconst value2 = 2;\n");

        let occupied = "const value2 = 0;\nconst value = 1;\nconst value = 2;\n";
        let found = actions(
            occupied,
            &[diagnostic(DiagnosticKind::DuplicateDeclaration, 3, 7)],
            3,
        );
        assert_eq!(found[0].replacement, "value3");

        let class = "class Item {}\nclass Item {}\n";
        let found = actions(
            class,
            &[diagnostic(DiagnosticKind::DuplicateDeclaration, 2, 7)],
            2,
        );
        assert_eq!(found[0].replacement, "Item2");

        assert!(
            actions(
                text,
                &[diagnostic(DiagnosticKind::DuplicateDeclaration, 1, 7)],
                2
            )
            .is_empty(),
            "the action belongs to the diagnostic line"
        );
        assert!(
            actions(
                text,
                &[diagnostic(DiagnosticKind::DuplicateDeclaration, 2, 1)],
                2
            )
            .is_empty(),
            "a non-identifier column is refused"
        );
        assert!(actions(text, &[diagnostic(DiagnosticKind::TypeMismatch, 2, 7)], 2).is_empty());
    }

    #[test]
    fn unused_parameter_prefixes_only_the_identifier() {
        let text = "function greet(name, age) {\n  return name;\n}\n";
        let found = actions(
            text,
            &[diagnostic(DiagnosticKind::UnusedParameter, 1, 22)],
            1,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].range, 21..24);
        assert_eq!(found[0].replacement, "_age");
        assert_eq!(found[0].title, "Prefix 'age' with underscore");

        assert!(
            actions(
                text,
                &[diagnostic(DiagnosticKind::UnusedParameter, 1, 1)],
                1
            )
            .is_empty()
        );
        assert!(actions(text, &[diagnostic(DiagnosticKind::TypeMismatch, 1, 22)], 1).is_empty());
        assert!(
            actions(
                text,
                &[diagnostic(DiagnosticKind::UnusedParameter, 1, 22)],
                2
            )
            .is_empty(),
            "actions belong to the diagnostic line"
        );
    }
}
