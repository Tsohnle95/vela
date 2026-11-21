//! Bounded local language diagnostics for the native editor.
//!
//! JSON and HTML are checked by bounded scanners that report the same syntax
//! errors Source's bundled JSON and HTML workers do. JavaScript/TypeScript and
//! CSS/SCSS use a string/comment-aware delimiter scanner (CSS adds a declaration
//! parser) so unmatched brackets and unterminated strings/comments surface
//! without a language server or network request.

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
    UnknownProperty,
    MissingProperty,
    UnknownMember,
    Validation,
    /// A worker lint finding Source reports as a warning rather than an error.
    Lint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Diagnostic {
    pub source: &'static str,
    pub kind: DiagnosticKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl Diagnostic {
    pub(super) fn is_warning(&self) -> bool {
        self.kind == DiagnosticKind::Lint
    }
}

pub(super) fn language_diagnostics(rel: &str, text: &str) -> Vec<Diagnostic> {
    let lower = rel.to_ascii_lowercase();
    match lower.rsplit('.').next().unwrap_or_default() {
        "json" => json_diagnostics(text),
        "jsonc" => jsonc_diagnostics(text),
        "js" | "mjs" | "cjs" | "jsx" => {
            script_diagnostics(&lower, text, "JavaScript", false, true)
        }
        "ts" | "tsx" | "mts" | "cts" => {
            script_diagnostics(&lower, text, "TypeScript", true, !lower.ends_with(".d.ts"))
        }
        "html" | "htm" => html_diagnostics(text),
        "css" => {
            let mut diagnostics = delimiters(text, false, "CSS");
            diagnostics.extend(css_diagnostics(text));
            diagnostics
        }
        "scss" => {
            let mut diagnostics = delimiters(text, true, "SCSS");
            diagnostics.extend(preprocessor_diagnostics(text, "SCSS"));
            diagnostics
        }
        "less" => {
            let mut diagnostics = delimiters(text, true, "Less");
            diagnostics.extend(preprocessor_diagnostics(text, "Less"));
            diagnostics
        }
        _ => Vec::new(),
    }
}

fn script_diagnostics(
    rel: &str,
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
    diagnostics.extend(
        crate::member_access::diagnostics(rel, text)
            .into_iter()
            .map(|found| {
                let line_start = text[..found.offset]
                    .rfind('\n')
                    .map(|index| index + 1)
                    .unwrap_or(0);
                Diagnostic {
                    source,
                    kind: DiagnosticKind::UnknownMember,
                    line: text[..found.offset].bytes().filter(|byte| *byte == b'\n').count() + 1,
                    column: found.offset - line_start + 1,
                    message: found.message,
                }
            }),
    );
    diagnostics.extend(crate::object_types::diagnostics(text).into_iter().map(|found| {
        let line_start = text[..found.offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
        Diagnostic {
            source,
            kind: match found.kind {
                crate::object_types::ObjectTypeKind::UnknownProperty => {
                    DiagnosticKind::UnknownProperty
                }
                crate::object_types::ObjectTypeKind::MissingProperty => {
                    DiagnosticKind::MissingProperty
                }
            },
            line: text[..found.offset].bytes().filter(|byte| *byte == b'\n').count() + 1,
            column: found.offset - line_start + 1,
            message: found.message,
        }
    }));
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

/// The JSON worker reports every syntax error in one pass; keep the native
/// scanner bounded so a pathological document cannot produce unbounded rows.
const JSON_DIAGNOSTIC_LIMIT: usize = 100;

pub(super) fn json_diagnostics(text: &str) -> Vec<Diagnostic> {
    JsonCheck::new(text, false).run()
}

/// `.jsonc` allows comments like Source's JSON worker; `.json` reports them.
pub(super) fn jsonc_diagnostics(text: &str) -> Vec<Diagnostic> {
    JsonCheck::new(text, true).run()
}

struct JsonCheck {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    allow_comments: bool,
    halted: bool,
    diagnostics: Vec<Diagnostic>,
}

impl JsonCheck {
    fn new(text: &str, allow_comments: bool) -> Self {
        Self {
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            allow_comments,
            halted: false,
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> Vec<Diagnostic> {
        self.skip_whitespace();
        if self.at_end() {
            return self.diagnostics;
        }
        self.parse_value();
        self.skip_whitespace();
        if !self.at_end() && !self.halted {
            let (line, column) = self.position();
            self.error(line, column, "End of file expected");
        }
        self.diagnostics.sort_by_key(|diagnostic| {
            (diagnostic.line, diagnostic.column, diagnostic.message.clone())
        });
        self.diagnostics
    }

    fn at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.index += 1;
        }
    }

    fn error(&mut self, line: usize, column: usize, message: &str) {
        if self.halted {
            return;
        }
        self.diagnostics.push(Diagnostic {
            source: "JSON",
            kind: DiagnosticKind::Syntax,
            line,
            column,
            message: message.to_string(),
        });
        if self.diagnostics.len() >= JSON_DIAGNOSTIC_LIMIT {
            self.halted = true;
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => self.bump(),
                Some('/') if self.peek_next() == Some('/') => {
                    let (line, column) = self.position();
                    if !self.allow_comments {
                        self.error(line, column, "Comments are not permitted in JSON.");
                    }
                    while let Some(ch) = self.peek() {
                        self.bump();
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                Some('/') if self.peek_next() == Some('*') => {
                    let (line, column) = self.position();
                    if !self.allow_comments {
                        self.error(line, column, "Comments are not permitted in JSON.");
                    }
                    self.bump();
                    self.bump();
                    while !self.at_end() {
                        if self.peek() == Some('*') && self.peek_next() == Some('/') {
                            self.bump();
                            self.bump();
                            break;
                        }
                        self.bump();
                    }
                }
                _ => return,
            }
        }
    }

    fn parse_value(&mut self) -> bool {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => {
                self.bump();
                self.parse_object();
                true
            }
            Some('[') => {
                self.bump();
                self.parse_array();
                true
            }
            Some('"') => self.parse_string(),
            Some(ch) if ch == '-' || ch.is_ascii_digit() => {
                self.parse_number();
                true
            }
            Some('t') | Some('f') | Some('n') => self.parse_literal(),
            _ => {
                let (line, column) = self.position();
                self.error(line, column, "Value expected");
                false
            }
        }
    }

    fn parse_object(&mut self) {
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return;
        }
        loop {
            if self.halted {
                return;
            }
            self.skip_whitespace();
            if self.peek() != Some('"') {
                let (line, column) = self.position();
                if self.at_end() {
                    self.error(line, column, "Expected '}'");
                } else {
                    self.error(line, column, "Property keys must be double-quoted");
                }
                self.skip_to_close();
                return;
            }
            if !self.parse_string() {
                self.skip_to_close();
                return;
            }
            self.skip_whitespace();
            if self.peek() == Some(':') {
                self.bump();
            } else {
                let (line, column) = self.position();
                self.error(line, column, "Expected ':'");
                self.skip_to_close();
                return;
            }
            if !self.parse_value() {
                self.skip_to_close();
                return;
            }
            self.skip_whitespace();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return;
                }
                Some(',') => {
                    let (line, column) = self.position();
                    self.bump();
                    self.skip_whitespace();
                    if self.peek() == Some('}') {
                        self.error(line, column, "Trailing comma");
                        self.bump();
                        return;
                    }
                }
                _ => {
                    let (line, column) = self.position();
                    if self.at_end() {
                        self.error(line, column, "Expected '}'");
                    } else {
                        self.error(line, column, "Expected comma");
                    }
                    self.skip_to_close();
                    return;
                }
            }
        }
    }

    fn parse_array(&mut self) {
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.bump();
            return;
        }
        loop {
            if self.halted {
                return;
            }
            self.skip_whitespace();
            if !self.parse_value() {
                self.skip_to_close();
                return;
            }
            self.skip_whitespace();
            match self.peek() {
                Some(']') => {
                    self.bump();
                    return;
                }
                Some(',') => {
                    let (line, column) = self.position();
                    self.bump();
                    self.skip_whitespace();
                    if self.peek() == Some(']') {
                        self.error(line, column, "Trailing comma");
                        self.bump();
                        return;
                    }
                }
                _ => {
                    let (line, column) = self.position();
                    if self.at_end() {
                        self.error(line, column, "Expected ']'");
                    } else {
                        self.error(line, column, "Expected comma");
                    }
                    self.skip_to_close();
                    return;
                }
            }
        }
    }

    fn parse_string(&mut self) -> bool {
        let (line, column) = self.position();
        self.bump();
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.bump();
                    return true;
                }
                '\\' => {
                    self.bump();
                    if self.peek().is_some() {
                        self.bump();
                    }
                }
                '\n' => break,
                _ => self.bump(),
            }
        }
        self.error(line, column, "Unexpected end of string");
        false
    }

    fn parse_number(&mut self) {
        let (line, column) = self.position();
        if self.peek() == Some('-') {
            self.bump();
        }
        let mut digits = 0;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.bump();
            digits += 1;
        }
        if digits == 0 {
            self.error(line, column, "Invalid number");
            return;
        }
        if self.peek() == Some('.') {
            self.bump();
            let mut fraction = 0;
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.bump();
                fraction += 1;
            }
            if fraction == 0 {
                self.error(line, column, "Invalid number");
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.bump();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.bump();
            }
            let mut exponent = 0;
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.bump();
                exponent += 1;
            }
            if exponent == 0 {
                self.error(line, column, "Invalid number");
            }
        }
    }

    fn parse_literal(&mut self) -> bool {
        let (line, column) = self.position();
        for literal in ["true", "false", "null"] {
            let expected = literal.chars().collect::<Vec<_>>();
            if self.chars[self.index..].starts_with(&expected) {
                for _ in 0..expected.len() {
                    self.bump();
                }
                return true;
            }
        }
        self.error(line, column, "Value expected");
        false
    }

    /// Recover inside one broken container without cascading errors: skip to
    /// its closing delimiter (tracking nested containers and strings).
    fn skip_to_close(&mut self) {
        let mut depth = 1usize;
        while !self.at_end() {
            match self.peek() {
                Some('{') | Some('[') => {
                    depth += 1;
                    self.bump();
                }
                Some('}') | Some(']') => {
                    depth -= 1;
                    self.bump();
                    if depth == 0 {
                        return;
                    }
                }
                Some('"') => {
                    self.parse_string();
                }
                Some('/') if matches!(self.peek_next(), Some('/') | Some('*')) => {
                    let allow = self.allow_comments;
                    self.allow_comments = true;
                    self.skip_whitespace();
                    self.allow_comments = allow;
                }
                _ => self.bump(),
            }
        }
    }
}

/// Source's CSS/SCSS/Less worker reports parser errors inside declaration
/// blocks plus the data-light lint rules the native problem surface can carry.
/// The data-driven rules (unknown property, vendor prefixes, color function
/// arity, display-aware properties) need Monaco's style database and stay
/// unsupported.
const CSS_DIAGNOSTIC_LIMIT: usize = 100;

pub(super) fn css_diagnostics(text: &str) -> Vec<Diagnostic> {
    CssCheck::new(text, CssFlavor::Css, "CSS").run()
}

/// SCSS and Less share the CSS parser family Source runs in its worker, but
/// allow `//` comments and directive/variable statements that are not
/// declarations.
pub(super) fn preprocessor_diagnostics(text: &str, source: &'static str) -> Vec<Diagnostic> {
    CssCheck::new(text, CssFlavor::Preprocessor, source).run()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CssFlavor {
    Css,
    Preprocessor,
}

struct CssDeclaration {
    name: String,
    line: usize,
    column: usize,
    value_starts_with_dash: bool,
}

struct CssFrame {
    prelude: String,
    prelude_line: usize,
    prelude_column: usize,
    has_content: bool,
    declarations: Vec<CssDeclaration>,
}

struct CssCheck {
    flavor: CssFlavor,
    source: &'static str,
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    halted: bool,
    diagnostics: Vec<Diagnostic>,
    frames: Vec<CssFrame>,
    statement: String,
    statement_started: bool,
    statement_line: usize,
    statement_column: usize,
    parens: usize,
    brackets: usize,
}

impl CssCheck {
    fn new(text: &str, flavor: CssFlavor, source: &'static str) -> Self {
        Self {
            flavor,
            source,
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            halted: false,
            diagnostics: Vec::new(),
            frames: Vec::new(),
            statement: String::new(),
            statement_started: false,
            statement_line: 1,
            statement_column: 1,
            parens: 0,
            brackets: 0,
        }
    }

    fn run(mut self) -> Vec<Diagnostic> {
        while let Some(ch) = self.peek() {
            if self.halted {
                break;
            }
            match ch {
                '/' if self.flavor == CssFlavor::Preprocessor
                    && !self.parenthesised()
                    && self.peek_next() == Some('/') =>
                {
                    while let Some(ch) = self.peek() {
                        self.bump();
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                '/' if self.peek_next() == Some('*') => {
                    self.bump();
                    self.bump();
                    while !self.at_end() {
                        if self.peek() == Some('*') && self.peek_next() == Some('/') {
                            self.bump();
                            self.bump();
                            break;
                        }
                        self.bump();
                    }
                }
                '"' | '\'' => self.read_string(ch),
                '(' => {
                    self.parens += 1;
                    self.push(ch);
                    self.bump();
                }
                ')' => {
                    self.parens = self.parens.saturating_sub(1);
                    self.push(ch);
                    self.bump();
                }
                '[' => {
                    self.brackets += 1;
                    self.push(ch);
                    self.bump();
                }
                ']' => {
                    self.brackets = self.brackets.saturating_sub(1);
                    self.push(ch);
                    self.bump();
                }
                '{' if self.parens == 0 && self.brackets == 0 => {
                    self.open_frame();
                    self.bump();
                }
                '}' if self.parens == 0 && self.brackets == 0 => {
                    self.finish_statement();
                    self.close_frame();
                    self.clear_statement();
                    self.bump();
                }
                ';' if self.parens == 0 && self.brackets == 0 => {
                    self.finish_statement();
                    self.bump();
                }
                _ => {
                    self.push(ch);
                    self.bump();
                }
            }
        }
        self.finish_statement();
        self.diagnostics
            .sort_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
        self.diagnostics
    }

    fn parenthesised(&self) -> bool {
        self.parens > 0 || self.brackets > 0
    }

    fn at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.index += 1;
        }
    }

    fn push(&mut self, ch: char) {
        if !self.statement_started && !ch.is_whitespace() {
            self.statement_started = true;
            self.statement_line = self.line;
            self.statement_column = self.column;
        }
        self.statement.push(ch);
    }

    fn clear_statement(&mut self) {
        self.statement.clear();
        self.statement_started = false;
    }

    fn read_string(&mut self, quote: char) {
        self.push(quote);
        self.bump();
        while let Some(ch) = self.peek() {
            self.push(ch);
            self.bump();
            if ch == '\\' {
                if let Some(escaped) = self.peek() {
                    self.push(escaped);
                    self.bump();
                }
            } else if ch == quote || ch == '\n' {
                return;
            }
        }
    }

    fn report(&mut self, kind: DiagnosticKind, line: usize, column: usize, message: &str) {
        if self.halted {
            return;
        }
        self.diagnostics.push(Diagnostic {
            source: self.source,
            kind,
            line,
            column,
            message: message.to_string(),
        });
        if self.diagnostics.len() >= CSS_DIAGNOSTIC_LIMIT {
            self.halted = true;
        }
    }

    fn error(&mut self, line: usize, column: usize, message: &str) {
        self.report(DiagnosticKind::Syntax, line, column, message);
    }

    fn warn(&mut self, line: usize, column: usize, message: &str) {
        self.report(DiagnosticKind::Lint, line, column, message);
    }

    fn open_frame(&mut self) {
        let prelude = self.statement.trim().to_owned();
        let prelude_line = self.statement_line;
        let prelude_column = self.statement_column;
        if let Some(parent) = self.frames.last_mut() {
            parent.has_content = true;
        }
        if self.flavor == CssFlavor::Css && prelude.starts_with('@') {
            self.check_at_rule(&prelude, prelude_line, prelude_column);
        }
        self.frames.push(CssFrame {
            prelude,
            prelude_line,
            prelude_column,
            has_content: false,
            declarations: Vec::new(),
        });
        self.clear_statement();
    }

    fn close_frame(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        if frame.prelude.is_empty() {
            return;
        }
        let at_rule = frame.prelude.starts_with('@');
        if !at_rule && !frame.has_content {
            self.warn(
                frame.prelude_line,
                frame.prelude_column,
                "Do not use empty rulesets",
            );
        }
        if frame.prelude.to_ascii_lowercase().starts_with("@font-face") {
            let defines = |name: &str| {
                frame
                    .declarations
                    .iter()
                    .any(|declaration| declaration.name == name)
            };
            if !defines("src") || !defines("font-family") {
                self.warn(
                    frame.prelude_line,
                    frame.prelude_column,
                    "@font-face rule must define 'src' and 'font-family' properties",
                );
            }
        }
        if !at_rule {
            for (index, declaration) in frame.declarations.iter().enumerate() {
                if declaration.name == "background" || declaration.value_starts_with_dash {
                    continue;
                }
                let duplicates = frame
                    .declarations
                    .iter()
                    .enumerate()
                    .filter(|(other, candidate)| {
                        *other != index && candidate.name == declaration.name
                    })
                    .count();
                if duplicates > 0 {
                    self.warn(
                        declaration.line,
                        declaration.column,
                        "Do not use duplicate style definitions",
                    );
                }
            }
        }
    }

    fn check_at_rule(&mut self, prelude: &str, line: usize, column: usize) {
        let name = at_rule_name(prelude);
        if name.is_empty() || known_at_rule(&name) {
            return;
        }
        self.warn(line, column, &format!("Unknown at rule @{name}"));
    }

    /// Classify one statement inside a declaration block. Nested rules and
    /// at-rules are dropped before the block, so only declarations reach here.
    fn finish_statement(&mut self) {
        let raw = std::mem::take(&mut self.statement);
        let started = self.statement_started;
        self.statement_started = false;
        if !started {
            return;
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }
        let base_line = self.statement_line;
        let base_column = self.statement_column;
        // `statement_line`/`statement_column` already point at the first
        // non-whitespace character, which is where `trimmed` starts.
        let position = |offset: usize| line_column_after(trimmed, base_line, base_column, offset);

        if let Some(frame) = self.frames.last_mut() {
            frame.has_content = true;
        }
        if trimmed.starts_with('@') {
            if self.flavor == CssFlavor::Css {
                let (line, column) = position(0);
                self.check_at_rule(trimmed, line, column);
            }
            return;
        }
        if self.frames.is_empty() {
            return;
        }
        if self.flavor == CssFlavor::Preprocessor {
            if trimmed.starts_with('$') {
                return;
            }
            if trimmed
                .chars()
                .next()
                .is_some_and(|ch| matches!(ch, '.' | '#' | '&' | '%' | '[' | '>' | '+' | '~' | '*' | ':'))
            {
                return;
            }
        }
        let Some(colon) = top_level_colon(trimmed) else {
            let (line, column) = position(0);
            self.error(line, column, "colon expected");
            return;
        };
        let name = trimmed[..colon].trim();
        if name.is_empty() {
            let (line, column) = position(0);
            self.error(line, column, "identifier expected");
            return;
        }
        let raw_value = &trimmed[colon + 1..];
        let value = raw_value.trim();
        if value.is_empty() {
            let (line, column) = position(colon);
            self.error(line, column, "property value expected");
            return;
        }
        let value_offset = colon + 1 + (raw_value.chars().count() - raw_value.trim_start().chars().count());
        self.hex_colors(value, value_offset, &position);
        let (line, column) = position(0);
        let key = if name.starts_with("--") {
            name.to_owned()
        } else {
            name.to_ascii_lowercase()
        };
        if let Some(frame) = self.frames.last_mut() {
            frame.declarations.push(CssDeclaration {
                name: key,
                line,
                column,
                value_starts_with_dash: value.starts_with('-'),
            });
        }
    }

    fn hex_colors(&mut self, value: &str, value_offset: usize, position: &impl Fn(usize) -> (usize, usize)) {
        let chars = value.chars().collect::<Vec<_>>();
        let mut index = 0usize;
        while index < chars.len() {
            if chars[index] == '#' {
                let start = index;
                index += 1;
                let mut digits = 0usize;
                while index < chars.len() && chars[index].is_ascii_hexdigit() {
                    index += 1;
                    digits += 1;
                }
                if digits > 0 && !matches!(digits, 3 | 4 | 6 | 8) {
                    let (line, column) = position(value_offset + start);
                    self.error(
                        line,
                        column,
                        "Hex colors must consist of three, four, six or eight hex numbers",
                    );
                }
                continue;
            }
            if (chars[index] == 'u' || chars[index] == 'U')
                && chars.len() > index + 3
                && chars[index + 1].eq_ignore_ascii_case(&'r')
                && chars[index + 2].eq_ignore_ascii_case(&'l')
                && chars[index + 3] == '('
            {
                index += 4;
                while index < chars.len() && chars[index] != ')' {
                    index += 1;
                }
                if index < chars.len() {
                    index += 1;
                }
                continue;
            }
            index += 1;
        }
    }
}

fn line_column_after(text: &str, line: usize, column: usize, offset: usize) -> (usize, usize) {
    let mut line = line;
    let mut column = column;
    for ch in text.chars().take(offset) {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn at_rule_name(prelude: &str) -> String {
    let rest = prelude.trim_start().trim_start_matches('@');
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '(' || ch == ';' || ch == '{')
        .unwrap_or(rest.len());
    rest[..end].to_ascii_lowercase()
}

fn known_at_rule(name: &str) -> bool {
    let stripped = ["-webkit-", "-moz-", "-ms-", "-o-"]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix))
        .unwrap_or(name);
    matches!(
        stripped,
        "charset"
            | "import"
            | "namespace"
            | "media"
            | "supports"
            | "layer"
            | "container"
            | "scope"
            | "keyframes"
            | "font-face"
            | "font-feature-values"
            | "font-palette-values"
            | "page"
            | "property"
            | "counter-style"
            | "starting-style"
            | "color-profile"
            | "position-try"
            | "view-transition"
            | "viewport"
            | "document"
            | "nest"
    )
}

/// First `:` outside strings and parentheses, so `url(data:image/png;base64,…)`
/// never supplies the declaration colon.
fn top_level_colon(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut parens = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(end) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == end {
                quote = None;
            }
            index += 1;
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            ':' if parens == 0 => return Some(index),
            _ => {}
        }
        index += 1;
    }
    None
}

/// Source's HTML worker only reports tokenizer-level malformations; it never
/// validates element nesting. This scanner mirrors its scanner states for HTML
/// documents.
const HTML_DIAGNOSTIC_LIMIT: usize = 100;

pub(super) fn html_diagnostics(text: &str) -> Vec<Diagnostic> {
    HtmlCheck::new(text).run()
}

struct HtmlCheck {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    halted: bool,
    diagnostics: Vec<Diagnostic>,
}

impl HtmlCheck {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            halted: false,
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> Vec<Diagnostic> {
        while !self.at_end() && !self.halted {
            if self.peek() == Some('<') {
                self.scan_markup();
            } else {
                self.bump();
            }
        }
        self.diagnostics
            .sort_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
        self.diagnostics
    }

    fn at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.index += 1;
        }
    }

    fn starts_with(&self, needle: &str) -> bool {
        needle
            .chars()
            .enumerate()
            .all(|(offset, expected)| self.peek_at(offset) == Some(expected))
    }

    fn matches_ignore_case(&self, needle: &str) -> bool {
        needle.chars().enumerate().all(|(offset, expected)| {
            self.peek_at(offset)
                .is_some_and(|actual| actual.eq_ignore_ascii_case(&expected))
        })
    }

    fn error(&mut self, line: usize, column: usize, message: &str) {
        if self.halted {
            return;
        }
        self.diagnostics.push(Diagnostic {
            source: "HTML",
            kind: DiagnosticKind::Syntax,
            line,
            column,
            message: message.to_string(),
        });
        if self.diagnostics.len() >= HTML_DIAGNOSTIC_LIMIT {
            self.halted = true;
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while self.peek().is_some_and(|ch| ch.is_whitespace()) {
            self.bump();
            skipped = true;
        }
        skipped
    }

    fn scan_markup(&mut self) {
        if self.starts_with("<!--") {
            for _ in 0..4 {
                self.bump();
            }
            while !self.at_end() {
                if self.starts_with("-->") {
                    for _ in 0..3 {
                        self.bump();
                    }
                    return;
                }
                self.bump();
            }
            return;
        }
        if self.starts_with("<!") || self.starts_with("<?") {
            self.bump();
            self.bump();
            while let Some(ch) = self.peek() {
                self.bump();
                if ch == '>' {
                    return;
                }
            }
            return;
        }
        if self.starts_with("</") {
            self.bump();
            self.bump();
            self.scan_end_tag();
            return;
        }
        self.bump();
        self.scan_start_tag();
    }

    fn read_element_name(&mut self) -> String {
        let mut name = String::new();
        match self.peek() {
            Some(ch) if ch == '_' || ch == ':' || ch.is_ascii_alphanumeric() => {
                name.push(ch);
                self.bump();
            }
            _ => return name,
        }
        while let Some(ch) = self.peek() {
            if ch == '_' || ch == ':' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric() {
                name.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        name
    }

    fn read_attribute_name(&mut self) -> bool {
        let mut read = false;
        while let Some(ch) = self.peek() {
            if ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '>' | '<' | '/' | '=')
                || (ch as u32) < 0x20
                || (ch as u32) == 0x7f
                || (0x80..=0x9f).contains(&(ch as u32))
            {
                break;
            }
            self.bump();
            read = true;
        }
        read
    }

    fn read_attribute_value(&mut self) {
        match self.peek() {
            Some(quote @ ('"' | '\'')) => {
                self.bump();
                while let Some(ch) = self.peek() {
                    self.bump();
                    if ch == quote {
                        return;
                    }
                }
            }
            _ => {
                while let Some(ch) = self.peek() {
                    if ch.is_whitespace() || ch == '>' || ch == '<' || ch == '"' || ch == '\'' || ch == '`' || ch == '=' {
                        break;
                    }
                    self.bump();
                }
            }
        }
    }

    fn scan_end_tag(&mut self) {
        if self.skip_whitespace() {
            let (line, column) = self.position();
            self.error(
                line,
                column,
                "Tag name must directly follow the open bracket.",
            );
            self.skip_whitespace();
        }
        let name = self.read_element_name();
        if name.is_empty() {
            let (line, column) = self.position();
            let mut consumed = 0;
            while let Some(ch) = self.peek() {
                if ch == '>' {
                    break;
                }
                self.bump();
                consumed += 1;
            }
            if consumed > 0 {
                self.error(line, column, "End tag name expected.");
            }
            if self.peek() == Some('>') {
                self.bump();
            }
            return;
        }
        self.skip_whitespace();
        match self.peek() {
            None => {}
            Some('>') => {
                self.bump();
            }
            Some(_) => {
                let (line, column) = self.position();
                self.error(line, column, "Closing bracket expected.");
                while let Some(ch) = self.peek() {
                    self.bump();
                    if ch == '>' {
                        return;
                    }
                }
            }
        }
    }

    fn scan_start_tag(&mut self) {
        if self.skip_whitespace() {
            let (line, column) = self.position();
            self.error(
                line,
                column,
                "Tag name must directly follow the open bracket.",
            );
            self.skip_whitespace();
        }
        let name = self.read_element_name();
        if name.is_empty() {
            let (line, column) = self.position();
            let mut consumed = 0;
            while let Some(ch) = self.peek() {
                if ch == '>' {
                    break;
                }
                self.bump();
                consumed += 1;
            }
            if consumed > 0 {
                self.error(line, column, "Start tag name expected.");
            }
            if self.peek() == Some('>') {
                self.bump();
            }
            return;
        }
        let mut self_closing = false;
        loop {
            let spaced = self.skip_whitespace();
            match self.peek() {
                None => return,
                Some('>') => {
                    self.bump();
                    break;
                }
                Some('/') if self.peek_at(1) == Some('>') => {
                    self.bump();
                    self.bump();
                    self_closing = true;
                    break;
                }
                Some('/') => {
                    let (line, column) = self.position();
                    self.error(line, column, "Unexpected character in tag.");
                    self.bump();
                }
                Some(_) if spaced => {
                    if !self.read_attribute_name() {
                        let (line, column) = self.position();
                        self.error(line, column, "Unexpected character in tag.");
                        self.bump();
                        continue;
                    }
                    self.skip_whitespace();
                    if self.peek() == Some('=') {
                        self.bump();
                        self.skip_whitespace();
                        self.read_attribute_value();
                    }
                }
                Some(_) => {
                    let (line, column) = self.position();
                    self.error(line, column, "Unexpected character in tag.");
                    self.bump();
                    while self
                        .peek()
                        .is_some_and(|ch| !ch.is_whitespace() && ch != '>' && ch != '/')
                    {
                        self.bump();
                    }
                }
            }
        }
        if self_closing {
            return;
        }
        let lower = name.to_ascii_lowercase();
        if matches!(lower.as_str(), "script" | "style" | "textarea" | "title") {
            self.skip_raw_text(&lower);
        }
    }

    /// Raw-text elements keep `<` literal until their own closing tag.
    fn skip_raw_text(&mut self, name: &str) {
        let needle = format!("</{name}");
        while !self.at_end() {
            if self.matches_ignore_case(&needle) {
                return;
            }
            self.bump();
        }
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
            diagnostic.message.contains("Value expected"),
            "{diagnostic:?}"
        );

        // The worker anchors a trailing comma at the comma itself.
        let trailing = json_diagnostics("{\n  \"a\": 1,\n}\n");
        assert_eq!(trailing.len(), 1);
        assert_eq!(trailing[0].line, 2);
        assert_eq!(trailing[0].column, 9);
        assert!(
            trailing[0].message.contains("Trailing comma"),
            "{:?}",
            trailing[0]
        );
    }

    #[test]
    fn css_lint_rules_match_the_worker() {
        let empty = language_diagnostics("x.css", "a {}\n");
        assert!(
            empty
                .iter()
                .any(|diagnostic| diagnostic.is_warning()
                    && diagnostic.message == "Do not use empty rulesets"),
            "{empty:?}"
        );

        let duplicates = language_diagnostics("x.css", "a { color: red; color: blue; }\n");
        assert_eq!(
            duplicates
                .iter()
                .filter(|diagnostic| diagnostic.message == "Do not use duplicate style definitions")
                .count(),
            2,
            "{duplicates:?}"
        );
        assert!(
            !language_diagnostics("x.css", "a { background: red; background: blue; }\n")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("duplicate")),
            "the worker exempts background duplicates"
        );

        let hex = language_diagnostics("x.css", "a { color: #12; }\n");
        assert!(
            hex.iter().any(|diagnostic| !diagnostic.is_warning()
                && diagnostic
                    .message
                    .starts_with("Hex colors must consist of")),
            "{hex:?}"
        );
        assert!(
            language_diagnostics("x.css", "a { color: #a1b2c3; }\n").is_empty(),
            "six-digit colors are valid"
        );

        let font = language_diagnostics("x.css", "@font-face { font-family: x; }\n");
        assert!(
            font.iter().any(|diagnostic| diagnostic
                .message
                .starts_with("@font-face rule must define")),
            "{font:?}"
        );
        assert!(
            language_diagnostics("x.css", "@font-face { src: url(x); font-family: y; }\n")
                .is_empty(),
            "complete @font-face rules stay clean"
        );

        let unknown = language_diagnostics("x.css", "@foo bar;\n");
        assert!(
            unknown
                .iter()
                .any(|diagnostic| diagnostic.message == "Unknown at rule @foo"),
            "{unknown:?}"
        );
        for known in [
            "@media (min-width: 1px) { a { color: red; } }\n",
            "@supports (display: grid) { a { color: red; } }\n",
            "@layer base { a { color: red; } }\n",
            "@container (min-width: 20em) { a { color: red; } }\n",
            "@property --x { syntax: \"*\"; inherits: false; }\n",
            "@-webkit-keyframes spin { from { opacity: 0; } to { opacity: 1; } }\n",
        ] {
            let diagnostics = language_diagnostics("x.css", known);
            assert!(
                !diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.starts_with("Unknown at rule")),
                "{known:?} -> {diagnostics:?}"
            );
        }
        assert!(
            !language_diagnostics("x.css", "@media (min-width: 1px) {}\n")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("empty rulesets")),
            "at-rule blocks are not empty rulesets"
        );

        // Preprocessors keep their directives and lint like Source's SCSS/Less worker.
        assert!(
            !language_diagnostics("x.scss", "@if $x { }\n")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Unknown at rule")),
            "SCSS directives are never unknown at-rules"
        );
        assert!(
            language_diagnostics("x.scss", ".a { }\n")
                .iter()
                .any(|diagnostic| diagnostic.message == "Do not use empty rulesets")
        );
    }

    #[test]
    fn preprocessor_declarations_skip_variables_directives_and_mixin_calls() {
        for (text, source) in [
            (
                "$gap: 8px;\n.a {\n  color red;\n}\n",
                "SCSS",
            ),
            (
                "@gap: 8px;\n.a {\n  color red;\n}\n",
                "Less",
            ),
        ] {
            let file = if source == "SCSS" { "x.scss" } else { "x.less" };
            let diagnostics = language_diagnostics(file, text);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == "colon expected"),
                "{file}: {diagnostics:?}"
            );
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.source == source),
                "{file}: {diagnostics:?}"
            );
        }

        for (file, text) in [
            ("x.scss", "$x: 1;\n%placeholder { color: red; }\n.a { @include mixin($x); }\n"),
            ("x.scss", ".a { &:hover { color: red; } }\n"),
            ("x.scss", ".a { font: { family: serif; size: 12px; } }\n"),
            ("x.scss", ".a { // a comment with ; and {\n  color: red;\n}\n"),
            ("x.scss", ".a { color: map-get($m, key); }\n"),
            ("x.less", "@c: red;\n.a { .mixin(); color: @c; }\n"),
            ("x.less", ".mixin(@a) { color: darken(@a, 10%); }\n.a { .mixin(#fff); }\n"),
            ("x.less", ".a { color: rgba(0, 0, 0, .5); }\n"),
        ] {
            let diagnostics = language_diagnostics(file, text);
            assert!(diagnostics.is_empty(), "{file}: {text:?} -> {diagnostics:?}");
        }
    }

    #[test]
    fn html_diagnostics_report_worker_tokenizer_errors() {
        let cases: &[(&str, &str)] = &[
            ("< div>", "Tag name must directly follow the open bracket."),
            ("</ div>", "Tag name must directly follow the open bracket."),
            ("<div>1 < 2</div>", "Tag name must directly follow the open bracket."),
            ("</div extra>", "Closing bracket expected."),
            ("</>x", "no"),
            ("<div\"x\">", "Unexpected character in tag."),
            ("<div a=<b>>", "Unexpected character in tag."),
            ("</=>", "End tag name expected."),
            ("<>body", "no"),
        ];
        for (text, expected) in cases {
            let diagnostics = html_diagnostics(text);
            if *expected == "no" {
                assert!(diagnostics.is_empty(), "{text:?} produced {diagnostics:?}");
                continue;
            }
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == *expected),
                "{text:?} did not report {expected}: {diagnostics:?}"
            );
        }

        // Nesting is never validated, raw-text contents stay literal, and
        // comments/doctypes are skipped.
        for valid in [
            "<html>\n<body><div><span>hi</span></div></body>\n</html>",
            "<script>if (a < b) { console.log(\"</div>\"); }</script>",
            "<style>a > b { color: red }</style>",
            "<!-- <div> --><p>x</p>",
            "<!doctype html><input type=\"text\" required />",
            "<my-element data-x=1></my-element>",
            "<div class=\"a\" id='b' hidden></div>",
        ] {
            let diagnostics = language_diagnostics("page.html", valid);
            assert!(diagnostics.is_empty(), "{valid:?} produced {diagnostics:?}");
        }

        assert!(
            language_diagnostics("page.htm", "< div>").len() == 1,
            ".htm routes through the HTML scanner"
        );
        assert!(
            language_diagnostics("fragment.xml", "< div>").is_empty(),
            "XML keeps its own provider"
        );
    }

    #[test]
    fn css_diagnostics_report_worker_parser_errors() {
        let missing_colon = language_diagnostics("x.css", "a {\n  color red;\n}\n");
        assert!(
            missing_colon
                .iter()
                .any(|diagnostic| diagnostic.message == "colon expected" && diagnostic.line == 2),
            "{missing_colon:?}"
        );
        let missing_value = language_diagnostics("x.css", "a {\n  color: ;\n}\n");
        assert!(
            missing_value
                .iter()
                .any(|diagnostic| diagnostic.message == "property value expected"),
            "{missing_value:?}"
        );
        let missing_name = language_diagnostics("x.css", "a {\n  : red;\n}\n");
        assert!(
            missing_name
                .iter()
                .any(|diagnostic| diagnostic.message == "identifier expected"),
            "{missing_name:?}"
        );

        for valid in [
            "a { color: red; }\n",
            "a:hover { color: rgb(1, 2, 3); }\n",
            ".a { .b { color: red } }\n",
            "@media (min-width: 10px) { .a { color: red } }\n",
            "a { background: url(data:image/png;base64,AAA); }\n",
            "a { content: \"x;y\"; }\n",
            "a { color: red }\n",
            "@import url(\"x.css\");\n",
            "a { --x: { color: red }; }\n",
        ] {
            let diagnostics = language_diagnostics("x.css", valid);
            assert!(diagnostics.is_empty(), "{valid:?} produced {diagnostics:?}");
        }

        // SCSS runs the same declaration parser with its own source label.
        let scss = language_diagnostics("x.scss", "a {\n  color red;\n}\n");
        assert!(
            scss
                .iter()
                .any(|diagnostic| diagnostic.message == "colon expected"
                    && diagnostic.source == "SCSS"),
            "{scss:?}"
        );
    }

    #[test]
    fn json_diagnostics_match_the_worker_messages() {
        let cases: &[(&str, &str)] = &[
            ("{\"a\": 1 \"b\": 2}", "Expected comma"),
            ("[1,,2]", "Value expected"),
            ("{\"a\": 1,}", "Trailing comma"),
            ("{a: 1}", "Property keys must be double-quoted"),
            ("{\"a\" 1}", "Expected ':'"),
            ("{\"a\": 1} x", "End of file expected"),
            ("{\"a\": \"x}", "Unexpected end of string"),
            ("{\"a\": 1e}", "Invalid number"),
            ("{\"a\": {\"b\": ,}}", "Value expected"),
        ];
        for (text, expected) in cases {
            let diagnostics = json_diagnostics(text);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{text} did not report {expected}: {diagnostics:?}"
            );
        }

        // `.json` rejects comments; `.jsonc` allows them, and each comment is
        // reported independently.
        let commented = "// note\n{\n  \"a\": 1\n}\n";
        let comments = json_diagnostics(commented);
        assert_eq!(comments.len(), 1, "{comments:?}");
        assert!(comments[0].message.contains("Comments are not permitted"));
        assert!(jsonc_diagnostics(commented).is_empty());
        let two = json_diagnostics("// one\n{} // two");
        assert_eq!(two.len(), 2, "{two:?}");

        // The scanner is bounded.
        let noisy = "// noisy\n".repeat(150);
        assert!(json_diagnostics(&noisy).len() <= JSON_DIAGNOSTIC_LIMIT);
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





