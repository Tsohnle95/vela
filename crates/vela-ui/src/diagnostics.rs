//! Bounded local language diagnostics for the native editor.
//!
//! JSON uses the real parser. JavaScript/TypeScript and CSS/SCSS use a
//! string/comment-aware delimiter scanner so unmatched brackets and unterminated
//! strings/comments surface without a language server or network request.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiagnosticKind {
    Syntax,
    TypeMismatch,
    DuplicateDeclaration,
    UnusedDeclaration,
    UnusedParameter,
    MissingReturn,
    UnusedImport,
    ArgumentCount,
    UnresolvedIdentifier,
    DuplicateObjectKey,
    DuplicateParameter,
    DuplicateImportSpecifier,
    DuplicateExportSpecifier,
    DuplicateDestructuringBinding,
    DuplicateClassMember,
    DuplicateSwitchCase,
    DuplicateEnumMember,
    DuplicateInterfaceMember,
    DuplicateUnionMember,
    DuplicateTypeLiteralProperty,
    DuplicateTypeParameter,
    Validation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Diagnostic {
    pub source: &'static str,
    pub kind: DiagnosticKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

pub(super) fn language_diagnostics(rel: &str, text: &str) -> Vec<Diagnostic> {
    let lower = rel.to_ascii_lowercase();
    match lower.rsplit('.').next().unwrap_or_default() {
        "json" => json_diagnostics(text),
        "js" | "mjs" | "cjs" | "jsx" => script_diagnostics(text, "JavaScript", false, true),
        "ts" | "tsx" | "mts" | "cts" => {
            script_diagnostics(text, "TypeScript", true, !lower.ends_with(".d.ts"))
        }
        "css" => delimiters(text, false, "CSS"),
        "scss" => delimiters(text, true, "SCSS"),
        _ => Vec::new(),
    }
}

fn script_diagnostics(
    text: &str,
    source: &'static str,
    typed: bool,
    unused: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = delimiters(text, true, source);
    let semantic = if typed {
        let mut diagnostics = crate::semantic::type_diagnostics(text);
        diagnostics.extend(crate::semantic::missing_returns(text));
        diagnostics
    } else {
        Vec::new()
    };
    let unused_declarations = if unused {
        crate::semantic::unused_declarations(text)
    } else {
        Vec::new()
    };
    let unused_imports = if unused {
        crate::semantic::unused_imports(text)
    } else {
        Vec::new()
    };
    let unused_parameters = if unused {
        crate::semantic::unused_parameters(text)
    } else {
        Vec::new()
    };
    diagnostics.extend(
        semantic
            .into_iter()
            .chain(crate::semantic::duplicate_declarations(text))
            .chain(unused_declarations)
            .chain(unused_parameters)
            .chain(unused_imports)
            .map(|found| Diagnostic {
                source,
                kind: match found.kind {
                    crate::semantic::SemanticKind::TypeMismatch => DiagnosticKind::TypeMismatch,
                    crate::semantic::SemanticKind::DuplicateDeclaration => {
                        DiagnosticKind::DuplicateDeclaration
                    }
                    crate::semantic::SemanticKind::UnusedDeclaration => {
                        DiagnosticKind::UnusedDeclaration
                    }
                    crate::semantic::SemanticKind::UnusedParameter => {
                        DiagnosticKind::UnusedParameter
                    }
                    crate::semantic::SemanticKind::MissingReturn => DiagnosticKind::MissingReturn,
                    crate::semantic::SemanticKind::UnusedImport => DiagnosticKind::UnusedImport,
                },
                line: found.line,
                column: found.column,
                message: found.message,
            }),
    );
    if typed {
        diagnostics.extend(
            crate::type_parameters::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateTypeParameter,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!(
                            "Duplicate type parameter '{}' (first declared on line {}).",
                            found.name, found.first_line
                        ),
                    }
                }),
        );
        diagnostics.extend(
            crate::type_literals::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateTypeLiteralProperty,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!(
                            "Duplicate object type property '{}' (first declared on line {}).",
                            found.name, found.first_line
                        ),
                    }
                }),
        );
        diagnostics.extend(
            crate::type_unions::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateUnionMember,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!(
                            "Duplicate union member '{}' (first declared on line {}).",
                            found.label, found.first_line
                        ),
                    }
                }),
        );
        diagnostics.extend(
            crate::interfaces::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateInterfaceMember,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!(
                            "Duplicate interface property '{}' (first declared on line {}).",
                            found.name, found.first_line
                        ),
                    }
                }),
        );
        diagnostics.extend(crate::enums::issues(text).into_iter().map(|issue| {
            let line = text[..issue.offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = text[..issue.offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            Diagnostic {
                source,
                kind: DiagnosticKind::DuplicateEnumMember,
                line,
                column: issue.offset - line_start + 1,
                message: match issue.kind {
                    crate::enums::IssueKind::DuplicateName => format!(
                        "Duplicate enum member '{}' (first declared on line {}).",
                        issue.name, issue.first_line
                    ),
                    crate::enums::IssueKind::DuplicateValue => format!(
                        "Duplicate enum member value '{}' for '{}' (also used by '{}').",
                        issue.value_label.unwrap_or_default(),
                        issue.name,
                        issue.first_name.unwrap_or_default()
                    ),
                },
            }
        }));
    }
    if unused {
        diagnostics.extend(
            crate::calls::issues(text)
                .into_iter()
                .map(|issue| Diagnostic {
                    source,
                    kind: DiagnosticKind::ArgumentCount,
                    line: issue.line,
                    column: issue.column,
                    message: issue.message,
                }),
        );
        diagnostics.extend(
            crate::switch_cases::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateSwitchCase,
                        line,
                        column: found.offset - line_start + 1,
                        message: if found.label == "default" {
                            "Duplicate switch default clause.".into()
                        } else {
                            format!("Duplicate switch case label '{}'.", found.label)
                        },
                    }
                }),
        );
        diagnostics.extend(
            crate::class_members::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateClassMember,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!("Duplicate class member '{}'.", found.name),
                    }
                }),
        );
        diagnostics.extend(
            crate::destructuring::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateDestructuringBinding,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!("Duplicate destructuring binding '{}'.", found.name),
                    }
                }),
        );
        diagnostics.extend(
            crate::export_duplicates::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateExportSpecifier,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!("Duplicate export name '{}'.", found.name),
                    }
                }),
        );
        diagnostics.extend(
            crate::import_duplicates::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateImportSpecifier,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!("Duplicate import binding '{}'.", found.name),
                    }
                }),
        );
        diagnostics.extend(
            crate::semantic::duplicate_parameters(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.later_offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.later_offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateParameter,
                        line,
                        column: found.later_offset - line_start + 1,
                        message: format!(
                            "Duplicate parameter '{}' (the later parameter wins).",
                            found.name
                        ),
                    }
                }),
        );
        diagnostics.extend(
            crate::object_keys::duplicates(text)
                .into_iter()
                .map(|found| {
                    let line = text[..found.offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let line_start = text[..found.offset]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Diagnostic {
                        source,
                        kind: DiagnosticKind::DuplicateObjectKey,
                        line,
                        column: found.offset - line_start + 1,
                        message: format!(
                            "Duplicate object key '{}' (first key on line {}).",
                            found.key, found.first_line
                        ),
                    }
                }),
        );
    }
    diagnostics
}

pub(super) fn json_diagnostics(text: &str) -> Vec<Diagnostic> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => Vec::new(),
        Err(error) => vec![Diagnostic {
            source: "JSON",
            kind: DiagnosticKind::Syntax,
            line: error.line().max(1),
            column: error.column().max(1),
            message: error.to_string(),
        }],
    }
}

#[derive(Clone, Copy)]
enum State {
    String {
        quote: char,
        line: usize,
        column: usize,
    },
    BlockComment {
        line: usize,
        column: usize,
    },
    LineComment,
}

fn bump(chars: &[char], index: &mut usize, line: &mut usize, column: &mut usize) {
    if chars.get(*index) == Some(&'\n') {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
    *index += 1;
}

fn matching(open: char, close: char) -> bool {
    matches!((open, close), ('{', '}') | ('[', ']') | ('(', ')'))
}

fn delimiters(text: &str, line_comments: bool, source: &'static str) -> Vec<Diagnostic> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let mut stack = Vec::<(char, usize, usize)>::new();
    let mut state = None;
    let (mut index, mut line, mut column) = (0usize, 1usize, 1usize);
    while index < chars.len() {
        let ch = chars[index];
        match state {
            Some(State::String {
                quote,
                line: start_line,
                column: start_column,
            }) => {
                if ch == '\\' {
                    bump(&chars, &mut index, &mut line, &mut column);
                    if index < chars.len() {
                        bump(&chars, &mut index, &mut line, &mut column);
                    }
                } else if ch == quote {
                    bump(&chars, &mut index, &mut line, &mut column);
                    state = None;
                } else if ch == '\n' && quote != '`' {
                    diagnostics.push(Diagnostic {
                        source,
                        kind: DiagnosticKind::Syntax,
                        line: start_line,
                        column: start_column,
                        message: "Unterminated string".into(),
                    });
                    state = None;
                    bump(&chars, &mut index, &mut line, &mut column);
                } else {
                    bump(&chars, &mut index, &mut line, &mut column);
                }
            }
            Some(State::BlockComment {
                line: start_line,
                column: start_column,
            }) => {
                if ch == '*' && chars.get(index + 1) == Some(&'/') {
                    bump(&chars, &mut index, &mut line, &mut column);
                    bump(&chars, &mut index, &mut line, &mut column);
                    state = None;
                } else {
                    bump(&chars, &mut index, &mut line, &mut column);
                }
                let _ = (start_line, start_column);
            }
            Some(State::LineComment) => {
                if ch == '\n' {
                    state = None;
                }
                bump(&chars, &mut index, &mut line, &mut column);
            }
            None => {
                if line_comments && ch == '/' && chars.get(index + 1) == Some(&'/') {
                    bump(&chars, &mut index, &mut line, &mut column);
                    bump(&chars, &mut index, &mut line, &mut column);
                    state = Some(State::LineComment);
                } else if ch == '/' && chars.get(index + 1) == Some(&'*') {
                    let start = (line, column);
                    bump(&chars, &mut index, &mut line, &mut column);
                    bump(&chars, &mut index, &mut line, &mut column);
                    state = Some(State::BlockComment {
                        line: start.0,
                        column: start.1,
                    });
                } else if matches!(ch, '"' | '\'' | '`') {
                    let start = (line, column);
                    bump(&chars, &mut index, &mut line, &mut column);
                    state = Some(State::String {
                        quote: ch,
                        line: start.0,
                        column: start.1,
                    });
                } else if matches!(ch, '{' | '[' | '(') {
                    stack.push((ch, line, column));
                    bump(&chars, &mut index, &mut line, &mut column);
                } else if matches!(ch, '}' | ']' | ')') {
                    if stack.last().is_some_and(|(open, _, _)| matching(*open, ch)) {
                        stack.pop();
                    } else {
                        diagnostics.push(Diagnostic {
                            source,
                            kind: DiagnosticKind::Syntax,
                            line,
                            column,
                            message: format!("Unexpected '{ch}'"),
                        });
                    }
                    bump(&chars, &mut index, &mut line, &mut column);
                } else {
                    bump(&chars, &mut index, &mut line, &mut column);
                }
            }
        }
    }
    if let Some(State::String { line, column, .. }) = state {
        diagnostics.push(Diagnostic {
            source,
            kind: DiagnosticKind::Syntax,
            line,
            column,
            message: "Unterminated string".into(),
        });
    } else if let Some(State::BlockComment { line, column }) = state {
        diagnostics.push(Diagnostic {
            source,
            kind: DiagnosticKind::Syntax,
            line,
            column,
            message: "Unterminated block comment".into(),
        });
    }
    for (open, line, column) in stack.into_iter().rev().take(5) {
        diagnostics.push(Diagnostic {
            source,
            kind: DiagnosticKind::Syntax,
            line,
            column,
            message: format!("Unclosed '{open}'"),
        });
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_json_produces_no_diagnostics() {
        assert!(json_diagnostics("").is_empty());
        assert!(json_diagnostics("  \n\t").is_empty());
        assert!(json_diagnostics(r#"{"a":[1,2,{"b":true}]}"#).is_empty());
        assert!(json_diagnostics("null").is_empty());
    }

    #[test]
    fn syntax_errors_report_one_based_line_and_column() {
        let diagnostics = json_diagnostics("{\n  \"a\": ,\n}\n");
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.source, "JSON");
        assert_eq!(diagnostic.line, 2);
        assert!(diagnostic.column >= 8, "{diagnostic:?}");
        assert!(
            diagnostic.message.contains("expected value"),
            "{diagnostic:?}"
        );

        let trailing = json_diagnostics("{\n  \"a\": 1,\n}\n");
        assert_eq!(trailing.len(), 1);
        assert_eq!(trailing[0].line, 3);
        assert!(
            trailing[0].message.contains("trailing comma") || trailing[0].message.contains("key")
        );
    }

    #[test]
    fn unused_declarations_route_through_javascript_and_skip_declaration_files() {
        let diagnostics = language_diagnostics("app.js", "const answer = 42;\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].source, "JavaScript");
        assert!(diagnostics[0].message.contains("Unused declaration"));
        assert!(
            language_diagnostics("app.js", "const answer = 42;\nconsole.log(answer);").is_empty()
        );
        assert!(language_diagnostics("types.d.ts", "const answer: number;\n").is_empty());
        assert!(language_diagnostics("types.ts", "const answer: number;\n").len() == 1);
        assert!(
            language_diagnostics("app.js", "function greet(name, age) { return name; }")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Parameter 'age'"))
        );
        assert!(
            language_diagnostics("types.d.ts", "function f(value: string): void;").is_empty(),
            "declaration signatures do not report unused parameters"
        );
    }

    #[test]
    fn returned_method_and_anonymous_function_type_parameters_route() {
        let method = language_diagnostics(
            "types.ts",
            "class A { read<T, T>(value: T): T { return value; } }\n",
        );
        assert_eq!(method.len(), 1);
        assert_eq!(method[0].kind, DiagnosticKind::DuplicateTypeParameter);
        assert_eq!(method[0].column, 19);

        let anonymous = language_diagnostics(
            "types.ts",
            "export const f = function <T, T>(value: T) { return value; };\n",
        );
        assert_eq!(anonymous.len(), 1);
        assert_eq!(anonymous[0].column, 31);

        let returned_object = language_diagnostics(
            "types.ts",
            "class A { read<T, U>(value: T, other: U): { x: T } { return { x: value }; } }\n",
        );
        assert!(returned_object.is_empty());
    }

    #[test]
    fn arrow_and_method_type_parameters_route_through_typescript_diagnostics() {
        let arrow =
            language_diagnostics("types.ts", "export const f = <T, T>(value: T) => value;\n");
        assert_eq!(arrow.len(), 1);
        assert_eq!(arrow[0].kind, DiagnosticKind::DuplicateTypeParameter);
        assert_eq!(arrow[0].line, 1);
        assert_eq!(arrow[0].column, 22);

        let method = language_diagnostics(
            "types.ts",
            "class A { read<T, T>(value: T) { return value; } }\n",
        );
        assert_eq!(method.len(), 1);
        assert_eq!(method[0].column, 19);

        let object = language_diagnostics(
            "types.ts",
            "export const o = { read<T, T>(value: T) { return value; } };\n",
        );
        assert_eq!(object.len(), 1);

        assert!(
            language_diagnostics(
                "types.ts",
                "class A { read<T, U>(value: T, other: U) { return [value, other]; } }\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_type_parameters_route_through_typescript_diagnostics() {
        let function =
            language_diagnostics("types.ts", "function f<T, T>(value: T) { return value; }\n");
        assert_eq!(function.len(), 1);
        assert_eq!(function[0].kind, DiagnosticKind::DuplicateTypeParameter);
        assert_eq!(function[0].line, 1);
        assert_eq!(function[0].column, 15);
        assert_eq!(
            function[0].message,
            "Duplicate type parameter 'T' (first declared on line 1)."
        );

        let alias = language_diagnostics("types.ts", "type Pair<T, T> = [T, T];\n");
        assert_eq!(alias.len(), 1);
        assert_eq!(alias[0].column, 14);

        let constrained = language_diagnostics(
            "types.ts",
            "type A<T extends string, T extends string> = T;\n",
        );
        assert_eq!(constrained.len(), 1);
        assert_eq!(constrained[0].column, 26);

        assert!(
            language_diagnostics(
                "types.ts",
                "function f<T, U>(a: T, b: U) { return [a, b]; }\n"
            )
            .is_empty()
        );
        assert!(
            language_diagnostics("app.js", "function f<T, T>(value: T) { return value; }\n")
                .is_empty(),
            "type parameters are TypeScript-only"
        );
    }

    #[test]
    fn nested_object_type_properties_route_through_typescript_diagnostics() {
        let text = "type Options = {\n  outer: {\n    read: string;\n    write: string;\n    read: string;\n  };\n};\n";
        let diagnostics = language_diagnostics("types.ts", text);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateTypeLiteralProperty
        );
        assert_eq!(diagnostics[0].line, 5);
        assert_eq!(diagnostics[0].column, 5);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate object type property 'read' (first declared on line 3)."
        );

        let interface = language_diagnostics(
            "types.ts",
            "interface A { outer: { read: string; read: string } }\n",
        );
        assert_eq!(interface.len(), 1);
        assert_eq!(interface[0].column, 38);

        assert!(
            language_diagnostics(
                "types.ts",
                "type A = { outer: { read: string; write: string } };\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn signature_object_type_properties_route_through_typescript_diagnostics() {
        let parameter = "function f(options: {\n  read: string;\n  write: string;\n  read: string;\n}) {\n  return options;\n}\n";
        let diagnostics = language_diagnostics("types.ts", parameter);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateTypeLiteralProperty
        );
        assert_eq!(diagnostics[0].line, 4);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate object type property 'read' (first declared on line 2)."
        );

        let returned = language_diagnostics(
            "types.ts",
            "function f(): { read: string; read: string } { return value; }\n",
        );
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].column, 31);

        assert!(
            language_diagnostics("types.ts", "function f(x = { read: 1, read: 2 }) {}\n")
                .iter()
                .all(|diagnostic| {
                    diagnostic.kind != DiagnosticKind::DuplicateTypeLiteralProperty
                }),
            "the object initializer is not a type literal"
        );
        assert!(
            language_diagnostics(
                "app.js",
                "function f(options: { read: string; read: string }) { return options; }\n"
            )
            .is_empty(),
            "type annotations are TypeScript-only"
        );
    }

    #[test]
    fn annotation_object_type_properties_route_through_typescript_diagnostics() {
        let text = "const options: {\n  read: string;\n  write: string;\n  read: string;\n} = value;\nconsole.log(options);\n";
        let diagnostics = language_diagnostics("types.ts", text);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateTypeLiteralProperty
        );
        assert_eq!(diagnostics[0].line, 4);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate object type property 'read' (first declared on line 2)."
        );

        assert!(
            language_diagnostics(
                "types.ts",
                "const options: { read: string; write: string } = value;\nconsole.log(options);\n"
            )
            .is_empty()
        );
        assert!(
            language_diagnostics(
                "app.js",
                "const options: { read: string; read: string } = value;\nconsole.log(options);\n"
            )
            .is_empty(),
            "type annotations are TypeScript-only"
        );
    }

    #[test]
    fn duplicate_object_type_properties_route_through_typescript_diagnostics() {
        let text = "type Options = {\n  read: string;\n  write: string;\n  read: string;\n};\n";
        let diagnostics = language_diagnostics("types.ts", text);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateTypeLiteralProperty
        );
        assert_eq!(diagnostics[0].line, 4);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate object type property 'read' (first declared on line 2)."
        );

        assert!(
            language_diagnostics(
                "types.ts",
                "type A = { read(): void; read(x: string): void };\n"
            )
            .is_empty(),
            "method overloads are legal"
        );
        let differing =
            language_diagnostics("types.ts", "type A = { read: string; read: number };\n");
        assert_eq!(differing.len(), 1);
        assert!(
            language_diagnostics("types.ts", "type A = { read: string; write: string };\n")
                .is_empty()
        );
        assert!(
            language_diagnostics("app.js", "type A = { read: string; read: string };\n").is_empty(),
            "type literals are TypeScript-only"
        );
    }

    #[test]
    fn duplicate_union_members_route_through_typescript_diagnostics() {
        let inline =
            language_diagnostics("types.ts", "type Mode = \"read\" | \"write\" | \"read\";\n");
        assert_eq!(inline.len(), 1);
        assert_eq!(inline[0].kind, DiagnosticKind::DuplicateUnionMember);
        assert_eq!(inline[0].line, 1);
        assert_eq!(inline[0].column, 32);
        assert_eq!(
            inline[0].message,
            "Duplicate union member '\"read\"' (first declared on line 1)."
        );

        let multiline =
            language_diagnostics("types.ts", "type Mode =\n  | \"read\"\n  | \"read\";\n");
        assert_eq!(multiline.len(), 1);
        assert_eq!(multiline[0].line, 3);
        assert_eq!(multiline[0].column, 5);

        let numbers = language_diagnostics("types.ts", "type N = 1 | 2 | 1;\n");
        assert_eq!(numbers.len(), 1);
        assert_eq!(
            numbers[0].message,
            "Duplicate union member '1' (first declared on line 1)."
        );

        assert!(language_diagnostics("types.ts", "type M = \"a\" | \"b\";\n").is_empty());
        assert!(
            language_diagnostics("types.ts", "type R = Foo | Foo;\n").is_empty(),
            "non-literal type references are skipped"
        );
        assert!(
            language_diagnostics("app.js", "type M = \"a\" | \"a\";\n").is_empty(),
            "type aliases are TypeScript-only"
        );
    }

    #[test]
    fn duplicate_interface_properties_route_through_typescript_diagnostics() {
        let text = "interface Reader {\n  read: string;\n  write: string;\n  read: string;\n}\n";
        let diagnostics = language_diagnostics("types.ts", text);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateInterfaceMember
        );
        assert_eq!(diagnostics[0].line, 4);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate interface property 'read' (first declared on line 2)."
        );

        assert!(
            language_diagnostics(
                "types.ts",
                "interface A { read(): void; read(x: string): void; }\n"
            )
            .is_empty(),
            "method overloads are legal"
        );
        let differing =
            language_diagnostics("types.ts", "interface A { read: string; read: number; }\n");
        assert_eq!(differing.len(), 1);
        assert!(
            language_diagnostics("app.js", "interface A { read: string; read: string; }\n")
                .is_empty(),
            "interfaces are TypeScript-only"
        );
    }

    #[test]
    fn duplicate_enum_members_route_through_typescript_diagnostics() {
        let explicit = language_diagnostics("types.ts", "enum E { A = 1, B = 1 }\n");
        assert_eq!(explicit.len(), 1);
        assert_eq!(explicit[0].kind, DiagnosticKind::DuplicateEnumMember);
        assert_eq!(explicit[0].line, 1);
        assert_eq!(explicit[0].column, 17);
        assert_eq!(
            explicit[0].message,
            "Duplicate enum member value '1' for 'B' (also used by 'A')."
        );

        let auto = language_diagnostics("types.ts", "enum E { A = 0, B, C = 1 }\n");
        assert_eq!(auto.len(), 1);
        assert_eq!(auto[0].column, 20);
        assert_eq!(
            auto[0].message,
            "Duplicate enum member value '1' for 'C' (also used by 'B')."
        );

        let names = language_diagnostics("types.ts", "enum E { Up = 1, Down = 2, Up = 1 }\n");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].column, 28);
        assert_eq!(
            names[0].message,
            "Duplicate enum member 'Up' (first declared on line 1)."
        );

        assert!(
            language_diagnostics("app.js", "enum E { A = 1, B = 1 }\n").is_empty(),
            "enums are TypeScript-only"
        );
        assert!(language_diagnostics("types.ts", "enum E { A = 1, B = 2 }\n").is_empty());
    }

    #[test]
    fn duplicate_switch_cases_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "switch (value) {\n  case 1:\n  case 1:\n  case 2:\n    work();\n}\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateSwitchCase);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].column, 8);
        assert_eq!(diagnostics[0].message, "Duplicate switch case label '1'.");
        assert!(
            language_diagnostics(
                "app.js",
                "switch (value) {\n  case 1:\n    break;\n  case 2:\n    work();\n}\n"
            )
            .is_empty()
        );

        let default = language_diagnostics(
            "app.js",
            "switch (value) { default: break; default: case 2: work(); }\n",
        );
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].message, "Duplicate switch default clause.");
    }

    #[test]
    fn duplicate_multiline_class_members_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "class Reader {\n  read() { return 1; }\n  read() { return 2; }\n}\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateClassMember);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(diagnostics[0].message, "Duplicate class member 'read'.");
        assert!(
            language_diagnostics(
                "app.js",
                "class A {\n  get size() {}\n  set size(_v) {}\n}\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_class_expression_members_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "const A = class { read() {} read() {} };\nconsole.log(A);\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateClassMember);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 29);
        assert_eq!(diagnostics[0].message, "Duplicate class member 'read'.");
        assert!(
            language_diagnostics(
                "app.js",
                "const text = \"class { read() {} read() {} }\";\nconsole.log(text);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_class_members_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "class Reader { read() { return 1; } read() { return 2; } }\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateClassMember);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 37);
        assert_eq!(diagnostics[0].message, "Duplicate class member 'read'.");
        assert!(language_diagnostics("app.js", "class A { read() {} write() {} }\n").is_empty());
    }

    #[test]
    fn duplicate_destructuring_bindings_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics("app.js", "const { read, read } = source;\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateDestructuringBinding
        );
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 15);
        assert_eq!(
            diagnostics[0].message,
            "Duplicate destructuring binding 'read'."
        );
        assert!(language_diagnostics("app.js", "const { read, write } = source;\n").is_empty());
    }

    #[test]
    fn duplicate_quoted_export_names_route_through_javascript_diagnostics() {
        let diagnostics =
            language_diagnostics("app.js", "export { a as \"read\", b as \"read\" };\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateExportSpecifier
        );
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 29);
        assert_eq!(diagnostics[0].message, "Duplicate export name 'read'.");
        assert!(
            language_diagnostics("app.js", "export { a as \"read\", b as \"write\" };\n")
                .is_empty()
        );
    }

    #[test]
    fn duplicate_export_specifiers_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics("app.js", "export { read, read };\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateExportSpecifier
        );
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 16);
        assert_eq!(diagnostics[0].message, "Duplicate export name 'read'.");
        assert!(language_diagnostics("app.js", "export { read, write };\n").is_empty());
    }

    #[test]
    fn duplicate_quoted_import_names_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "import { \"a\" as read, \"a\" as read } from \"./pkg\";\nconsole.log(read);\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateImportSpecifier
        );
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 30);
        assert_eq!(diagnostics[0].message, "Duplicate import binding 'read'.");
        assert!(
            language_diagnostics(
                "app.js",
                "import { \"a\" as read, \"a\" as write } from \"./pkg\";\nconsole.log(read, write);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn duplicate_import_specifiers_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "import { read, read } from \"./io\";\nconsole.log(read);\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            DiagnosticKind::DuplicateImportSpecifier
        );
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 16);
        assert_eq!(diagnostics[0].message, "Duplicate import binding 'read'.");
        assert!(
            language_diagnostics(
                "app.js",
                "import { read, write } from \"./io\";\nconsole.log(read, write);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn method_duplicate_parameters_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "class A { read(value, value) { return value; } }\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateParameter);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 23);
        assert!(
            diagnostics[0]
                .message
                .contains("Duplicate parameter 'value'")
        );

        assert!(
            language_diagnostics("app.js", "class A { read(a, b) { return [a, b]; } }\n")
                .is_empty()
        );
        assert!(
            language_diagnostics(
                "app.js",
                "const o = { read(value, value) { return value; } };\nconsole.log(o);\n"
            )
            .len()
                == 1
        );
    }

    #[test]
    fn duplicate_parameters_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "function pick(value, value) {\n  return value;\n}\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateParameter);
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].column, 22);
        assert!(diagnostics[0].message.contains("later parameter wins"));
        assert!(
            language_diagnostics("app.js", "function pick(value, _other) { return value; }\n")
                .is_empty()
        );
    }

    #[test]
    fn duplicate_object_keys_route_through_javascript_diagnostics() {
        let diagnostics = language_diagnostics(
            "app.js",
            "const options = {\n  size: 1,\n  size: 2,\n};\nconsole.log(options);\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::DuplicateObjectKey);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].column, 3);
        assert!(diagnostics[0].message.contains("first key on line 2"));
        assert!(
            language_diagnostics(
                "app.js",
                "const options = { size: 1, color: 'red' };\nconsole.log(options);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn script_scanner_ignores_strings_comments_and_templates() {
        let text =
            "const a = \"}\";\n// }\n/* } */\nconst b = `{${a}}`;\nfunction f() { return [a, b]; }";
        assert!(delimiters(text, true, "JavaScript").is_empty());

        let unclosed = delimiters("function f() {\n  return 1;\n", true, "JavaScript");
        assert_eq!(unclosed.len(), 1);
        assert_eq!(unclosed[0].line, 1);
        assert!(unclosed[0].message.contains("Unclosed '{'"));

        let extra = delimiters("let a = 1;\n}\n", true, "JavaScript");
        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0].line, 2);
        assert!(extra[0].message.contains("Unexpected '}'"));

        let string = delimiters("const a = 'oops\nconst b = 1;", true, "JavaScript");
        assert_eq!(string.len(), 1);
        assert_eq!(string[0].line, 1);
        assert!(string[0].message.contains("Unterminated string"));

        let comment = delimiters("/* never closes", true, "JavaScript");
        assert_eq!(comment.len(), 1);
        assert!(comment[0].message.contains("Unterminated block comment"));
    }

    #[test]
    fn css_and_scss_use_their_comment_rules_and_extension_routing() {
        assert!(language_diagnostics("styles.css", "body { color: red; }").is_empty());
        assert!(
            delimiters(
                "body { color: red; }\n/* } */\n.a { --x: \"}\"; }",
                false,
                "CSS"
            )
            .is_empty()
        );
        let unclosed = language_diagnostics("styles.css", ".a {\n  color: red;\n");
        assert_eq!(unclosed.len(), 1);
        assert_eq!(unclosed[0].source, "CSS");
        assert_eq!(unclosed[0].line, 1);

        assert!(
            language_diagnostics("styles.scss", "// }\n.a { color: red; }").is_empty(),
            "SCSS line comments are ignored"
        );
        assert!(
            language_diagnostics("styles.css", "// }\n.a { color: red; }")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Unexpected '}'")),
            "CSS comments require block syntax"
        );

        assert!(language_diagnostics("app.js", "const a = 1;\nconsole.log(a);").is_empty());
        assert_eq!(
            language_diagnostics("app.ts", "function f() {")[0].source,
            "TypeScript"
        );
        assert!(language_diagnostics("notes.md", "{").is_empty());
    }
}
