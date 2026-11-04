//! Bounded document formatting for the native editor.
//!
//! Source exposes Monaco's worker-backed formatters. Native does not embed a
//! language server, so this module implements the safe subset that preserves
//! every non-whitespace token: JSON uses an order-preserving pretty printer and
//! JavaScript/TypeScript/CSS-family files normalize indentation and trailing
//! whitespace. Multiline strings, templates and block comments are left
//! byte-for-byte intact. JSX and HTML are deliberately skipped because text
//! whitespace is meaningful there.

pub(super) struct Edit {
    pub range: std::ops::Range<usize>,
    pub text: String,
}

pub(super) fn document(rel: &str, text: &str) -> Option<String> {
    let lower = rel.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or_default();
    let formatted = match extension {
        "json" => format_json(text)?,
        "css" | "scss" | "less" => format_braced(text, false),
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" => format_braced(text, true),
        "jsx" | "tsx" => format_jsx(text),
        "html" | "htm" => format_html(text),
        _ => return None,
    };
    (formatted != text).then_some(formatted)
}

/// Source maps an empty selection to the caret line before formatting. The
/// replacement always covers whole lines, so text outside the selection stays
/// byte-for-byte intact. An unchanged edit is still returned so the editor can
/// preserve every cursor while formatting a multi-selection.
pub(super) fn selection_edit(
    rel: &str,
    text: &str,
    selected: std::ops::Range<usize>,
) -> Option<Edit> {
    let range = line_range(text, selected);
    let source = &text[range.clone()];
    if source.is_empty() {
        return None;
    }
    let lower = rel.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or_default();
    let formatted = match extension {
        "json" => {
            let depth = json_prefix_depth(&text[..range.start])?;
            format_json_selection(source, depth)?
        }
        "css" | "scss" | "less" => {
            let depth = braced_prefix_depth(&text[..range.start], false)?;
            format_braced_from(source, false, depth)
        }
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" => {
            let depth = braced_prefix_depth(&text[..range.start], true)?;
            format_braced_from(source, true, depth)
        }
        "jsx" | "tsx" => {
            let state = jsx_prefix_state(&text[..range.start])?;
            format_jsx_from(source, state)
        }
        "html" | "htm" => {
            let depth = html_prefix_depth(&text[..range.start])?;
            format_html_from(source, depth)
        }
        _ => return None,
    };
    let formatted = formatted
        .strip_suffix('\n')
        .unwrap_or(&formatted)
        .to_owned();
    Some(Edit {
        range,
        text: formatted,
    })
}

/// Expand an empty selection to its caret line and a non-empty selection to
/// the whole lines it touches. Source merges touching/overlapping expanded
/// ranges before formatting.
pub(super) fn line_range(text: &str, selected: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let start = selected.start.min(text.len());
    let end = selected.end.min(text.len()).max(start);
    let start = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let mut end = end;
    if end > start && text.as_bytes().get(end - 1) == Some(&b'\n') {
        end -= 1;
    }
    let end = text[end..]
        .find('\n')
        .map_or(text.len(), |index| end + index);
    start..end
}

fn braced_prefix_depth(prefix: &str, script: bool) -> Option<usize> {
    let mut depth = 0usize;
    let mut block_comment = false;
    let mut template = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for line in prefix.split('\n') {
        scan_line(
            line,
            script,
            &mut depth,
            &mut block_comment,
            &mut template,
            &mut quote,
            &mut escaped,
        );
    }
    if block_comment || template || quote.is_some() {
        return None;
    }
    Some(depth)
}

fn json_prefix_depth(prefix: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in prefix.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' | '[' => depth += 1,
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    (!in_string).then_some(depth)
}

fn format_json_selection(source: &str, depth: usize) -> Option<String> {
    let indent = "  ".repeat(depth);
    if let Some(formatted) = format_json(source) {
        return Some(indent_json_lines(&formatted, &indent));
    }
    let trimmed = source.trim_end();
    let (body, trailing_comma) = match trimmed.strip_suffix(',') {
        Some(body) => (body, true),
        None => (trimmed, false),
    };
    if body.trim().is_empty() {
        return None;
    }
    for (open, close) in [('{', '}'), ('[', ']')] {
        let wrapped = format!("{open}\n{body}\n{close}");
        let Some(formatted) = format_json(&wrapped) else {
            continue;
        };
        let mut lines = formatted.lines().collect::<Vec<_>>();
        if lines.len() < 2 {
            continue;
        }
        lines.remove(0);
        lines.pop();
        let mut out = lines
            .into_iter()
            .map(|line| format!("{indent}{}", line.strip_prefix("  ").unwrap_or(line)))
            .collect::<Vec<_>>()
            .join("\n");
        if trailing_comma {
            out.push(',');
        }
        return Some(out);
    }
    None
}

fn indent_json_lines(text: &str, indent: &str) -> String {
    text.trim_end()
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_json(text: &str) -> Option<String> {
    // Validate with the real parser, but format the original token stream so
    // key order and number spellings are preserved.
    serde_json::from_str::<serde_json::Value>(text).ok()?;
    let mut out = String::with_capacity(text.len() + 32);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut just_opened = false;
    let indent = |depth: usize| "  ".repeat(depth);
    for ch in text.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                just_opened = false;
                out.push(ch);
            }
            '{' | '[' => {
                out.push(ch);
                depth += 1;
                out.push('\n');
                out.push_str(&indent(depth));
                just_opened = true;
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                if just_opened {
                    while out.ends_with(' ') {
                        out.pop();
                    }
                    if out.ends_with('\n') {
                        out.pop();
                    }
                } else {
                    out.push('\n');
                    out.push_str(&indent(depth));
                }
                out.push(ch);
                just_opened = false;
            }
            ',' => {
                out.push(',');
                out.push('\n');
                out.push_str(&indent(depth));
                just_opened = false;
            }
            ':' => {
                out.push_str(": ");
                just_opened = false;
            }
            ch if ch.is_whitespace() => {}
            other => {
                out.push(other);
                just_opened = false;
            }
        }
    }
    while out.ends_with([' ', '\n']) {
        out.pop();
    }
    out.push('\n');
    Some(out)
}

#[derive(Default)]
struct HtmlState {
    depth: usize,
    raw: Option<String>,
    comment: bool,
}

impl HtmlState {
    fn format_line(&mut self, line: &str) -> String {
        if let Some(tag) = self.raw.clone() {
            let trimmed = line.trim_start();
            let closing = format!("</{tag}");
            if trimmed.to_ascii_lowercase().starts_with(&closing) {
                self.depth = self.depth.saturating_sub(1);
                self.raw = None;
                format!("{}{}", "  ".repeat(self.depth), trimmed.trim_end())
            } else {
                line.to_owned()
            }
        } else if self.comment {
            if line.contains("-->") {
                self.comment = false;
            }
            line.to_owned()
        } else {
            let trimmed = line.trim_end();
            if trimmed.trim().is_empty() {
                String::new()
            } else if !trimmed.trim_start().starts_with('<') {
                // Text nodes are whitespace-sensitive in inline contexts.
                line.to_owned()
            } else {
                let content = trimmed.trim_start();
                let indent = self
                    .depth
                    .saturating_sub(usize::from(content.starts_with("</")));
                let mut scan = HtmlScan::default();
                scan_html_line(content, &mut self.depth, &mut scan);
                if scan.comment {
                    self.comment = true;
                }
                if let Some(tag) = scan.raw
                    && !scan.raw_closed
                {
                    self.raw = Some(tag);
                }
                format!("{}{content}", "  ".repeat(indent))
            }
        }
    }
}

#[derive(Default)]
struct JsxState {
    script_depth: usize,
    paren_depth: usize,
    jsx_stack: Vec<String>,
    jsx_expr_depth: usize,
    jsx_parens: Vec<bool>,
    jsx_continuation_depth: usize,
    generic_depth: Option<usize>,
    generic_lines: usize,
    generic_paren_base: Option<usize>,
    block_comment: bool,
    template: bool,
    quote: Option<char>,
    escaped: bool,
}

impl JsxState {
    fn format_line(&mut self, line: &str) -> String {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            return String::new();
        }
        if self.block_comment || self.template || self.quote.is_some() {
            return line.to_owned();
        }
        let content = trimmed.trim_start();
        if let Some(depth) = self.generic_depth {
            let (next_depth, closed) = update_angle_depth(content, depth);
            let indent = if closed {
                self.script_depth
            } else {
                self.script_depth + depth
            };
            let parens_before = self.paren_depth;
            scan_line(
                content,
                true,
                &mut self.script_depth,
                &mut self.block_comment,
                &mut self.template,
                &mut self.quote,
                &mut self.escaped,
            );
            scan_parens(content, &mut self.paren_depth);
            if closed {
                self.generic_depth = None;
                self.generic_lines = 0;
                if content.contains('(') {
                    self.generic_paren_base = Some(parens_before);
                }
            } else {
                self.generic_depth = Some(next_depth);
                self.generic_lines += 1;
                if self.generic_lines > 32 {
                    self.generic_depth = None;
                    self.generic_lines = 0;
                }
            }
            return format!("{}{content}", "  ".repeat(indent));
        }
        if starts_jsx_tag(content) || !self.jsx_stack.is_empty() {
            let closing = content.starts_with("</") || content.starts_with("/>");
            let (opens, closes) = brace_delta(content);
            let closing_expression =
                closes > opens || (content.starts_with(')') && self.jsx_expr_depth > 0);
            let continuation_extra = self.jsx_continuation_depth.saturating_sub(1);
            let indent = (self.script_depth
                + self.paren_depth
                + self.jsx_stack.len()
                + self.jsx_expr_depth
                + continuation_extra)
                .saturating_sub(usize::from(closing_expression))
                .saturating_sub(usize::from(closing));
            scan_jsx_tags(content, &mut self.jsx_stack);
            self.jsx_expr_depth = self
                .jsx_expr_depth
                .saturating_add(opens)
                .saturating_sub(closes);
            scan_jsx_parens(
                content,
                &mut self.jsx_parens,
                &mut self.jsx_continuation_depth,
            );
            if self.jsx_stack.is_empty() && self.jsx_expr_depth == 0 {
                self.jsx_parens.clear();
                self.jsx_continuation_depth = 0;
            }
            return format!("{}{content}", "  ".repeat(indent));
        }
        let closing = content.starts_with('}') || content.starts_with(']');
        let closing_paren = content.starts_with(')');
        let mut indent = self.script_depth.saturating_sub(usize::from(closing));
        if let Some(base) = self.generic_paren_base {
            indent += self
                .paren_depth
                .saturating_sub(base)
                .saturating_sub(usize::from(closing_paren));
        }
        let generic_start = generic_start_depth(content);
        let generic_params = looks_like_generic_parameters(content);
        let has_jsx = !generic_params && contains_jsx_tag(content);
        let normalized = if has_jsx {
            content.to_owned()
        } else {
            normalize_script_spacing(content)
        };
        scan_line(
            &normalized,
            true,
            &mut self.script_depth,
            &mut self.block_comment,
            &mut self.template,
            &mut self.quote,
            &mut self.escaped,
        );
        scan_parens(&normalized, &mut self.paren_depth);
        if let Some(depth) = generic_start {
            self.generic_depth = Some(depth);
            self.generic_lines = 0;
            self.generic_paren_base = None;
        } else if self
            .generic_paren_base
            .is_some_and(|base| self.paren_depth <= base)
        {
            self.generic_paren_base = None;
        }
        if has_jsx {
            scan_jsx_tags(&normalized, &mut self.jsx_stack);
        }
        format!("{}{normalized}", "  ".repeat(indent))
    }
}

fn scan_jsx_parens(line: &str, stack: &mut Vec<bool>, continuation_depth: &mut usize) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut previous = '\0';
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
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '\'' || ch == '"' || ch == '`' {
            quote = Some(ch);
            previous = ch;
            index += 1;
            continue;
        }
        if ch == '(' {
            let continuation = matches!(previous, '?' | ':' | '>' | '&' | '|');
            stack.push(continuation);
            if continuation {
                *continuation_depth += 1;
            }
        } else if ch == ')' && stack.pop() == Some(true) {
            *continuation_depth = continuation_depth.saturating_sub(1);
        }
        if !ch.is_whitespace() {
            previous = ch;
        }
        index += 1;
    }
}

fn brace_delta(line: &str) -> (usize, usize) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut index = 0usize;
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
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '\'' || ch == '"' || ch == '`' {
            quote = Some(ch);
        } else if ch == '{' {
            opens += 1;
        } else if ch == '}' {
            closes += 1;
        }
        index += 1;
    }
    (opens, closes)
}

fn generic_start_depth(content: &str) -> Option<usize> {
    let chars = content.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if *ch != '<' {
            continue;
        }
        let prefix = chars[..index].iter().collect::<String>();
        let prefix = prefix.trim_end();
        let expression_start = prefix.is_empty()
            || prefix.ends_with(['=', '(', ',', ':'])
            || prefix.split_whitespace().next_back().is_some_and(|word| {
                matches!(word, "return" | "yield" | "await" | "async" | "case")
            });
        if !expression_start {
            continue;
        }
        let tail = &chars[index + 1..];
        let tail_text = tail.iter().collect::<String>();
        let trimmed_tail = tail_text.trim();
        let has_marker = trimmed_tail.is_empty()
            || trimmed_tail.contains(',')
            || trimmed_tail.contains('=')
            || trimmed_tail
                .split_whitespace()
                .any(|word| word == "extends");
        if !has_marker {
            continue;
        }
        let mut depth = 1usize;
        let mut quote: Option<char> = None;
        let mut skip = false;
        for ch in tail {
            if let Some(end) = quote {
                if skip {
                    skip = false;
                } else if *ch == '\\' {
                    skip = true;
                } else if *ch == end {
                    quote = None;
                }
                continue;
            }
            if *ch == '"' || *ch == '\'' {
                quote = Some(*ch);
            } else if *ch == '<' {
                depth += 1;
            } else if *ch == '>' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
        }
        if depth > 0 {
            return Some(depth);
        }
    }
    None
}

fn update_angle_depth(line: &str, mut depth: usize) -> (usize, bool) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
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
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == '<' {
            depth += 1;
        } else if ch == '>' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return (0, true);
            }
        }
        index += 1;
    }
    (depth, false)
}

fn format_jsx(text: &str) -> String {
    format_jsx_from(text, JsxState::default())
}

fn format_jsx_from(text: &str, mut state: JsxState) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let mut lines = text.split('\n').collect::<Vec<_>>();
    if trailing {
        lines.pop();
    }
    let mut out = String::with_capacity(text.len() + 16);
    for (index, line) in lines.iter().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let next = state.format_line(line);
        if index > 0 {
            out.push_str(eol);
        }
        out.push_str(&next);
    }
    if trailing {
        out.push_str(eol);
    }
    out
}

fn jsx_prefix_state(prefix: &str) -> Option<JsxState> {
    let mut state = JsxState::default();
    for line in prefix.split('\n') {
        state.format_line(line);
    }
    if state.block_comment || state.template || state.quote.is_some() {
        None
    } else {
        Some(state)
    }
}

fn starts_jsx_tag(text: &str) -> bool {
    if looks_like_generic_parameters(text) {
        return false;
    }
    let mut chars = text.chars();
    if chars.next() != Some('<') {
        return false;
    }
    match chars.next() {
        Some('>' | '/') => true,
        Some(ch) => ch.is_alphabetic() || ch == '_' || ch == '$',
        None => false,
    }
}

fn looks_like_generic_parameters(text: &str) -> bool {
    looks_like_generic_parameters_chars(&text.chars().collect::<Vec<_>>())
}

fn looks_like_generic_parameters_chars(chars: &[char]) -> bool {
    if chars.first() != Some(&'<') {
        return false;
    }
    let mut index = 1usize;
    let mut depth = 1usize;
    let mut quote: Option<char> = None;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(end) = quote {
            if ch == end {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == '<' {
            depth += 1;
        } else if ch == '>' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        index += 1;
    }
    if depth != 0 || index >= chars.len() {
        return false;
    }
    let params = chars[1..index].iter().collect::<String>();
    if params.trim().is_empty() {
        return false;
    }
    let mut cleaned = String::with_capacity(params.len());
    let mut quote: Option<char> = None;
    for ch in params.chars() {
        if let Some(end) = quote {
            if ch == end {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else {
            cleaned.push(ch);
        }
    }
    let rest = chars[index + 1..]
        .iter()
        .collect::<String>()
        .trim_start()
        .to_string();
    let generic_marker = cleaned.contains(',')
        || (cleaned.contains('=') && rest.starts_with('('))
        || cleaned.split_whitespace().any(|word| word == "extends");
    if !generic_marker {
        return false;
    }
    rest.is_empty() || rest.starts_with('(')
}

fn contains_jsx_tag(line: &str) -> bool {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
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
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if ch == '\'' || ch == '"' || ch == '`' {
            quote = Some(ch);
            index += 1;
            continue;
        }
        if ch == '<' {
            if looks_like_generic_parameters_chars(&chars[index..]) {
                index += 1;
                continue;
            }
            let Some(next) = chars.get(index + 1) else {
                break;
            };
            let candidate =
                matches!(next, '>' | '/') || next.is_alphabetic() || matches!(next, '_' | '$');
            if candidate {
                let prefix = chars[..index].iter().collect::<String>();
                let prefix = prefix.trim_end();
                if prefix.is_empty()
                    || prefix.ends_with(['(', '=', ',', '?', ':', '[', '{', '>'])
                    || prefix.split_whitespace().next_back().is_some_and(|word| {
                        matches!(word, "return" | "case" | "yield" | "await" | "else" | "do")
                    })
                {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

fn scan_jsx_tags(line: &str, stack: &mut Vec<String>) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
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
        if ch == '\'' || ch == '"' || ch == '`' {
            quote = Some(ch);
            index += 1;
            continue;
        }
        if ch != '<' {
            index += 1;
            continue;
        }
        let Some(next) = chars.get(index + 1) else {
            break;
        };
        if *next == '/' {
            let end = scan_jsx_tag_end(&chars, index);
            stack.pop();
            index = end;
            continue;
        }
        if *next == '>' {
            stack.push(String::new());
            index += 2;
            continue;
        }
        if !(next.is_alphabetic() || matches!(next, '_' | '$')) {
            index += 1;
            continue;
        }
        let name_start = index + 1;
        let mut name_end = name_start;
        while name_end < chars.len()
            && (chars[name_end].is_alphanumeric()
                || matches!(chars[name_end], '-' | ':' | '_' | '.' | '$'))
        {
            name_end += 1;
        }
        let name = chars[name_start..name_end]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        let end = scan_jsx_tag_end(&chars, index);
        let mut before = end.saturating_sub(2);
        while before > index && chars[before].is_whitespace() {
            before -= 1;
        }
        let self_closing = chars.get(before) == Some(&'/');
        if !self_closing {
            stack.push(name);
        }
        index = end;
    }
}

fn scan_jsx_tag_end(chars: &[char], start: usize) -> usize {
    let mut index = start + 1;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut braces = 0usize;
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
        } else if ch == '"' || ch == '\'' || ch == '`' {
            quote = Some(ch);
        } else if ch == '{' {
            braces += 1;
        } else if ch == '}' {
            braces = braces.saturating_sub(1);
        } else if ch == '>' && braces == 0 {
            return index + 1;
        }
        index += 1;
    }
    chars.len()
}

fn scan_parens(line: &str, depth: &mut usize) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut block = false;
    while index < chars.len() {
        let ch = chars[index];
        if block {
            if ch == '*' && chars.get(index + 1) == Some(&'/') {
                block = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
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
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            block = true;
            index += 2;
            continue;
        }
        if ch == '\'' || ch == '"' || ch == '`' {
            quote = Some(ch);
            index += 1;
            continue;
        }
        match ch {
            '(' => *depth += 1,
            ')' => *depth = depth.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
}

fn format_html(text: &str) -> String {
    format_html_from(text, 0)
}

fn format_html_from(text: &str, start_depth: usize) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let mut lines = text.split('\n').collect::<Vec<_>>();
    if trailing {
        lines.pop();
    }
    let mut state = HtmlState {
        depth: start_depth,
        ..Default::default()
    };
    let mut out = String::with_capacity(text.len() + 16);
    for (index, line) in lines.iter().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let next = state.format_line(line);
        if index > 0 {
            out.push_str(eol);
        }
        out.push_str(&next);
    }
    if trailing {
        out.push_str(eol);
    }
    out
}

fn html_prefix_depth(prefix: &str) -> Option<usize> {
    let mut state = HtmlState::default();
    for line in prefix.split('\n') {
        state.format_line(line);
    }
    if state.raw.is_some() || state.comment {
        None
    } else {
        Some(state.depth)
    }
}

#[derive(Default)]
struct HtmlScan {
    raw: Option<String>,
    raw_closed: bool,
    comment: bool,
}

fn scan_html_line(line: &str, depth: &mut usize, scan: &mut HtmlScan) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] != '<' {
            index += 1;
            continue;
        }
        if chars[index..].starts_with(&['<', '!', '-', '-']) {
            if let Some(end) = line[index + 4..].find("-->") {
                index += 4 + end + 3;
            } else {
                scan.comment = true;
                return;
            }
            continue;
        }
        if chars[index..].starts_with(&['<', '!']) {
            index = scan_tag_end(&chars, index);
            continue;
        }
        let closing = chars.get(index + 1) == Some(&'/');
        let name_start = index + if closing { 2 } else { 1 };
        let mut name_end = name_start;
        while name_end < chars.len()
            && (chars[name_end].is_ascii_alphanumeric()
                || matches!(chars[name_end], '-' | ':' | '_'))
        {
            name_end += 1;
        }
        if name_end == name_start {
            index += 1;
            continue;
        }
        let name = chars[name_start..name_end]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        let end = scan_tag_end(&chars, index);
        let self_closing = end >= index + 2 && chars[end - 2] == '/';
        if closing {
            *depth = depth.saturating_sub(1);
            if scan.raw.as_deref() == Some(name.as_str()) {
                scan.raw_closed = true;
            }
        } else if !self_closing && !html_void(&name) {
            *depth += 1;
            if matches!(name.as_str(), "pre" | "script" | "style" | "textarea") {
                scan.raw = Some(name);
            }
        }
        index = end;
    }
}

fn scan_tag_end(chars: &[char], start: usize) -> usize {
    let mut index = start + 1;
    let mut quote: Option<char> = None;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(end) = quote {
            if ch == end {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == '>' {
            return index + 1;
        }
        index += 1;
    }
    chars.len()
}

fn html_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Word,
    Number,
    String,
    Regex,
    Comment,
    Punct,
}

struct Token {
    text: String,
    kind: TokenKind,
    space_before: bool,
    binary: bool,
}

fn normalize_script_spacing(line: &str) -> String {
    // Template contents are whitespace-sensitive. Leave the whole line alone;
    // indentation is still normalized by the caller.
    if line.contains('`') {
        return line.to_owned();
    }
    let mut tokens = tokenize_script(line);
    classify_comparisons(&mut tokens);
    let ternary = ternary_colons(&tokens);
    let mut out = String::with_capacity(line.len() + 8);
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 && needs_space(&tokens, index, ternary[index]) {
            out.push(' ');
        }
        out.push_str(&token.text);
    }
    out
}

fn tokenize_script(line: &str) -> Vec<Token> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut space_before = false;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            space_before = true;
            index += 1;
            continue;
        }
        let start = index;
        let kind;
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            let text = chars[index..].iter().collect::<String>();
            tokens.push(Token {
                text,
                kind: TokenKind::Comment,
                space_before,
                binary: false,
            });
            break;
        } else if ch == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    break;
                }
                index += 1;
            }
            kind = TokenKind::Comment;
        } else if ch == '\'' || ch == '"' {
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index += 2;
                    continue;
                }
                if chars[index] == ch {
                    index += 1;
                    break;
                }
                index += 1;
            }
            index = index.min(chars.len());
            kind = TokenKind::String;
        } else if ch == '/'
            && regex_allowed(tokens.last())
            && chars.get(index + 1) != Some(&'=')
            && let Some(end) = regex_end(&chars, index)
        {
            index = end;
            kind = TokenKind::Regex;
        } else if ch.is_ascii_digit() {
            index += 1;
            while index < chars.len() {
                let next = chars[index];
                let exponent_sign =
                    matches!(next, '+' | '-') && matches!(chars.get(index - 1), Some('e' | 'E'));
                if next.is_ascii_alphanumeric() || matches!(next, '_' | '.') || exponent_sign {
                    index += 1;
                } else {
                    break;
                }
            }
            kind = TokenKind::Number;
        } else if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            index += 1;
            while index < chars.len()
                && (chars[index].is_alphanumeric() || matches!(chars[index], '_' | '$'))
            {
                index += 1;
            }
            kind = TokenKind::Word;
        } else {
            let mut matched = None;
            for operator in [
                ">>>=", "===", "!==", "**=", "&&=", "||=", "??=", ">>>", "<<=", ">>=", "...", "=>",
                "==", "!=", "<=", ">=", "&&", "||", "??", "?.", "?:", "++", "--", "+=", "-=", "*=",
                "/=", "%=", "&=", "|=", "^=", "**", "<<", ">>",
            ] {
                if chars[index..].starts_with(&operator.chars().collect::<Vec<_>>()) {
                    matched = Some(operator.len());
                    break;
                }
            }
            index += matched.unwrap_or(1);
            kind = TokenKind::Punct;
        }
        let text = chars[start..index].iter().collect::<String>();
        let binary = classify_binary(tokens.last(), &text, kind);
        tokens.push(Token {
            text,
            kind,
            space_before,
            binary,
        });
        space_before = false;
    }
    tokens
}

fn regex_allowed(previous: Option<&Token>) -> bool {
    match previous {
        None => true,
        Some(token) => match token.kind {
            TokenKind::Word => matches!(
                token.text.as_str(),
                "return"
                    | "case"
                    | "throw"
                    | "typeof"
                    | "instanceof"
                    | "in"
                    | "of"
                    | "new"
                    | "delete"
                    | "void"
                    | "yield"
                    | "await"
            ),
            TokenKind::Punct => matches!(
                token.text.as_str(),
                "(" | "["
                    | "{"
                    | ","
                    | ";"
                    | ":"
                    | "="
                    | "=="
                    | "==="
                    | "!="
                    | "!=="
                    | "!"
                    | "?"
                    | "&&"
                    | "||"
                    | "??"
                    | "+"
                    | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "^"
                    | "~"
                    | "<"
                    | ">"
                    | "=>"
            ),
            _ => false,
        },
    }
}

fn regex_end(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    let mut escaped = false;
    let mut class = false;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '[' {
            class = true;
        } else if ch == ']' {
            class = false;
        } else if ch == '/' && !class {
            index += 1;
            while chars.get(index).is_some_and(|ch| ch.is_ascii_alphabetic()) {
                index += 1;
            }
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_binary(text: &str) -> bool {
    matches!(
        text,
        "=" | "=="
            | "==="
            | "!="
            | "!=="
            | "=>"
            | "&&"
            | "||"
            | "??"
            | "+="
            | "-="
            | "*="
            | "/="
            | "%="
            | "&="
            | "|="
            | "^="
            | "**="
            | "&&="
            | "||="
            | "??="
            | "&"
            | "|"
            | "^"
            | "<="
            | ">="
            | "<<"
            | "<<="
    )
}

fn is_value(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Word | TokenKind::Number | TokenKind::String | TokenKind::Regex
    ) || matches!(token.text.as_str(), ")" | "]" | "}")
}

fn keyword_value(text: &str) -> bool {
    matches!(
        text,
        "return"
            | "throw"
            | "case"
            | "typeof"
            | "void"
            | "delete"
            | "in"
            | "of"
            | "new"
            | "yield"
            | "await"
    )
}

fn classify_comparisons(tokens: &mut [Token]) {
    let mut opens = Vec::new();
    let mut close_of = vec![None; tokens.len()];
    let mut closed_by_multi = vec![false; tokens.len()];
    let mut multi_opens = vec![Vec::new(); tokens.len()];
    let mut paired_close = vec![false; tokens.len()];
    for index in 0..tokens.len() {
        match tokens[index].text.as_str() {
            "<" => opens.push(index),
            ">" => {
                if let Some(open) = opens.pop() {
                    close_of[open] = Some(index);
                    paired_close[index] = true;
                }
            }
            ">>" | ">>>" | ">>=" | ">>>=" => {
                let closers = if matches!(tokens[index].text.as_str(), ">>" | ">>=") {
                    2
                } else {
                    3
                };
                let mut closed = Vec::new();
                for _ in 0..closers {
                    let Some(open) = opens.pop() else {
                        break;
                    };
                    closed_by_multi[open] = true;
                    closed.push(open);
                }
                multi_opens[index] = closed;
            }
            _ => {}
        }
    }
    for index in 0..tokens.len() {
        match tokens[index].text.as_str() {
            "<" => {
                let Some(close) = close_of[index] else {
                    if !closed_by_multi[index] {
                        tokens[index].binary = true;
                    }
                    continue;
                };
                if paired_comparison(tokens, index, close, angle_type_context(tokens, index)) {
                    tokens[index].binary = true;
                    tokens[close].binary = true;
                }
            }
            ">" if !paired_close[index] => {
                tokens[index].binary = true;
            }
            // A `>>`/`>>>` token is either a generic closer or a shift
            // operator. Reuse the comparison heuristic against the outermost
            // closed `<`: `a < b >> c` is arithmetic, while
            // `Map<string, Array<number>>(x)` stays a generic call.
            ">>" | ">>>" | ">>=" | ">>>=" => {
                let Some(outer) = multi_opens[index].last().copied() else {
                    tokens[index].binary = true;
                    continue;
                };
                if paired_comparison(tokens, outer, index, angle_type_context(tokens, outer)) {
                    tokens[index].binary = true;
                    for open in &multi_opens[index] {
                        tokens[*open].binary = true;
                    }
                }
            }
            _ => {}
        }
    }
}

fn paired_comparison(
    tokens: &[Token],
    open: usize,
    close: usize,
    type_context: bool,
) -> bool {
    let previous = open.checked_sub(1).map(|index| &tokens[index]);
    let next = tokens.get(close + 1);
    let previous_value = previous.is_some_and(comparison_operand);
    let next_value = next.is_some_and(|token| {
        expression_start(token) && !(type_context && token.text == "[")
    });
    if !previous_value || !next_value {
        return false;
    }
    if previous.is_some_and(|token| {
        token.kind == TokenKind::Word
            && token
                .text
                .chars()
                .next()
                .is_some_and(|ch| ch.is_uppercase() || matches!(ch, '_' | '$'))
    }) {
        return false;
    }
    if next.is_some_and(|token| {
        matches!(
            token.text.as_str(),
            "(" | "."
                | "?."
                | "?"
                | "as"
                | "satisfies"
                | "extends"
                | "implements"
                | "instanceof"
                | "in"
                | "of"
        )
    }) {
        return false;
    }
    true
}

/// Whether an angle pair occurs in type syntax, where `Foo<Bar>[]` must stay
/// tight. Explicit declarations, heritage/`as` types, parameter annotations,
/// generic-parameter defaults and return annotations mark that context;
/// expression assignments, object/ternary colons and default values do not.
fn angle_type_context(tokens: &[Token], open: usize) -> bool {
    let prefix = &tokens[..open];
    if prefix
        .iter()
        .any(|token| matches!(token.text.as_str(), "type" | "interface"))
    {
        return true;
    }
    if prefix
        .iter()
        .rev()
        .take_while(|token| {
            !matches!(
                token.text.as_str(),
                ";" | "{" | "}" | "=" | "=>" | "return"
            )
        })
        .any(|token| matches!(token.text.as_str(), "extends" | "implements" | "as" | "satisfies"))
    {
        return true;
    }
    parameter_type_context(prefix)
        || generic_default_context(prefix)
        || return_type_context(prefix)
        || top_level_colon_context(prefix)
        || class_property_type_context(prefix)
}

/// A colon after an unmatched `(` marks a parameter type, unless the colon is
/// a ternary/object-literal separator or a later `=` starts a default value.
fn parameter_type_context(tokens: &[Token]) -> bool {
    let mut parens = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "(" => parens.push(index),
            ")" => {
                parens.pop();
            }
            _ => {}
        }
    }
    parens
        .iter()
        .rev()
        .any(|paren| parameter_group_has_type(&tokens[paren + 1..]))
}

fn parameter_group_has_type(tokens: &[Token]) -> bool {
    let mut brace = 0usize;
    let mut ternary = false;
    let mut type_seen = false;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "{" => brace += 1,
            "}" => brace = brace.saturating_sub(1),
            "?" => {
                let optional = tokens.get(index + 1).is_some_and(|next| next.text == ":");
                if !optional {
                    ternary = true;
                }
            }
            ":" | "?:" => {
                if ternary && brace == 0 {
                    ternary = false;
                } else if brace == 0 || type_seen {
                    type_seen = true;
                }
            }
            "=" => {
                if type_seen {
                    type_seen = false;
                }
            }
            "," if brace == 0 => {
                type_seen = false;
                ternary = false;
            }
            "=>" => type_seen = false,
            _ => {}
        }
    }
    type_seen
}

/// A `<...>` generic parameter list with a default (`T = ...`) or constraint
/// (`T extends ...`) before the current angle marks a type position. Ternary
/// branches introduce `?`/`;`/`return` and therefore cannot be generic params.
fn generic_default_context(tokens: &[Token]) -> bool {
    let mut angles = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "<" => angles.push(index),
            ">" => {
                angles.pop();
            }
            ">>" => {
                angles.pop();
                angles.pop();
            }
            ">>>" => {
                angles.pop();
                angles.pop();
                angles.pop();
            }
            _ => {}
        }
    }
    angles.iter().rev().any(|angle| {
        let mut has_default = false;
        let mut invalid = false;
        let mut brace = 0usize;
        for token in &tokens[angle + 1..] {
            match token.text.as_str() {
                "{" => brace += 1,
                "}" => brace = brace.saturating_sub(1),
                "=" | "extends" => has_default = true,
                "?" | ";" | "return" if brace == 0 => invalid = true,
                ":" if brace == 0 => invalid = true,
                _ => {}
            }
        }
        has_default && !invalid
    })
}

/// Class fields carry their type after a colon at the class-body brace depth.
/// An initializer `=` or a method body resets to expression context, while
/// nested object/function types keep the property's type state. Nested class
/// bodies are tracked so methods inside a class do not inherit its fields.
fn class_property_type_context(tokens: &[Token]) -> bool {
    let mut brace = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut angle = 0usize;
    let mut pending: Option<usize> = None;
    let mut bodies: Vec<(usize, usize)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "class" if paren == 0 && bracket == 0 && angle == 0 => pending = Some(brace),
            "{" if paren == 0 && bracket == 0 && angle == 0 => {
                let before = brace;
                brace += 1;
                if pending == Some(before) {
                    bodies.push((brace, index + 1));
                    pending = None;
                }
            }
            "{" => brace += 1,
            "}" => {
                if bodies.last().is_some_and(|(depth, _)| *depth == brace) {
                    bodies.pop();
                }
                brace = brace.saturating_sub(1);
            }
            "(" => paren += 1,
            ")" => paren = paren.saturating_sub(1),
            "[" => bracket += 1,
            "]" => bracket = bracket.saturating_sub(1),
            "<" => angle += 1,
            ">" => angle = angle.saturating_sub(1),
            ">>" => angle = angle.saturating_sub(2),
            ">>>" => angle = angle.saturating_sub(3),
            _ => {}
        }
    }
    let Some(&(base, start)) = bodies.last() else {
        return false;
    };
    class_body_type_state(&tokens[start..], base)
}

fn class_body_type_state(tokens: &[Token], base: usize) -> bool {
    let mut brace = base;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut type_seen = false;
    let mut method_signature = false;
    let mut previous: Option<&str> = None;
    for token in tokens {
        match token.text.as_str() {
            "{" if brace == base => {
                let type_brace = matches!(
                    previous,
                    Some(":") | Some("=>") | Some("|") | Some("&") | Some("[") | Some("<") | Some(",")
                );
                if previous == Some("static") || (method_signature && !type_brace) {
                    type_seen = false;
                }
                method_signature = false;
                brace += 1;
            }
            "{" => brace += 1,
            "}" => {
                brace = brace.saturating_sub(1);
                if brace < base {
                    type_seen = false;
                }
            }
            "(" => {
                if brace == base {
                    method_signature = true;
                }
                paren += 1;
            }
            ")" => paren = paren.saturating_sub(1),
            "[" => bracket += 1,
            "]" => bracket = bracket.saturating_sub(1),
            ":" | "?:" if brace == base && paren == 0 && bracket == 0 => {
                type_seen = true;
            }
            "=" if brace == base && paren == 0 && bracket == 0 => {
                type_seen = false;
                method_signature = false;
            }
            ";" | "," if brace == base && paren == 0 && bracket == 0 => {
                type_seen = false;
                method_signature = false;
            }
            "=>" if brace == base && !type_seen => {
                method_signature = false;
            }
            _ => {}
        }
        previous = Some(token.text.as_str());
    }
    type_seen
}

/// A colon after `)` is a return type unless a ternary `?` precedes it at the
/// same nesting level.
fn return_type_context(tokens: &[Token]) -> bool {
    let Some(colon) = tokens.iter().rposition(|token| token.text == ":") else {
        return false;
    };
    if tokens.get(colon.wrapping_sub(1)).map(|token| token.text.as_str()) != Some(")") {
        return false;
    }
    let mut depth = 0usize;
    for token in tokens[..colon].iter().rev() {
        match token.text.as_str() {
            ")" => depth += 1,
            "(" => depth = depth.saturating_sub(1),
            "?" if depth == 0 => return false,
            ";" | "return" | "=>" if depth == 0 => break,
            _ => {}
        }
    }
    let mut previous = ":";
    for token in &tokens[colon + 1..] {
        match token.text.as_str() {
            "{" => {
                let type_brace = matches!(
                    previous,
                    ":" | "=>" | "|" | "&" | "(" | "[" | "," | "<"
                );
                if !type_brace {
                    return false;
                }
            }
            ";" | "=" => return false,
            "=>" if previous != ")" => return false,
            _ => {}
        }
        previous = token.text.as_str();
    }
    true
}

/// Top-level annotation colon before an assignment. Colons inside parens,
/// braces or brackets are handled by the more specific helpers above.
fn top_level_colon_context(tokens: &[Token]) -> bool {
    let mut paren = 0usize;
    let mut brace = 0usize;
    let mut bracket = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => paren += 1,
            ")" => paren = paren.saturating_sub(1),
            "{" => brace += 1,
            "}" => brace = brace.saturating_sub(1),
            "[" => bracket += 1,
            "]" => bracket = bracket.saturating_sub(1),
            "=" if paren == 0 && brace == 0 && bracket == 0 => return false,
            ":" if paren == 0 && brace == 0 && bracket == 0 => return true,
            ";" => return false,
            _ => {}
        }
    }
    false
}

/// Tokens that can begin the right operand of a comparison. TypeScript parses
/// `a<b>[c]` and `a<b>!c` as comparisons, but keeps `a<b>(c)`, `a<b>?.c`,
/// `a<b> as T`, `a<b> && c` and `a<b> ** c` as instantiation expressions.
fn expression_start(token: &Token) -> bool {
    match token.kind {
        TokenKind::Word => !matches!(
            token.text.as_str(),
            "as" | "satisfies" | "extends" | "implements" | "in" | "of" | "instanceof"
        ),
        TokenKind::Number | TokenKind::String | TokenKind::Regex => true,
        _ => matches!(
            token.text.as_str(),
            "!" | "~" | "+" | "-" | "++" | "--" | "[" | "{"
        ),
    }
}

fn comparison_operand(token: &Token) -> bool {
    match token.kind {
        TokenKind::Word => can_end_expression(token),
        TokenKind::Number | TokenKind::String | TokenKind::Regex => true,
        _ => matches!(token.text.as_str(), ")" | "]"),
    }
}

fn classify_binary(previous: Option<&Token>, text: &str, kind: TokenKind) -> bool {
    if kind != TokenKind::Punct {
        return false;
    }
    match text {
        "+" | "-" => previous.is_some_and(can_end_expression),
        "*" => previous.is_some_and(|token| {
            !matches!(token.text.as_str(), "function" | "yield") && can_end_expression(token)
        }),
        "/" | "%" => previous.is_some_and(can_end_expression),
        "**" | "?" => true,
        _ => false,
    }
}

fn can_end_expression(token: &Token) -> bool {
    match token.kind {
        TokenKind::Word => !matches!(
            token.text.as_str(),
            "return"
                | "throw"
                | "case"
                | "typeof"
                | "void"
                | "delete"
                | "new"
                | "in"
                | "of"
                | "yield"
                | "await"
                | "else"
                | "do"
                | "if"
                | "for"
                | "while"
                | "switch"
                | "catch"
                | "function"
                | "class"
                | "extends"
                | "implements"
                | "as"
                | "satisfies"
        ),
        TokenKind::Number | TokenKind::String | TokenKind::Regex => true,
        _ => matches!(token.text.as_str(), ")" | "]" | "}" | "++" | "--"),
    }
}

fn ternary_colons(tokens: &[Token]) -> Vec<bool> {
    let mut marks = vec![false; tokens.len()];
    for index in 0..tokens.len() {
        if tokens[index].text != ":" {
            continue;
        }
        let mut depth = 0usize;
        for previous in tokens[..index].iter().rev() {
            match previous.text.as_str() {
                ")" | "]" | "}" => depth += 1,
                "(" | "[" | "{" => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                ";" | "," if depth == 0 => break,
                "?" if depth == 0 => {
                    marks[index] = true;
                    break;
                }
                _ => {}
            }
        }
    }
    marks
}

fn needs_space(tokens: &[Token], index: usize, ternary_colon: bool) -> bool {
    let previous = &tokens[index - 1];
    let current = &tokens[index];
    let left = previous.text.as_str();
    let right = current.text.as_str();
    if current.kind == TokenKind::Comment {
        return !matches!(left, "(" | "[" | "{" | "." | "?." | "..." | "#" | "@");
    }
    if previous.kind == TokenKind::Comment {
        return !matches!(right, ")" | "]" | "}" | "," | ";" | "." | "?.");
    }
    if is_binary(left) || is_binary(right) || previous.binary || current.binary {
        return true;
    }
    if matches!(right, "+" | "-") && !current.binary {
        return keyword_value(left);
    }
    if matches!(left, "+" | "-") && !previous.binary {
        return false;
    }
    if right == "*" && !current.binary {
        return false;
    }
    if left == "*" && !previous.binary {
        return true;
    }
    if right == "," || right == ";" {
        return false;
    }
    if right == ":" {
        return ternary_colon;
    }
    if left == "," || left == ";" {
        if right == "}" {
            return true;
        }
        return !matches!(right, ")" | "]" | ">" | ">>" | ">>>" | "," | ";");
    }
    if right == "?:" {
        return false;
    }
    if left == "?:" {
        return true;
    }
    if left == ":" {
        return true;
    }
    if right == "(" {
        return matches!(
            left,
            "if" | "for" | "while" | "switch" | "catch" | "with" | "function"
        ) || keyword_value(left);
    }
    if right == "[" {
        return keyword_value(left);
    }
    if right == "{" {
        return !matches!(left, "(" | "[" | "{" | "," | "." | "?." | "..." | "#" | "@");
    }
    if right == "}" {
        return left != "{";
    }
    if left == "{" {
        return right != "}";
    }
    if matches!(right, ")" | "]" | "." | "?.") {
        return false;
    }
    if matches!(left, "(" | "[" | "." | "?." | "..." | "#" | "@") {
        return false;
    }
    if right == "++" || right == "--" {
        return false;
    }
    if left == "}" {
        if matches!(right, ")" | "]" | "," | ";" | "." | "?.") {
            return false;
        }
        return matches!(right, "else" | "catch" | "finally" | "while") || is_value(current);
    }
    if is_value(previous) && is_value(current) {
        return true;
    }
    current.space_before
}

fn format_braced(text: &str, script: bool) -> String {
    format_braced_from(text, script, 0)
}

fn format_braced_from(text: &str, script: bool, start_depth: usize) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let mut lines = text.split('\n').collect::<Vec<_>>();
    if trailing {
        lines.pop();
    }
    if lines.is_empty() {
        return text.to_owned();
    }
    let mut depth = start_depth;
    let mut block_comment = false;
    let mut template = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut out = String::with_capacity(text.len() + 16);
    for (index, line) in lines.iter().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let preserved = block_comment || template || quote.is_some();
        let next = if preserved {
            line.to_owned()
        } else {
            let trimmed = line.trim_end();
            if trimmed.trim().is_empty() {
                String::new()
            } else {
                let leading = trimmed.len() - trimmed.trim_start().len();
                let content = &trimmed[leading..];
                let content = if script {
                    normalize_script_spacing(content)
                } else {
                    content.to_owned()
                };
                let closing = content.starts_with('}') || content.starts_with(']');
                let indent = depth.saturating_sub(usize::from(closing));
                format!("{}{content}", "  ".repeat(indent))
            }
        };
        if index > 0 {
            out.push_str(eol);
        }
        out.push_str(&next);
        scan_line(
            next.trim_start(),
            script,
            &mut depth,
            &mut block_comment,
            &mut template,
            &mut quote,
            &mut escaped,
        );
    }
    if trailing {
        out.push_str(eol);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn scan_line(
    line: &str,
    script: bool,
    depth: &mut usize,
    block_comment: &mut bool,
    template: &mut bool,
    quote: &mut Option<char>,
    escaped: &mut bool,
) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut previous_significant = '\0';
    while index < chars.len() {
        let ch = chars[index];
        if *block_comment {
            if ch == '*' && chars.get(index + 1) == Some(&'/') {
                *block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if *template {
            if *escaped {
                *escaped = false;
            } else if ch == '\\' {
                *escaped = true;
            } else if ch == '`' {
                *template = false;
            }
            index += 1;
            continue;
        }
        if let Some(end) = *quote {
            if *escaped {
                *escaped = false;
            } else if ch == '\\' {
                *escaped = true;
            } else if ch == end {
                *quote = None;
            }
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            return;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            *block_comment = true;
            index += 2;
            continue;
        }
        if script && ch == '/' && regex_can_start(previous_significant) {
            index = skip_regex(&chars, index);
            previous_significant = '/';
            continue;
        }
        if ch == '\'' || ch == '"' {
            *quote = Some(ch);
            previous_significant = ch;
            index += 1;
            continue;
        }
        if script && ch == '`' {
            *template = true;
            previous_significant = ch;
            index += 1;
            continue;
        }
        match ch {
            '{' | '[' => *depth += 1,
            '}' | ']' => *depth = depth.saturating_sub(1),
            _ => {}
        }
        if !ch.is_whitespace() {
            previous_significant = ch;
        }
        index += 1;
    }
}

fn regex_can_start(previous: char) -> bool {
    previous == '\0'
        || matches!(
            previous,
            '(' | '=' | ':' | ',' | '!' | '&' | '|' | '?' | '{' | '[' | ';' | '>'
        )
}

fn skip_regex(chars: &[char], start: usize) -> usize {
    let mut index = start + 1;
    let mut escaped = false;
    let mut class = false;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '[' {
            class = true;
        } else if ch == ']' {
            class = false;
        } else if ch == '/' && !class {
            index += 1;
            while chars.get(index).is_some_and(|ch| ch.is_ascii_alphabetic()) {
                index += 1;
            }
            return index;
        }
        index += 1;
    }
    chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_formatting_preserves_key_order_and_number_spelling() {
        let source = r#"{"z":1.0,"a":[true,{"nested":"value"}],"empty":{}}"#;
        let formatted = document("data.json", source).expect("valid JSON formats");
        assert_eq!(
            formatted,
            "{\n  \"z\": 1.0,\n  \"a\": [\n    true,\n    {\n      \"nested\": \"value\"\n    }\n  ],\n  \"empty\": {}\n}\n"
        );
        assert_eq!(document("data.json", &formatted), None);
    }

    #[test]
    fn json_formatting_leaves_invalid_documents_alone() {
        assert_eq!(document("data.json", "{\"a\":}"), None);
        assert_eq!(document("data.json", "[]\n"), None);
    }

    #[test]
    fn script_formatting_reindents_code_and_preserves_templates_comments_and_strings() {
        let source = "function run() {\nconst message = \"  keep  \";\nconst block = `first\n   keep template\nlast`;\n/* keep\n   comment */\nif (message) {\nreturn message;\n}\n}\n";
        let formatted = document("main.ts", source).expect("TypeScript formats");
        assert_eq!(
            formatted,
            "function run() {\n  const message = \"  keep  \";\n  const block = `first\n   keep template\nlast`;\n  /* keep\n   comment */\n  if (message) {\n    return message;\n  }\n}\n"
        );
        assert_eq!(document("main.ts", &formatted), None);
    }

    #[test]
    fn braced_formatting_handles_css_strings_and_nested_rules() {
        let source = "a {\ncolor: red;\ncontent: \"} not a brace\";\n.b {\ndisplay: block;\n}\n}\n";
        let formatted = document("styles.css", source).expect("CSS formats");
        assert_eq!(
            formatted,
            "a {\n  color: red;\n  content: \"} not a brace\";\n  .b {\n    display: block;\n  }\n}\n"
        );
    }

    #[test]
    fn script_spacing_normalizes_delimiters_operators_and_keywords() {
        let source = "function run(a,b){\nconst x={a:1,b:2};\nif(x.a===1){\nreturn x.b;\n}\n}\n";
        let expected = "function run(a, b) {\n  const x = { a: 1, b: 2 };\n  if (x.a === 1) {\n    return x.b;\n  }\n}\n";
        assert_eq!(document("spacing.ts", source).as_deref(), Some(expected));
        assert_eq!(document("spacing.ts", expected), None);
    }

    #[test]
    fn script_spacing_normalizes_arithmetic_ternary_and_generator_forms() {
        let source = "const x=a+b*c;\nconst y=a<b?c:d;\nconst z=-a+ +b;\nfunction*gen(){yield*other();}\ntype T=Array<string>;\nconst f=<T>(x:T)=>x;\nconst p=foo?.bar;\nconst q=foo?.[0];\n";
        let expected = "const x = a + b * c;\nconst y = a < b ? c : d;\nconst z = -a + +b;\nfunction* gen() { yield* other(); }\ntype T = Array<string>;\nconst f = <T>(x: T) => x;\nconst p = foo?.bar;\nconst q = foo?.[0];\n";
        assert_eq!(document("tokens.ts", source).as_deref(), Some(expected));
        assert_eq!(document("tokens.ts", expected), None);
    }

    #[test]
    fn script_spacing_preserves_unary_optional_regex_strings_and_comments() {
        let source = "const a=-x;\nconst b=!flag;\nconst c=obj?.value;\nconst re=/a,  b {}/g; // keep,  spaces\nconst s=\"a,  b\";\nconst t=`a,  b`;\n";
        let expected = "const a = -x;\nconst b = !flag;\nconst c = obj?.value;\nconst re = /a,  b {}/g; // keep,  spaces\nconst s = \"a,  b\";\nconst t=`a,  b`;\n";
        assert_eq!(document("spacing.ts", source).as_deref(), Some(expected));
    }

    #[test]
    fn html_selection_formats_against_surrounding_tag_depth() {
        let source = "<html>\n<body>\n<div>\n<span>hi</span>\n</div>\n</body>\n</html>\n";
        let start = source.find("<div>").unwrap();
        let end = source.find("</div>").unwrap() + "</div>".len();
        let edit = selection_edit("index.html", source, start..end).expect("HTML range formats");
        assert_eq!(edit.range, start..end);
        assert_eq!(edit.text, "    <div>\n      <span>hi</span>\n    </div>");
        assert!(
            selection_edit("script.html", "<script>\nconst x = 1;\n</script>\n", 10..10).is_none(),
            "a selection inside raw HTML content is left alone"
        );
    }

    #[test]
    fn script_spacing_adds_spaces_around_safe_comparisons_but_keeps_generics() {
        let source = "const a=i<10;\nconst b=10>i;\nconst c=foo()<bar;\nconst d=arr[0]>x;\nconst e=a<b;\nconst f=Array<T>;\nconst g=i<-1;\nconst h=a>b;\nconst i=a<b>c;\nconst j=foo<T>();\nconst k=a<b && c>d;\nconst l=Array<T> as const;\nconst m=a<b | c>d;\nconst n=a<b ? c : d>e;\n";
        let expected = "const a = i < 10;\nconst b = 10 > i;\nconst c = foo() < bar;\nconst d = arr[0] > x;\nconst e = a < b;\nconst f = Array<T>;\nconst g = i < -1;\nconst h = a > b;\nconst i = a < b > c;\nconst j = foo<T>();\nconst k = a < b && c > d;\nconst l = Array<T> as const;\nconst m = a < b | c > d;\nconst n = a < b ? c : d > e;\n";
        assert_eq!(
            document("comparisons.ts", source).as_deref(),
            Some(expected)
        );
        assert_eq!(document("comparisons.ts", expected), None);
    }

    #[test]
    fn script_spacing_distinguishes_shifts_from_generic_angle_closers() {
        let source = "const a=x<<2;\nconst b=x>>2;\nconst c=x>>>2;\nconst d=x<<=2;\nconst e=x>>=2;\nconst f=x<y>>2;\nconst g=Map<string,Array<number>>();\nconst h=foo<Bar<Baz>>(x);\nconst i=a<b>c;\nconst j=Array<Array<T>>;\nconst k=a<b<C>>d;\nconst l=A<B<C>>;\n";
        let expected = "const a = x << 2;\nconst b = x >> 2;\nconst c = x >>> 2;\nconst d = x <<= 2;\nconst e = x >>= 2;\nconst f = x < y >> 2;\nconst g = Map<string, Array<number>>();\nconst h = foo<Bar<Baz>>(x);\nconst i = a < b > c;\nconst j = Array<Array<T>>;\nconst k = a < b < C >> d;\nconst l = A<B<C>>;\n";
        assert_eq!(
            document("shifts.ts", source).as_deref(),
            Some(expected)
        );
        assert_eq!(document("shifts.ts", expected), None);
    }

    #[test]
    fn script_spacing_handles_generic_instantiation_vs_comparison_operands() {
        let source = "const a=x<y>[0];\nconst b=x<y>!z;\nconst c=x<y>+z;\nconst d=x<y>-z;\nconst e=x<y>~z;\nconst f=x<y>typeof z;\nconst g=x<y>new Z();\nconst h=x<y>{a:1};\nconst i=x<y>\"s\";\nconst j=x<y>true;\nconst k=foo<Bar>(x);\nconst l=foo<Bar> as T;\nconst m=x<y> && z;\nconst n=x<y> ** z;\nconst o=x<y> ? z : w;\nconst p=x<y>?.z;\ntype T=foo<bar>[];\ntype U=foo<bar>[c];\ninterface I extends foo<bar>[] {}\nconst v: foo<bar>[] = y;\nconst obj={ field: a<b>[c] };\nconst ternary = flag ? a : a<b>[c];\n";
        let expected = "const a = x < y > [0];\nconst b = x < y > !z;\nconst c = x < y > +z;\nconst d = x < y > -z;\nconst e = x < y > ~z;\nconst f = x < y > typeof z;\nconst g = x < y > new Z();\nconst h = x < y > { a: 1 };\nconst i = x < y > \"s\";\nconst j = x < y > true;\nconst k = foo<Bar>(x);\nconst l = foo<Bar> as T;\nconst m = x<y> && z;\nconst n = x<y> ** z;\nconst o = x<y> ? z : w;\nconst p = x<y>?.z;\ntype T = foo<bar>[];\ntype U = foo<bar>[c];\ninterface I extends foo<bar>[] {}\nconst v: foo<bar>[] = y;\nconst obj = { field: a < b > [c] };\nconst ternary = flag ? a : a < b > [c];\n";
        assert_eq!(
            document("instantiation.ts", source).as_deref(),
            Some(expected)
        );
        assert_eq!(document("instantiation.ts", expected), None);
    }

    #[test]
    fn script_spacing_keeps_lowercase_generic_types_in_parameters_and_defaults() {
        let source = "const fn1 = (x: foo<bar>[]) => x;\nconst fn2 = (x: foo<bar>[c], y = a<b>[d]) => a<b>[e];\nfunction fn3<T = foo<bar>[]>(x: T) {}\nconst fn4 = <T = foo<bar>[]>(x: T) => x;\nclass C<T = foo<bar>[]> {}\nconst ret = (x): foo<bar>[] => x;\nconst def = (x: T = a<b>[c]) => a<b>[d];\n";
        let expected = "const fn1 = (x: foo<bar>[]) => x;\nconst fn2 = (x: foo<bar>[c], y = a < b > [d]) => a < b > [e];\nfunction fn3<T = foo<bar>[]>(x: T) {}\nconst fn4 = <T = foo<bar>[]>(x: T) => x;\nclass C<T = foo<bar>[]> {}\nconst ret = (x): foo<bar>[] => x;\nconst def = (x: T = a < b > [c]) => a < b > [d];\n";
        assert_eq!(
            document("generic-types.ts", source).as_deref(),
            Some(expected)
        );
        assert_eq!(document("generic-types.ts", expected), None);
    }

    #[test]
    fn script_spacing_keeps_lowercase_generics_in_class_properties() {
        for (source, expected) in [
            (
                "class A { x: foo<bar>[]; }",
                "class A { x: foo<bar>[]; }",
            ),
            (
                "class A { x?: foo<bar>[c]; }",
                "class A { x?: foo<bar>[c]; }",
            ),
            (
                "class A { x: {a: foo<bar>[]}; }",
                "class A { x: { a: foo<bar>[] }; }",
            ),
            (
                "class A { x: foo<bar>[] = a<b>[c]; }",
                "class A { x: foo<bar>[] = a < b > [c]; }",
            ),
            (
                "class A { x = a<b>[c]; }",
                "class A { x = a < b > [c]; }",
            ),
            (
                "class A { m() { const v = a<b>[c]; } }",
                "class A { m() { const v = a < b > [c]; } }",
            ),
            (
                "class A { m(): foo<bar>[] { const v = a<b>[c]; } }",
                "class A { m(): foo<bar>[] { const v = a < b > [c]; } }",
            ),
            (
                "class A { value: () => foo<bar>[]; }",
                "class A { value: () => foo<bar>[]; }",
            ),
            (
                "class A { static { const s = a<b>[c]; } }",
                "class A { static { const s = a < b > [c]; } }",
            ),
            (
                "class A { m() { class B { x: foo<bar>[]; } const v = a<b>[c]; } }",
                "class A { m() { class B { x: foo<bar>[]; } const v = a < b > [c]; } }",
            ),
            (
                "class A extends Base<foo<bar>[]> { x: foo<bar>[]; }",
                "class A extends Base<foo<bar>[]> { x: foo<bar>[]; }",
            ),
            (
                "class A { x: {a: foo<bar>[]} = a<b>[c]; }",
                "class A { x: { a: foo<bar>[] } = a < b > [c]; }",
            ),
            (
                "class A { x: () => {a: foo<bar>[]}; }",
                "class A { x: () => { a: foo<bar>[] }; }",
            ),
            (
                "class A { x?: {a: foo<bar>[]}; }",
                "class A { x?: { a: foo<bar>[] }; }",
            ),
        ] {
            assert_eq!(
                document("class.ts", source).as_deref().unwrap_or(source),
                expected,
                "source: {source}"
            );
            assert_eq!(document("class.ts", expected), None, "not idempotent: {expected}");
        }
    }

    #[test]
    fn script_spacing_normalizes_bitwise_union_and_intersection_operators() {
        let source = "const x=a&b;\nconst y=a|b;\nconst z=a^b;\nconst w=a&=b;\nconst v=a|=b;\nconst u=a^=b;\ntype T=A|B;\ntype U=A&B;\ntype V=Record<A|B,U&B>;\n";
        let expected = "const x = a & b;\nconst y = a | b;\nconst z = a ^ b;\nconst w = a &= b;\nconst v = a |= b;\nconst u = a ^= b;\ntype T = A | B;\ntype U = A & B;\ntype V = Record<A | B, U & B>;\n";
        assert_eq!(
            document("bitwise.ts", source).as_deref(),
            Some(expected)
        );
        assert_eq!(document("bitwise.ts", expected), None);
    }

    #[test]
    fn type_assertions_after_keywords_keep_angle_spacing() {
        let source = "function f() {\nreturn <Foo>bar;\n}\nconst x = <Foo>bar;\n";
        let expected = "function f() {\n  return <Foo>bar;\n}\nconst x = <Foo>bar;\n";
        assert_eq!(document("assert.ts", source).as_deref(), Some(expected));
    }

    #[test]
    fn generic_start_depth_counts_one_top_level_parameter_list() {
        assert_eq!(generic_start_depth("const identity = <"), Some(1));
        assert_eq!(
            generic_start_depth("const f = <T extends Array<U>,"),
            Some(1)
        );
    }

    #[test]
    fn jsx_nested_arrow_continuations_accumulate_alignment() {
        let source = "const view = (\n<div>\n{items.map((item) => (\nitem.children.map((child) => (\n<span>{child}</span>\n))\n))}\n</div>\n);\n";
        let expected = "const view = (\n  <div>\n    {items.map((item) => (\n      item.children.map((child) => (\n        <span>{child}</span>\n      ))\n    ))}\n  </div>\n);\n";
        assert_eq!(document("arrows.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("arrows.tsx", expected), None);
    }

    #[test]
    fn jsx_nested_ternary_expression_branches_accumulate_alignment() {
        let source = "const view = (\n<div>\n{outer ? (\ncondition ? (\n<span>yes</span>\n) : (\n<b>no</b>\n)\n) : null}\n</div>\n);\n";
        let expected = "const view = (\n  <div>\n    {outer ? (\n      condition ? (\n        <span>yes</span>\n      ) : (\n        <b>no</b>\n      )\n    ) : null}\n  </div>\n);\n";
        assert_eq!(document("nested.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("nested.tsx", expected), None);
    }

    #[test]
    fn jsx_ternary_expression_branches_align_closing_parentheses() {
        let source = "const view = (\n<div>\n{condition ? (\n<span>yes</span>\n) : (\n<span>no</span>\n)}\n</div>\n);\n";
        let expected = "const view = (\n  <div>\n    {condition ? (\n      <span>yes</span>\n    ) : (\n      <span>no</span>\n    )}\n  </div>\n);\n";
        assert_eq!(document("ternary.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("ternary.tsx", expected), None);
    }

    #[test]
    fn jsx_expression_containers_indent_nested_tags_and_close_back() {
        let source =
            "const view = (\n<div>\n{items.map((item) => (\n<span>{item}</span>\n))}\n</div>\n);\n";
        let expected = "const view = (\n  <div>\n    {items.map((item) => (\n      <span>{item}</span>\n    ))}\n  </div>\n);\n";
        assert_eq!(document("expr.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("expr.tsx", expected), None);
    }

    #[test]
    fn tsx_multiline_generic_arrows_track_parameter_depth() {
        let source = "const identity = <\nT,\nU\n>(\nvalue: T\n) => value;\n";
        let expected = "const identity = <\n  T,\n  U\n>(\n  value: T\n) => value;\n";
        assert_eq!(document("multi.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("multi.tsx", expected), None);
    }

    #[test]
    fn tsx_generic_arrows_are_not_mistaken_for_jsx_tags() {
        let source = "const identity=<T,>(value:T)=>value;\nconst constrained=<T extends object>(value:T)=>value;\nconst defaulted=<T=string>(value:T)=>value;\n";
        let expected = "const identity = <T,>(value: T) => value;\nconst constrained = <T extends object>(value: T) => value;\nconst defaulted = <T = string>(value: T) => value;\n";
        assert_eq!(document("generics.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("generics.tsx", expected), None);
    }

    #[test]
    fn jsx_selection_formats_against_the_surrounding_jsx_state() {
        let source = "function App() {\nreturn (\n<div>\n<span>Hi</span>\n</div>\n);\n}\n";
        let start = source.find("<div>").unwrap();
        let end = source.find("</div>").unwrap() + "</div>".len();
        let edit = selection_edit("view.tsx", source, start..end).expect("JSX range formats");
        assert_eq!(edit.range, start..end);
        assert_eq!(edit.text, "    <div>\n      <span>Hi</span>\n    </div>");
    }

    #[test]
    fn jsx_formatting_indents_components_fragments_and_text_lines() {
        let source = "function App(){\nreturn (\n<div className=\"root\">\n<span>Hi</span>\n<Icon/>\n</div>\n);\n}\n";
        let expected = "function App() {\n  return (\n    <div className=\"root\">\n      <span>Hi</span>\n      <Icon/>\n    </div>\n  );\n}\n";
        assert_eq!(document("view.tsx", source).as_deref(), Some(expected));
        assert_eq!(document("view.tsx", expected), None);

        let source = "const view=(\n<>\n<p>Hello</p>\nworld\n</>\n);\n";
        let expected = "const view = (\n  <>\n    <p>Hello</p>\n    world\n  </>\n);\n";
        assert_eq!(document("view.tsx", source).as_deref(), Some(expected));
    }

    #[test]
    fn jsx_formatting_preserves_attributes_and_only_spaces_surrounding_code() {
        let source = "const x = <div className=\"a\" data-x={1} />;\nconst n=1+2;\n";
        let expected = "const x = <div className=\"a\" data-x={1} />;\nconst n = 1 + 2;\n";
        assert_eq!(document("view.tsx", source).as_deref(), Some(expected));
    }

    #[test]
    fn html_formatting_reindents_structure_and_preserves_raw_and_text_lines() {
        let source = "<!DOCTYPE html>\n<html>\n<head>\n<title>Hi</title>\n</head>\n<body>\n<div class=\"a\">\n<p>Hello <strong>world</strong></p>\n<img src=\"x\">\n</div>\n<pre>\n  keep\n</pre>\n<script>\nconst x = {a:1};\n</script>\n</body>\n</html>\n";
        let expected = "<!DOCTYPE html>\n<html>\n  <head>\n    <title>Hi</title>\n  </head>\n  <body>\n    <div class=\"a\">\n      <p>Hello <strong>world</strong></p>\n      <img src=\"x\">\n    </div>\n    <pre>\n  keep\n    </pre>\n    <script>\nconst x = {a:1};\n    </script>\n  </body>\n</html>\n";
        assert_eq!(document("index.html", source).as_deref(), Some(expected));
        assert_eq!(document("index.html", expected), None);
    }

    #[test]
    fn selection_formats_only_the_selected_lines_with_surrounding_depth() {
        let source = "function run() {\nconst a = 1;\nconst b = 2;\nreturn a + b;\n}\n";
        let start = source.find("const a").unwrap();
        let end = source.find("return").unwrap();
        let edit = selection_edit("main.ts", source, start..end).expect("selection formats");
        assert_eq!(
            edit.range,
            source.find("const a").unwrap()..source.find("return").unwrap() - 1
        );
        assert_eq!(edit.text, "  const a = 1;\n  const b = 2;");
        let mut next = source.to_owned();
        next.replace_range(edit.range, &edit.text);
        assert_eq!(
            next,
            "function run() {\n  const a = 1;\n  const b = 2;\nreturn a + b;\n}\n"
        );
    }

    #[test]
    fn empty_selection_formats_the_caret_line_only() {
        let source = "function run() {\nconst a = 1;\nreturn a;\n}\n";
        let caret = source.find("const a").unwrap() + 3;
        let edit = selection_edit("main.ts", source, caret..caret).expect("caret line formats");
        assert_eq!(
            edit.range,
            source.find("const a").unwrap()..source.find("\nreturn").unwrap()
        );
        assert_eq!(edit.text, "  const a = 1;");
    }

    #[test]
    fn json_selection_formats_a_complete_nested_value() {
        let source = "{\n  \"outer\": {\n\"a\": 1,\n\"b\": [2, 3]\n  }\n}\n";
        let start = source.find("\"a\"").unwrap();
        let end = source.find("\n  }").unwrap();
        let edit = selection_edit("data.json", source, start..end).expect("nested value formats");
        assert_eq!(edit.range, start..end);
        assert_eq!(
            edit.text,
            "    \"a\": 1,\n    \"b\": [\n      2,\n      3\n    ]"
        );
    }
}
