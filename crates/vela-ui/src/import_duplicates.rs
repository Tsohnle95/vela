//! Bounded duplicate binding checks for single-line ES imports.
//!
//! A safe quick fix exists only for two identical named specifiers
//! (`a`/`a` or `a as b`/`a as b`); dropping the later redundant specifier
//! preserves which module export is bound. Default/namespace collisions and
//! aliases that bind different exports are reported without an action.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateImport {
    pub name: String,
    /// Byte offset of the later local binding.
    pub offset: usize,
    /// Document range removing the redundant later specifier, when safe.
    pub removal: Option<Range<usize>>,
}

#[derive(Clone)]
struct Binding {
    name: String,
    canonical: String,
    type_only: bool,
    /// Byte offset of the local binding within the line.
    offset: usize,
    named: Option<NamedSpecifier>,
}

#[derive(Clone)]
struct NamedSpecifier {
    start: usize,
    end: usize,
}

fn import_clause(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("import")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let from = rest.rfind(" from ")?;
    let clause = &rest[..from];
    let clause_leading = clause.len() - clause.trim_start().len();
    let clause = clause.trim_start();
    Some((leading + "import".len() + clause_leading, clause))
}

/// Identifier or quoted imported module name.
fn imported_name(text: &str) -> Option<String> {
    let text = text.trim();
    let first = text.chars().next()?;
    if matches!(first, '"' | '\'') {
        let close = text[1..].find(first)? + 1;
        let inner = &text[1..close];
        if inner.is_empty() || inner.contains('\n') {
            return None;
        }
        return Some(inner.to_owned());
    }
    crate::rename::valid_identifier(text).then(|| text.to_owned())
}

/// Imported name, local binding, local offset within the trimmed specifier and
/// whether the specifier is type-only. Quoted imported names require `as`.
fn import_specifier(specifier: &str) -> Option<(String, String, usize, bool)> {
    let type_only = specifier.starts_with("type ");
    let rest = specifier.strip_prefix("type ").unwrap_or(specifier);
    let (imported_text, local_text, local_offset) = match rest.split_once(" as ") {
        Some((imported, local)) => {
            let local_leading = local.len() - local.trim_start().len();
            (
                imported.trim(),
                local.trim(),
                imported.len() + " as ".len() + local_leading,
            )
        }
        None => {
            if matches!(rest.trim().chars().next(), Some('"' | '\'')) {
                return None;
            }
            let leading = rest.len() - rest.trim_start().len();
            (rest.trim(), rest.trim(), leading)
        }
    };
    let imported = imported_name(imported_text)?;
    if !crate::rename::valid_identifier(local_text) {
        return None;
    }
    Some((imported, local_text.to_owned(), local_offset, type_only))
}

fn parse_named(clause: &str, clause_offset: usize, bindings: &mut Vec<Binding>) {
    let Some(open) = clause.find('{') else {
        return;
    };
    let Some(close) = clause.rfind('}') else {
        return;
    };
    if close <= open {
        return;
    }
    let inner = &clause[open + 1..close];
    let inner_offset = clause_offset + open + 1;
    let mut cursor = 0;
    for part in inner.split(',') {
        let trimmed = part.trim();
        let leading = part.len() - part.trim_start().len();
        if !trimmed.is_empty() {
            let start = inner_offset + cursor + leading;
            let end = start + trimmed.len();
            if let Some((imported, local, local_offset, type_only)) = import_specifier(trimmed) {
                bindings.push(Binding {
                    name: local.clone(),
                    canonical: format!(
                        "{}|{}|{}",
                        if type_only { "type" } else { "value" },
                        imported,
                        local
                    ),
                    type_only,
                    offset: start + local_offset,
                    named: Some(NamedSpecifier { start, end }),
                });
            }
        }
        cursor += part.len() + 1;
    }
}

fn line_bindings(line: &str) -> Vec<Binding> {
    let Some((clause_offset, clause)) = import_clause(line) else {
        return Vec::new();
    };
    let mut bindings = Vec::new();
    let rest = clause;
    if rest.starts_with('{') {
        parse_named(rest, clause_offset, &mut bindings);
        return bindings;
    }
    if let Some(namespace) = rest.strip_prefix("* as ") {
        let name = namespace.trim();
        if crate::rename::valid_identifier(name) {
            bindings.push(Binding {
                name: name.to_owned(),
                canonical: format!("*|{name}"),
                type_only: false,
                offset: clause_offset + clause.find(name).unwrap_or(0),
                named: None,
            });
        }
        return bindings;
    }
    let (default, tail) = rest.split_once(',').unwrap_or((rest, ""));
    let default = default.trim();
    if crate::rename::valid_identifier(default) {
        bindings.push(Binding {
            name: default.to_owned(),
            canonical: format!("default|{default}"),
            type_only: false,
            offset: clause_offset + clause.find(default).unwrap_or(0),
            named: None,
        });
    }
    let tail = tail.trim_start();
    let tail_offset = clause_offset + clause.find(tail).unwrap_or(0);
    if tail.starts_with('{') {
        parse_named(tail, tail_offset, &mut bindings);
    } else if let Some(namespace) = tail.strip_prefix("* as ") {
        let name = namespace.trim();
        if crate::rename::valid_identifier(name) {
            bindings.push(Binding {
                name: name.to_owned(),
                canonical: format!("*|{name}"),
                type_only: false,
                offset: tail_offset + tail.find(name).unwrap_or(0),
                named: None,
            });
        }
    }
    bindings
}

fn named_removal(line: &str, later: &NamedSpecifier) -> Option<Range<usize>> {
    if line[later.end..].starts_with(',') {
        let mut end = later.end + 1;
        while line[end..].starts_with(' ') {
            end += 1;
        }
        return Some(later.start..end);
    }
    // The later specifier is last: remove the comma that precedes it.
    let comma = line[..later.start].rfind(',')?;
    Some(comma..later.end)
}

/// Duplicate local bindings within one single-line import statement.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateImport> {
    let masked = crate::semantic::mask_comments(text);
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in masked.split('\n') {
        let bindings = line_bindings(line);
        let mut seen = HashMap::<String, Binding>::new();
        for binding in bindings {
            if let Some(first) = seen.get(&binding.name) {
                if first.type_only != binding.type_only {
                    continue;
                }
                let removal = match (&first.named, &binding.named) {
                    (Some(_), Some(later)) if first.canonical == binding.canonical => {
                        named_removal(line, later)
                            .map(|range| line_start + range.start..line_start + range.end)
                    }
                    _ => None,
                };
                found.push(DuplicateImport {
                    name: binding.name,
                    offset: line_start + binding.offset,
                    removal,
                });
            } else {
                seen.insert(binding.name.clone(), binding);
            }
        }
        line_start += line.len() + 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_named_specifiers_report_and_remove_the_redundant_one() {
        let text = "import { read, read } from \"./io\";\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "import { read } from \"./io\";\n");

        let middle = "import { a, a, b } from \"./pkg\";\n";
        let removal = duplicates(middle)[0].removal.clone().unwrap();
        let mut edited = middle.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "import { a, b } from \"./pkg\";\n");

        let last = "import { a, b, a } from \"./pkg\";\n";
        let removal = duplicates(last)[0].removal.clone().unwrap();
        let mut edited = last.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "import { a, b } from \"./pkg\";\n");

        let alias = "import { x as y, x as y } from \"./pkg\";\n";
        let removal = duplicates(alias)[0].removal.clone().unwrap();
        let mut edited = alias.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "import { x as y } from \"./pkg\";\n");

        let type_only = "import { type X, type X } from \"./pkg\";\n";
        assert!(duplicates(type_only)[0].removal.is_some());
    }

    #[test]
    fn different_export_bindings_are_reported_without_an_action() {
        let aliased = "import { a, b as a } from \"./pkg\";\n";
        assert_eq!(duplicates(aliased).len(), 1);
        assert!(duplicates(aliased)[0].removal.is_none());

        let default_named = "import x, { x } from \"./pkg\";\n";
        assert_eq!(duplicates(default_named).len(), 1);
        assert!(duplicates(default_named)[0].removal.is_none());

        let namespace_named = "import * as x, { x } from \"./pkg\";\n";
        let _ = namespace_named;
        assert!(duplicates("import * as x from \"./a\";\n").is_empty());
        assert!(
            duplicates("import x, * as x from \"./pkg\";\n")[0]
                .removal
                .is_none()
        );
    }

    #[test]
    fn distinct_names_multiple_statements_and_comments_are_bounded() {
        assert!(duplicates("import { a, b } from \"./pkg\";\n").is_empty());
        assert!(duplicates("import { a } from \"./a\";\nimport { a } from \"./b\";\n").is_empty());
        assert!(duplicates("// import { a, a } from \"./pkg\";\n").is_empty());
        assert!(duplicates("import { a /* a */, b } from \"./pkg\";\n").is_empty());
        assert!(duplicates("import { a,\na } from \"./pkg\";\n").is_empty());
        assert!(duplicates("import \"./side-effect\";\n").is_empty());
        assert!(duplicates("const a = 1;\n").is_empty());
    }

    #[test]
    fn quoted_imported_names_compare_locals_and_remove_when_redundant() {
        let identical = "import { \"a\" as read, \"a\" as read } from \"./pkg\";\n";
        let found = duplicates(identical);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = identical.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "import { \"a\" as read } from \"./pkg\";\n");

        let different = "import { \"a\" as read, \"b\" as read } from \"./pkg\";\n";
        assert_eq!(duplicates(different).len(), 1);
        assert!(duplicates(different)[0].removal.is_none());

        let distinct_locals = "import { \"a\" as read, \"a\" as write } from \"./pkg\";\n";
        assert!(duplicates(distinct_locals).is_empty());

        let mixed = "import { 'a' as read, \"a\" as read } from \"./pkg\";\n";
        assert_eq!(duplicates(mixed).len(), 1);
        assert!(duplicates(mixed)[0].removal.is_some());

        let unicode = "import { \"café\" as read, \"café\" as read } from \"./pkg\";\n";
        assert!(duplicates(unicode)[0].removal.is_some());

        let type_value = "import { type \"a\" as Read, \"a\" as Read } from \"./pkg\";\n";
        assert!(duplicates(type_value).is_empty());

        assert!(
            duplicates("import { \"a\", \"a\" } from \"./pkg\";\n").is_empty(),
            "quoted imports without a local alias are invalid"
        );
    }

    #[test]
    fn unicode_and_type_value_specifiers_are_handled() {
        let text = "import { café, café } from \"./pkg\";\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert!(found[0].removal.is_some());

        assert!(duplicates("import { type X, X } from \"./pkg\";\n").is_empty());
    }
}
