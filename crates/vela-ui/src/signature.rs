//! Bounded signature help for local function calls.
//!
//! Declarations are read from the same document symbol index as hover and
//! go-to-definition. The active parameter is the top-level comma bucket at the
//! caret; nested calls/arrays/objects and string commas do not advance it.

use crate::symbols;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SignatureHelp {
    pub name: String,
    pub params: Vec<String>,
    pub active: usize,
}

impl SignatureHelp {
    pub fn label(&self) -> String {
        format!("{}({})", self.name, self.params.join(", "))
    }

    pub fn active_label(&self) -> Option<&str> {
        self.params.get(self.active).map(String::as_str)
    }
}

fn split_params(text: &str) -> Vec<String> {
    let mut params = Vec::new();
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
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                let param = text[start..index].trim();
                if !param.is_empty() {
                    params.push(param.to_owned());
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let param = text[start..].trim();
    if !param.is_empty() {
        params.push(param.to_owned());
    }
    params
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text[open..].char_indices() {
        let index = open + index;
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
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parameters(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let is_function = trimmed.starts_with("function")
        || trimmed.starts_with("async function")
        || trimmed.contains("=>")
        || trimmed
            .split_once('=')
            .is_some_and(|(_, value)| value.trim_start().starts_with("function"));
    if !is_function {
        return None;
    }
    if let Some(open) = line.find('(').filter(|open| line[*open..].contains(')'))
        && let Some(close) = matching_paren(line, open)
    {
        return Some(split_params(&line[open + 1..close]));
    }
    if let Some((before, _)) = line.split_once("=>") {
        let single = before
            .rsplit('=')
            .next()
            .unwrap_or(before)
            .trim()
            .trim_end();
        let name = single
            .char_indices()
            .rev()
            .take_while(|(_, ch)| ch.is_alphanumeric() || *ch == '_' || *ch == '$')
            .last()
            .map(|(index, _)| single[index..].trim())
            .filter(|name| !name.is_empty() && !name.contains(char::is_whitespace));
        if let Some(name) = name {
            return Some(vec![name.to_owned()]);
        }
    }
    Some(Vec::new())
}

fn call_context(text: &str, caret: usize) -> Option<(String, usize)> {
    let caret = caret.min(text.len());
    let mut stack = Vec::<(char, usize)>::new();
    let mut quote = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let chars = text.char_indices().collect::<Vec<_>>();
    for (position, (index, ch)) in chars.iter().copied().enumerate() {
        if index >= caret {
            break;
        }
        if line_comment {
            if ch == '\n' {
                line_comment = false;
            }
            continue;
        }
        if block_comment {
            if ch == '*' && text[index..].starts_with("*/") {
                block_comment = false;
            }
            continue;
        }
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
        if ch == '/' && text[index..].starts_with("//") {
            line_comment = true;
            continue;
        }
        if ch == '/' && text[index..].starts_with("/*") {
            block_comment = true;
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => stack.push((ch, index)),
            ')' | ']' | '}' => {
                if let Some((open, _)) = stack.last()
                    && matches!((*open, ch), ('(', ')') | ('[', ']') | ('{', '}'))
                {
                    stack.pop();
                }
            }
            _ => {}
        }
        let _ = position;
    }
    let open = stack.iter().rposition(|(ch, _)| *ch == '(')?;
    let (_, offset) = stack[open];
    let before = text[..offset].trim_end();
    let name_start = before
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, _)| index)?;
    let name = &before[name_start..];
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), offset))
}

fn active_parameter(text: &str) -> usize {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    let mut active = 0;
    for ch in text.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => active += 1,
            _ => {}
        }
    }
    active
}

pub(super) fn signature_help(text: &str, caret: usize) -> Option<SignatureHelp> {
    let (name, open) = call_context(text, caret)?;
    let before = text[..open].trim_end();
    let name_start = before
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
        .last()
        .map(|(index, _)| index)?;
    let prefix = text[..name_start].trim_end();
    if prefix.ends_with("function")
        || prefix.ends_with("function*")
        || prefix.ends_with('=')
        || prefix.ends_with("=>")
    {
        return None;
    }
    let declaration = symbols::declarations(text)
        .into_iter()
        .find(|declaration| declaration.name == name)?;
    let line = text
        .split('\n')
        .nth(declaration.location.line.saturating_sub(1))?;
    let params = parameters(line)?;
    let active = active_parameter(&text[open + 1..caret.min(text.len())])
        .min(params.len().saturating_sub(1));
    Some(SignatureHelp {
        name,
        params,
        active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_help_tracks_active_parameter_and_nested_commas() {
        let text = "function greet(who, punctuation) {}\ngreet('hi, there', );\n";
        let caret = text.rfind(" )").unwrap();
        let help = signature_help(text, caret).unwrap();
        assert_eq!(help.name, "greet");
        assert_eq!(help.params, vec!["who", "punctuation"]);
        assert_eq!(help.active, 1);
        assert_eq!(help.active_label(), Some("punctuation"));
        assert_eq!(help.label(), "greet(who, punctuation)");

        let first = text.rfind("greet(").unwrap() + "greet(".len() + 3;
        assert_eq!(signature_help(text, first).unwrap().active, 0);

        let nested = "function run(a, b) {}\nrun({ x: 1, y: 2 }, [1, 2]);";
        let caret = nested.find(", [1").unwrap() + 2;
        assert_eq!(signature_help(nested, caret).unwrap().active, 1);
    }

    #[test]
    fn arrow_and_function_expression_declarations_are_supported() {
        let arrow = "const pick = (left, right) => left;\npick(1, 2);";
        let caret = arrow.rfind("2").unwrap();
        let help = signature_help(arrow, caret).unwrap();
        assert_eq!(help.params, vec!["left", "right"]);
        assert_eq!(help.active, 1);

        let single = "const wrap = value => value;\nwrap(";
        let help = signature_help(single, single.len()).unwrap();
        assert_eq!(help.params, vec!["value"]);

        let expression = "const parse = function(input, radix) {};\nparse(";
        let help = signature_help(expression, expression.len()).unwrap();
        assert_eq!(help.params, vec!["input", "radix"]);
    }

    #[test]
    fn signature_help_requires_a_known_open_call() {
        let text = "function known(a) {}\nunknown(";
        assert!(signature_help(text, text.len()).is_none());
        let complete = "function known(a) {}\nknown(1)";
        assert!(signature_help(complete, complete.len()).is_none());
        let member = "function item(a) {}\nobject.item(";
        assert_eq!(signature_help(member, member.len()).unwrap().name, "item");
        let no_params = "function ping() {}\nping(";
        assert_eq!(
            signature_help(no_params, no_params.len())
                .unwrap()
                .params
                .len(),
            0
        );
        let declaration = "function greet(a)";
        assert!(signature_help(declaration, declaration.find('(').unwrap() + 1).is_none());
    }
}
