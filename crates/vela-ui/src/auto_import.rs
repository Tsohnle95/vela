//! Bounded unresolved-identifier diagnostics and auto-import quick fixes.
//!
//! Uppercase identifiers are considered anywhere; lowercase identifiers only
//! when called and uniquely exported as a function. This keeps the heuristic
//! conservative while covering components, classes, types and utility calls.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use vela_core::document::DocumentKey;

use crate::diagnostics::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug)]
pub(super) struct ExportTarget {
    pub specifier: String,
    pub default: bool,
    pub function: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Unresolved {
    pub name: String,
    pub offset: usize,
    pub target: ExportTarget,
}

fn value_is_function(value: &str) -> bool {
    let mut value = value.trim_start();
    if let Some(rest) = value.strip_prefix("async ") {
        value = rest.trim_start();
    }
    if let Some(rest) = value.strip_prefix("function") {
        return rest
            .chars()
            .next()
            .is_none_or(|ch| ch == '*' || ch == '(' || ch.is_whitespace());
    }
    if value.starts_with('(') {
        let chars = value.chars().collect::<Vec<_>>();
        let mut depth = 0i32;
        let mut quote = None;
        let mut escaped = false;
        for (index, ch) in chars.iter().copied().enumerate() {
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
                        let tail = chars[index + 1..].iter().collect::<String>();
                        let tail = tail.trim_start();
                        if tail.starts_with("=>") {
                            return true;
                        }
                        return tail.starts_with(':') && tail.contains("=>");
                    }
                }
                _ => {}
            }
        }
        return false;
    }
    let name_len = value
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    name_len > 0 && value[name_len..].trim_start().starts_with("=>")
}

fn exported_name(line: &str) -> Option<(String, bool, bool)> {
    let mut rest = line.strip_prefix("export ")?;
    let mut default = false;
    if let Some(after) = rest.strip_prefix("default ") {
        default = true;
        rest = after;
    }
    if let Some(after) = rest.strip_prefix("async ") {
        rest = after;
    }
    let mut keyword_len = 0;
    let mut function = false;
    for keyword in [
        "function",
        "class",
        "const",
        "let",
        "var",
        "type",
        "interface",
        "enum",
    ] {
        if rest.starts_with(keyword)
            && rest[keyword.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            keyword_len = keyword.len();
            function = keyword == "function";
            break;
        }
    }
    if keyword_len == 0 {
        return None;
    }
    let tail = rest[keyword_len..].trim_start();
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
    let function = function
        || (matches!(rest[..keyword_len].trim(), "const" | "let" | "var")
            && tail[name_len..]
                .find('=')
                .map(|equals| value_is_function(&tail[name_len + equals + 1..]))
                .unwrap_or(false));
    Some((name.to_owned(), default, function))
}

fn local_names(text: &str) -> HashSet<String> {
    let mut names = crate::symbols::declarations(text)
        .into_iter()
        .map(|declaration| declaration.name)
        .collect::<HashSet<_>>();
    for line in text.split('\n') {
        for (_, name) in crate::semantic::import_bindings(line) {
            names.insert(name);
        }
    }
    names
}

/// Blank strings as well as comments so prose cannot trigger an import.
fn code_mask(text: &str) -> String {
    let masked = crate::semantic::mask_comments(text);
    let chars = masked.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(masked.len());
    let mut quote = None;
    let mut escaped = false;
    for ch in chars {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            if ch == '\n' {
                out.push('\n');
            } else {
                out.extend(std::iter::repeat_n(' ', ch.len_utf8()));
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn relative_specifier(current: &str, target: &str) -> String {
    let current_dir = Path::new(current).parent().unwrap_or(Path::new(""));
    let current_parts = current_dir
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    let target_path = Path::new(target);
    let target_parts = target_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    let common = current_parts
        .iter()
        .zip(&target_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; current_parts.len() - common];
    parts.extend(target_parts[common..].iter().copied());
    let mut joined = parts.join("/");
    for extension in [".tsx", ".ts", ".jsx", ".js", ".mts", ".cts", ".mjs", ".cjs"] {
        if let Some(stripped) = joined.strip_suffix(extension) {
            joined = stripped.to_owned();
            break;
        }
    }
    if joined.starts_with("..") {
        joined
    } else {
        format!("./{joined}")
    }
}

pub(super) fn unresolved(
    current_key: &DocumentKey,
    text: &str,
    files: &[(DocumentKey, String)],
) -> Vec<Unresolved> {
    let mut exports = HashMap::<String, Vec<ExportTarget>>::new();
    for (key, other) in files {
        if key == current_key {
            continue;
        }
        for line in other.split('\n') {
            if !line.starts_with("export ") {
                continue;
            }
            if let Some((name, default, function)) = exported_name(line) {
                exports.entry(name).or_default().push(ExportTarget {
                    specifier: relative_specifier(&current_key.rel, &key.rel),
                    default,
                    function,
                });
            }
        }
    }
    let local = local_names(text);
    let globals = [
        "Array",
        "Boolean",
        "Date",
        "Error",
        "Function",
        "JSON",
        "Map",
        "Math",
        "Number",
        "Object",
        "Promise",
        "RegExp",
        "Set",
        "String",
        "Symbol",
        "WeakMap",
        "WeakSet",
        "console",
        "document",
        "globalThis",
        "window",
        "fetch",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
        "queueMicrotask",
        "structuredClone",
        "parseInt",
        "parseFloat",
        "isNaN",
        "isFinite",
        "encodeURIComponent",
        "decodeURIComponent",
        "require",
    ];
    let masked = code_mask(text);
    let chars = masked.char_indices().collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for (index, (offset, ch)) in chars.iter().copied().enumerate() {
        if !ch.is_alphabetic() {
            continue;
        }
        let uppercase = ch.is_uppercase();
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
        let name = &masked[start..end];
        if !crate::rename::valid_identifier(name)
            || local.contains(name)
            || globals.contains(&name)
            || seen.contains(name)
        {
            continue;
        }
        if masked[..start].trim_end().ends_with('.') {
            continue;
        }
        let after = masked[end..].trim_start();
        if after.starts_with(':') || after.starts_with('=') {
            continue;
        }
        if !uppercase && (name.chars().count() < 2 || !after.starts_with('(')) {
            continue;
        }
        let Some(targets) = exports.get(name) else {
            continue;
        };
        if targets.len() != 1 {
            continue;
        }
        if !uppercase && !targets[0].function {
            continue;
        }
        seen.insert(name.to_owned());
        found.push(Unresolved {
            name: name.to_owned(),
            offset: start,
            target: targets[0].clone(),
        });
    }
    found
}

fn location(text: &str, offset: usize) -> (usize, usize) {
    let line = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    let start = text[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    (line, offset - start + 1)
}

pub(super) fn diagnostics(
    current_key: &DocumentKey,
    text: &str,
    files: &[(DocumentKey, String)],
) -> Vec<Diagnostic> {
    unresolved(current_key, text, files)
        .into_iter()
        .map(|issue| {
            let (line, column) = location(text, issue.offset);
            Diagnostic {
                source: "Module",
                kind: DiagnosticKind::UnresolvedIdentifier,
                line,
                column,
                message: format!("Cannot find name '{}'.", issue.name),
            }
        })
        .collect()
}

/// Insert after a shebang or a single `"use strict"` / `'use strict'`
/// directive so auto-imports never split a preamble.
pub(super) fn insert_offset(text: &str) -> usize {
    let mut offset = 0;
    if text.starts_with("#!") {
        offset = text.find('\n').map(|index| index + 1).unwrap_or(text.len());
    }
    let rest = &text[offset..];
    let leading = rest.len() - rest.trim_start().len();
    let trimmed = rest.trim_start();
    let line_end = trimmed.find('\n').unwrap_or(trimmed.len());
    let line = trimmed[..line_end].trim_end();
    if matches!(
        line,
        "\"use strict\";" | "\"use strict\"" | "'use strict';" | "'use strict'"
    ) {
        offset += leading + line_end;
        if text[offset..].starts_with('\n') {
            offset += 1;
        }
    }
    offset
}

pub(super) fn import_action(
    current_key: &DocumentKey,
    text: &str,
    files: &[(DocumentKey, String)],
    line: usize,
    column: usize,
) -> Option<(String, String)> {
    let offset = location_to_offset(text, line, column)?;
    let issue = unresolved(current_key, text, files)
        .into_iter()
        .find(|issue| issue.offset == offset)?;
    let statement = if issue.target.default {
        format!(
            "import {} from \"{}\";\n",
            issue.name, issue.target.specifier
        )
    } else {
        format!(
            "import {{ {} }} from \"{}\";\n",
            issue.name, issue.target.specifier
        )
    };
    let title = format!("Import '{}' from '{}'", issue.name, issue.target.specifier);
    Some((title, statement))
}

fn location_to_offset(text: &str, line: usize, column: usize) -> Option<usize> {
    if line == 0 || column == 0 {
        return None;
    }
    let mut start = 0;
    for _ in 1..line {
        start += text[start..].find('\n')? + 1;
    }
    let offset = start + column - 1;
    (offset <= text.len() && text.is_char_boundary(offset)).then_some(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> DocumentKey {
        DocumentKey::new("/ws".into(), name.into())
    }

    fn files() -> Vec<(DocumentKey, String)> {
        vec![
            (key("app.tsx"), "export const view = Widget();\n".into()),
            (
                key("widget.tsx"),
                "export function Widget() { return null; }\n".into(),
            ),
        ]
    }

    #[test]
    fn unique_uppercase_exports_produce_unresolved_issues() {
        let files = files();
        let found = unresolved(&files[0].0, &files[0].1, &files);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Widget");
        assert_eq!(found[0].target.specifier, "./widget");
        assert!(!found[0].target.default);

        let diagnostics = diagnostics(&files[0].0, &files[0].1, &files);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 21);
        assert_eq!(diagnostics[0].message, "Cannot find name 'Widget'.");
    }

    #[test]
    fn locals_imports_members_keys_strings_and_ambiguous_exports_are_excluded() {
        let widget = key("widget.tsx");
        let exported = "export function Widget() {}\n".to_owned();
        let cases = [
            ("const Widget = 1;\nexport const view = Widget;\n", false),
            (
                "import { Widget } from \"./widget\";\nexport const view = Widget();\n",
                false,
            ),
            ("export const view = obj.Widget;\n", false),
            ("export const view = { Widget: 1 };\n", false),
            ("export const view = \"Widget\";\n", false),
            ("export const view = Widget();\n", true),
        ];
        for (source, expected) in cases {
            let files = vec![
                (key("app.tsx"), source.into()),
                (widget.clone(), exported.clone()),
            ];
            assert_eq!(
                !unresolved(&files[0].0, &files[0].1, &files).is_empty(),
                expected,
                "{source:?}"
            );
        }
        let duplicate = vec![
            (key("app.tsx"), "export const view = Widget();\n".into()),
            (widget.clone(), exported.clone()),
            (key("other.tsx"), exported),
        ];
        assert!(unresolved(&duplicate[0].0, &duplicate[0].1, &duplicate).is_empty());
    }

    #[test]
    fn insertion_skips_shebang_and_use_strict_preamble() {
        assert_eq!(insert_offset("export const value = 1;\n"), 0);
        assert_eq!(insert_offset("\"use strict\";\nnext();\n"), 14);
        assert_eq!(insert_offset("'use strict'\nnext();\n"), 13);
        assert_eq!(insert_offset("  \"use strict\";\nnext();\n"), 16);
        assert_eq!(
            insert_offset("#!/usr/bin/env node\nexport const value = 1;\n"),
            20
        );
        assert_eq!(
            insert_offset("#!/usr/bin/env node\n\"use strict\";\nvalue();\n"),
            34
        );
        assert_eq!(insert_offset("#!/usr/bin/env node"), 19);
    }

    #[test]
    fn lowercase_names_auto_import_only_when_called_and_exported_as_functions() {
        let app = key("app.ts");
        let util = key("util.ts");
        let call = "export const value = helper();\n";
        let export = "export function helper() { return 1; }\n";
        let files: Vec<(DocumentKey, String)> =
            vec![(app.clone(), call.into()), (util.clone(), export.into())];
        let found = unresolved(&app, call, &files);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "helper");
        assert_eq!(
            import_action(&app, call, &files, 1, 22).unwrap().1,
            "import { helper } from \"./util\";\n"
        );

        let not_called = "export const value = helper;\n";
        let files: Vec<(DocumentKey, String)> = vec![
            (app.clone(), not_called.into()),
            (util.clone(), export.into()),
        ];
        assert!(unresolved(&app, not_called, &files).is_empty());

        let arrow = "export const helper = () => 1;\n";
        let files: Vec<(DocumentKey, String)> =
            vec![(app.clone(), call.into()), (util.clone(), arrow.into())];
        assert_eq!(unresolved(&app, call, &files)[0].name, "helper");
        assert_eq!(
            import_action(&app, call, &files, 1, 22).unwrap().1,
            "import { helper } from \"./util\";\n"
        );
        for export in [
            "export const helper = async (value: number) => value;\n",
            "export const helper = function () { return 1; };\n",
            "export const helper = async function () { return 1; };\n",
        ] {
            let files: Vec<(DocumentKey, String)> =
                vec![(app.clone(), call.into()), (util.clone(), export.into())];
            assert_eq!(
                unresolved(&app, call, &files)[0].name,
                "helper",
                "{export:?}"
            );
        }
        for export in [
            "export const helper = 1;\n",
            "export const helper = condition ? (() => 1) : 2;\n",
        ] {
            let files: Vec<(DocumentKey, String)> =
                vec![(app.clone(), call.into()), (util.clone(), export.into())];
            assert!(unresolved(&app, call, &files).is_empty(), "{export:?}");
        }

        let member = "export const value = object.helper();\n";
        let files: Vec<(DocumentKey, String)> =
            vec![(app.clone(), member.into()), (util.clone(), export.into())];
        assert!(unresolved(&app, member, &files).is_empty());

        let default_export = "export default function helper() { return 1; }\n";
        let files: Vec<(DocumentKey, String)> =
            vec![(app.clone(), call.into()), (util, default_export.into())];
        assert_eq!(
            import_action(&app, call, &files, 1, 22).unwrap().1,
            "import helper from \"./util\";\n"
        );
    }

    #[test]
    fn import_action_uses_relative_paths_and_default_exports() {
        let files = files();
        let found = import_action(&files[0].0, &files[0].1, &files, 1, 21).unwrap();
        assert_eq!(found.0, "Import 'Widget' from './widget'");
        assert_eq!(found.1, "import { Widget } from \"./widget\";\n");

        let default_files: Vec<(DocumentKey, String)> = vec![
            (key("src/app.tsx"), "export const view = Panel();\n".into()),
            (
                key("src/ui/panel.tsx"),
                "export default function Panel() {}\n".into(),
            ),
        ];
        let found = import_action(
            &default_files[0].0,
            &default_files[0].1,
            &default_files,
            1,
            21,
        )
        .unwrap();
        assert_eq!(found.0, "Import 'Panel' from './ui/panel'");
        assert_eq!(found.1, "import Panel from \"./ui/panel\";\n");

        let nested: Vec<(DocumentKey, String)> = vec![
            (
                key("src/ui/app.tsx"),
                "export const view = Widget();\n".into(),
            ),
            (
                key("src/widget.tsx"),
                "export function Widget() {}\n".into(),
            ),
        ];
        let found = import_action(&nested[0].0, &nested[0].1, &nested, 1, 21).unwrap();
        assert_eq!(found.0, "Import 'Widget' from '../widget'");
    }
}
