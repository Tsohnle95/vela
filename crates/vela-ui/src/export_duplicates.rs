//! Bounded duplicate export-name checks for single-line named exports.
//!
//! `export { a, a }` is a syntax error; removing the later redundant specifier
//! preserves the exported binding. Aliases that export the same name from
//! different locals (`export { a as b, c as b }`) are reported without an
//! action because either value could be intended.

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct DuplicateExport {
    pub name: String,
    /// Byte offset of the later exported name.
    pub offset: usize,
    /// Document range removing the redundant later specifier, when safe.
    pub removal: Option<Range<usize>>,
}

#[derive(Clone)]
struct Binding {
    name: String,
    canonical: String,
    type_only: bool,
    offset: usize,
    start: usize,
    end: usize,
}

/// Identifier or quoted module-export name with the offset of its first text
/// byte relative to `text`.
fn module_name(text: &str) -> Option<(String, usize)> {
    let text = text.trim();
    let first = text.chars().next()?;
    if matches!(first, '"' | '\'') {
        let close = text[1..].find(first)? + 1;
        let inner = &text[1..close];
        if inner.is_empty() || inner.contains('\n') {
            return None;
        }
        return Some((inner.to_owned(), 1));
    }
    crate::rename::valid_identifier(text).then(|| (text.to_owned(), 0))
}

fn specifier_binding(specifier: &str, start: usize) -> Option<Binding> {
    let trimmed = specifier.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('}') {
        return None;
    }
    let (type_only, rest) = match trimmed.strip_prefix("type ") {
        Some(rest) => (true, rest),
        None => (false, trimmed),
    };
    let type_prefix = trimmed.len() - rest.len();
    let (imported_text, exported_text, exported_in_rest) = match rest.split_once(" as ") {
        Some((imported, exported)) => (imported, exported, imported.len() + " as ".len()),
        None => (rest, rest, 0),
    };
    let (imported, _) = module_name(imported_text)?;
    let exported_leading = exported_text.len() - exported_text.trim_start().len();
    let (exported, exported_name_offset) = module_name(exported_text)?;
    let offset = start + type_prefix + exported_in_rest + exported_leading + exported_name_offset;
    Some(Binding {
        name: exported.clone(),
        canonical: format!(
            "{}|{}|{}",
            if type_only { "type" } else { "value" },
            imported,
            exported
        ),
        type_only,
        offset,
        start,
        end: start + trimmed.len(),
    })
}

fn export_bindings(line: &str) -> Vec<Binding> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let Some(rest) = trimmed.strip_prefix("export ") else {
        return Vec::new();
    };
    let Some(open) = rest.find('{') else {
        return Vec::new();
    };
    let Some(close) = rest.rfind('}') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    let inner = &rest[open + 1..close];
    let inner_offset = leading + "export ".len() + open + 1;
    let mut bindings = Vec::new();
    let mut cursor = 0;
    for part in inner.split(',') {
        let leading = part.len() - part.trim_start().len();
        if !part.trim().is_empty() {
            let start = inner_offset + cursor + leading;
            if let Some(binding) = specifier_binding(part, start) {
                bindings.push(binding);
            }
        }
        cursor += part.len() + 1;
    }
    bindings
}

fn named_removal(line: &str, later: &Binding) -> Option<Range<usize>> {
    if line[later.end..].starts_with(',') {
        let mut end = later.end + 1;
        while line[end..].starts_with(' ') {
            end += 1;
        }
        return Some(later.start..end);
    }
    let comma = line[..later.start].rfind(',')?;
    Some(comma..later.end)
}

/// Duplicate exported names within one single-line named export statement.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateExport> {
    let masked = crate::semantic::mask_comments(text);
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in masked.split('\n') {
        let mut seen = HashMap::<String, Binding>::new();
        for binding in export_bindings(line) {
            if let Some(first) = seen.get(&binding.name) {
                if first.type_only != binding.type_only {
                    continue;
                }
                let removal = (first.canonical == binding.canonical)
                    .then(|| named_removal(line, &binding))
                    .flatten()
                    .map(|range| line_start + range.start..line_start + range.end);
                found.push(DuplicateExport {
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
    fn identical_export_specifiers_report_and_remove_the_redundant_one() {
        let text = "export { read, read };\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export { read };\n");

        let middle = "export { a, a, b };\n";
        let removal = duplicates(middle)[0].removal.clone().unwrap();
        let mut edited = middle.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export { a, b };\n");

        let last = "export { a, b, a };\n";
        let removal = duplicates(last)[0].removal.clone().unwrap();
        let mut edited = last.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export { a, b };\n");

        let aliases = "export { x as y, x as y };\n";
        let removal = duplicates(aliases)[0].removal.clone().unwrap();
        let mut edited = aliases.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export { x as y };\n");

        let reexport = "export { a, a } from \"./pkg\";\n";
        assert!(duplicates(reexport)[0].removal.is_some());

        let identical_alias = "export { a, a as a };\n";
        assert!(duplicates(identical_alias)[0].removal.is_some());
    }

    #[test]
    fn different_locals_exporting_the_same_name_have_no_action() {
        let text = "export { a as b, c as b };\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "b");
        assert!(found[0].removal.is_none());

        let default_alias = "export { default as read, read };\n";
        assert_eq!(duplicates(default_alias).len(), 1);
        assert!(duplicates(default_alias)[0].removal.is_none());
    }

    #[test]
    fn distinct_names_other_statements_and_comments_are_bounded() {
        assert!(duplicates("export { a, b };\n").is_empty());
        assert!(duplicates("export { a as b, c as d };\n").is_empty());
        assert!(duplicates("export { a };\nexport { a };\n").is_empty());
        assert!(duplicates("// export { a, a };\n").is_empty());
        assert!(duplicates("export { a /* a */, b };\n").is_empty());
        assert!(duplicates("export { a,\na };\n").is_empty());
        assert!(duplicates("export * from \"./pkg\";\n").is_empty());
        assert!(duplicates("export * as ns from \"./pkg\";\n").is_empty());
        assert!(duplicates("const a = 1;\n").is_empty());
    }

    #[test]
    fn quoted_export_names_are_compared_and_removed_when_redundant() {
        let different = "export { a as \"read\", b as \"read\" };\n";
        let found = duplicates(different);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        assert!(found[0].removal.is_none());

        let identical = "export { a as \"read\", a as \"read\" };\n";
        let removal = duplicates(identical)[0].removal.clone().unwrap();
        let mut edited = identical.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export { a as \"read\" };\n");

        let mixed_quotes = "export { a as 'read', a as \"read\" };\n";
        assert_eq!(duplicates(mixed_quotes).len(), 1);
        assert!(duplicates(mixed_quotes)[0].removal.is_some());

        let unquoted = "export { a as \"read\", a as read };\n";
        assert_eq!(duplicates(unquoted).len(), 1);
        assert!(duplicates(unquoted)[0].removal.is_some());

        let reexport = "export { \"read\", \"read\" } from \"./pkg\";\n";
        assert_eq!(duplicates(reexport).len(), 1);
        assert!(duplicates(reexport)[0].removal.is_some());

        let unicode = "export { a as \"café\", b as \"café\" };\n";
        assert_eq!(duplicates(unicode).len(), 1);
        assert!(duplicates(unicode)[0].removal.is_none());

        assert!(duplicates("export { a as \"read\", b as \"write\" };\n").is_empty());
        assert!(duplicates("export { type a as \"T\", b as \"T\" };\n").is_empty());
    }

    #[test]
    fn type_only_and_value_exports_are_distinct() {
        assert!(duplicates("export { type X, X };\n").is_empty());
        let text = "export { type X, type X };\n";
        assert!(duplicates(text)[0].removal.is_some());
        let unicode = "export { café, café };\n";
        assert!(duplicates(unicode)[0].removal.is_some());
    }
}
