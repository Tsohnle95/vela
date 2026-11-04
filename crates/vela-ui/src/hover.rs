//! Bounded semantic hover for document declarations.
//!
//! The hover provider resolves the identifier under the pointer to a local
//! declaration and reports its source line plus a shallow inferred scalar type.
//! It deliberately avoids pretending to be a language server.

use crate::symbols;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hover {
    pub word: String,
    pub kind: &'static str,
    pub signature: String,
    pub detail: String,
}

fn line_text(text: &str, line: usize) -> Option<&str> {
    text.split('\n').nth(line.saturating_sub(1))
}

fn statement_kind(line: &str) -> &'static str {
    let line = line.trim_start();
    for (prefix, kind) in [
        ("function", "function"),
        ("class", "class"),
        ("interface", "interface"),
        ("type", "type"),
        ("enum", "enum"),
        ("const", "const"),
        ("let", "let"),
        ("var", "var"),
        ("def", "function"),
        ("fn", "function"),
        ("struct", "struct"),
        ("trait", "trait"),
    ] {
        if line.starts_with(prefix) {
            return kind;
        }
    }
    "symbol"
}

fn inferred_type(line: &str) -> Option<&'static str> {
    let (_, expression) = line.split_once('=')?;
    let expression = expression.trim().trim_end_matches(';').trim();
    if expression.starts_with('"') || expression.starts_with('\'') || expression.starts_with('`') {
        return Some("string");
    }
    if expression.starts_with('[') {
        return Some("array");
    }
    if expression.starts_with('{') {
        return Some("object");
    }
    if expression.starts_with("function")
        || expression.contains("=>")
        || expression.starts_with("async ")
    {
        return Some("function");
    }
    if expression == "true" || expression == "false" {
        return Some("boolean");
    }
    if expression == "null" {
        return Some("null");
    }
    if expression.parse::<f64>().is_ok() {
        return Some("number");
    }
    None
}

pub(super) fn hover_at(text: &str, offset: usize) -> Option<Hover> {
    let (_, word) = symbols::word_at(text, offset)?;
    let declaration = symbols::declarations(text)
        .into_iter()
        .find(|declaration| declaration.name == word)?;
    let line = line_text(text, declaration.location.line)?;
    let signature = line.trim().to_owned();
    let kind = statement_kind(line);
    let detail = if matches!(kind, "const" | "let" | "var") {
        inferred_type(line)
            .map(|inferred| format!("{inferred} · line {}", declaration.location.line))
            .unwrap_or_else(|| format!("line {}", declaration.location.line))
    } else {
        format!("{kind} · line {}", declaration.location.line)
    };
    Some(Hover {
        word,
        kind,
        signature,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_resolves_declarations_and_scalar_types() {
        let text = "const answer = 42;\nlet name = 'Vela';\nconst flags = [1, 2];\nconst config = { a: 1 };\nconst fnValue = () => 1;\nfunction greet(who) {}\nclass Widget {}\ntype Alias = string;\nanswer;";
        let answer = text.rfind("answer").unwrap() + 2;
        let hover = hover_at(text, answer).unwrap();
        assert_eq!(hover.word, "answer");
        assert_eq!(hover.kind, "const");
        assert_eq!(hover.signature, "const answer = 42;");
        assert_eq!(hover.detail, "number · line 1");

        let name = text.find("name =").unwrap();
        assert_eq!(hover_at(text, name).unwrap().detail, "string · line 2");
        let flags = text.find("flags =").unwrap();
        assert_eq!(hover_at(text, flags).unwrap().detail, "array · line 3");
        let config = text.find("config =").unwrap();
        assert_eq!(hover_at(text, config).unwrap().detail, "object · line 4");
        let fn_value = text.find("fnValue =").unwrap();
        assert_eq!(
            hover_at(text, fn_value).unwrap().detail,
            "function · line 5"
        );
        let greet = text.find("greet(").unwrap();
        let hover = hover_at(text, greet).unwrap();
        assert_eq!(hover.kind, "function");
        assert_eq!(hover.detail, "function · line 6");
        let widget = text.find("Widget").unwrap();
        assert_eq!(hover_at(text, widget).unwrap().kind, "class");
        let alias = text.find("Alias").unwrap();
        assert_eq!(hover_at(text, alias).unwrap().kind, "type");
    }

    #[test]
    fn hover_ignores_unknown_words_and_whitespace() {
        let text = "const answer = 42;\nmissing;";
        assert!(hover_at(text, text.find("missing").unwrap()).is_none());
        assert!(hover_at(text, 0).is_none());
        assert!(hover_at("  \n\t", 1).is_none());
        assert!(hover_at("const café = true;", "const café".len()).is_some());
    }
}
