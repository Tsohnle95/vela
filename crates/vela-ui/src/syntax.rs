//! Bounded syntax tokenization for Markdown code fences. This is deliberately
//! a small, dependency-free approximation of the Shiki grammars Source loads:
//! enough to color common keywords/strings/numbers/comments without claiming a
//! full language parser. Unknown text and languages stay uncolored.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AnsiStyle {
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxToken {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Property,
    Type,
    Tag,
    Attribute,
    Added,
    Removed,
    Ansi(AnsiStyle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineCodeKind {
    Path,
    Url,
}

fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn has_uri_scheme(text: &str) -> bool {
    let Some(index) = text.find("://") else {
        return false;
    };
    let mut scheme = text[..index].chars();
    scheme.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && scheme.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
}

/// Classify inline code the way Source's Markdown projection does: only
/// credential-free HTTP(S) is a URL, filesystem-looking spans become paths,
/// and everything else stays plain. Extension/filename data comes from the
/// same Linguist-derived tables Source ships.
pub fn inline_code_kind(text: &str) -> Option<InlineCodeKind> {
    if starts_with_ignore_ascii_case(text, "https://")
        || starts_with_ignore_ascii_case(text, "http://")
    {
        return Some(InlineCodeKind::Url);
    }
    if has_uri_scheme(text) || text == "/" {
        return None;
    }
    if text.starts_with('/')
        && text[1..].chars().next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && text[1..]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return None;
    }
    if text.chars().any(char::is_whitespace)
        || text
            .chars()
            .any(|ch| "()[]{}*+=<>|&^\"';".contains(ch))
    {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    if text.contains('/')
        || text.contains('\\')
        || lower.starts_with("./")
        || lower.starts_with("../")
        || crate::inline_code::has_path_extension(&lower)
        || crate::inline_code::has_path_file_name(&lower)
    {
        return Some(InlineCodeKind::Path);
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub token: SyntaxToken,
}

pub fn language(info: &str) -> Option<&'static str> {
    let name = info
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    Some(match name.as_str() {
        "javascript" | "js" | "jsx" => "javascript",
        "typescript" | "ts" | "tsx" => "typescript",
        "json" => "json",
        "css" | "scss" | "less" => "css",
        "html" | "htm" | "xml" => "html",
        "python" | "py" => "python",
        "go" | "golang" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "c++" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => "cpp",
        "ruby" | "rb" => "ruby",
        "kotlin" | "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "dockerfile" | "docker" => "dockerfile",
        "toml" => "toml",
        "lua" => "lua",
        "r" | "rprofile" => "r",
        "scala" | "sc" => "scala",
        "dart" => "dart",
        "powershell" | "ps1" | "pwsh" => "powershell",
        "elixir" | "ex" | "exs" => "elixir",
        "haskell" | "hs" => "haskell",
        "clojure" | "clj" | "cljs" | "cljc" | "edn" => "clojure",
        "julia" | "jl" => "julia",
        "ocaml" | "ml" | "mli" => "ocaml",
        "erlang" | "erl" | "hrl" => "erlang",
        "elm" => "elm",
        "hcl" | "terraform" | "tf" => "hcl",
        "nix" => "nix",
        "puppet" | "pp" => "puppet",
        "graphql" | "gql" => "graphql",
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "zig" => "zig",
        "solidity" | "sol" => "solidity",
        "mdx" => "mdx",
        "prisma" => "prisma",
        "handlebars" => "handlebars",
        "liquid" => "liquid",
        "blade" => "blade",
        "templ" => "templ",
        "pascal" | "pas" => "pascal",
        "makefile" | "make" => "makefile",
        "twig" => "twig",
        "pug" | "jade" => "pug",
        "ini" | "properties" => "ini",
        "csharp" | "cs" | "c#" => "csharp",
        "php" => "php",
        "bash" | "sh" | "shell" | "shellscript" | "zsh" => "bash",
        "yaml" | "yml" => "yaml",
        "sql" => "sql",
        "diff" | "patch" => "diff",
        "markdown" | "md" => "markdown",
        "rust" | "rs" => "rust",
        "ansi" => "ansi",
        _ => return None,
    })
}

fn ansi_named_color(index: u32) -> u32 {
    const COLORS: [u32; 16] = [
        0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
        0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
    ];
    COLORS[(index % 16) as usize]
}

fn ansi_table_color(index: u32) -> u32 {
    if index < 16 {
        return ansi_named_color(index);
    }
    if index < 232 {
        let index = index - 16;
        let steps = [0u32, 95, 135, 175, 215, 255];
        let r = steps[(index / 36) as usize % 6];
        let g = steps[(index / 6) as usize % 6];
        let b = steps[(index % 6) as usize];
        return (r << 16) | (g << 8) | b;
    }
    let gray = 8 + (index.saturating_sub(232)).min(23) * 10;
    (gray << 16) | (gray << 8) | gray
}

fn ansi_color(parts: &[u32], index: &mut usize) -> Option<u32> {
    let mode = *parts.get(*index)?;
    *index += 1;
    match mode {
        5 => {
            let value = *parts.get(*index)?;
            *index += 1;
            Some(ansi_table_color(value))
        }
        2 => {
            let r = *parts.get(*index)?;
            let g = *parts.get(*index + 1)?;
            let b = *parts.get(*index + 2)?;
            *index += 3;
            Some(((r & 0xff) << 16) | ((g & 0xff) << 8) | (b & 0xff))
        }
        _ => None,
    }
}

fn ansi_sgr(sequence: &str, style: &mut AnsiStyle) {
    let parts = sequence
        .split(';')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < parts.len() {
        let code = parts[index];
        index += 1;
        match code {
            0 => *style = AnsiStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            9 => style.strikethrough = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            29 => style.strikethrough = false,
            30..=37 => style.foreground = Some(ansi_named_color(code - 30)),
            38 => style.foreground = ansi_color(&parts, &mut index),
            39 => style.foreground = None,
            40..=47 => style.background = Some(ansi_named_color(code - 40)),
            48 => style.background = ansi_color(&parts, &mut index),
            49 => style.background = None,
            90..=97 => style.foreground = Some(ansi_named_color(code - 90 + 8)),
            100..=107 => style.background = Some(ansi_named_color(code - 100 + 8)),
            _ => {}
        }
    }
}

/// Strip SGR escape sequences and return the visible text plus styled spans.
/// Mirrors Shiki's ANSI path: the escape bytes are not rendered, and styles
/// carry foreground/background colors plus common text decorations.
pub fn ansi(text: &str) -> (String, Vec<Span>) {
    let mut output = String::with_capacity(text.len());
    let mut spans = Vec::new();
    let mut style = AnsiStyle::default();
    let mut run_start = 0usize;
    let mut index = 0usize;
    let flush = |output: &String, run_start: usize, style: AnsiStyle, spans: &mut Vec<Span>| {
        if style != AnsiStyle::default() && run_start < output.len() {
            spans.push(Span {
                start: run_start,
                end: output.len(),
                token: SyntaxToken::Ansi(style),
            });
        }
    };
    while index < text.len() {
        if text[index..].starts_with("\x1b[")
            && let Some(close) = text[index + 2..].find('m')
        {
            let sequence = &text[index + 2..index + 2 + close];
            flush(&output, run_start, style, &mut spans);
            ansi_sgr(sequence, &mut style);
            index += 2 + close + 1;
            run_start = output.len();
            continue;
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        output.push(ch);
        index += ch.len_utf8();
    }
    flush(&output, run_start, style, &mut spans);
    (output, spans)
}

pub fn highlight(language: &str, text: &str) -> Vec<Span> {
    let spans = match language {
        "json" => json(text),
        "javascript" | "typescript" => script(text),
        "css" => css(text),
        "html" => html(text),
        "python" => python(text),
        "go" => go(text),
        "java" => java(text),
        "c" => c_family(text, false),
        "cpp" => c_family(text, true),
        "ruby" => ruby(text),
        "kotlin" => kotlin(text),
        "swift" => swift(text),
        "dockerfile" => dockerfile(text),
        "toml" => toml(text),
        "lua" => lua(text),
        "r" => r(text),
        "scala" => scala(text),
        "dart" => dart(text),
        "powershell" => powershell(text),
        "elixir" => elixir(text),
        "haskell" => haskell(text),
        "clojure" => clojure(text),
        "julia" => julia(text),
        "ocaml" => ocaml(text),
        "erlang" => erlang(text),
        "elm" => elm(text),
        "hcl" => hcl(text),
        "nix" => nix(text),
        "puppet" => puppet(text),
        "graphql" => graphql(text),
        "vue" => vue(text),
        "svelte" => svelte(text),
        "astro" => astro(text),
        "zig" => zig(text),
        "solidity" => solidity(text),
        "mdx" => mdx(text),
        "prisma" => prisma(text),
        "handlebars" => handlebars(text),
        "liquid" => liquid(text),
        "blade" => blade(text),
        "templ" => templ(text),
        "pascal" => pascal(text),
        "makefile" => makefile(text),
        "twig" => twig(text),
        "pug" => pug(text),
        "ini" => ini(text),
        "csharp" => csharp(text),
        "php" => php(text),
        "bash" => bash(text),
        "yaml" => yaml(text),
        "sql" => sql(text),
        "diff" => diff(text),
        "markdown" => markdown(text),
        "rust" => rust(text),
        "ansi" => Vec::new(),
        _ => Vec::new(),
    };
    spans
        .into_iter()
        .filter(|span| {
            span.start < span.end
                && span.end <= text.len()
                && text.is_char_boundary(span.start)
                && text.is_char_boundary(span.end)
        })
        .collect()
}

fn push(spans: &mut Vec<Span>, start: usize, end: usize, token: SyntaxToken) {
    if start < end {
        spans.push(Span { start, end, token });
    }
}

fn next_non_space_index(text: &str, mut index: usize) -> Option<(usize, char)> {
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if !ch.is_whitespace() {
            return Some((index, ch));
        }
        index += ch.len_utf8();
    }
    None
}

fn next_non_space(text: &str, index: usize) -> Option<char> {
    next_non_space_index(text, index).map(|(_, ch)| ch)
}

fn previous_non_space(text: &str, index: usize) -> Option<char> {
    text[..index].chars().rev().find(|ch| !ch.is_whitespace())
}

fn skip_quoted(text: &str, start: usize, quote: char, triple: bool) -> usize {
    let mut index = start;
    let token = if triple {
        quote.to_string().repeat(3)
    } else {
        quote.to_string()
    };
    index += token.len();
    while index < text.len() {
        if triple && text[index..].starts_with(&token) {
            return index + token.len();
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        if !triple && ch == quote {
            return index + ch.len_utf8();
        }
        if ch == '\\' {
            index += ch.len_utf8();
            if let Some(next) = text[index..].chars().next() {
                index += next.len_utf8();
            }
            continue;
        }
        index += ch.len_utf8();
    }
    text.len()
}

fn skip_number(text: &str, start: usize) -> usize {
    let mut index = start;
    let mut previous = '\0';
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        let exponent_sign = matches!(ch, '+' | '-') && matches!(previous, 'e' | 'E');
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_') || exponent_sign {
            previous = ch;
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    index.max(start + 1).min(text.len())
}

fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$')
}

fn identifier_end(text: &str, start: usize) -> usize {
    identifier_end_with(text, start, false)
}

fn identifier_end_hyphen(text: &str, start: usize) -> usize {
    identifier_end_with(text, start, true)
}

fn identifier_end_markup(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if !is_ident_continue(ch) && !matches!(ch, '-' | ':' | '.') {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn identifier_end_with(text: &str, start: usize, hyphen: bool) -> usize {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if !is_ident_continue(ch) && !(hyphen && ch == '-') {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn keyword(word: &str, keywords: &[&str], case_insensitive: bool) -> bool {
    if case_insensitive {
        keywords.iter().any(|keyword| keyword.eq_ignore_ascii_case(word))
    } else {
        keywords.contains(&word)
    }
}

fn json(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '"' {
            let end = skip_quoted(text, index, '"', false);
            let token = if next_non_space(text, end) == Some(':') {
                SyntaxToken::Property
            } else {
                SyntaxToken::String
            };
            push(&mut spans, index, end, token);
            index = end;
        } else if ch == '-' || ch.is_ascii_digit() {
            let end = skip_number(text, index);
            push(&mut spans, index, end, SyntaxToken::Number);
            index = end;
        } else if ch.is_alphabetic() {
            let end = identifier_end(text, index);
            let word = &text[index..end];
            if matches!(word, "true" | "false" | "null") {
                push(&mut spans, index, end, SyntaxToken::Keyword);
            }
            index = end;
        } else {
            index += ch.len_utf8();
        }
    }
    spans
}

fn script(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "accessor", "as", "asserts", "async", "await", "break", "case",
        "catch", "class", "const", "continue", "debugger", "declare", "default", "delete",
        "do", "else", "enum", "export", "extends", "finally", "for", "from", "function",
        "get", "global", "if", "implements", "import", "in", "infer", "instanceof",
        "interface", "is", "keyof", "let", "module", "namespace", "new", "of", "out",
        "override", "private", "protected", "public", "readonly", "require", "return",
        "satisfies", "set", "static", "super", "switch", "this", "throw", "try", "type",
        "typeof", "undefined", "unique", "using", "var", "void", "while", "with", "yield",
        "true", "false", "null", "any", "bigint", "boolean", "never", "number", "object",
        "string", "symbol", "unknown",
    ];
    let mut spans = Vec::new();
    let mut index = 0;
    if text.starts_with("#!") {
        index = text.find('\n').unwrap_or(text.len());
        push(&mut spans, 0, index, SyntaxToken::Comment);
    }
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '/' && regex_allowed(text, index) {
                let end = skip_regex(text, index);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '`' {
                index = template(text, index, &mut spans);
            } else if matches!(ch, '\'' | '"') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index + 1, end, SyntaxToken::Function);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '#' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = skip_number(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if keyword(word, KEYWORDS, false) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else {
                    let next = next_non_space_index(text, end);
                    let function = next.is_some_and(|(position, ch)| {
                        ch == '('
                            || (ch == '='
                                && next_non_space(text, position + 1) == Some('('))
                    });
                    if function {
                        push(&mut spans, index, end, SyntaxToken::Function);
                    } else if (index > 0 && text[..index].trim_end().ends_with('.'))
                        || next_non_space(text, end) == Some(':')
                    {
                        push(&mut spans, index, end, SyntaxToken::Property);
                    }
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn template(text: &str, start: usize, spans: &mut Vec<Span>) -> usize {
    let mut index = start + 1;
    let mut literal = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '\\' {
            index += ch.len_utf8();
            if let Some(next) = text[index..].chars().next() {
                index += next.len_utf8();
            }
            continue;
        }
        if ch == '`' {
            index += 1;
            push(spans, literal, index, SyntaxToken::String);
            return index;
        }
        if ch == '$' && text[index + 1..].starts_with('{') {
            push(spans, literal, index, SyntaxToken::String);
            let expression_start = index + 2;
            if let Some(close) = interpolation_end(text, expression_start) {
                push(spans, index, expression_start, SyntaxToken::String);
                for span in script(&text[expression_start..close]) {
                    push(
                        spans,
                        expression_start + span.start,
                        expression_start + span.end,
                        span.token,
                    );
                }
                push(spans, close, close + 1, SyntaxToken::String);
                index = close + 1;
                literal = index;
                continue;
            }
        }
        index += ch.len_utf8();
    }
    push(spans, literal, text.len(), SyntaxToken::String);
    text.len()
}

fn interpolation_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '\'' || ch == '"' {
            index = skip_quoted(text, index, ch, false);
        } else if ch == '`' {
            index = skip_quoted(text, index, '`', false);
        } else if text[index..].starts_with("//") {
            index = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
        } else if text[index..].starts_with("/*") {
            index = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
        } else if ch == '{' {
            depth += 1;
            index += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
            index += 1;
        } else {
            index += ch.len_utf8();
        }
    }
    None
}

fn regex_allowed(text: &str, index: usize) -> bool {
    let prefix = text[..index].trim_end();
    if prefix.is_empty() {
        return true;
    }
    if prefix
        .chars()
        .next_back()
        .is_some_and(|ch| "=([{,:;!&|?+-*%^~<>".contains(ch))
    {
        return true;
    }
    prefix
        .split_whitespace()
        .next_back()
        .is_some_and(|word| {
            matches!(
                word,
                "return" | "case" | "throw" | "typeof" | "delete" | "void" | "new" | "in" | "of" | "yield" | "await"
            )
        })
}

fn skip_regex(text: &str, start: usize) -> usize {
    let mut index = start + 1;
    let mut escaped = false;
    let mut class = false;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
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
            while index < text.len() {
                let flag = text[index..].chars().next().unwrap_or('\0');
                if flag.is_ascii_alphabetic() {
                    index += flag.len_utf8();
                } else {
                    break;
                }
            }
            return index;
        }
        index += ch.len_utf8();
    }
    text.len()
}

/// Ranges of JavaScript/TypeScript regex literals, using the same tokenizer
/// heuristics as the Markdown fence highlighter. Strings, templates and
/// comments are skipped so their slashes cannot start a regex.
pub(super) fn regex_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            index = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
        } else if text[index..].starts_with("/*") {
            index = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '/' && regex_allowed(text, index) {
                let line_end = text[index..]
                    .find('\n')
                    .map_or(text.len(), |offset| index + offset);
                let end = skip_regex(text, index).min(line_end);
                if end > index + 1 {
                    ranges.push(index..end);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '`' {
                index = template(text, index, &mut spans);
            } else if matches!(ch, '\'' | '"') {
                index = skip_quoted(text, index, ch, false);
            } else {
                index += ch.len_utf8();
            }
        }
    }
    ranges
}

fn css_at_rule(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "charset"
            | "container"
            | "content"
            | "debug"
            | "document"
            | "each"
            | "else"
            | "error"
            | "extend"
            | "font-face"
            | "for"
            | "forward"
            | "function"
            | "if"
            | "import"
            | "include"
            | "keyframes"
            | "layer"
            | "media"
            | "mixin"
            | "namespace"
            | "page"
            | "property"
            | "return"
            | "scope"
            | "starting-style"
            | "supports"
            | "use"
            | "view-transition"
            | "warn"
            | "while"
            | "at-root"
    )
}

fn css_property_name(segment: &str) -> bool {
    let segment = segment
        .rsplit(['\n', '\r'])
        .next()
        .unwrap_or(segment)
        .trim();
    !segment.is_empty()
        && !segment.contains([' ', '\t'])
        && !segment.starts_with(['.', '#', '&', ':', '[', '>', '+', '~', '*'])
        && segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '$' | '@'))
}

fn css_pseudo_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "active"
            | "after"
            | "any-link"
            | "autofill"
            | "backdrop"
            | "before"
            | "checked"
            | "default"
            | "defined"
            | "dir"
            | "disabled"
            | "empty"
            | "enabled"
            | "first-child"
            | "first-letter"
            | "first-line"
            | "first-of-type"
            | "focus"
            | "focus-visible"
            | "focus-within"
            | "fullscreen"
            | "has"
            | "host"
            | "host-context"
            | "hover"
            | "in-range"
            | "indeterminate"
            | "invalid"
            | "is"
            | "lang"
            | "last-child"
            | "last-of-type"
            | "link"
            | "marker"
            | "modal"
            | "not"
            | "nth-child"
            | "nth-last-child"
            | "nth-last-of-type"
            | "nth-of-type"
            | "only-child"
            | "only-of-type"
            | "open"
            | "optional"
            | "out-of-range"
            | "placeholder"
            | "placeholder-shown"
            | "popover-open"
            | "read-only"
            | "read-write"
            | "required"
            | "root"
            | "scope"
            | "selection"
            | "target"
            | "target-within"
            | "user-invalid"
            | "user-valid"
            | "valid"
            | "visited"
            | "where"
    )
}

/// A colon is a pseudo-selector separator when the following name is a known
/// pseudo class/element or is followed by selector punctuation/opening brace.
/// Declaration values instead end at `;`, `}` or continue with a value.
fn css_pseudo_selector(text: &str, colon: usize) -> bool {
    let mut index = colon + 1;
    while text[index..].starts_with(':') {
        index += 1;
    }
    let start = index;
    let end = identifier_end_hyphen(text, index);
    if end == start {
        return false;
    }
    if css_pseudo_name(&text[start..end]) {
        return true;
    }
    index = end;
    matches!(
        next_non_space(text, index),
        Some('{') | Some(',') | Some(':') | Some('>') | Some('+') | Some('~') | Some('.') | Some('#')
            | Some('[')
    )
}

fn css_hex_color(text: &str, start: usize) -> Option<usize> {
    let mut index = start + 1;
    while let Some(ch) = text[index..].chars().next() {
        if !ch.is_ascii_hexdigit() {
            break;
        }
        index += ch.len_utf8();
    }
    matches!(index - start - 1, 3 | 4 | 6 | 8).then_some(index)
}

fn css(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut index = 0;
    let mut brace = 0usize;
    let mut value = false;
    let mut segment_start = 0usize;
    while index < text.len() {
        if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if matches!(ch, '\'' | '"') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' {
                let end = identifier_end_hyphen(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '@' {
                let end = identifier_end_hyphen(text, index + 1);
                let token = if css_at_rule(&text[index + 1..end]) {
                    SyntaxToken::Keyword
                } else {
                    SyntaxToken::Property
                };
                push(&mut spans, index, end, token);
                index = end;
            } else if ch == '{' {
                brace += 1;
                value = false;
                segment_start = index + 1;
                index += 1;
            } else if ch == '}' {
                brace = brace.saturating_sub(1);
                value = false;
                segment_start = index + 1;
                index += 1;
            } else if ch == ';' {
                value = false;
                segment_start = index + 1;
                index += 1;
            } else if ch == ':' && !value && css_pseudo_selector(text, index) {
                let mut start = index + 1;
                while text[start..].starts_with(':') {
                    start += 1;
                }
                let end = identifier_end_hyphen(text, start);
                if end > start {
                    push(&mut spans, start, end, SyntaxToken::Tag);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == ':' {
                if !value {
                    let segment = text[segment_start..index].trim();
                    if brace > 0 && css_property_name(segment) {
                        value = true;
                    }
                }
                index += 1;
            } else if ch == '!' {
                let end = identifier_end(text, index + 1);
                if text[index + 1..end].eq_ignore_ascii_case("important") {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    index = end;
                } else {
                    index += 1;
                }
            } else if (ch == '.' || ch == '#') && !value {
                let end = identifier_end_hyphen(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Tag);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '#' && value {
                if let Some(end) = css_hex_color(text, index) {
                    push(&mut spans, index, end, SyntaxToken::Number);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '[' && !value {
                let start = index + 1;
                let end = identifier_end_hyphen(text, start);
                if end > start {
                    push(&mut spans, start, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '-' && text[index + 1..].starts_with('-') {
                let end = identifier_end_hyphen(text, index);
                if brace > 0 && !value && next_non_space(text, end) == Some(':') {
                    push(&mut spans, index, end, SyntaxToken::Property);
                }
                index = end;
            } else if ch.is_ascii_digit() {
                let end = skip_number(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end_hyphen(text, index);
                let function = next_non_space(text, end) == Some('(');
                if function {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if brace > 0
                    && !value
                    && let Some((colon, ':')) = next_non_space_index(text, end)
                    && !css_pseudo_selector(text, colon)
                {
                    push(&mut spans, index, end, SyntaxToken::Property);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn entity_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with('&') {
        return None;
    }
    let mut index = start + 1;
    let mut length = 0usize;
    while index < text.len() && length < 32 {
        let ch = text[index..].chars().next()?;
        if ch == ';' {
            return (length > 0).then_some(index + 1);
        }
        if !(ch.is_ascii_alphanumeric() || ch == '#') {
            return None;
        }
        index += ch.len_utf8();
        length += 1;
    }
    None
}

fn html_declaration_end(text: &str, start: usize) -> usize {
    let mut index = start;
    let mut quote: Option<char> = None;
    let mut subset = 0usize;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == '[' {
            subset += 1;
        } else if ch == ']' {
            subset = subset.saturating_sub(1);
        } else if ch == '>' && subset == 0 {
            return index + 1;
        }
        index += ch.len_utf8();
    }
    text.len()
}

fn html(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut index = 0;
    while index < text.len() {
        if text[index..].starts_with("<![CDATA[") {
            let end = text[index + 9..]
                .find("]]>")
                .map_or(text.len(), |offset| index + 9 + offset + 3);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
            continue;
        }
        if text[index..]
            .get(..9)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<!doctype"))
        {
            let end = html_declaration_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Keyword);
            index = end;
            continue;
        }
        if text[index..].starts_with("<?") {
            let end = text[index + 2..]
                .find("?>")
                .map_or_else(|| html_declaration_end(text, index), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Keyword);
            index = end;
            continue;
        }
        if text[index..].starts_with("<!--") {
            let end = text[index + 4..]
                .find("-->")
                .map_or(text.len(), |offset| index + 4 + offset + 3);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
            continue;
        }
        if let Some(end) = entity_end(text, index) {
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
            continue;
        }
        if !text[index..].starts_with('<') {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let mut cursor = index + 1;
        let closing = text[cursor..].starts_with('/');
        if closing || text[cursor..].starts_with('!') {
            cursor += 1;
        }
        let name_start = cursor;
        let name_end = identifier_end_markup(text, cursor);
        let name = text[name_start..name_end].to_ascii_lowercase();
        if name_end > name_start {
            push(&mut spans, name_start, name_end, SyntaxToken::Tag);
            cursor = name_end;
        }
        while cursor < text.len() {
            let ch = text[cursor..].chars().next().unwrap_or('\0');
            if ch == '>' {
                cursor += 1;
                break;
            }
            if ch.is_whitespace() || ch == '/' {
                cursor += ch.len_utf8();
                continue;
            }
            if matches!(ch, '\'' | '"') {
                let end = skip_quoted(text, cursor, ch, false);
                push(&mut spans, cursor, end, SyntaxToken::String);
                cursor = end;
                continue;
            }
            if let Some(end) = entity_end(text, cursor) {
                push(&mut spans, cursor, end, SyntaxToken::String);
                cursor = end;
                continue;
            }
            let attr_end = identifier_end_markup(text, cursor);
            if attr_end > cursor {
                push(&mut spans, cursor, attr_end, SyntaxToken::Attribute);
                cursor = attr_end;
                let mut probe = cursor;
                while probe < text.len()
                    && text[probe..].chars().next().is_some_and(char::is_whitespace)
                {
                    probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                }
                if text[probe..].starts_with('=') {
                    probe += 1;
                    while probe < text.len()
                        && text[probe..].chars().next().is_some_and(char::is_whitespace)
                    {
                        probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                    }
                    if text[probe..]
                        .chars()
                        .next()
                        .is_some_and(|ch| !matches!(ch, '"' | '\''))
                    {
                        let value_start = probe;
                        if text[probe..].starts_with('{')
                            && let Some(end) = svelte_expression_end(text, probe)
                        {
                            cursor = end;
                            continue;
                        }
                        while probe < text.len() {
                            let value = text[probe..].chars().next().unwrap_or('\0');
                            if value.is_whitespace() || value == '>' {
                                break;
                            }
                            probe += value.len_utf8();
                        }
                        push(&mut spans, value_start, probe, SyntaxToken::String);
                        cursor = probe;
                    }
                }
            } else {
                cursor += ch.len_utf8();
            }
        }
        let self_closing = text[..cursor].trim_end().ends_with("/>");
        if !closing && !self_closing && matches!(name.as_str(), "script" | "style") {
            let lower = text.to_ascii_lowercase();
            let marker = format!("</{name}");
            let content_end = lower[cursor..]
                .find(&marker)
                .map_or(text.len(), |offset| cursor + offset);
            let nested = if name == "script" {
                script(&text[cursor..content_end])
            } else {
                css(&text[cursor..content_end])
            };
            for span in nested {
                push(&mut spans, cursor + span.start, cursor + span.end, span.token);
            }
            cursor = content_end;
        }
        index = cursor;
    }
    spans
}

fn python_builtin(word: &str) -> bool {
    matches!(
        word,
        "abs" | "all" | "any" | "bool" | "bytes" | "bytearray" | "callable" | "classmethod"
            | "compile" | "delattr" | "dict" | "dir" | "enumerate" | "eval" | "exec" | "filter"
            | "float" | "format" | "frozenset" | "getattr" | "globals" | "hasattr" | "hash"
            | "help" | "id" | "input" | "int" | "isinstance" | "issubclass" | "iter" | "len"
            | "list" | "locals" | "map" | "max" | "min" | "next" | "object" | "open" | "ord"
            | "pow" | "print" | "property" | "range" | "repr" | "reversed" | "round" | "set"
            | "setattr" | "slice" | "sorted" | "staticmethod" | "str" | "sum" | "super"
            | "tuple" | "type" | "vars" | "zip" | "Exception" | "ValueError" | "TypeError"
            | "KeyError" | "IndexError" | "RuntimeError" | "StopIteration" | "ZeroDivisionError"
            | "FileNotFoundError" | "AssertionError" | "OSError" | "IOError" | "NotImplementedError"
    )
}

fn bash_builtin(word: &str) -> bool {
    matches!(
        word,
        "alias" | "bg" | "bind" | "builtin" | "caller" | "cd" | "command" | "compgen"
            | "complete" | "compopt" | "declare" | "dirs" | "disown" | "echo" | "enable"
            | "eval" | "exec" | "exit" | "fg" | "getopts" | "hash" | "help" | "history"
            | "jobs" | "kill" | "let" | "logout" | "mapfile" | "popd" | "printf" | "pushd"
            | "pwd" | "read" | "readarray" | "set" | "shopt" | "source" | "suspend" | "test"
            | "times" | "trap" | "true" | "false" | "type" | "typeset" | "ulimit" | "umask"
            | "unalias" | "unset" | "wait"
    )
}

fn python_prefixed_string(text: &str, start: usize) -> Option<(usize, char, bool, bool)> {
    let mut index = start;
    while index < text.len()
        && index - start < 2
        && text[index..]
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'r' | 'b' | 'u' | 'f'))
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    if index == start {
        return None;
    }
    let quote = text[index..].chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let triple = text[index..].starts_with(&quote.to_string().repeat(3));
    Some((
        index,
        quote,
        text[start..index]
            .chars()
            .any(|ch| ch.eq_ignore_ascii_case(&'f')),
        triple,
    ))
}

fn python_fstring(
    text: &str,
    start: usize,
    quote_index: usize,
    quote: char,
    triple: bool,
    spans: &mut Vec<Span>,
) -> usize {
    let mut index = quote_index + 1;
    let mut literal = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '\\' {
            index += ch.len_utf8();
            if let Some(next) = text[index..].chars().next() {
                index += next.len_utf8();
            }
            continue;
        }
        if triple && text[index..].starts_with(&quote.to_string().repeat(3)) {
            index += 3;
            push(spans, literal, index, SyntaxToken::String);
            return index;
        }
        if !triple && ch == quote {
            index += 1;
            push(spans, literal, index, SyntaxToken::String);
            return index;
        }
        if ch == '{' {
            if text[index + 1..].starts_with('{') {
                index += 2;
                continue;
            }
            push(spans, literal, index, SyntaxToken::String);
            let expression_start = index + 1;
            if let Some(close) = python_interpolation_end(text, expression_start) {
                push(spans, index, expression_start, SyntaxToken::String);
                for span in python(&text[expression_start..close]) {
                    push(
                        spans,
                        expression_start + span.start,
                        expression_start + span.end,
                        span.token,
                    );
                }
                push(spans, close, close + 1, SyntaxToken::String);
                index = close + 1;
                literal = index;
                continue;
            }
        }
        if ch == '}' && text[index + 1..].starts_with('}') {
            index += 2;
            continue;
        }
        index += ch.len_utf8();
    }
    push(spans, literal, text.len(), SyntaxToken::String);
    text.len()
}

fn python_interpolation_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if matches!(ch, '\'' | '"') {
            let triple = text[index..].starts_with(&format!("{ch}{ch}{ch}"));
            index = skip_quoted(text, index, ch, triple);
        } else if ch == '#' {
            index = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
        } else if ch == '{' {
            depth += 1;
            index += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
            index += 1;
        } else {
            index += ch.len_utf8();
        }
    }
    None
}

fn python(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in",
        "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
        "with", "yield", "True", "False", "None",
    ];
    let mut spans = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '#' {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if let Some((quote_index, quote, formatted, triple)) =
            python_prefixed_string(text, index)
        {
            if formatted {
                index = python_fstring(text, index, quote_index, quote, triple, &mut spans);
            } else {
                let end = skip_quoted(text, quote_index, quote, triple);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            }
        } else if ch == '@' {
            let end = identifier_end(text, index + 1);
            if end > index + 1 {
                push(&mut spans, index + 1, end, SyntaxToken::Function);
                index = end;
            } else {
                index += 1;
            }
        } else if matches!(ch, '\'' | '"') {
            let triple = text[index..].starts_with(&format!("{ch}{ch}{ch}"));
            let end = skip_quoted(text, index, ch, triple);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = skip_number(text, index);
            push(&mut spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end(text, index);
            let word = &text[index..end];
            if keyword(word, KEYWORDS, false) {
                push(&mut spans, index, end, SyntaxToken::Keyword);
            } else if python_builtin(word) {
                push(&mut spans, index, end, SyntaxToken::Function);
            } else if index > 0 {
                let previous = text[..index].trim_end();
                if previous.ends_with("def") || previous.ends_with("class") {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
            }
            index = end;
        } else {
            index += ch.len_utf8();
        }
    }
    spans
}

fn bash_heredoc_delimiter(text: &str, start: usize) -> Option<(usize, usize, String)> {
    // `<<EOF`, `<<-EOF`, `<<"EOF"` and `<<'EOF'` all begin a heredoc body.
    let mut index = start;
    if text[index..].starts_with("<<") {
        index += 2;
    } else {
        return None;
    }
    if text[index..].starts_with('-') {
        index += 1;
    }
    while text[index..].starts_with([' ', '\t']) {
        index += 1;
    }
    let quote = text[index..].chars().next().filter(|ch| matches!(ch, '\'' | '"'));
    let (token_start, name_start, name_end, token_end) = if let Some(quote) = quote {
        let end = skip_quoted(text, index, quote, false);
        (index, index + 1, end.saturating_sub(1), end)
    } else {
        let end = identifier_end_hyphen(text, index);
        (index, index, end, end)
    };
    if name_end <= name_start {
        return None;
    }
    Some((token_start, token_end, text[name_start..name_end].to_string()))
}

fn bash_heredoc_body(text: &str, body_start: usize, delimiter: &str, dash: bool) -> Option<usize> {
    let mut line_start = body_start;
    while line_start <= text.len() {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |offset| line_start + offset);
        let line = &text[line_start..line_end];
        let candidate = if dash {
            line.trim_start_matches('\t')
        } else {
            line
        };
        let candidate = candidate.strip_suffix('\r').unwrap_or(candidate);
        if candidate == delimiter {
            return Some(line_end);
        }
        if line_end >= text.len() {
            return Some(text.len());
        }
        line_start = line_end + 1;
    }
    None
}

fn bash(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in",
        "select", "then", "until", "while", "export", "local", "readonly", "return",
    ];
    let mut spans = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '#' {
            let comment = index == 0
                || text[..index]
                    .chars()
                    .next_back()
                    .is_some_and(|previous| {
                        previous.is_whitespace() || matches!(previous, ';' | '|' | '&' | '(' | ')' | '{' | '}' | '<' | '>')
                    });
            if comment {
                let end = text[index..]
                    .find('\n')
                    .map_or(text.len(), |offset| index + offset);
                push(&mut spans, index, end, SyntaxToken::Comment);
                index = end;
            } else {
                index += 1;
            }
        } else if text[index..].starts_with("<<") && !text[index..].starts_with("<<<") {
            if let Some((delimiter_start, delimiter_end, delimiter)) =
                bash_heredoc_delimiter(text, index)
            {
                let header_end = text[index..]
                    .find('\n')
                    .map_or(text.len(), |offset| index + offset + 1);
                let dash = text[index..delimiter_start].contains('-');
                let body_start = header_end.min(text.len());
                if let Some(body_end) = bash_heredoc_body(text, body_start, &delimiter, dash) {
                    push(&mut spans, index, delimiter_start, SyntaxToken::Keyword);
                    push(
                        &mut spans,
                        delimiter_start,
                        delimiter_end.min(header_end),
                        SyntaxToken::String,
                    );
                    push(&mut spans, body_start, body_end, SyntaxToken::String);
                    index = body_end;
                } else {
                    index += 2;
                }
            } else {
                index += 2;
            }
        } else if matches!(ch, '\'' | '"') {
            let end = skip_quoted(text, index, ch, false);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch == '$'
            && text[index + 1..]
                .chars()
                .next()
                .is_some_and(|next| matches!(next, '\'' | '"'))
        {
            let quote = text[index + 1..].chars().next().unwrap_or('\'');
            let end = skip_quoted(text, index + 1, quote, false);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch == '$' {
            let start = index;
            let next = text[index + 1..].chars().next();
            let end = if next == Some('{') {
                text[index + 1..]
                    .find('}')
                    .map_or(text.len(), |offset| index + 1 + offset + 1)
            } else if next.is_some_and(|ch| ch.is_ascii_digit() || "#?@*$!-".contains(ch)) {
                index + 1 + next.map_or(0, char::len_utf8)
            } else {
                identifier_end(text, index + 1).max(index + 2).min(text.len())
            };
            push(&mut spans, start, end, SyntaxToken::Property);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = skip_number(text, index);
            push(&mut spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end(text, index);
            if keyword(&text[index..end], KEYWORDS, false) {
                push(&mut spans, index, end, SyntaxToken::Keyword);
            } else if bash_builtin(&text[index..end]) {
                push(&mut spans, index, end, SyntaxToken::Function);
            } else if text[end..].starts_with('=') && !text[end..].starts_with("==") {
                push(&mut spans, index, end, SyntaxToken::Property);
            } else if text[end..].trim_start().starts_with("()") {
                push(&mut spans, index, end, SyntaxToken::Function);
            }
            index = end;
        } else {
            index += ch.len_utf8();
        }
    }
    spans
}

/// A YAML block-scalar header: `|`, `>` plus optional indentation/chomping
/// indicators (`|2-`, `>+`, ...). Content under it is literal string text.
fn yaml_block_header(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    matches!(first, '|' | '>') && chars.all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-'))
}

fn yaml_literal(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null"
    )
}

fn yaml(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    // Indentation of the key that opened a block scalar. Every following line
    // indented deeper (including blank lines) is scalar content, so keys,
    // numbers, quotes and `#` inside it must not be tokenized.
    let mut scalar_indent: Option<usize> = None;
    for (line_start, line) in text.split('\n').scan(0usize, |start, line| {
        let current = *start;
        *start += line.len() + 1;
        Some((current, line))
    }) {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(base) = scalar_indent {
            if trimmed.is_empty() {
                continue;
            }
            if indent > base {
                push(&mut spans, line_start + indent, line_start + line.len(), SyntaxToken::String);
                continue;
            }
            scalar_indent = None;
        }
        if trimmed.starts_with('#') {
            push(&mut spans, line_start + indent, line_start + line.len(), SyntaxToken::Comment);
            continue;
        }
        if let Some(word) = trimmed.split_whitespace().next()
            && word.starts_with('%')
        {
            push(
                &mut spans,
                line_start + indent,
                line_start + indent + word.len(),
                SyntaxToken::Keyword,
            );
            continue;
        }
        if trimmed == "---" || trimmed == "..." {
            push(&mut spans, line_start + indent, line_start + line.len(), SyntaxToken::Keyword);
            continue;
        }
        let body = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let offset = indent + (trimmed.len() - body.len());
        if let Some(colon) = body.find(':') {
            let key = body[..colon].trim();
            if !key.is_empty() && !key.contains(' ') {
                let key_start = line_start + offset;
                push(&mut spans, key_start, key_start + key.len(), SyntaxToken::Property);
            }
            let raw_value = &body[colon + 1..];
            let value = raw_value.trim();
            if yaml_block_header(value) {
                let value_start = line_start + offset + colon + 1 + (raw_value.len() - raw_value.trim_start().len());
                push(&mut spans, value_start, value_start + value.len(), SyntaxToken::String);
                scalar_indent = Some(indent);
                continue;
            }
        }
        let mut index = 0;
        while index < line.len() {
            let ch = line[index..].chars().next().unwrap_or('\0');
            if line[index..].starts_with("<<") {
                push(&mut spans, line_start + index, line_start + index + 2, SyntaxToken::Property);
                index += 2;
            } else if ch == '&' || ch == '*' {
                let end = identifier_end_hyphen(line, index + 1);
                if end > index + 1 {
                    push(&mut spans, line_start + index, line_start + end, SyntaxToken::Property);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '!' {
                let end = if line[index..].starts_with("!<") {
                    line[index + 2..]
                        .find('>')
                        .map_or(index + 1, |offset| index + 2 + offset + 1)
                } else {
                    let mut cursor = index + 1;
                    if line[cursor..].starts_with('!') {
                        cursor += 1;
                    }
                    identifier_end_hyphen(line, cursor)
                };
                if end > index + 1 {
                    push(&mut spans, line_start + index, line_start + end, SyntaxToken::Keyword);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '~'
                && next_non_space(line, index + 1).is_none_or(|next| next == '#')
            {
                push(&mut spans, line_start + index, line_start + index + 1, SyntaxToken::Keyword);
                index += 1;
            } else if matches!(ch, '\'' | '"') {
                let end = skip_quoted(line, index, ch, false);
                let previous = line[..index].trim_end().chars().next_back();
                let key_context = line[..index].trim().is_empty()
                    || line[..index].trim_start().starts_with("- ")
                    || line[..index].trim_start().starts_with("{")
                    || matches!(previous, Some('{' | ','));
                let token = if key_context && next_non_space(line, end) == Some(':') {
                    SyntaxToken::Property
                } else {
                    SyntaxToken::String
                };
                push(&mut spans, line_start + index, line_start + end, token);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end_hyphen(line, index);
                let next = next_non_space(line, end);
                let previous = line[..index].trim_end().chars().next_back();
                if next == Some(':') && matches!(previous, Some('{' | ',')) {
                    push(&mut spans, line_start + index, line_start + end, SyntaxToken::Property);
                } else if yaml_literal(&line[index..end])
                    && !matches!(next, Some(':'))
                {
                    push(&mut spans, line_start + index, line_start + end, SyntaxToken::Keyword);
                }
                index = end;
            } else if ch.is_ascii_digit() {
                let end = skip_number(line, index);
                push(&mut spans, line_start + index, line_start + end, SyntaxToken::Number);
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

/// SQL delimited identifiers double their closing delimiter (`""`, ````,
/// `]]`). Single-quoted SQL strings follow the same doubling rule.
fn skip_sql_quoted(text: &str, start: usize, opening: char, closing: char) -> usize {
    let mut index = start + opening.len_utf8();
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == closing {
            let next_index = index + ch.len_utf8();
            if text[next_index..].starts_with(closing) {
                index = next_index + closing.len_utf8();
                continue;
            }
            return next_index;
        }
        index += ch.len_utf8();
    }
    text.len()
}

fn sql_dollar_quote(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with('$') {
        return None;
    }
    let mut end = start + 1;
    while let Some(ch) = text[end..].chars().next() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    if !text[end..].starts_with('$') {
        return None;
    }
    let tag = &text[start..=end];
    let body_start = end + 1;
    text[body_start..]
        .find(tag)
        .map(|offset| body_start + offset + tag.len())
}

fn sql(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "add", "all", "alter", "and", "as", "asc", "begin", "between", "by", "case", "commit",
        "create", "delete", "desc", "distinct", "drop", "else", "end", "exists", "from",
        "group", "having", "in", "index", "insert", "into", "is", "join", "left", "like",
        "limit", "not", "null", "on", "or", "order", "outer", "primary", "references",
        "right", "rollback", "select", "set", "table", "then", "union", "unique", "update",
        "values", "when", "where", "bigint", "blob", "boolean", "bytea", "char", "date",
        "decimal", "double", "int", "integer", "json", "jsonb", "numeric", "precision",
        "real", "serial", "smallint", "text", "time", "timestamp", "uuid", "varchar",
    ];
    let mut spans = Vec::new();
    let mut index = 0;
    while index < text.len() {
        if text[index..].starts_with("--") || text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '\'' {
                let end = skip_sql_quoted(text, index, '\'', '\'');
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '`') {
                let end = skip_sql_quoted(text, index, ch, ch);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch == '[' {
                let end = skip_sql_quoted(text, index, '[', ']');
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch == '$' {
                if let Some(end) = sql_dollar_quote(text, index) {
                    push(&mut spans, index, end, SyntaxToken::String);
                    index = end;
                } else {
                    let end = index
                        + 1
                        + text[index + 1..]
                            .chars()
                            .take_while(char::is_ascii_digit)
                            .map(char::len_utf8)
                            .sum::<usize>();
                    if end > index + 1 {
                        push(&mut spans, index, end, SyntaxToken::Property);
                    }
                    index = end;
                }
            } else if (ch == ':' || ch == '@') && !text[index + 1..].starts_with('=') {
                if ch == ':' && text[index + 1..].starts_with(':') {
                    push(&mut spans, index, index + 2, SyntaxToken::Keyword);
                    index += 2;
                } else {
                    let end = identifier_end(text, index + 1);
                    if end > index + 1 {
                        push(&mut spans, index, end, SyntaxToken::Property);
                        index = end;
                    } else {
                        index += 1;
                    }
                }
            } else if ch == '?' {
                push(&mut spans, index, index + 1, SyntaxToken::Property);
                index += 1;
            } else if matches!(ch, '=' | '<' | '>' | '!' | '|' | '+' | '-' | '*' | '/' | '%') {
                let length = if text[index..].starts_with("->>") {
                    3
                } else if ["<>", "<=", ">=", "!=", "||", "->", "=>"]
                    .iter()
                    .any(|operator| text[index..].starts_with(operator))
                {
                    2
                } else {
                    1
                };
                push(&mut spans, index, index + length, SyntaxToken::Keyword);
                index += length;
            } else if ch.is_ascii_digit() {
                let end = skip_number(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                if keyword(&text[index..end], KEYWORDS, true) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn php_heredoc_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with("<<<") {
        return None;
    }
    let mut index = start + 3;
    while text[index..]
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, ' ' | '\t'))
    {
        index += 1;
    }
    let quote = text[index..]
        .chars()
        .next()
        .filter(|ch| matches!(ch, '\'' | '"'));
    let quoted = quote.is_some();
    if quoted {
        index += 1;
    }
    let name_end = identifier_end(text, index);
    if name_end == index {
        return None;
    }
    let terminator = &text[index..name_end];
    let mut after = name_end;
    if let Some(quote) = quote {
        if !text[after..].starts_with(quote) {
            return None;
        }
        after += 1;
    }
    let marker_end = text[after..].find('\n').map(|offset| after + offset)?;
    if !text[after..marker_end].trim().is_empty() {
        return None;
    }
    let mut cursor = marker_end + 1;
    while cursor <= text.len() {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset);
        let line = text[cursor..line_end].trim_start();
        let end_marker = line == terminator
            || line
                .strip_prefix(terminator)
                .is_some_and(|rest| rest.trim() == ";");
        if end_marker {
            let offset = text[cursor..line_end].find(terminator)?;
            return Some(cursor + offset + terminator.len());
        }
        if line_end == text.len() {
            break;
        }
        cursor = line_end + 1;
    }
    Some(text.len())
}

fn php(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "and", "as", "break", "case", "catch", "class", "clone", "const",
        "continue", "declare", "default", "do", "echo", "else", "elseif", "empty",
        "enddeclare", "endfor", "endforeach", "endif", "endswitch", "endwhile", "enum",
        "extends", "final", "finally", "fn", "for", "foreach", "from", "function", "global",
        "goto", "if", "implements", "include", "include_once", "instanceof", "insteadof",
        "interface", "isset", "list", "match", "namespace", "new", "or", "print", "private",
        "protected", "public", "readonly", "require", "require_once", "return", "static",
        "switch", "throw", "trait", "try", "unset", "use", "var", "while", "xor", "yield",
    ];
    const TYPES: &[&str] = &[
        "array", "bool", "callable", "float", "int", "iterable", "mixed", "never", "object",
        "string", "void", "parent", "self",
    ];
    const TYPE_DECLS: &[&str] = &["class", "enum", "interface", "trait"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("<?") {
            let end = if text[index..].starts_with("<?php") {
                index + 5
            } else if text[index..].starts_with("<?=") {
                index + 3
            } else {
                index + 2
            };
            push(&mut spans, index, end, SyntaxToken::Keyword);
            index = end;
        } else if text[index..].starts_with("?>") {
            push(&mut spans, index, index + 2, SyntaxToken::Keyword);
            index += 2;
        } else if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') && !text[index..].starts_with("#[") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("#[") {
                let end = text[index..]
                    .find(']')
                    .map_or(text.len(), |offset| index + offset + 1);
                push(&mut spans, index, end, SyntaxToken::Attribute);
                index = end;
            } else if text[index..].starts_with("<<<")
                && let Some(end) = php_heredoc_end(text, index)
            {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                } else {
                    index += 1;
                }
            } else if matches!(ch, '"' | '\'' | '`') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "function" || TYPE_DECLS.contains(&word))
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && (is_ident_start(ch) || (word == "function" && ch == '&'))
                    {
                        let mut name_start = position;
                        if word == "function" && ch == '&' {
                            name_start += 1;
                            while text[name_start..]
                                .chars()
                                .next()
                                .is_some_and(char::is_whitespace)
                            {
                                name_start += text[name_start..]
                                    .chars()
                                    .next()
                                    .map_or(1, char::len_utf8);
                            }
                        }
                        let name_end = identifier_end(text, name_start);
                        if name_end > name_start {
                            push(
                                &mut spans,
                                name_start,
                                name_end,
                                if word == "function" {
                                    SyntaxToken::Function
                                } else {
                                    SyntaxToken::Type
                                },
                            );
                            index = name_end;
                            continue;
                        }
                    }
                } else if TYPES.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(')
                    || text[end..].starts_with("::")
                {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn ruby_name_end(text: &str, start: usize) -> usize {
    let end = identifier_end(text, start);
    if text[end..]
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '?' | '!'))
    {
        end + 1
    } else {
        end
    }
}

fn ruby_heredoc_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with("<<") {
        return None;
    }
    let mut index = start + 2;
    let modifier = text[index..]
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '-' | '~'));
    if modifier {
        index += 1;
    }
    let quote = text[index..]
        .chars()
        .next()
        .filter(|ch| matches!(ch, '\'' | '"'));
    let quoted = quote.is_some();
    if !modifier && !quoted {
        return None;
    }
    if quoted {
        index += 1;
    }
    let name_end = identifier_end(text, index);
    if name_end == index {
        return None;
    }
    let terminator = &text[index..name_end];
    let mut after = name_end;
    if let Some(quote) = quote {
        if !text[after..].starts_with(quote) {
            return None;
        }
        after += 1;
    }
    let marker_end = text[after..].find('\n').map(|offset| after + offset)?;
    if !text[after..marker_end].trim().is_empty() {
        return None;
    }
    let mut cursor = marker_end + 1;
    while cursor <= text.len() {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset);
        if text[cursor..line_end].trim() == terminator {
            return Some(if line_end < text.len() {
                line_end + 1
            } else {
                line_end
            });
        }
        if line_end == text.len() {
            break;
        }
        cursor = line_end + 1;
    }
    Some(text.len())
}

fn ruby_percent_literal_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with('%') {
        return None;
    }
    let mut index = start + 1;
    if let Some(kind) = text[index..].chars().next()
        && "qQwWiIsxr".contains(kind)
    {
        index += kind.len_utf8();
    }
    let open = text[index..].chars().next()?;
    if open.is_alphanumeric() || open.is_whitespace() {
        return None;
    }
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };
    let paired = open != close;
    let mut depth = 0usize;
    let mut cursor = index + open.len_utf8();
    while cursor < text.len() {
        let ch = text[cursor..].chars().next()?;
        if ch == '\\' {
            cursor += ch.len_utf8();
            if let Some(escaped) = text[cursor..].chars().next() {
                cursor += escaped.len_utf8();
            }
            continue;
        }
        if paired && ch == open {
            depth += 1;
        } else if ch == close {
            if depth == 0 {
                return Some(cursor + ch.len_utf8());
            }
            depth -= 1;
        }
        cursor += ch.len_utf8();
    }
    Some(text.len())
}

fn ruby_block_comment_end(text: &str, start: usize) -> Option<usize> {
    if !(start == 0 || text[..start].ends_with('\n')) || !text[start..].starts_with("=begin") {
        return None;
    }
    if !text[start + 6..]
        .chars()
        .next()
        .is_none_or(|ch| ch.is_whitespace())
    {
        return None;
    }
    let Some(offset) = text[start..].find("\n=end") else {
        return Some(text.len());
    };
    let after = start + offset + 5;
    Some(
        text[after..]
            .find('\n')
            .map_or(text.len(), |line| after + line + 1),
    )
}

fn ruby(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "alias", "and", "begin", "break", "case", "class", "def", "defined", "do", "else",
        "elsif", "end", "ensure", "extend", "for", "if", "in", "include", "module",
        "module_function", "next", "not", "or", "prepend", "private", "protected",
        "public", "redo", "rescue", "retry", "return", "self", "super", "then", "undef",
        "unless", "until", "when", "while", "yield", "attr_accessor", "attr_reader",
        "attr_writer",
    ];
    const BUILTINS: &[&str] = &[
        "abort", "exit", "format", "gets", "lambda", "loop", "open", "p", "pp", "print",
        "printf", "proc", "puts", "raise", "require", "require_relative", "sleep", "sprintf",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if let Some(end) = ruby_block_comment_end(text, index) {
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("<<")
                && let Some(end) = ruby_heredoc_end(text, index)
            {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '%'
                && let Some(end) = ruby_percent_literal_end(text, index)
            {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' || ch == '$' {
                let mut name_start = index + 1;
                if ch == '@' && text[name_start..].starts_with('@') {
                    name_start += 1;
                }
                let end = ruby_name_end(text, name_start);
                if end > name_start {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == ':'
                && !text[index..].starts_with("::")
                && text[index + 1..]
                    .chars()
                    .next()
                    .is_some_and(is_ident_start)
            {
                let end = ruby_name_end(text, index + 1);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if matches!(ch, '"' | '\'' | '`') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                if text[index..].starts_with("defined")
                    && text[index + 7..].starts_with('?')
                {
                    push(&mut spans, index, index + 8, SyntaxToken::Keyword);
                    index += 8;
                    continue;
                }
                let end = ruby_name_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(word, "class" | "module" | "def")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let mut name_start = position;
                        let mut name_end = ruby_name_end(text, name_start);
                        if word == "def"
                            && &text[name_start..name_end] == "self"
                            && text[name_end..].starts_with('.')
                        {
                            name_start = name_end + 1;
                            name_end = ruby_name_end(text, name_start);
                        }
                        if word == "def" && text[name_end..].starts_with('=') {
                            name_end += 1;
                        }
                        push(
                            &mut spans,
                            name_start,
                            name_end,
                            if word == "def" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if BUILTINS.contains(&word)
                    || next_non_space(text, end) == Some('(')
                {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn c_number_end(text: &str, start: usize, cpp: bool) -> usize {
    let bytes = text.as_bytes();
    let separator = |byte: u8| byte == b'_' || (cpp && byte == b'\'');
    let mut index;
    if text[start..].starts_with("0x") || text[start..].starts_with("0X") {
        index = start + 2;
        while index < text.len()
            && (bytes[index].is_ascii_hexdigit()
                || separator(bytes[index])
                || bytes[index] == b'.')
        {
            index += 1;
        }
        if index < text.len() && matches!(bytes[index], b'p' | b'P') {
            let mut exponent = index + 1;
            if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
                exponent += 1;
            }
            let digits = exponent;
            while exponent < text.len()
                && (bytes[exponent].is_ascii_digit() || separator(bytes[exponent]))
            {
                exponent += 1;
            }
            if exponent > digits {
                index = exponent;
            }
        }
    } else if text[start..].starts_with("0b") || text[start..].starts_with("0B") {
        index = start + 2;
        while index < text.len()
            && (matches!(bytes[index], b'0' | b'1') || separator(bytes[index]))
        {
            index += 1;
        }
    } else if text[start..].starts_with("0o") || text[start..].starts_with("0O") {
        index = start + 2;
        while index < text.len()
            && (matches!(bytes[index], b'0'..=b'7') || separator(bytes[index]))
        {
            index += 1;
        }
    } else {
        index = start;
        while index < text.len()
            && (bytes[index].is_ascii_digit() || separator(bytes[index]))
        {
            index += 1;
        }
        if index + 1 < text.len()
            && bytes[index] == b'.'
            && (bytes[index + 1].is_ascii_digit() || separator(bytes[index + 1]))
        {
            index += 1;
            while index < text.len()
                && (bytes[index].is_ascii_digit() || separator(bytes[index]))
            {
                index += 1;
            }
        }
        if index < text.len() && matches!(bytes[index], b'e' | b'E') {
            let mut exponent = index + 1;
            if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
                exponent += 1;
            }
            let digits = exponent;
            while exponent < text.len()
                && (bytes[exponent].is_ascii_digit() || separator(bytes[exponent]))
            {
                exponent += 1;
            }
            if exponent > digits {
                index = exponent;
            }
        }
    }
    while index < text.len()
        && (bytes[index].is_ascii_alphanumeric() || separator(bytes[index]))
    {
        index += 1;
    }
    index
}

fn cpp_raw_string_end(text: &str, start: usize, prefix: usize) -> usize {
    let mut index = start + prefix;
    if !text[index..].starts_with('"') {
        return text.len();
    }
    index += 1;
    let Some(open) = text[index..].find('(') else {
        return text.len();
    };
    let delimiter = &text[index..index + open];
    let close = format!("){delimiter}\"");
    text[index + open..]
        .find(&close)
        .map_or(text.len(), |offset| index + open + offset + close.len())
}

fn c_family(text: &str, cpp: bool) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "auto", "break", "case", "const", "continue", "default", "do", "else", "enum",
        "extern", "for", "goto", "if", "inline", "register", "restrict", "return",
        "sizeof", "static", "struct", "switch", "typedef", "union", "volatile", "while",
        "_Atomic", "_Bool", "_Complex", "_Generic", "_Imaginary", "_Noreturn",
        "_Static_assert", "_Thread_local", "alignas", "alignof", "constexpr",
        "static_assert", "thread_local", "typeof", "typeof_unqual",
    ];
    const CPP_KEYWORDS: &[&str] = &[
        "and", "and_eq", "asm", "bitand", "bitor", "catch", "class", "compl", "concept",
        "consteval", "constinit", "const_cast", "co_await", "co_return", "co_yield",
        "decltype", "delete", "dynamic_cast", "explicit", "export", "friend", "mutable",
        "namespace", "new", "noexcept", "not", "not_eq", "operator", "or", "or_eq",
        "private", "protected", "public", "reinterpret_cast", "requires", "static_cast",
        "template", "this", "throw", "try", "typeid", "typename", "using", "virtual",
        "xor", "xor_eq",
    ];
    const TYPES: &[&str] = &[
        "bool", "char", "char8_t", "char16_t", "char32_t", "double", "float", "int",
        "long", "short", "signed", "unsigned", "void", "wchar_t", "size_t", "ssize_t",
        "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
        "uint64_t", "intptr_t", "uintptr_t", "ptrdiff_t", "max_align_t",
    ];
    const TYPE_DECLS: &[&str] = &["class", "enum", "struct", "union"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') {
            let line_end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            let mut word_start = index + 1;
            while word_start < line_end
                && text[word_start..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                word_start += text[word_start..]
                    .chars()
                    .next()
                    .map_or(1, char::len_utf8);
            }
            let word_end = identifier_end(text, word_start).min(line_end);
            if word_end > word_start {
                push(&mut spans, index, word_end, SyntaxToken::Keyword);
                let directive = &text[word_start..word_end];
                if matches!(directive, "include" | "import")
                    && let Some((position, ch)) = next_non_space_index(text, word_end)
                    && position < line_end
                {
                    if ch == '<' {
                        if let Some(offset) = text[position..line_end].find('>') {
                            let end = position + offset + 1;
                            push(&mut spans, position, end, SyntaxToken::String);
                            index = end;
                            continue;
                        }
                    } else if ch == '"' {
                        let end = skip_quoted(text, position, '"', false);
                        push(&mut spans, position, end, SyntaxToken::String);
                        index = end;
                        continue;
                    }
                }
                index = word_end;
            } else {
                index += 1;
            }
        } else if cpp && text[index..].starts_with("[[") {
            let end = text[index..]
                .find("]]")
                .map_or(text.len(), |offset| index + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Attribute);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            let raw = if cpp {
                ["u8R", "uR", "UR", "LR", "R"].iter().find(|prefix| {
                    text[index..].starts_with(**prefix)
                        && text[index + prefix.len()..].starts_with('"')
                })
            } else {
                None
            };
            let prefixed = ["u8", "u", "U", "L"].iter().find(|prefix| {
                text[index..].starts_with(**prefix)
                    && text[index + prefix.len()..]
                        .chars()
                        .next()
                        .is_some_and(|next| matches!(next, '"' | '\''))
            });
            if let Some(prefix) = raw {
                let end = cpp_raw_string_end(text, index, prefix.len());
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if let Some(prefix) = prefixed {
                let quote = index + prefix.len();
                let end = skip_quoted(
                    text,
                    quote,
                    text[quote..].chars().next().unwrap_or('"'),
                    false,
                );
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, cpp);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nullptr") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) || (cpp && CPP_KEYWORDS.contains(&word)) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if TYPE_DECLS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn csharp_verbatim_string_end(text: &str, start: usize) -> usize {
    let mut index = start + 1;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '"' {
            if text[index + 1..].starts_with('"') {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += ch.len_utf8();
    }
    text.len()
}

fn csharp_string_end(text: &str, index: usize) -> Option<usize> {
    let rest = &text[index..];
    if rest.starts_with("$@\"") || rest.starts_with("@$\"") {
        return Some(csharp_verbatim_string_end(text, index + 2));
    }
    if rest.starts_with("@\"") {
        return Some(csharp_verbatim_string_end(text, index + 1));
    }
    if rest.starts_with("$\"\"\"") {
        return Some(skip_quoted(text, index + 1, '"', true));
    }
    if rest.starts_with("$\"") {
        return Some(skip_quoted(text, index + 1, '"', false));
    }
    if rest.starts_with("\"\"\"") {
        return Some(skip_quoted(text, index, '"', true));
    }
    None
}

fn csharp(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "add", "alias", "and", "as", "ascending", "async", "await", "base",
        "break", "by", "case", "catch", "checked", "class", "const", "continue",
        "default", "delegate", "descending", "do", "dynamic", "else", "enum", "equals",
        "event", "explicit", "extern", "file", "finally", "fixed", "for", "foreach",
        "from", "get", "global", "goto", "group", "if", "implicit", "in", "init",
        "interface", "internal", "into", "is", "join", "let", "lock", "managed",
        "nameof", "namespace", "new", "not", "notnull", "on", "operator", "or",
        "orderby", "out", "override", "params", "partial", "private", "protected",
        "public", "readonly", "record", "ref", "remove", "required", "return",
        "scoped", "sealed", "select", "set", "sizeof", "stackalloc", "static",
        "struct", "switch", "this", "throw", "try", "typeof", "unchecked", "unmanaged",
        "unsafe", "using", "value", "var", "virtual", "volatile", "when", "where",
        "while", "with", "yield",
    ];
    const TYPES: &[&str] = &[
        "bool", "byte", "char", "decimal", "double", "float", "int", "long", "object",
        "sbyte", "short", "string", "uint", "ulong", "ushort", "void",
    ];
    const TYPE_DECLS: &[&str] = &[
        "class", "delegate", "enum", "interface", "namespace", "record", "struct",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        let line_start = text[..index].rfind('\n').map_or(0, |offset| offset + 1);
        let at_line_start = text[line_start..index].trim().is_empty();
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '#' {
                let end = identifier_end(text, index + 1).min(
                    text[index..]
                        .find('\n')
                        .map_or(text.len(), |offset| index + offset),
                );
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    index = end;
                } else {
                    index += 1;
                }
            } else if at_line_start && ch == '[' {
                let end = text[index..]
                    .find(']')
                    .map_or(text.len(), |offset| index + offset + 1);
                let name_start = index + 1;
                let name = text[name_start..end]
                    .trim_start()
                    .chars()
                    .next()
                    .unwrap_or('\0');
                if name.is_uppercase() {
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if let Some(end) = csharp_string_end(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if TYPE_DECLS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn toml_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '-')
}

fn toml_word_end(text: &str, start: usize) -> usize {
    let mut index = start;
    while let Some(ch) = text[index..].chars().next() {
        if !toml_word_char(ch) {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn lua_long_bracket_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    if !rest.starts_with('[') {
        return None;
    }
    let mut offset = 1usize;
    while rest[offset..].starts_with('=') {
        offset += 1;
    }
    if !rest[offset..].starts_with('[') {
        return None;
    }
    let close = format!("]{}]", "=".repeat(offset - 1));
    let content_start = start + offset + 1;
    Some(
        text[content_start..]
            .find(&close)
            .map_or(text.len(), |index| content_start + index + close.len()),
    )
}

fn lua_function_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while let Some(separator) = text[end..].chars().next() {
        if !matches!(separator, '.' | ':') {
            break;
        }
        let name_end = identifier_end(text, end + 1);
        if name_end == end + 1 {
            break;
        }
        end = name_end;
    }
    end
}

fn lua(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
        "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
        "until", "while",
    ];
    const BUILTINS: &[&str] = &[
        "assert", "collectgarbage", "dofile", "error", "getfenv", "getmetatable", "ipairs",
        "load", "loadfile", "loadstring", "module", "next", "pairs", "pcall", "print",
        "rawequal", "rawget", "rawlen", "rawset", "require", "select", "setfenv",
        "setmetatable", "tonumber", "tostring", "type", "unpack", "xpcall",
    ];
    const LIBRARIES: &[&str] = &[
        "coroutine", "debug", "io", "math", "os", "package", "string", "table", "utf8",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if index == 0 && text.starts_with("#!") {
            let end = text.find('\n').unwrap_or(text.len());
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("--") {
            if let Some(end) = lua_long_bracket_end(text, index + 2) {
                push(&mut spans, index, end, SyntaxToken::Comment);
                index = end;
            } else {
                let end = text[index..]
                    .find('\n')
                    .map_or(text.len(), |offset| index + offset);
                push(&mut spans, index, end, SyntaxToken::Comment);
                index = end;
            }
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if let Some(end) = lua_long_bracket_end(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'' | '`') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if word == "function"
                        && let Some((position, next)) = next_non_space_index(text, end)
                        && is_ident_start(next)
                    {
                        let name_end = lua_function_chain_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                } else if BUILTINS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if LIBRARIES.contains(&word)
                    && let Some((position, '.')) = next_non_space_index(text, end)
                {
                    let name_end = identifier_end(text, position + 1);
                    if name_end > position + 1 {
                        push(&mut spans, index, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                } else if matches!(next_non_space(text, end), Some('(' | '{' | '"' | '\'')) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn r_identifier_end(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if !(ch.is_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn r_raw_string_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    let mut chars = rest.chars();
    if !matches!(chars.next()?, 'r' | 'R') {
        return None;
    }
    let quote = chars.next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }
    let mut offset = 2usize;
    while rest[offset..].starts_with('-') {
        offset += 1;
    }
    let dashes = offset - 2;
    let opener = rest[offset..].chars().next()?;
    let closer = match opener {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return None,
    };
    let content_start = start + offset + opener.len_utf8();
    let close = format!("{closer}{}{quote}", "-".repeat(dashes));
    Some(
        text[content_start..]
            .find(&close)
            .map_or(text.len(), |index| content_start + index + close.len()),
    )
}

fn r_namespace_call_end(text: &str, end: usize) -> Option<usize> {
    let rest = &text[end..];
    if !rest.starts_with("::") {
        return None;
    }
    let separator = if rest.starts_with(":::") { 3 } else { 2 };
    let name_end = r_identifier_end(text, end + separator);
    (name_end > end + separator).then_some(name_end)
}

fn r(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "break", "else", "for", "function", "if", "in", "next", "repeat", "while",
    ];
    const BUILTINS: &[&str] = &[
        "NextMethod", "R6Class", "UseMethod", "attach", "detach", "library", "require",
        "return", "setClass", "setGeneric", "setGroupGeneric", "setMethod", "setRefClass",
        "source", "stop", "switch", "try", "tryCatch", "warning",
    ];
    const CONSTANTS: &[&str] = &[
        "FALSE", "Inf", "NA", "NA_character_", "NA_complex_", "NA_integer_", "NA_real_",
        "NULL", "NaN", "TRUE",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if let Some(end) = r_raw_string_end(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch)
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_alphabetic() || matches!(next, '_' | '.')))
            {
                let end = r_identifier_end(text, index);
                let word = &text[index..end];
                if CONSTANTS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word)
                    || (BUILTINS.contains(&word) && next_non_space(text, end) == Some('('))
                {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if let Some(name_end) = r_namespace_call_end(text, end)
                    && next_non_space(text, name_end) == Some('(')
                {
                    push(&mut spans, index, name_end, SyntaxToken::Function);
                    index = name_end;
                    continue;
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn powershell_identifier_end(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if is_ident_continue(ch) {
            index += ch.len_utf8();
            continue;
        }
        if ch == '-'
            && index > start
            && text[index + 1..]
                .chars()
                .next()
                .is_some_and(is_ident_continue)
        {
            index += 1;
            continue;
        }
        break;
    }
    index
}

fn powershell_string_end(text: &str, start: usize, quote: char) -> usize {
    let mut index = start + 1;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if quote == '\'' && ch == '\'' {
            if text[index + 1..].starts_with('\'') {
                index += 2;
                continue;
            }
            return index + 1;
        }
        if quote == '"' && ch == '`' {
            index += 1;
            if index < text.len() {
                index += text[index..].chars().next().map_or(1, char::len_utf8);
            }
            continue;
        }
        if ch == quote {
            return index + 1;
        }
        index += ch.len_utf8();
    }
    text.len()
}

fn powershell_here_string_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    let quote = if rest.starts_with("@'") {
        '\''
    } else if rest.starts_with("@\"") {
        '"'
    } else {
        return None;
    };
    let close = format!("{quote}@");
    let content_start = start + 2;
    Some(
        text[content_start..]
            .find(&close)
            .map_or(text.len(), |index| content_start + index + close.len()),
    )
}

fn powershell_variable_end(text: &str, start: usize) -> usize {
    let mut index = start + 1;
    if text[index..].starts_with('{') {
        return text[index..]
            .find('}')
            .map_or(text.len(), |offset| index + offset + 1);
    }
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if is_ident_continue(ch) || matches!(ch, ':' | '?') {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    index
}

fn powershell_bracket_token(text: &str, index: usize) -> Option<(usize, SyntaxToken)> {
    let close = text[index + 1..].find(']')?;
    let end = index + 1 + close + 1;
    let inner = text[index + 1..end - 1].trim();
    if inner.is_empty()
        || inner
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    if inner.contains('(') {
        return Some((end, SyntaxToken::Attribute));
    }
    if inner.chars().all(|ch| {
        ch.is_alphanumeric() || matches!(ch, '_' | '.' | '-' | ':' | ',' | '[' | ']')
    }) {
        return Some((end, SyntaxToken::Type));
    }
    None
}

fn powershell_builtin(word: &str) -> bool {
    [
        "Add-Content", "Clear-Host", "ConvertFrom-Json", "ConvertTo-Json", "Copy-Item",
        "ForEach-Object", "Get-ChildItem", "Get-Command", "Get-Content", "Get-Date",
        "Get-Help", "Get-Item", "Get-Location", "Get-Member", "Get-Process",
        "Get-Service", "Group-Object", "Invoke-Expression", "Invoke-RestMethod",
        "Invoke-WebRequest", "Join-Path", "Measure-Object", "Move-Item", "New-Item",
        "Out-File", "Read-Host", "Remove-Item", "Resolve-Path", "Select-Object",
        "Set-Content", "Set-Location", "Sort-Object", "Split-Path", "Start-Process",
        "Stop-Process", "Test-Path", "Where-Object", "Write-Debug", "Write-Error",
        "Write-Host", "Write-Output", "Write-Verbose", "Write-Warning",
    ]
    .iter()
    .any(|builtin| builtin.eq_ignore_ascii_case(word))
}

fn powershell(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "begin", "break", "catch", "class", "continue", "data", "define", "do",
        "dynamicparam", "else", "elseif", "end", "enum", "exit", "filter", "finally",
        "for", "foreach", "from", "function", "hidden", "if", "in", "param", "process",
        "return",
        "static", "switch", "throw", "trap", "try", "until", "using", "var", "while",
        "workflow",
    ];
    const OPERATORS: &[&str] = &[
        "and", "as", "contains", "eq", "ge", "gt", "in", "is", "isnot", "le", "like",
        "lt", "match", "ne", "not", "notcontains", "notlike", "notmatch", "or",
        "replace", "split", "join",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("#") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("<#") {
            let end = text[index + 2..]
                .find("#>")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if let Some(end) = powershell_here_string_end(text, index) {
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if matches!(ch, '"' | '\'') {
                let end = powershell_string_end(text, index, ch);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' {
                let end = powershell_variable_end(text, index);
                let word = &text[index..end];
                push(
                    &mut spans,
                    index,
                    end,
                    if matches!(word, "$true" | "$false" | "$null") {
                        SyntaxToken::Number
                    } else {
                        SyntaxToken::Property
                    },
                );
                index = end;
            } else if ch == '[' {
                if let Some((end, token)) = powershell_bracket_token(text, index) {
                    push(&mut spans, index, end, token);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = powershell_identifier_end(text, index);
                let word = &text[index..end];
                if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(word, "class" | "enum" | "function" | "filter")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = powershell_identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "class" || word == "enum" {
                                SyntaxToken::Type
                            } else {
                                SyntaxToken::Function
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if OPERATORS.contains(&word.to_ascii_lowercase().as_str()) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if powershell_builtin(word)
                    || word.contains('-')
                    || next_non_space(text, end) == Some('(')
                {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn elixir_identifier_end(text: &str, start: usize) -> usize {
    let end = identifier_end(text, start);
    if text[end..].starts_with(['?', '!']) {
        end + 1
    } else {
        end
    }
}

fn elixir_chain_end(text: &str, start: usize) -> usize {
    let mut end = elixir_identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = elixir_identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn elixir_sigil_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start + 1;
    let letter = text[index..].chars().next()?;
    if !letter.is_alphabetic() {
        return None;
    }
    index += letter.len_utf8();
    let delimiter = text[index..].chars().next()?;
    let closer = match delimiter {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        '/' => '/',
        '|' => '|',
        '"' => '"',
        '\'' => '\'',
        _ => return None,
    };
    index += delimiter.len_utf8();
    let paired = delimiter != closer;
    let mut depth = 1usize;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '\\' {
            index += ch.len_utf8();
            if let Some(next) = text[index..].chars().next() {
                index += next.len_utf8();
            }
            continue;
        }
        if paired && ch == delimiter {
            depth += 1;
        } else if ch == closer {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                index += ch.len_utf8();
                while text[index..]
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_ascii_alphabetic())
                {
                    index += 1;
                }
                return Some(index);
            }
        }
        index += ch.len_utf8();
    }
    Some(text.len())
}

fn elixir(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "after", "alias", "and", "case", "catch", "cond", "def", "defdelegate",
        "defexception", "defguard", "defimpl", "defmacro", "defmodule", "defp",
        "defprotocol", "defstruct", "do", "else", "end", "fn", "for", "if", "import",
        "in", "not", "or", "quote", "raise", "receive", "require", "rescue", "reraise",
        "try", "unless", "unquote", "use", "when", "while", "with",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if text[index..].starts_with("'''") {
                let end = skip_quoted(text, index, '\'', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '~' {
                if let Some(end) = elixir_sigil_end(text, index) {
                    push(&mut spans, index, end, SyntaxToken::String);
                    index = end;
                } else {
                    index += 1;
                }
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == ':' && text[index + 1..].chars().next().is_some_and(is_ident_start) {
                let end = elixir_identifier_end(text, index + 1);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch == '@' {
                let end = elixir_identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = elixir_identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(
                        word,
                        "def" | "defdelegate" | "defguard" | "defmacro" | "defp"
                    ) && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = elixir_identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                    if matches!(word, "defimpl" | "defmodule" | "defprotocol")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = elixir_chain_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    let chain_end = elixir_chain_end(text, end);
                    if chain_end > end && next_non_space(text, chain_end) == Some('(') {
                        push(&mut spans, index, chain_end, SyntaxToken::Function);
                        index = chain_end;
                        continue;
                    }
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn haskell_identifier_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('\'') {
        end += 1;
    }
    end
}

fn haskell_block_comment_end(text: &str, start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < text.len() {
        if text[index..].starts_with("{-") {
            depth += 1;
            index += 2;
        } else if text[index..].starts_with("-}") {
            depth = depth.saturating_sub(1);
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

fn haskell_builtin(word: &str) -> bool {
    matches!(
        word,
        "concat" | "concatMap" | "drop" | "elem" | "filter" | "foldl" | "foldl'"
            | "foldr" | "getLine" | "head" | "init" | "last" | "length" | "lines"
            | "lookup" | "map" | "mapM" | "mapM_" | "maximum" | "minimum" | "notElem"
            | "null" | "print" | "product" | "pure" | "putStr" | "putStrLn" | "read"
            | "readLn" | "reverse" | "show" | "sort" | "sum" | "tail" | "take"
            | "unlines" | "unwords" | "words" | "zip" | "zipWith"
    )
}

fn haskell(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "as", "case", "class", "data", "default", "deriving", "do", "else", "export",
        "foreign", "forall", "hiding", "if", "import", "in", "infix", "infixl",
        "infixr", "instance", "let", "mdo", "module", "newtype", "of", "proc",
        "qualified", "rec", "safe", "then", "type", "unsafe", "where",
    ];
    const DECLARATIONS: &[&str] = &[
        "class", "data", "module", "newtype", "type",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("{-#") {
            let end = text[index + 3..]
                .find("#-}")
                .map_or(text.len(), |offset| index + 3 + offset + 3);
            push(&mut spans, index, end, SyntaxToken::Keyword);
            index = end;
        } else if text[index..].starts_with("{-") {
            let end = haskell_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("--") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = haskell_identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "True" | "False") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if DECLARATIONS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = haskell_identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if haskell_builtin(word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else {
                    let line_start = text[..index].rfind('\n').map_or(0, |offset| offset + 1);
                    let line_end = text[index..]
                        .find('\n')
                        .map_or(text.len(), |offset| index + offset);
                    let rest = &text[end..line_end];
                    if text[line_start..index].trim().is_empty()
                        && (rest.contains("::") || rest.contains('='))
                    {
                        push(&mut spans, index, end, SyntaxToken::Function);
                    }
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn clojure_symbol_start(ch: char) -> bool {
    is_ident_start(ch) || matches!(ch, '-' | '+' | '*' | '/' | '<' | '>' | '=' | '!' | '?')
}

fn clojure_symbol_end(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_alphanumeric() || matches!(ch, '_' | '-' | '+' | '*' | '/' | '.' | '<' | '>' | '=' | '!' | '?') {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    index
}

fn clojure_builtin(word: &str) -> bool {
    matches!(
        word,
        "apply" | "assoc" | "comp" | "conj" | "cons" | "count" | "dec" | "dissoc"
            | "empty?" | "filter" | "first" | "fn?" | "get" | "get-in" | "hash-map"
            | "identity" | "inc" | "keys" | "keyword" | "keyword?" | "list" | "map"
            | "map?" | "max" | "merge" | "min" | "mod" | "name" | "namespace" | "neg?"
            | "next" | "nil?" | "number?" | "partial" | "pos?" | "prn" | "print"
            | "println" | "quot" | "range" | "reduce" | "rem" | "rest" | "seq"
            | "seq?" | "set" | "set?" | "some?" | "str" | "string?" | "symbol"
            | "true?" | "false?" | "update" | "update-in" | "vals" | "vector"
            | "vector?" | "zero?"
    )
}

fn clojure(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "catch", "cond", "condp", "def", "defmacro", "defmethod", "defmulti",
        "defn", "defonce", "defrecord", "defstruct", "deftype", "do", "dosync",
        "doseq", "dotimes", "fn", "for", "finally", "if", "if-let", "if-not",
        "import", "let", "letfn", "loop", "monitor-enter", "monitor-exit", "new",
        "ns", "quote", "recur", "require", "set!", "throw", "try", "var", "when",
        "when-let", "when-not", "while", "with-open", "with-redefs",
    ];
    const DEF_FORMS: &[&str] = &[
        "def", "defmacro", "defmethod", "defmulti", "defn", "defonce",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with(';') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\\' {
                let mut end = index + 1;
                while end < text.len() {
                    let next = text[end..].chars().next().unwrap_or('\0');
                    if next.is_whitespace() || matches!(next, '(' | ')' | '[' | ']' | '{' | '}') {
                        break;
                    }
                    end += next.len_utf8();
                }
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == ':' && text[index + 1..].chars().next().is_some_and(clojure_symbol_start) {
                let end = clojure_symbol_end(text, index + 1);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if clojure_symbol_start(ch) {
                let end = clojure_symbol_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if DEF_FORMS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && clojure_symbol_start(ch)
                    {
                        let name_end = clojure_symbol_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                } else if clojure_builtin(word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn julia_identifier_end(text: &str, start: usize) -> usize {
    let end = identifier_end(text, start);
    if text[end..].starts_with(['!', '?']) {
        end + 1
    } else {
        end
    }
}

fn julia_block_comment_end(text: &str, start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < text.len() {
        if text[index..].starts_with("#=") {
            depth += 1;
            index += 2;
        } else if text[index..].starts_with("=#") {
            depth = depth.saturating_sub(1);
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

fn julia_builtin(word: &str) -> bool {
    matches!(
        word,
        "axes" | "collect" | "convert" | "eachindex" | "eltype" | "enumerate"
            | "fill" | "filter" | "first" | "getindex" | "isempty" | "join" | "last"
            | "length" | "map" | "mean" | "ones" | "open" | "parse" | "pop!" | "print"
            | "println" | "push!" | "range" | "read" | "reduce" | "setindex!" | "show"
            | "size" | "sort" | "sort!" | "split" | "string" | "sum" | "typeof"
            | "write" | "zeros" | "zip"
    )
}

fn julia(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "baremodule", "begin", "break", "catch", "const",
        "continue", "do", "else", "elseif", "end", "export", "false", "finally",
        "for", "function", "global", "if", "import", "in", "isa", "let", "local",
        "macro", "module", "mutable", "outer", "primitive", "public", "quote",
        "return", "struct", "true", "try", "type", "using", "where", "while",
    ];
    const DECLARATIONS: &[&str] = &["baremodule", "module", "struct", "type"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("#=") {
            let end = julia_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'' | '`') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == ':'
                && !text[..index].ends_with(':')
                && text[index + 1..].chars().next().is_some_and(is_ident_start)
            {
                let end = julia_identifier_end(text, index + 1);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch == '@' {
                let end = julia_identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = julia_identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(word, "function" | "macro")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = julia_identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                    if DECLARATIONS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = julia_identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if julia_builtin(word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn ocaml_block_comment_end(text: &str, start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < text.len() {
        if text[index..].starts_with("(*") {
            depth += 1;
            index += 2;
        } else if text[index..].starts_with("*)") {
            depth = depth.saturating_sub(1);
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

fn ocaml_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn ocaml_let_function(text: &str, keyword_end: usize) -> Option<(usize, usize)> {
    let (mut position, ch) = next_non_space_index(text, keyword_end)?;
    if !is_ident_start(ch) {
        return None;
    }
    let mut name_end = identifier_end(text, position);
    if &text[position..name_end] == "rec" {
        let (next, ch) = next_non_space_index(text, name_end)?;
        if !is_ident_start(ch) {
            return None;
        }
        position = next;
        name_end = identifier_end(text, position);
    }
    let line_end = text[name_end..]
        .find('\n')
        .map_or(text.len(), |offset| name_end + offset);
    let rest = &text[name_end..line_end];
    let equals = rest.find('=')?;
    let params = rest[..equals].trim();
    if params.is_empty() || params.starts_with(':') {
        return None;
    }
    Some((position, name_end))
}

fn ocaml_builtin(word: &str) -> bool {
    matches!(
        word,
        "Array.length" | "Array.map" | "Array.iter" | "List.filter" | "List.fold_left"
            | "List.fold_right" | "List.iter" | "List.map" | "List.rev" | "Printf.printf"
            | "String.concat" | "String.length" | "String.split_on_char" | "int_of_string"
            | "print_endline" | "print_int" | "print_string" | "string_of_int"
    )
}

fn ocaml_quoted_string_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with("{|") {
        return None;
    }
    let close = text[start + 2..].find("|}")?;
    Some(start + 2 + close + 2)
}

fn ocaml(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "and", "as", "assert", "begin", "class", "constraint", "do", "done",
        "downto", "else", "end", "exception", "external", "false", "for", "fun",
        "function", "functor", "if", "in", "include", "inherit", "initializer",
        "lazy", "let", "match", "method", "module", "mutable", "new", "nonrec",
        "object", "of", "open", "or", "private", "rec", "sig", "struct", "then",
        "to", "true", "try", "type", "val", "virtual", "when", "while", "with",
    ];
    const MODULES: &[&str] = &[
        "Array", "Buffer", "Hashtbl", "List", "Map", "Printf", "Set", "String",
        "Sys", "Unix",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("(*") {
            let end = ocaml_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if let Some(end) = ocaml_quoted_string_end(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\'' {
                let (end, token) = single_quote_end(text, index);
                push(&mut spans, index, end, token);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(word, "let" | "and") {
                        if word == "let"
                            && let Some((position, ch)) = next_non_space_index(text, end)
                            && is_ident_start(ch)
                        {
                            let rec_end = identifier_end(text, position);
                            if &text[position..rec_end] == "rec" {
                                push(&mut spans, position, rec_end, SyntaxToken::Keyword);
                            }
                        }
                        if let Some((position, name_end)) = ocaml_let_function(text, end) {
                            push(&mut spans, position, name_end, SyntaxToken::Function);
                            index = name_end;
                            continue;
                        }
                    }
                    if matches!(word, "type" | "module" | "val" | "external")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "val" || word == "external" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    let chain_end = ocaml_chain_end(text, end);
                    if chain_end > end
                        && MODULES.contains(&word)
                        && text[end..chain_end].contains('.')
                    {
                        push(&mut spans, index, chain_end, SyntaxToken::Function);
                        index = chain_end;
                        continue;
                    }
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if ocaml_builtin(word) || next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn erlang_number_end(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len()
        && (text.as_bytes()[index].is_ascii_digit() || text.as_bytes()[index] == b'_')
    {
        index += 1;
    }
    if text[index..].starts_with('#') {
        index += 1;
        while index < text.len()
            && (text.as_bytes()[index].is_ascii_alphanumeric()
                || text.as_bytes()[index] == b'_')
        {
            index += 1;
        }
        index
    } else {
        c_number_end(text, start, false)
    }
}

fn erlang_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with(':') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn erlang_type(word: &str) -> bool {
    matches!(
        word,
        "atom" | "binary" | "bitstring" | "boolean" | "byte" | "char" | "float"
            | "function" | "identifier" | "integer" | "iodata" | "iolist" | "list"
            | "map" | "maybe" | "mfa" | "module" | "neg_integer" | "nil" | "no_return"
            | "node" | "non_neg_integer" | "none" | "number" | "pid" | "port"
            | "pos_integer" | "reference" | "string" | "term" | "timeout" | "tuple"
    )
}

fn erlang(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "after", "andalso", "band", "begin", "bnot", "bor", "bsl", "bsr", "bxor",
        "case", "catch", "cond", "div", "end", "fun", "if", "let", "not", "of",
        "or", "orelse", "receive", "rem", "try", "when", "xor",
    ];
    const ATTRIBUTES: &[&str] = &[
        "behaviour", "callback", "export", "import", "module", "record", "spec",
        "type",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with('%') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '-'
                && text[index + 1..]
                    .chars()
                    .next()
                    .is_some_and(is_ident_start)
            {
                let end = identifier_end(text, index + 1);
                let name = &text[index + 1..end];
                if ATTRIBUTES.contains(&name) {
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\'' {
                let end = skip_quoted(text, index, '\'', false);
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if ch == '$' {
                let mut end = index + 1;
                if text[end..].starts_with('\\') {
                    end += 1;
                }
                if let Some(next) = text[end..].chars().next() {
                    end += next.len_utf8();
                }
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit() {
                let end = erlang_number_end(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if erlang_type(word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Property);
                } else {
                    let chain_end = erlang_chain_end(text, end);
                    if chain_end > end
                        || next_non_space(text, end) == Some('(')
                    {
                        push(&mut spans, index, chain_end, SyntaxToken::Function);
                        index = chain_end;
                        continue;
                    }
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn elm_block_comment_end(text: &str, start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < text.len() {
        if text[index..].starts_with("{-") {
            depth += 1;
            index += 2;
        } else if text[index..].starts_with("-}") {
            depth = depth.saturating_sub(1);
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

fn elm_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn elm_function_definition(text: &str, start: usize, end: usize) -> bool {
    let line_start = text[..start].rfind('\n').map_or(0, |offset| offset + 1);
    if !text[line_start..start].trim().is_empty() {
        return false;
    }
    let line_end = text[end..]
        .find('\n')
        .map_or(text.len(), |offset| end + offset);
    let rest = &text[end..line_end];
    if rest.trim_start().starts_with(':') {
        return true;
    }
    let Some(equals) = rest.find('=') else {
        return false;
    };
    let params = rest[..equals].trim();
    &text[start..end] == "main" || !params.is_empty()
}

fn elm(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "alias", "as", "case", "else", "exposing", "if", "import", "in", "infix",
        "infixl", "infixr", "let", "module", "of", "port", "then", "type", "where",
    ];
    const MODULES: &[&str] = &[
        "Browser", "Cmd", "Debug", "Dict", "Html", "Http", "Json", "List", "Maybe",
        "Platform", "Set", "String", "Sub", "Task", "Time", "Url",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("{-") {
            let end = elm_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("--") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "True" | "False") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if word == "type"
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let mut name_end = identifier_end(text, position);
                        if &text[position..name_end] == "alias"
                            && let Some((next, ch)) = next_non_space_index(text, name_end)
                            && is_ident_start(ch)
                        {
                            push(&mut spans, position, name_end, SyntaxToken::Keyword);
                            name_end = identifier_end(text, next);
                            push(&mut spans, next, name_end, SyntaxToken::Type);
                        } else {
                            push(&mut spans, position, name_end, SyntaxToken::Type);
                        }
                        index = name_end;
                        continue;
                    }
                    if word == "module"
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    let chain_end = elm_chain_end(text, end);
                    if chain_end > end && MODULES.contains(&word) {
                        push(&mut spans, index, chain_end, SyntaxToken::Function);
                        index = chain_end;
                        continue;
                    }
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if elm_function_definition(text, index, end) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn hcl_heredoc_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    if !rest.starts_with("<<") {
        return None;
    }
    let mut index = start + 2;
    if text[index..].starts_with('-') {
        index += 1;
    }
    let marker_start = index;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_alphanumeric() || ch == '_' {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    if index == marker_start {
        return None;
    }
    let marker = &text[marker_start..index];
    let mut line_start = text[index..]
        .find('\n')
        .map_or(text.len(), |offset| index + offset + 1);
    while line_start < text.len() {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |offset| line_start + offset);
        if text[line_start..line_end].trim() == marker {
            return Some(line_end);
        }
        line_start = (line_end + 1).min(text.len());
    }
    Some(text.len())
}

fn hcl_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn hcl_builtin(word: &str) -> bool {
    matches!(
        word,
        "abs" | "can" | "ceil" | "cidrsubnet" | "coalesce" | "concat" | "element"
            | "file" | "flatten" | "floor" | "format" | "join" | "jsondecode"
            | "jsonencode" | "length" | "lookup" | "lower" | "max" | "merge" | "min"
            | "range" | "regex" | "replace" | "split" | "substr" | "templatefile"
            | "tolist" | "tomap" | "tonumber" | "toset" | "tostring" | "try" | "upper"
    )
}

fn hcl(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "action", "backend", "check", "connection", "content", "count", "data",
        "depends_on", "dynamic", "else", "endfor", "endif", "for", "for_each", "if",
        "import", "in", "lifecycle", "locals", "module", "moved", "output",
        "postcondition", "precondition", "provider", "provisioner", "required_providers",
        "resource", "run", "source", "terraform", "validation", "variable",
    ];
    const NAMESPACES: &[&str] = &[
        "count", "data", "each", "local", "module", "path", "terraform", "var",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') || text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if let Some(end) = hcl_heredoc_end(text, index) {
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if let Some((equals, '=')) = next_non_space_index(text, end)
                    && !text[equals + 1..].starts_with(['=', '>'])
                {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                    continue;
                }
                let chain_end = hcl_chain_end(text, end);
                if NAMESPACES.contains(&word) && chain_end > end {
                    push(&mut spans, index, chain_end, SyntaxToken::Property);
                    index = chain_end;
                    continue;
                }
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if hcl_builtin(word) || next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn nix_indented_string_end(text: &str, start: usize) -> Option<usize> {
    if !text[start..].starts_with("''") {
        return None;
    }
    let close = text[start + 2..].find("''")?;
    Some(start + 2 + close + 2)
}

fn nix_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn nix_builtin(word: &str) -> bool {
    matches!(
        word,
        "abort" | "baseNameOf" | "builtins" | "derivation" | "dirOf" | "fetchTarball"
            | "import" | "isNull" | "map" | "removeAttrs" | "throw" | "toString"
    )
}

fn nix(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "assert", "else", "if", "in", "inherit", "let", "or", "rec", "then", "with",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if let Some(end) = nix_indented_string_end(text, index) {
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let chain_end = nix_chain_end(text, index);
                let word = &text[index..chain_end];
                if let Some((equals, '=')) = next_non_space_index(text, chain_end)
                    && !text[equals + 1..].starts_with('=')
                {
                    push(&mut spans, index, chain_end, SyntaxToken::Property);
                    index = chain_end;
                    continue;
                }
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, chain_end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, chain_end, SyntaxToken::Keyword);
                } else if nix_builtin(word) || word.starts_with("builtins.") || word.starts_with("lib.")
                {
                    push(&mut spans, index, chain_end, SyntaxToken::Function);
                }
                index = chain_end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn puppet_builtin(word: &str) -> bool {
    matches!(
        word,
        "contain" | "each" | "epp" | "fail" | "file" | "filter" | "hiera" | "include"
            | "join" | "lookup" | "map" | "notice" | "realize" | "regsubst" | "require"
            | "split" | "sprintf" | "str2bool" | "tag" | "template" | "versioncmp"
            | "warning"
    )
}

fn puppet(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "and", "application", "attr", "case", "class", "consumes", "default", "define",
        "else", "elsif", "false", "function", "if", "in", "inherits", "node", "or",
        "plan", "private", "produces", "public", "site", "true", "type", "undef",
        "unless",
    ];
    const DECLARATIONS: &[&str] = &["class", "define", "function", "node", "plan", "type"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' {
                if text[index + 1..].starts_with('{') {
                    let end = text[index + 1..]
                        .find('}')
                        .map_or(text.len(), |offset| index + 1 + offset + 1);
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                } else {
                    let end = identifier_end(text, index + 1);
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                }
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if let Some((arrow, '=')) = next_non_space_index(text, end)
                    && text[arrow..].starts_with("=>")
                {
                    push(&mut spans, index, end, SyntaxToken::Property);
                    index = end;
                    continue;
                }
                if matches!(word, "true" | "false" | "undef") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if DECLARATIONS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && (is_ident_start(ch) || matches!(ch, '\'' | '"'))
                    {
                        let name_end = if matches!(ch, '\'' | '"') {
                            skip_quoted(text, position, ch, false)
                        } else {
                            identifier_end(text, position)
                        };
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase)
                    || next_non_space(text, end) == Some('{')
                {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if puppet_builtin(word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn graphql(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "directive", "enum", "extend", "fragment", "implements", "input", "interface",
        "mutation", "on", "query", "repeatable", "scalar", "schema", "subscription",
        "type", "union",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with('#') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' || ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    push(
                        &mut spans,
                        index,
                        end,
                        if ch == '$' {
                            SyntaxToken::Property
                        } else {
                            SyntaxToken::Attribute
                        },
                    );
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if ch == '-'
                && text[index + 1..]
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_ascii_digit())
            {
                let end = c_number_end(text, index + 1, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn vue_template_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let lower = text.to_ascii_lowercase();
    let mut ranges = Vec::new();
    let mut index = 0usize;
    while let Some(offset) = lower[index..].find("<template") {
        let open = index + offset;
        let Some(close_open) = lower[open..].find('>') else {
            break;
        };
        let content_start = open + close_open + 1;
        let Some(close_offset) = lower[content_start..].find("</template>") else {
            break;
        };
        let close = content_start + close_offset + "</template>".len();
        ranges.push(open..close);
        index = close;
    }
    ranges
}

fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by_key(|span| span.start);
    let mut merged = Vec::with_capacity(spans.len());
    for span in spans {
        if span.start >= span.end || merged.last().is_some_and(|prev: &Span| span.start < prev.end) {
            continue;
        }
        merged.push(span);
    }
    merged
}

fn vue(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    for range in vue_template_ranges(text) {
        let mut index = range.start;
        while index < range.end {
            let ch = masked[index..range.end].chars().next().unwrap_or('\0');
            if matches!(ch, '@' | ':') {
                let previous = masked[..index].chars().next_back();
                let next = masked[index + 1..].chars().next();
                if previous.is_some_and(|prev| prev.is_whitespace() || prev == '<')
                    && next.is_some_and(is_ident_start)
                {
                    let end = identifier_end_markup(&masked, index + 1).min(range.end);
                    if end > index + 1 {
                        push(&mut spans, index, end, SyntaxToken::Attribute);
                        index = end;
                        continue;
                    }
                }
            }
            if ch == '{'
                && masked[index..range.end].starts_with("{{")
                && let Some(close) = masked[index + 2..range.end].find("}}")
            {
                let end = index + 2 + close + 2;
                push(&mut spans, index, end, SyntaxToken::Keyword);
                index = end;
                continue;
            }
            index += ch.len_utf8();
        }
    }
    merge_spans(spans)
}

fn markup_tag_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    let mut quote = None;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == '>' {
            return Some(index + 1);
        }
        index += ch.len_utf8();
    }
    None
}

fn svelte_raw_end(lower: &str, text: &str, index: usize, name: &str) -> Option<usize> {
    let prefix = format!("<{name}");
    if !starts_with_ignore_ascii_case(&text[index..], &prefix) {
        return None;
    }
    let after = index + prefix.len();
    if !text[after..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_whitespace() || matches!(ch, '>' | '/'))
    {
        return None;
    }
    let end = markup_tag_end(text, after)?;
    if end >= 2 && &text[end - 2..end] == "/>" {
        return Some(end);
    }
    let marker = format!("</{name}");
    Some(
        lower[end..]
            .find(&marker)
            .map_or(text.len(), |offset| end + offset),
    )
}

fn svelte_expression_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start + 1;
    let mut depth = 1usize;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        match ch {
            '\'' | '"' | '`' => index = skip_quoted(text, index, ch, false),
            '{' => {
                depth += 1;
                index += 1;
            }
            '}' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += ch.len_utf8(),
        }
    }
    None
}

fn svelte_prefix_end(text: &str, open: usize) -> usize {
    let mut cursor = open + 1;
    while cursor < text.len()
        && text[cursor..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
    }
    let Some(marker) = text[cursor..].chars().next() else {
        return open + 1;
    };
    if !matches!(marker, '#' | ':' | '/' | '@') {
        return open + 1;
    }
    let name_start = cursor + marker.len_utf8();
    let name_end = identifier_end(text, name_start);
    if name_end == name_start {
        return open + 1;
    }
    let mut prefix_end = name_end;
    if &text[name_start..name_end] == "else" {
        let mut probe = name_end;
        while probe < text.len()
            && text[probe..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            probe += text[probe..].chars().next().map_or(1, char::len_utf8);
        }
        let if_end = identifier_end(text, probe);
        if &text[probe..if_end] == "if" {
            prefix_end = if_end;
        }
    }
    prefix_end
}

fn svelte(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    let lower = text.to_ascii_lowercase();
    let mut index = 0usize;
    while index < text.len() {
        if let Some(end) = svelte_raw_end(&lower, text, index, "script")
            .or_else(|| svelte_raw_end(&lower, text, index, "style"))
        {
            index = end;
            continue;
        }
        if text[index..].starts_with("<!--") {
            index = text[index + 4..]
                .find("-->")
                .map_or(text.len(), |offset| index + 4 + offset + 3);
            continue;
        }
        if text[index..].starts_with('{')
            && let Some(close) = svelte_expression_end(text, index)
        {
            let prefix_end = svelte_prefix_end(text, index);
            push(&mut spans, index, prefix_end, SyntaxToken::Keyword);
            if prefix_end < close - 1 {
                for span in script(&text[prefix_end..close - 1]) {
                    push(
                        &mut spans,
                        prefix_end + span.start,
                        prefix_end + span.end,
                        span.token,
                    );
                }
            }
            push(&mut spans, close - 1, close, SyntaxToken::Keyword);
            index = close;
            continue;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    merge_spans(spans)
}

fn astro_frontmatter(
    text: &str,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let first_end = text.find('\n').unwrap_or(text.len());
    if !text.starts_with("---") || !text[3..first_end].trim().is_empty() {
        return None;
    }
    let content_start = (first_end + 1).min(text.len());
    let mut cursor = content_start;
    loop {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset);
        let line = text[cursor..line_end].trim();
        if line == "---" || line == "..." {
            return Some((0..line_end, content_start..cursor));
        }
        if line_end == text.len() {
            return Some((0..text.len(), content_start..text.len()));
        }
        cursor = line_end + 1;
    }
}

fn astro(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let lower = text.to_ascii_lowercase();
    let mut skip_ranges = Vec::new();
    let mut scan_start = 0usize;
    if let Some((full, content)) = astro_frontmatter(text) {
        let open_end = text[..full.end].find('\n').unwrap_or(full.end);
        push(&mut spans, 0, open_end, SyntaxToken::Comment);
        if content.end < full.end {
            push(&mut spans, content.end, full.end, SyntaxToken::Comment);
        }
        for span in script(&text[content.clone()]) {
            push(
                &mut spans,
                content.start + span.start,
                content.start + span.end,
                span.token,
            );
        }
        scan_start = full.end;
        skip_ranges.push(full);
    }
    let mut index = scan_start;
    while index < text.len() {
        if let Some(end) = svelte_raw_end(&lower, text, index, "script")
            .or_else(|| svelte_raw_end(&lower, text, index, "style"))
        {
            skip_ranges.push(index..end);
            index = end;
            continue;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    skip_ranges.sort_by_key(|range| range.start);
    let mut index = 0usize;
    let mut next_skip = 0usize;
    while index < text.len() {
        while next_skip < skip_ranges.len() && skip_ranges[next_skip].end <= index {
            next_skip += 1;
        }
        if let Some(range) = skip_ranges.get(next_skip)
            && range.start <= index
        {
            index = range.end.max(index + 1);
            next_skip += 1;
            continue;
        }
        let ch = masked[index..].chars().next().unwrap_or('\0');
        if ch == '{'
            && let Some(close) = svelte_expression_end(text, index)
        {
            push(&mut spans, index, index + 1, SyntaxToken::Keyword);
            if index + 1 < close - 1 {
                for span in script(&text[index + 1..close - 1]) {
                    push(
                        &mut spans,
                        index + 1 + span.start,
                        index + 1 + span.end,
                        span.token,
                    );
                }
            }
            push(&mut spans, close - 1, close, SyntaxToken::Keyword);
            index = close;
            continue;
        }
        index += ch.len_utf8();
    }
    merge_spans(spans)
}

fn zig(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "allowzero", "and", "anyerror", "anyframe", "anyopaque", "anytype", "asm",
        "bool", "break", "callconv", "catch", "comptime", "comptime_float", "comptime_int",
        "const", "continue", "defer", "else", "enum", "errdefer", "error", "export",
        "extern", "f128", "f16", "f32", "f64", "f80", "fn", "for", "i128", "i16",
        "i32", "i64", "i8", "if", "inline", "isize", "linksection", "noalias",
        "noinline", "nosuspend", "opaque", "or", "orelse", "packed", "pub", "resume",
        "return", "struct", "suspend", "switch", "test", "threadlocal", "try", "type",
        "u128", "u16", "u32", "u64", "u8", "union", "unreachable", "usize", "var",
        "void", "volatile", "while", "align", "noreturn", "c_char", "c_short",
        "c_ushort", "c_int", "c_uint", "c_long", "c_ulong", "c_longlong",
        "c_ulonglong", "c_longdouble",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("\\\\") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\'' {
                let end = skip_quoted(text, index, '\'', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' {
                if text[index + 1..].starts_with('"') {
                    let end = skip_quoted(text, index + 1, '"', false);
                    push(&mut spans, index, end, SyntaxToken::String);
                    index = end;
                } else {
                    let end = identifier_end(text, index + 1);
                    if end > index + 1 {
                        push(&mut spans, index, end, SyntaxToken::Function);
                        index = end;
                    } else {
                        index += 1;
                    }
                }
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null" | "undefined") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if word == "fn"
                        && let Some((position, next)) = next_non_space_index(text, end)
                        && is_ident_start(next)
                    {
                        let name_end = identifier_end(text, position);
                        let token = if next.is_uppercase() {
                            SyntaxToken::Type
                        } else {
                            SyntaxToken::Function
                        };
                        push(&mut spans, position, name_end, token);
                        index = name_end;
                        continue;
                    }
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if ch.is_uppercase() {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn solidity_primitive(word: &str) -> bool {
    ["address", "string", "bytes", "bool", "uint", "int"]
        .iter()
        .any(|base| {
            word == *base
                || (word.starts_with(base)
                    && word[base.len()..]
                        .chars()
                        .all(|ch| ch.is_ascii_digit()))
        })
}

fn solidity(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "assembly", "break", "calldata", "catch", "constant",
        "constructor", "continue", "contract", "delete", "do", "else", "emit", "enum",
        "error", "event", "external", "finally", "for", "function", "global", "if",
        "immutable", "import", "indexed", "inherited", "interface", "internal", "is",
        "let", "library", "mapping", "memory", "modifier", "new", "nonpayable",
        "override", "payable", "pragma", "private", "public", "pure", "return",
        "returns", "storage", "struct", "throw", "transient", "try", "type",
        "unchecked", "using", "view", "virtual", "while",
    ];
    const GLOBAL_FUNCTIONS: &[&str] = &[
        "require", "assert", "revert", "selfdestruct", "suicide", "addmod", "mulmod",
        "keccak256", "sha256", "sha3", "ripemd160", "ecrecover", "unicode", "blockhash",
        "gasleft",
    ];
    const GLOBAL_VARIABLES: &[&str] = &["this", "super", "abi", "msg", "block", "tx", "now"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false")
                    || matches!(word, "ether" | "wei" | "gwei" | "finney" | "szabo")
                    || matches!(
                        word,
                        "second"
                            | "seconds"
                            | "minute"
                            | "minutes"
                            | "hour"
                            | "hours"
                            | "day"
                            | "days"
                            | "week"
                            | "weeks"
                            | "year"
                            | "years"
                    )
                {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    let declaration = matches!(word, "function" | "modifier");
                    let type_declaration = matches!(
                        word,
                        "contract"
                            | "interface"
                            | "library"
                            | "struct"
                            | "enum"
                            | "error"
                            | "event"
                            | "type"
                    );
                    if (declaration || type_declaration)
                        && let Some((position, next)) = next_non_space_index(text, end)
                        && is_ident_start(next)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if declaration {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if solidity_primitive(word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if GLOBAL_FUNCTIONS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if GLOBAL_VARIABLES.contains(&word)
                    || previous_non_space(text, index) == Some('.')
                {
                    push(&mut spans, index, end, SyntaxToken::Property);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if ch.is_uppercase() {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn prisma_line_start(text: &str, index: usize) -> bool {
    let start = text[..index].rfind('\n').map_or(0, |offset| offset + 1);
    text[start..index].chars().all(char::is_whitespace)
}

fn prisma(text: &str) -> Vec<Span> {
    const BLOCKS: &[&str] = &["datasource", "enum", "generator", "model", "namespace", "types", "view"];
    const PRIMITIVES: &[&str] = &[
        "BigInt", "Boolean", "Bytes", "DateTime", "Decimal", "Float", "Int", "Json", "String",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' {
                let mut cursor = index + 1;
                if text[cursor..].starts_with('@') {
                    cursor += 1;
                }
                let end = identifier_end_markup(text, cursor);
                if end > cursor {
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                let line_start = prisma_line_start(text, index);
                if line_start && BLOCKS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if let Some((position, next)) = next_non_space_index(text, end)
                        && is_ident_start(next)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if line_start && word == "type" {
                    let next = next_non_space_index(text, end);
                    let name = next.filter(|(_, ch)| is_ident_start(*ch));
                    let assignment = name
                        .and_then(|(position, _)| {
                            let name_end = identifier_end(text, position);
                            next_non_space_index(text, name_end)
                                .filter(|(_, ch)| *ch == '=')
                                .map(|_| (position, name_end))
                        });
                    if let Some((position, name_end)) = assignment {
                        push(&mut spans, index, end, SyntaxToken::Keyword);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if matches!(word, "true" | "false") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if PRIMITIVES.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                } else if line_start {
                    push(&mut spans, index, end, SyntaxToken::Property);
                } else if ch.is_uppercase() {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn handlebars_mustache(
    text: &str,
    content: std::ops::Range<usize>,
    spans: &mut Vec<Span>,
) {
    let mut index = content.start;
    while index < content.end
        && text[index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    let mut sigil = None;
    if let Some(ch) = text[index..content.end].chars().next()
        && matches!(ch, '#' | '/' | '>' | '^' | '&' | '!')
    {
        sigil = Some(ch);
        push(spans, index, index + ch.len_utf8(), SyntaxToken::Keyword);
        index += ch.len_utf8();
    }
    while index < content.end
        && text[index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    if index < content.end && is_ident_start(text[index..].chars().next().unwrap_or('\0')) {
        let end = identifier_end_markup(text, index).min(content.end);
        let word = &text[index..end];
        let token = if word == "else"
            || (sigil.is_none()
                && matches!(
                    word,
                    "if" | "unless" | "each" | "with" | "lookup" | "log"
                ))
        {
            SyntaxToken::Keyword
        } else if sigil.is_some() {
            SyntaxToken::Function
        } else {
            SyntaxToken::Property
        };
        if token == SyntaxToken::Property {
            let chain = identifier_end_markup(text, index).min(content.end);
            push(spans, index, chain, token);
            index = chain;
        } else {
            push(spans, index, end, token);
            index = end;
        }
    }
    while index < content.end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else if matches!(ch, '"' | '\'') {
            let end = skip_quoted(text, index, ch, false).min(content.end);
            push(spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = c_number_end(text, index, false).min(content.end);
            push(spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end_markup(text, index).min(content.end);
            let mut probe = end;
            while probe < content.end
                && text[probe..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                probe += text[probe..].chars().next().map_or(1, char::len_utf8);
            }
            if text[probe..content.end].starts_with('=') {
                push(spans, index, end, SyntaxToken::Attribute);
                probe += 1;
                while probe < content.end
                    && text[probe..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                }
                if probe < content.end
                    && matches!(text[probe..].chars().next(), Some('"' | '\''))
                {
                    let close = skip_quoted(
                        text,
                        probe,
                        text[probe..].chars().next().unwrap_or('\''),
                        false,
                    )
                    .min(content.end);
                    push(spans, probe, close, SyntaxToken::String);
                    index = close;
                } else {
                    let value_start = probe;
                    while probe < content.end
                        && text[probe..]
                            .chars()
                            .next()
                            .is_some_and(|ch| !ch.is_whitespace())
                    {
                        probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                    }
                    if probe > value_start {
                        let token = if text[value_start..]
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_digit())
                        {
                            SyntaxToken::Number
                        } else {
                            SyntaxToken::String
                        };
                        push(spans, value_start, probe, token);
                    }
                    index = probe;
                }
            } else {
                push(spans, index, end, SyntaxToken::Property);
                index = end;
            }
        } else {
            index += ch.len_utf8();
        }
    }
}

fn handlebars(text: &str) -> Vec<Span> {
    let frontmatter = mdx_frontmatter(text).filter(|(_, _, toml)| !toml);
    let mut spans = html(text);
    if let Some((full, _, _)) = &frontmatter {
        spans.retain(|span| span.end <= full.start || span.start >= full.end);
    }
    if let Some((full, body, _)) = &frontmatter {
        let open_end = body.start.saturating_sub(1);
        push(&mut spans, 0, open_end, SyntaxToken::String);
        for span in yaml(&text[body.clone()]) {
            push(
                &mut spans,
                body.start + span.start,
                body.start + span.end,
                span.token,
            );
        }
        if body.end < full.end {
            push(&mut spans, body.end, full.end, SyntaxToken::String);
        }
    }
    let mut index = 0usize;
    while index < text.len() {
        if let Some((full, _, _)) = &frontmatter
            && full.start <= index
            && index < full.end
        {
            index = full.end;
            continue;
        }
        if text[index..].starts_with("{{!--") {
            let end = text[index + 6..]
                .find("--}}")
                .map_or(text.len(), |offset| index + 6 + offset + 4);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
            continue;
        }
        if text[index..].starts_with("{{!") {
            let end = text[index + 3..]
                .find("}}")
                .map_or(text.len(), |offset| index + 3 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
            continue;
        }
        if text[index..].starts_with("{{{") {
            if let Some(close) = text[index + 3..].find("}}}") {
                let close_start = index + 3 + close;
                push(&mut spans, index, index + 3, SyntaxToken::Keyword);
                handlebars_mustache(text, index + 3..close_start, &mut spans);
                push(&mut spans, close_start, close_start + 3, SyntaxToken::Keyword);
                index = close_start + 3;
                continue;
            }
        } else if text[index..].starts_with("{{")
            && let Some(close) = text[index + 2..].find("}}")
        {
            let close_start = index + 2 + close;
            push(&mut spans, index, index + 2, SyntaxToken::Keyword);
            handlebars_mustache(text, index + 2..close_start, &mut spans);
            push(&mut spans, close_start, close_start + 2, SyntaxToken::Keyword);
            index = close_start + 2;
            continue;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    merge_spans(spans)
}

fn liquid_tag_bounds(
    text: &str,
    start: usize,
    close: &str,
) -> Option<(usize, usize, usize)> {
    let close_position = start + 2 + text[start + 2..].find(close)?;
    let mut content_start = start + 2;
    let mut content_end = close_position;
    if text[content_start..].starts_with('-') {
        content_start += 1;
    }
    if text[content_start..content_end].ends_with('-') {
        content_end -= 1;
    }
    Some((content_start, content_end, close_position + close.len()))
}

fn liquid_word(text: &str, range: std::ops::Range<usize>) -> &str {
    text[range]
        .trim_matches(|ch: char| ch.is_whitespace() || ch == '-')
        .split_whitespace()
        .next()
        .unwrap_or("")
}

fn liquid_find_end_tag(text: &str, start: usize, name: &str) -> Option<(usize, usize)> {
    let mut cursor = start;
    while let Some(offset) = text[cursor..].find("{%") {
        let open = cursor + offset;
        let (content_start, content_end, end) = liquid_tag_bounds(text, open, "%}")?;
        if liquid_word(text, content_start..content_end) == name {
            return Some((open, end));
        }
        cursor = end;
    }
    None
}

fn liquid_push_closing_tag(text: &str, start: usize, end: usize, spans: &mut Vec<Span>) {
    let Some((content_start, content_end, tag_end)) = liquid_tag_bounds(text, start, "%}")
    else {
        push(spans, start, end, SyntaxToken::Keyword);
        return;
    };
    push(spans, start, content_start, SyntaxToken::Keyword);
    let word = liquid_word(text, content_start..content_end);
    if !word.is_empty() {
        let word_start =
            content_start + text[content_start..content_end].find(word).unwrap_or(0);
        push(
            spans,
            word_start,
            word_start + word.len(),
            SyntaxToken::Keyword,
        );
    }
    push(spans, content_end, tag_end, SyntaxToken::Keyword);
}

fn liquid_markup(
    text: &str,
    content: std::ops::Range<usize>,
    tag: bool,
    spans: &mut Vec<Span>,
) {
    const TAG_KEYWORDS: &[&str] = &[
        "assign", "break", "capture", "case", "comment", "continue", "cycle",
        "decrement", "doc", "echo", "else", "elsif", "endcapture", "endcase",
        "endcomment", "enddoc", "endfor", "endif", "endjavascript", "endraw",
        "endschema", "endstyle", "endstylesheet", "endunless", "for", "if",
        "include", "increment", "javascript", "liquid", "paginate", "raw", "render",
        "schema", "style", "stylesheet", "tablerow", "unless", "when",
    ];
    let mut index = content.start;
    while index < content.end
        && text[index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    if index < content.end && is_ident_start(text[index..].chars().next().unwrap_or('\0')) {
        let end = identifier_end(text, index).min(content.end);
        let word = &text[index..end];
        let token = if tag && TAG_KEYWORDS.contains(&word) {
            SyntaxToken::Keyword
        } else if matches!(word, "true" | "false" | "nil" | "blank" | "empty") {
            SyntaxToken::Number
        } else if tag {
            SyntaxToken::Function
        } else {
            let chain = identifier_end_markup(text, index).min(content.end);
            push(spans, index, chain, SyntaxToken::Property);
            index = chain;
            return liquid_markup_tail(text, index..content.end, spans);
        };
        push(spans, index, end, token);
        index = end;
    }
    liquid_markup_tail(text, index..content.end, spans);
}

fn liquid_markup_tail(text: &str, content: std::ops::Range<usize>, spans: &mut Vec<Span>) {
    const OPERATORS: &[&str] = &[
        "and", "as", "by", "cols", "contains", "for", "in", "limit", "offset", "or",
        "reversed", "with",
    ];
    let mut index = content.start;
    while index < content.end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else if ch == '|' {
            index += 1;
            while index < content.end
                && text[index..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                index += text[index..].chars().next().map_or(1, char::len_utf8);
            }
            if index < content.end
                && is_ident_start(text[index..].chars().next().unwrap_or('\0'))
            {
                let end = identifier_end_markup(text, index).min(content.end);
                push(spans, index, end, SyntaxToken::Function);
                index = end;
            }
        } else if let Some(operator) =
            ["==", "!=", ">=", "<=", ">", "<"].iter().find(|operator| {
                text[index..content.end].starts_with(*operator)
            })
        {
            push(
                spans,
                index,
                index + operator.len(),
                SyntaxToken::Keyword,
            );
            index += operator.len();
        } else if matches!(ch, '"' | '\'') {
            let end = skip_quoted(text, index, ch, false).min(content.end);
            push(spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = c_number_end(text, index, false).min(content.end);
            push(spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end(text, index).min(content.end);
            let word = &text[index..end];
            if text[end..content.end].starts_with(':') {
                push(spans, index, end + 1, SyntaxToken::Attribute);
                index = end + 1;
            } else if matches!(word, "true" | "false" | "nil" | "blank" | "empty") {
                push(spans, index, end, SyntaxToken::Number);
                index = end;
            } else if OPERATORS.contains(&word) {
                push(spans, index, end, SyntaxToken::Keyword);
                index = end;
            } else {
                let chain = identifier_end_markup(text, index).min(content.end);
                push(spans, index, chain, SyntaxToken::Property);
                index = chain;
            }
        } else {
            index += ch.len_utf8();
        }
    }
}

fn liquid(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("{%") {
            let Some((content_start, content_end, end)) =
                liquid_tag_bounds(text, index, "%}")
            else {
                index += 2;
                continue;
            };
            let word = liquid_word(text, content_start..content_end);
            let word_start =
                content_start + text[content_start..content_end].find(word).unwrap_or(0);
            let word_end = word_start + word.len();
            if matches!(word, "comment" | "doc")
                && let Some((_, end_after)) =
                    liquid_find_end_tag(text, end, &format!("end{word}"))
            {
                push(&mut spans, index, end_after, SyntaxToken::Comment);
                index = end_after;
                continue;
            }
            if word == "raw"
                && let Some((end_open, end_after)) =
                    liquid_find_end_tag(text, end, "endraw")
            {
                push(&mut spans, index, content_start, SyntaxToken::Keyword);
                push(&mut spans, word_start, word_end, SyntaxToken::Keyword);
                push(&mut spans, end, end_open, SyntaxToken::String);
                liquid_push_closing_tag(text, end_open, end_after, &mut spans);
                index = end_after;
                continue;
            }
            let embedded = match word {
                "style" | "stylesheet" => Some(css(&text[end..])
                    .into_iter()
                    .collect::<Vec<_>>()),
                "schema" => Some(json(&text[end..]).into_iter().collect::<Vec<_>>()),
                "javascript" => Some(script(&text[end..]).into_iter().collect::<Vec<_>>()),
                _ => None,
            };
            if let Some(nested) = embedded
                && let Some((end_open, end_after)) =
                    liquid_find_end_tag(text, end, &format!("end{word}"))
            {
                push(&mut spans, index, content_start, SyntaxToken::Keyword);
                push(&mut spans, word_start, word_end, SyntaxToken::Keyword);
                for span in nested
                    .into_iter()
                    .filter(|span| end + span.end <= end_open)
                {
                    push(&mut spans, end + span.start, end + span.end, span.token);
                }
                liquid_push_closing_tag(text, end_open, end_after, &mut spans);
                index = end_after;
                continue;
            }
            push(&mut spans, index, content_start, SyntaxToken::Keyword);
            liquid_markup(text, content_start..content_end, true, &mut spans);
            push(&mut spans, content_end, end, SyntaxToken::Keyword);
            index = end;
        } else if text[index..].starts_with("{{") {
            let Some((content_start, content_end, end)) =
                liquid_tag_bounds(text, index, "}}")
            else {
                index += 2;
                continue;
            };
            push(&mut spans, index, content_start, SyntaxToken::Keyword);
            liquid_markup(text, content_start..content_end, false, &mut spans);
            push(&mut spans, content_end, end, SyntaxToken::Keyword);
            index = end;
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    merge_spans(spans)
}

fn blade_paren_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start + 1;
    let mut depth = 1usize;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        match ch {
            '\'' | '"' | '`' => index = skip_quoted(text, index, ch, false),
            '(' => {
                depth += 1;
                index += 1;
            }
            ')' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += ch.len_utf8(),
        }
    }
    None
}

fn blade(text: &str) -> Vec<Span> {
    const DIRECTIVES: &[&str] = &[
        "auth", "aware", "break", "can", "cannot", "case", "checked", "choice", "class",
        "continue", "continueIf", "csrf", "default", "disabled", "else", "elseif", "empty",
        "endauth", "endcan", "endcannot", "enderror", "endforelse", "endforeach", "endfor",
        "endfragment", "endguest", "endif", "endjs", "endonce", "endphp", "endproduction",
        "endpush", "endsection", "endswitch", "endunless", "endverbatim", "endwhile",
        "env", "error", "extends", "forelse", "foreach", "for", "fragment", "guest", "if",
        "include", "includeIf", "includeWhen", "inject", "js", "json", "lang", "livewire",
        "method", "once", "php", "production", "props", "push", "readonly", "required",
        "section", "selected", "show", "stack", "stop", "style", "switch", "unless", "use",
        "verbatim", "vite", "while", "wire", "yield",
    ];
    let mut spans = html(text);
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("{{--") {
            let end = text[index + 4..]
                .find("--}}")
                .map_or(text.len(), |offset| index + 4 + offset + 4);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
            continue;
        }
        if text[index..].starts_with("{!!")
            && let Some(close) = text[index + 3..].find("!!}")
        {
            let close_start = index + 3 + close;
            push(&mut spans, index, index + 3, SyntaxToken::Keyword);
            for span in php(&text[index + 3..close_start]) {
                push(
                    &mut spans,
                    index + 3 + span.start,
                    index + 3 + span.end,
                    span.token,
                );
            }
            push(&mut spans, close_start, close_start + 3, SyntaxToken::Keyword);
            index = close_start + 3;
            continue;
        }
        if text[index..].starts_with("{{{")
            && let Some(close) = text[index + 3..].find("}}}")
        {
            let close_start = index + 3 + close;
            push(&mut spans, index, index + 3, SyntaxToken::Keyword);
            for span in php(&text[index + 3..close_start]) {
                push(
                    &mut spans,
                    index + 3 + span.start,
                    index + 3 + span.end,
                    span.token,
                );
            }
            push(&mut spans, close_start, close_start + 3, SyntaxToken::Keyword);
            index = close_start + 3;
            continue;
        }
        if text[index..].starts_with("{{")
            && !text[..index].ends_with('@')
            && let Some(close) = text[index + 2..].find("}}")
        {
            let close_start = index + 2 + close;
            push(&mut spans, index, index + 2, SyntaxToken::Keyword);
            for span in php(&text[index + 2..close_start]) {
                push(
                    &mut spans,
                    index + 2 + span.start,
                    index + 2 + span.end,
                    span.token,
                );
            }
            push(&mut spans, close_start, close_start + 2, SyntaxToken::Keyword);
            index = close_start + 2;
            continue;
        }
        if text[index..].starts_with("<?") {
            let end = text[index + 2..]
                .find("?>")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            spans.retain(|span| span.end <= index || span.start >= end);
            for span in php(&text[index..end]) {
                push(&mut spans, index + span.start, index + span.end, span.token);
            }
            index = end;
            continue;
        }
        if text[index..].starts_with("@{{") {
            index += 3;
            continue;
        }
        if text[index..].starts_with('@') {
            let name_start = index + 1;
            let name_end = identifier_end(text, name_start);
            let name = &text[name_start..name_end];
            if !name.is_empty() && DIRECTIVES.contains(&name) {
                if name == "verbatim"
                    && let Some(offset) = text[name_end..].find("@endverbatim")
                {
                    let content_start = name_end;
                    let content_end = name_end + offset;
                    let end = content_end + "@endverbatim".len();
                    spans.retain(|span| span.end <= index || span.start >= end);
                    push(&mut spans, index, name_end, SyntaxToken::Function);
                    push(
                        &mut spans,
                        content_start,
                        content_end,
                        SyntaxToken::String,
                    );
                    push(&mut spans, content_end, end, SyntaxToken::Function);
                    index = end;
                    continue;
                }
                if name == "php"
                    && next_non_space(text, name_end) != Some('(')
                    && let Some(offset) = text[name_end..].find("@endphp")
                {
                    let content_start = name_end;
                    let content_end = name_end + offset;
                    let end = content_end + "@endphp".len();
                    spans.retain(|span| span.end <= index || span.start >= end);
                    push(&mut spans, index, name_end, SyntaxToken::Function);
                    for span in php(&text[content_start..content_end]) {
                        push(
                            &mut spans,
                            content_start + span.start,
                            content_start + span.end,
                            span.token,
                        );
                    }
                    push(&mut spans, content_end, end, SyntaxToken::Function);
                    index = end;
                    continue;
                }
                push(&mut spans, index, name_end, SyntaxToken::Function);
                index = name_end;
                if next_non_space(text, index) == Some('(')
                    && let Some((open, _)) = next_non_space_index(text, index)
                    && let Some(close) = blade_paren_end(text, open)
                {
                    push(&mut spans, open, open + 1, SyntaxToken::Keyword);
                    for span in php(&text[open + 1..close - 1]) {
                        push(
                            &mut spans,
                            open + 1 + span.start,
                            open + 1 + span.end,
                            span.token,
                        );
                    }
                    push(&mut spans, close - 1, close, SyntaxToken::Keyword);
                    index = close;
                }
                continue;
            }
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    merge_spans(spans)
}

fn templ(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    spans.extend(go(text));
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut index = 0usize;
    while index < text.len() {
        let ch = masked[index..].chars().next().unwrap_or('\0');
        if is_ident_start(ch) {
            let end = identifier_end(&masked, index);
            if &masked[index..end] == "templ" {
                push(&mut spans, index, end, SyntaxToken::Keyword);
            }
            index = end;
        } else if ch == '@' {
            let mut end = index + 1;
            while end < text.len() {
                let next = masked[end..].chars().next().unwrap_or('\0');
                if is_ident_continue(next) || next == '.' {
                    end += next.len_utf8();
                } else {
                    break;
                }
            }
            if end > index + 1 {
                push(&mut spans, index, end, SyntaxToken::Function);
                index = end;
            } else {
                index += 1;
            }
        } else if ch == '{'
            && let Some(close) = svelte_expression_end(text, index)
        {
            let call = text[index + 1..].starts_with('!');
            let inner = &text[index + 1..close - 1];
            if call || !inner.contains('\n') {
                push(&mut spans, index, index + 1, SyntaxToken::Keyword);
                for span in go(inner) {
                    push(
                        &mut spans,
                        index + 1 + span.start,
                        index + 1 + span.end,
                        span.token,
                    );
                }
                push(&mut spans, close - 1, close, SyntaxToken::Keyword);
                index = close;
            } else {
                index += 1;
            }
        } else {
            index += ch.len_utf8();
        }
    }
    merge_spans(spans)
}

fn make_variable_end(text: &str, start: usize) -> Option<usize> {
    let open = text[start + 1..].chars().next()?;
    let close = match open {
        '(' => ')',
        '{' => '}',
        _ => return None,
    };
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            index += ch.len_utf8();
            if depth == 0 {
                return Some(index);
            }
            continue;
        }
        index += ch.len_utf8();
    }
    None
}

fn make_variable(text: &str, start: usize, spans: &mut Vec<Span>) -> Option<usize> {
    const FUNCTIONS: &[&str] = &[
        "abspath", "addprefix", "addsuffix", "and", "basename", "call", "dir", "error",
        "eval", "file", "filter", "filter-out", "findstring", "firstword", "flavor",
        "foreach", "guile", "if", "info", "join", "lastword", "notdir", "or", "origin",
        "patsubst", "realpath", "shell", "sort", "strip", "subst", "suffix", "value",
        "warning", "wildcard", "word", "wordlist",
    ];
    let end = make_variable_end(text, start)?;
    let content_start = start + 2;
    let content_end = end - 1;
    push(spans, start, content_start, SyntaxToken::Keyword);
    let mut cursor = content_start;
    while cursor < content_end
        && text[cursor..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
    }
    if cursor < content_end {
        let name_end = identifier_end_hyphen(text, cursor).min(content_end);
        let name = &text[cursor..name_end];
        let function = name_end < content_end
            && text[name_end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
        if function && FUNCTIONS.contains(&name) {
            push(spans, cursor, name_end, SyntaxToken::Function);
            make_scan_text(text, name_end..content_end, spans);
        } else {
            push(spans, content_start, content_end, SyntaxToken::Property);
        }
    }
    push(spans, content_end, end, SyntaxToken::Keyword);
    Some(end)
}

fn make_scan_text(text: &str, content: std::ops::Range<usize>, spans: &mut Vec<Span>) {
    let mut index = content.start;
    while index < content.end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '$' {
            if matches!(text[index + 1..].chars().next(), Some('(' | '{')) {
                if let Some(end) = make_variable(text, index, spans) {
                    index = end;
                    continue;
                }
            } else if text[index + 1..]
                .chars()
                .next()
                .is_some_and(|next| matches!(next, '@' | '<' | '^' | '?' | '*' | '%' | '+' | '|' | '$'))
            {
                push(spans, index, index + 2, SyntaxToken::Property);
                index += 2;
                continue;
            }
        } else if ch == '%' {
            push(spans, index, index + 1, SyntaxToken::String);
        }
        index += ch.len_utf8();
    }
}

fn makefile(text: &str) -> Vec<Span> {
    const DIRECTIVES: &[&str] = &[
        "-include", "define", "else", "endef", "endif", "export", "ifdef", "ifeq",
        "ifndef", "ifneq", "include", "override", "sinclude", "unexport", "vpath",
    ];
    let mut spans = Vec::new();
    let mut line_start = 0usize;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        if text[line_start..].starts_with('\t') {
            let mut cursor = line_start + 1;
            while cursor < line_end
                && text[cursor..]
                    .chars()
                    .next()
                    .is_some_and(|ch| matches!(ch, '@' | '-' | '+'))
            {
                push(&mut spans, cursor, cursor + 1, SyntaxToken::Keyword);
                cursor += 1;
            }
            make_scan_text(text, cursor..line_end, &mut spans);
            for span in bash(&text[cursor..line_end]) {
                push(
                    &mut spans,
                    cursor + span.start,
                    cursor + span.end,
                    span.token,
                );
            }
            line_start += line.len();
            continue;
        }
        let comment = text[line_start..line_end].find('#').map(|offset| line_start + offset);
        let code_end = comment.unwrap_or(line_end);
        if code_end > line_start {
            let leading = text[line_start..code_end]
                .len()
                .saturating_sub(text[line_start..code_end].trim_start().len());
            let code_start = line_start + leading;
            let word_end = text[code_start..code_end]
                .find(char::is_whitespace)
                .map_or(code_end, |offset| code_start + offset);
            let word = &text[code_start..word_end];
            if DIRECTIVES.contains(&word) {
                push(&mut spans, code_start, word_end, SyntaxToken::Keyword);
                make_scan_text(text, word_end..code_end, &mut spans);
            } else if let Some((name, operator, value)) =
                make_assignment(text, code_start, code_end)
            {
                push(&mut spans, name.start, name.end, SyntaxToken::Property);
                push(
                    &mut spans,
                    operator.start,
                    operator.end,
                    SyntaxToken::Keyword,
                );
                make_scan_text(text, value, &mut spans);
            } else if let Some(colon) = text[code_start..code_end].find(':') {
                let colon = code_start + colon;
                let target = &text[code_start..colon];
                let mut target_start = code_start;
                for part in target.split_whitespace() {
                    let offset = text[target_start..colon].find(part).unwrap_or(0);
                    let start = target_start + offset;
                    let end = start + part.len();
                    if part.starts_with('$') {
                        make_scan_text(text, start..end, &mut spans);
                    } else {
                        push(&mut spans, start, end, SyntaxToken::Function);
                    }
                    target_start = end;
                }
                push(&mut spans, colon, colon + 1, SyntaxToken::Keyword);
                make_scan_text(text, colon + 1..code_end, &mut spans);
            } else {
                make_scan_text(text, code_start..code_end, &mut spans);
            }
        }
        if let Some(comment) = comment {
            push(&mut spans, comment, line_end, SyntaxToken::Comment);
        }
        line_start += line.len();
    }
    merge_spans(spans)
}

fn make_assignment(
    text: &str,
    start: usize,
    end: usize,
) -> Option<(
    std::ops::Range<usize>,
    std::ops::Range<usize>,
    std::ops::Range<usize>,
)> {
    let equals = start + text[start..end].find('=')?;
    let operator_start = if equals > start
        && matches!(
            text[..equals].chars().next_back(),
            Some(':' | '?' | '+' | '!')
        )
    {
        equals - 1
    } else {
        equals
    };
    let name = text[start..operator_start].trim();
    if name.is_empty()
        || name.contains(char::is_whitespace)
        || name.contains(':')
    {
        return None;
    }
    let name_start = start + text[start..operator_start].find(name)?;
    let operator = operator_start..equals + 1;
    Some((name_start..name_start + name.len(), operator.clone(), operator.end..end))
}

fn twig_markup(
    text: &str,
    content: std::ops::Range<usize>,
    statement: bool,
    spans: &mut Vec<Span>,
) {
    const KEYWORDS: &[&str] = &[
        "and", "as", "autoescape", "block", "do", "else", "elseif", "embed",
        "endautoescape", "endblock", "endembed", "endfilter", "endfor", "endif",
        "endmacro", "endraw", "endsandbox", "endspaceless", "endtrans", "endverbatim",
        "extends", "filter", "flush", "for", "from", "if", "ignore", "import",
        "include", "in", "is", "macro", "matches", "missing", "not", "only", "or",
        "raw", "sandbox", "set", "spaceless", "starts", "trans", "types", "use",
        "verbatim", "with", "ends", "defined", "empty", "even", "iterable", "odd",
        "same",
    ];
    let mut index = content.start;
    while index < content.end
        && text[index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    if index < content.end && is_ident_start(text[index..].chars().next().unwrap_or('\0')) {
        let end = identifier_end(text, index).min(content.end);
        let word = &text[index..end];
        let token = if statement && KEYWORDS.contains(&word) {
            SyntaxToken::Keyword
        } else if matches!(word, "true" | "false" | "null" | "none") {
            SyntaxToken::Number
        } else if next_non_space(text, end) == Some('(') {
            SyntaxToken::Function
        } else {
            SyntaxToken::Property
        };
        if token == SyntaxToken::Property {
            let chain = identifier_end_markup(text, index).min(content.end);
            push(spans, index, chain, token);
            index = chain;
        } else {
            push(spans, index, end, token);
            index = end;
        }
    }
    while index < content.end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else if ch == '|' {
            index += 1;
            while index < content.end
                && text[index..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                index += text[index..].chars().next().map_or(1, char::len_utf8);
            }
            if index < content.end
                && is_ident_start(text[index..].chars().next().unwrap_or('\0'))
            {
                let end = identifier_end_markup(text, index).min(content.end);
                push(spans, index, end, SyntaxToken::Function);
                index = end;
            }
        } else if let Some(operator) =
            ["==", "!=", ">=", "<=", ">", "<"].iter().find(|operator| {
                text[index..content.end].starts_with(*operator)
            })
        {
            push(
                spans,
                index,
                index + operator.len(),
                SyntaxToken::Keyword,
            );
            index += operator.len();
        } else if matches!(ch, '"' | '\'') {
            let end = skip_quoted(text, index, ch, false).min(content.end);
            push(spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = c_number_end(text, index, false).min(content.end);
            push(spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end(text, index).min(content.end);
            let word = &text[index..end];
            if text[end..content.end].starts_with(':') {
                push(spans, index, end + 1, SyntaxToken::Attribute);
                index = end + 1;
            } else if matches!(word, "true" | "false" | "null" | "none") {
                push(spans, index, end, SyntaxToken::Number);
                index = end;
            } else if KEYWORDS.contains(&word) {
                push(spans, index, end, SyntaxToken::Keyword);
                index = end;
            } else if next_non_space(text, end) == Some('(') {
                push(spans, index, end, SyntaxToken::Function);
                index = end;
            } else {
                let chain = identifier_end_markup(text, index).min(content.end);
                push(spans, index, chain, SyntaxToken::Property);
                index = chain;
            }
        } else {
            index += ch.len_utf8();
        }
    }
}

fn twig(text: &str) -> Vec<Span> {
    let mut spans = html(text);
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("{#") {
            let end = text[index + 2..]
                .find("#}")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
            continue;
        }
        if text[index..].starts_with("{%") {
            let Some((content_start, content_end, end)) =
                liquid_tag_bounds(text, index, "%}")
            else {
                index += 2;
                continue;
            };
            let word = liquid_word(text, content_start..content_end);
            if matches!(word, "raw" | "verbatim")
                && let Some((end_open, end_after)) =
                    liquid_find_end_tag(text, end, &format!("end{word}"))
            {
                let word_start =
                    content_start + text[content_start..content_end].find(word).unwrap_or(0);
                push(&mut spans, index, content_start, SyntaxToken::Keyword);
                push(
                    &mut spans,
                    word_start,
                    word_start + word.len(),
                    SyntaxToken::Keyword,
                );
                push(&mut spans, end, end_open, SyntaxToken::String);
                liquid_push_closing_tag(text, end_open, end_after, &mut spans);
                index = end_after;
                continue;
            }
            push(&mut spans, index, content_start, SyntaxToken::Keyword);
            twig_markup(text, content_start..content_end, true, &mut spans);
            push(&mut spans, content_end, end, SyntaxToken::Keyword);
            index = end;
            continue;
        }
        if text[index..].starts_with("{{") {
            let Some((content_start, content_end, end)) =
                liquid_tag_bounds(text, index, "}}")
            else {
                index += 2;
                continue;
            };
            push(&mut spans, index, content_start, SyntaxToken::Keyword);
            twig_markup(text, content_start..content_end, false, &mut spans);
            push(&mut spans, content_end, end, SyntaxToken::Keyword);
            index = end;
            continue;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    merge_spans(spans)
}

fn pug_tag_name_end(text: &str, start: usize, end: usize) -> usize {
    let mut index = start;
    while index < end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_alphanumeric() || matches!(ch, '_' | '-') {
            index += ch.len_utf8();
        } else if ch == ':'
            && text[index + 1..end]
                .chars()
                .next()
                .is_some_and(|next| !next.is_whitespace())
        {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn pug_interpolations(text: &str, content: std::ops::Range<usize>, spans: &mut Vec<Span>) {
    let mut index = content.start;
    while index < content.end {
        if (text[index..].starts_with("#{") || text[index..].starts_with("!{"))
            && let Some(close) = svelte_expression_end(text, index + 1)
        {
            spans.push(Span {
                start: index,
                end: index + 2,
                token: SyntaxToken::Keyword,
            });
            for span in script(&text[index + 2..close - 1]) {
                push(
                    spans,
                    index + 2 + span.start,
                    index + 2 + span.end,
                    span.token,
                );
            }
            push(spans, close - 1, close, SyntaxToken::Keyword);
            index = close;
            continue;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
}

fn pug_attributes(text: &str, content: std::ops::Range<usize>, spans: &mut Vec<Span>) {
    let mut index = content.start;
    while index < content.end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() || ch == ',' {
            index += ch.len_utf8();
        } else if matches!(ch, '"' | '\'') {
            let end = skip_quoted(text, index, ch, false).min(content.end);
            push(spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = c_number_end(text, index, false).min(content.end);
            push(spans, index, end, SyntaxToken::Number);
            index = end;
        } else if is_ident_start(ch) {
            let end = identifier_end_markup(text, index).min(content.end);
            push(spans, index, end, SyntaxToken::Attribute);
            index = end;
            let mut probe = index;
            while probe < content.end
                && text[probe..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                probe += text[probe..].chars().next().map_or(1, char::len_utf8);
            }
            if text[probe..content.end].starts_with('=') {
                push(spans, probe, probe + 1, SyntaxToken::Keyword);
                probe += 1;
                while probe < content.end
                    && text[probe..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                }
                if probe < content.end
                    && matches!(text[probe..].chars().next(), Some('"' | '\''))
                {
                    let quote = text[probe..].chars().next().unwrap_or('\'');
                    let close = skip_quoted(text, probe, quote, false).min(content.end);
                    push(spans, probe, close, SyntaxToken::String);
                    index = close;
                } else {
                    let value_start = probe;
                    while probe < content.end
                        && !text[probe..]
                            .chars()
                            .next()
                            .is_some_and(|ch| matches!(ch, ',' | ')') || ch.is_whitespace())
                    {
                        probe += text[probe..].chars().next().map_or(1, char::len_utf8);
                    }
                    if probe > value_start {
                        let token = if text[value_start..]
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_digit())
                        {
                            SyntaxToken::Number
                        } else {
                            SyntaxToken::Property
                        };
                        push(spans, value_start, probe, token);
                    }
                    index = probe;
                }
            }
        } else if ch == '#' || ch == '!' {
            pug_interpolations(text, index..content.end, spans);
            break;
        } else {
            index += ch.len_utf8();
        }
    }
}

fn pug_line_text(text: &str, content: std::ops::Range<usize>, spans: &mut Vec<Span>) {
    pug_interpolations(text, content, spans);
}

fn pug_tag(text: &str, start: usize, end: usize, spans: &mut Vec<Span>) {
    let mut index = start;
    if text[index..end]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_alphabetic())
    {
        let name_end = pug_tag_name_end(text, index, end);
        if name_end > index {
            push(spans, index, name_end, SyntaxToken::Tag);
            index = name_end;
        }
    }
    while index < end {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if matches!(ch, '.' | '#') {
            let mut end_name = index + 1;
            while end_name < end {
                let next = text[end_name..].chars().next().unwrap_or('\0');
                if next.is_alphanumeric() || matches!(next, '_' | '-') {
                    end_name += next.len_utf8();
                } else {
                    break;
                }
            }
            if end_name > index + 1 {
                push(spans, index, end_name, SyntaxToken::Attribute);
                index = end_name;
                continue;
            }
        } else if ch == '('
            && let Some(close) = blade_paren_end(text, index)
        {
            push(spans, index, index + 1, SyntaxToken::Keyword);
            pug_attributes(text, index + 1..close - 1, spans);
            push(spans, close - 1, close, SyntaxToken::Keyword);
            index = close;
            continue;
        } else if ch == ':' {
            push(spans, index, index + 1, SyntaxToken::Keyword);
            index += 1;
            continue;
        } else {
            break;
        }
    }
    while index < end
        && text[index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
    if index >= end {
        return;
    }
    if text[index..end].starts_with("!=") || text[index..end].starts_with('=') {
        let marker_end = if text[index..end].starts_with("!=") {
            index + 2
        } else {
            index + 1
        };
        push(spans, index, marker_end, SyntaxToken::Keyword);
        for span in script(&text[marker_end..end]) {
            push(spans, marker_end + span.start, marker_end + span.end, span.token);
        }
    } else {
        pug_line_text(text, index..end, spans);
    }
}

fn pug(text: &str) -> Vec<Span> {
    const FLOW: &[&str] = &[
        "if", "else", "unless", "each", "for", "while", "case", "when", "default",
    ];
    const DIRECTIVES: &[&str] = &[
        "append", "block", "extends", "include", "prepend", "yield",
    ];
    let mut spans = Vec::new();
    let mut line_start = 0usize;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        let leading = text[line_start..line_end]
            .len()
            .saturating_sub(text[line_start..line_end].trim_start().len());
        let start = line_start + leading;
        let trimmed = &text[start..line_end];
        if trimmed.is_empty() {
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with("//") {
            push(&mut spans, start, line_end, SyntaxToken::Comment);
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with("<!--") {
            let end = text[start + 4..]
                .find("-->")
                .map_or(line_end, |offset| start + 4 + offset + 3);
            push(&mut spans, start, end, SyntaxToken::Comment);
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with("!!!") || trimmed.starts_with("doctype") {
            let word_end = start + trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            push(&mut spans, start, word_end, SyntaxToken::Keyword);
            if word_end < line_end {
                push(&mut spans, word_end, line_end, SyntaxToken::String);
            }
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with('+') {
            let name_end = identifier_end_hyphen(text, start + 1).min(line_end);
            push(&mut spans, start, start + 1, SyntaxToken::Keyword);
            if name_end > start + 1 {
                push(&mut spans, start + 1, name_end, SyntaxToken::Function);
                if text[name_end..].starts_with('(')
                    && let Some(close) = blade_paren_end(text, name_end)
                {
                    for span in script(&text[name_end + 1..close - 1]) {
                        push(
                            &mut spans,
                            name_end + 1 + span.start,
                            name_end + 1 + span.end,
                            span.token,
                        );
                    }
                }
            }
            line_start += line.len();
            continue;
        }
        let word_end = start + trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let word = &text[start..word_end];
        if word == "mixin" {
            push(&mut spans, start, word_end, SyntaxToken::Keyword);
            if let Some((position, next)) = next_non_space_index(text, word_end)
                && is_ident_start(next)
            {
                let name_end = identifier_end_hyphen(text, position).min(line_end);
                push(&mut spans, position, name_end, SyntaxToken::Function);
                if text[name_end..].starts_with('(')
                    && let Some(close) = blade_paren_end(text, name_end)
                {
                    for span in script(&text[name_end + 1..close - 1]) {
                        push(
                            &mut spans,
                            name_end + 1 + span.start,
                            name_end + 1 + span.end,
                            span.token,
                        );
                    }
                }
            }
            line_start += line.len();
            continue;
        }
        if DIRECTIVES.contains(&word) {
            push(&mut spans, start, word_end, SyntaxToken::Keyword);
            if let Some((path_start, _)) = next_non_space_index(text, word_end) {
                push(&mut spans, path_start, line_end, SyntaxToken::String);
            }
            line_start += line.len();
            continue;
        }
        if FLOW.contains(&word) {
            push(&mut spans, start, word_end, SyntaxToken::Keyword);
            for span in script(&text[word_end..line_end]) {
                push(&mut spans, word_end + span.start, word_end + span.end, span.token);
            }
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with('-') {
            push(&mut spans, start, start + 1, SyntaxToken::Keyword);
            for span in script(&text[start + 1..line_end]) {
                push(&mut spans, start + 1 + span.start, start + 1 + span.end, span.token);
            }
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with("!=") || trimmed.starts_with('=') {
            let marker_end = if trimmed.starts_with("!=") {
                start + 2
            } else {
                start + 1
            };
            push(&mut spans, start, marker_end, SyntaxToken::Keyword);
            for span in script(&text[marker_end..line_end]) {
                push(
                    &mut spans,
                    marker_end + span.start,
                    marker_end + span.end,
                    span.token,
                );
            }
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with('|') {
            push(&mut spans, start, start + 1, SyntaxToken::Keyword);
            pug_line_text(text, start + 1..line_end, &mut spans);
            line_start += line.len();
            continue;
        }
        pug_tag(text, start, line_end, &mut spans);
        line_start += line.len();
    }
    merge_spans(spans)
}

fn ini(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut line_start = 0usize;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        let leading = text[line_start..line_end]
            .len()
            .saturating_sub(text[line_start..line_end].trim_start().len());
        let start = line_start + leading;
        let trimmed = &text[start..line_end];
        if trimmed.is_empty() {
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            push(&mut spans, start, line_end, SyntaxToken::Comment);
            line_start += line.len();
            continue;
        }
        if trimmed.starts_with('[')
            && let Some(close) = trimmed.find(']')
        {
            let content_start = start + 1;
            let content_end = start + close;
            push(&mut spans, start, start + 1, SyntaxToken::Keyword);
            if content_end > content_start {
                push(
                    &mut spans,
                    content_start,
                    content_end,
                    SyntaxToken::Type,
                );
            }
            push(
                &mut spans,
                content_end,
                content_end + 1,
                SyntaxToken::Keyword,
            );
            line_start += line.len();
            continue;
        }
        if let Some(equals) = trimmed.find('=') {
            let key = trimmed[..equals].trim();
            if !key.is_empty() && !key.contains(char::is_whitespace) {
                let key_start = start + trimmed[..equals].find(key).unwrap_or(0);
                push(
                    &mut spans,
                    key_start,
                    key_start + key.len(),
                    SyntaxToken::Property,
                );
                let operator = start + equals;
                push(&mut spans, operator, operator + 1, SyntaxToken::Keyword);
                let mut index = operator + 1;
                while index < line_end {
                    let ch = text[index..].chars().next().unwrap_or('\0');
                    if matches!(ch, '"' | '\'') {
                        let end = skip_quoted(text, index, ch, false).min(line_end);
                        push(&mut spans, index, end, SyntaxToken::String);
                        index = end;
                    } else {
                        index += ch.len_utf8();
                    }
                }
            }
        }
        line_start += line.len();
    }
    merge_spans(spans)
}

fn pascal_string_end(text: &str, start: usize) -> usize {
    let mut index = start + 1;
    while index < text.len() {
        if text[index..].starts_with("''") {
            index += 2;
            continue;
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        index += ch.len_utf8();
        if ch == '\'' {
            return index;
        }
    }
    text.len()
}

fn pascal_chain_end(text: &str, start: usize) -> usize {
    let mut end = identifier_end(text, start);
    while text[end..].starts_with('.') {
        let next = identifier_end(text, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

fn pascal(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "absolute", "abstract", "add", "all", "and_then", "array", "asc", "asm",
        "assembler", "assert", "async", "attribute", "autoreleasepool", "await",
        "begin", "bindable", "block", "by", "case", "cdecl", "class", "concat",
        "const", "constref", "constructor", "contains", "copy", "cppdecl", "default",
        "delegate", "deprecated", "desc", "destructor", "distinct", "div", "do",
        "downto", "dynamic", "each", "else", "empty", "end", "ensure", "enum",
        "equals", "event", "except", "exit", "exports", "extension", "external",
        "far", "file", "finalization", "finalizer", "finally", "flags", "for",
        "forward", "from", "function", "future", "generic", "goto", "group", "has",
        "helper", "if", "implementation", "implements", "implies", "import", "in",
        "index", "inherited", "initialization", "inline", "interface", "interrupt",
        "into", "invariants", "is", "iterator", "join", "label", "lazy", "library",
        "lifetimestrategy", "locked", "locking", "loop", "mapped", "matching",
        "message", "method", "mod", "module", "name", "namespace", "near", "nested",
        "new", "nostackframe", "not", "notify", "nullable", "object", "of", "old",
        "oldfpccall", "on", "only", "operator", "optional", "or_else", "order",
        "otherwise", "out", "overload", "override", "package", "packed", "parallel",
        "params", "partial", "pascal", "pinned", "platform", "pow", "private",
        "procedure", "program", "property", "protected", "public", "published",
        "qualified", "queryable", "raise", "raises", "read", "readonly", "record",
        "reference", "register", "reintroduce", "remove", "repeat", "requires",
        "resident", "resourcestring", "restricted", "result", "reverse", "safecall",
        "sealed", "segment", "select", "selector", "sequence", "set", "shl", "shr",
        "skip", "specialize", "soft", "static", "stdcall", "step", "stored",
        "strict", "strong", "take", "then", "threadvar", "to", "try", "tuple",
        "type", "unconstrained", "unit", "unmanaged", "unretained", "unsafe",
        "until", "uses", "using", "var", "view", "virtual", "volatile", "weak",
        "where", "while", "with", "write", "xor", "yield",
    ];
    const TYPES: &[&str] = &[
        "ansichar", "ansistring", "boolean", "byte", "cardinal", "char", "comp",
        "currency", "double", "dword", "extended", "file", "integer", "int8",
        "int16", "int32", "int64", "intptr", "longint", "longword", "nativeint",
        "nativeuint", "olevariant", "pansichar", "pchar", "pwidechar", "pointer",
        "real", "shortint", "shortstring", "single", "smallint", "string", "uint8",
        "uint16", "uint32", "uint64", "uintptr", "variant", "widechar", "widestring",
        "word", "wordbool",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("(*") {
            let end = text[index + 2..]
                .find("*)")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("{$") {
            let end = text[index + 2..]
                .find('}')
                .map_or(text.len(), |offset| index + 2 + offset + 1);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if text[index..].starts_with('{') {
            let end = text[index + 1..]
                .find('}')
                .map_or(text.len(), |offset| index + 1 + offset + 1);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '\'' {
                let end = pascal_string_end(text, index);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '#' && text[index + 1..].chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                let mut end = index + 1;
                while end < text.len()
                    && text[end..].chars().next().is_some_and(|c| c.is_ascii_digit())
                {
                    end += text[end..].chars().next().map_or(1, char::len_utf8);
                }
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '$' {
                let mut end = index + 1;
                while end < text.len()
                    && text[end..].chars().next().is_some_and(|c| c.is_ascii_hexdigit())
                {
                    end += text[end..].chars().next().map_or(1, char::len_utf8);
                }
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Number);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit() {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if word.eq_ignore_ascii_case("true")
                    || word.eq_ignore_ascii_case("false")
                    || word.eq_ignore_ascii_case("nil")
                {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if word.eq_ignore_ascii_case("self")
                    || word.eq_ignore_ascii_case("result")
                {
                    push(&mut spans, index, end, SyntaxToken::Property);
                } else if keyword(word, KEYWORDS, true) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if matches!(
                        word.to_ascii_lowercase().as_str(),
                        "function" | "procedure" | "constructor" | "destructor" | "property"
                    ) && let Some((position, next)) = next_non_space_index(text, end)
                        && is_ident_start(next)
                    {
                        let name_end = pascal_chain_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Function);
                        index = name_end;
                        continue;
                    }
                } else if keyword(word, TYPES, true) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn mdx_in_skip(ranges: &[std::ops::Range<usize>], index: usize) -> Option<usize> {
    ranges
        .iter()
        .find(|range| range.start <= index && index < range.end)
        .map(|range| range.end)
}

fn mdx_frontmatter(
    text: &str,
) -> Option<(
    std::ops::Range<usize>,
    std::ops::Range<usize>,
    bool,
)> {
    let first_end = text.find('\n').unwrap_or(text.len());
    let marker = match text[..first_end].trim_end_matches('\r') {
        "---" => "---",
        "+++" => "+++",
        _ => return None,
    };
    let body_start = (first_end + 1).min(text.len());
    let mut cursor = body_start;
    loop {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset);
        if text[cursor..line_end].trim_end_matches('\r') == marker {
            return Some((0..line_end, body_start..cursor, marker == "+++"));
        }
        if line_end == text.len() {
            return None;
        }
        cursor = line_end + 1;
    }
}

fn mdx_fence_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut line_start = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        if let Some((marker, start)) = fence {
            let run = trimmed.chars().take_while(|ch| *ch == marker).count();
            if run >= 3 && trimmed[run..].trim().is_empty() {
                ranges.push(start..line_end);
                fence = None;
            }
        } else if let Some(marker) = trimmed
            .chars()
            .next()
            .filter(|ch| matches!(ch, '`' | '~'))
        {
            let run = trimmed.chars().take_while(|ch| *ch == marker).count();
            if run >= 3 {
                fence = Some((marker, line_start));
            }
        }
        line_start += line.len();
    }
    if let Some((_, start)) = fence {
        ranges.push(start..text.len());
    }
    ranges
}

fn mdx_esm_ranges(
    text: &str,
    skips: &[std::ops::Range<usize>],
) -> Vec<std::ops::Range<usize>> {
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut line_start = 0usize;
    let mut line_index = 0usize;
    while line_index < lines.len() {
        let line = lines[line_index];
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        if mdx_in_skip(skips, line_start).is_some() {
            line_start += line.len();
            line_index += 1;
            continue;
        }
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("import ") || lower.starts_with("export ") {
            let block_start = line_start;
            let mut block_end = line_end;
            let mut next_start = line_start + line.len();
            let mut next_index = line_index + 1;
            while next_index < lines.len() {
                let next = lines[next_index];
                if next.trim().is_empty() || mdx_in_skip(skips, next_start).is_some() {
                    break;
                }
                block_end = next_start + next.trim_end_matches(['\r', '\n']).len();
                next_start += next.len();
                next_index += 1;
            }
            ranges.push(block_start..block_end);
            line_start = next_start;
            line_index = next_index;
            continue;
        }
        line_start += line.len();
        line_index += 1;
    }
    ranges
}

fn mdx_jsx_spans(
    text: &str,
    skips: &[std::ops::Range<usize>],
    spans: &mut Vec<Span>,
) {
    let mut index = 0usize;
    while index < text.len() {
        if let Some(end) = mdx_in_skip(skips, index) {
            index = end;
            continue;
        }
        if text[index..].starts_with('<') {
            let rest = &text[index + 1..];
            let lower = rest.to_ascii_lowercase();
            let autolink = lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("mailto:");
            if !autolink
                && rest
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphabetic() || matches!(ch, '/' | '!'))
                && let Some(end) = markup_tag_end(text, index)
            {
                for span in html(&text[index..end]) {
                    spans.push(Span {
                        start: index + span.start,
                        end: index + span.end,
                        token: span.token,
                    });
                }
                index = end;
                continue;
            }
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
    }
}

fn mdx_expression_spans(
    text: &str,
    skips: &[std::ops::Range<usize>],
    spans: &mut Vec<Span>,
) {
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut index = 0usize;
    while index < text.len() {
        if let Some(end) = mdx_in_skip(skips, index) {
            index = end;
            continue;
        }
        let ch = masked[index..].chars().next().unwrap_or('\0');
        if ch == '{'
            && let Some(close) = svelte_expression_end(text, index)
        {
            push(spans, index, index + 1, SyntaxToken::Keyword);
            if index + 1 < close - 1 {
                for span in script(&text[index + 1..close - 1]) {
                    push(
                        spans,
                        index + 1 + span.start,
                        index + 1 + span.end,
                        span.token,
                    );
                }
            }
            push(spans, close - 1, close, SyntaxToken::Keyword);
            index = close;
            continue;
        }
        index += ch.len_utf8();
    }
}

fn mdx(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut skips = Vec::new();
    if let Some((full, body, toml_frontmatter)) = mdx_frontmatter(text) {
        let open_end = body.start.saturating_sub(1);
        push(&mut spans, 0, open_end, SyntaxToken::String);
        let nested = if toml_frontmatter {
            toml(&text[body.clone()])
        } else {
            yaml(&text[body.clone()])
        };
        for span in nested {
            push(
                &mut spans,
                body.start + span.start,
                body.start + span.end,
                span.token,
            );
        }
        if body.end < full.end {
            push(&mut spans, body.end, full.end, SyntaxToken::String);
        }
        skips.push(full);
    }
    skips.extend(mdx_fence_ranges(text));
    skips.sort_by_key(|range| range.start);
    for range in mdx_esm_ranges(text, &skips) {
        for span in script(&text[range.clone()]) {
            push(
                &mut spans,
                range.start + span.start,
                range.start + span.end,
                span.token,
            );
        }
        skips.push(range);
    }
    skips.sort_by_key(|range| range.start);
    mdx_jsx_spans(text, &skips, &mut spans);
    mdx_expression_spans(text, &skips, &mut spans);
    spans.extend(markdown(text));
    merge_spans(spans)
}

fn toml(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        let line_start = text[..index].rfind('\n').map_or(0, |offset| offset + 1);
        let at_line_start = text[line_start..index].trim().is_empty();
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch == '#' {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if at_line_start && ch == '[' {
            let close = text[index..]
                .find(']')
                .map_or(text.len(), |offset| index + offset);
            let inner_start = index + 1 + usize::from(text[index..].starts_with("[["));
            let inner_end = text[inner_start..close]
                .rfind(']')
                .map_or(close, |offset| inner_start + offset);
            push(&mut spans, inner_start, inner_end, SyntaxToken::Property);
            index = close + 1;
        } else if text[index..].starts_with("\"\"\"") {
            let end = skip_quoted(text, index, '"', true);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if text[index..].starts_with("'''") {
            let end = skip_quoted(text, index, '\'', true);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if matches!(ch, '"' | '\'') {
            let end = skip_quoted(text, index, ch, false);
            push(&mut spans, index, end, SyntaxToken::String);
            index = end;
        } else if ch.is_ascii_digit() {
            let end = c_number_end(text, index, false);
            push(&mut spans, index, end, SyntaxToken::Number);
            index = end;
        } else if toml_word_char(ch) {
            let end = toml_word_end(text, index);
            let word = &text[index..end];
            let next = next_non_space_index(text, end);
            if matches!(word, "true" | "false") {
                push(&mut spans, index, end, SyntaxToken::Number);
            } else if next.is_some_and(|(_, ch)| matches!(ch, '=' | '.')) {
                push(&mut spans, index, end, SyntaxToken::Property);
            }
            index = end;
        } else {
            index += ch.len_utf8();
        }
    }
    spans
}

fn dockerfile(text: &str) -> Vec<Span> {
    const INSTRUCTIONS: &[&str] = &[
        "ADD", "ARG", "AS", "CMD", "COPY", "ENTRYPOINT", "ENV", "EXPOSE", "FROM",
        "HEALTHCHECK", "LABEL", "MAINTAINER", "ONBUILD", "RUN", "SHELL", "STOPSIGNAL",
        "USER", "VOLUME", "WORKDIR",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        let line_start = text[..index].rfind('\n').map_or(0, |offset| offset + 1);
        let at_line_start = text[line_start..index].trim().is_empty();
        if text[index..].starts_with('#') && at_line_start {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '$' {
                let end = if text[index..].starts_with("${") {
                    text[index + 2..]
                        .find('}')
                        .map_or(text.len(), |offset| index + 2 + offset + 1)
                } else {
                    identifier_end(text, index + 1)
                };
                push(&mut spans, index, end, SyntaxToken::Property);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if (at_line_start || word.eq_ignore_ascii_case("as"))
                    && INSTRUCTIONS
                        .iter()
                        .any(|instruction| word.eq_ignore_ascii_case(instruction))
                {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn swift_raw_string_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    let mut hashes = 0usize;
    while text[index..].starts_with('#') {
        hashes += 1;
        index += 1;
    }
    if hashes == 0 || !text[index..].starts_with('"') {
        return None;
    }
    let triple = text[index..].starts_with("\"\"\"");
    index += if triple { 3 } else { 1 };
    let quote = if triple { "\"\"\"" } else { "\"" };
    let close = format!("{quote}{}", "#".repeat(hashes));
    text[index..]
        .find(&close)
        .map(|offset| index + offset + close.len())
        .or(Some(text.len()))
}

fn single_quote_end(text: &str, start: usize) -> (usize, SyntaxToken) {
    let rest = &text[start + 1..];
    let mut chars = rest.chars();
    if let Some(first) = chars.next() {
        if chars.next() == Some('\'') {
            return (start + 1 + first.len_utf8() + 1, SyntaxToken::String);
        }
        if first == '\\' {
            let end = skip_quoted(text, start, '\'', false);
            if end > start + 1 {
                return (end, SyntaxToken::String);
            }
        }
    }
    let end = identifier_end(text, start + 1);
    (end.max(start + 1), SyntaxToken::Property)
}

fn scala(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "case", "catch", "class", "def", "do", "else", "enum", "export",
        "extends", "extension", "false", "final", "finally", "for", "forSome", "given",
        "if", "implicit", "import", "lazy", "match", "new", "null", "object", "override",
        "package", "private", "protected", "return", "sealed", "super", "then", "this",
        "throw", "trait", "transparent", "true", "try", "type", "using", "val", "var",
        "while", "with", "yield",
    ];
    const TYPE_DECLS: &[&str] = &["class", "enum", "object", "trait", "type"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = rust_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\'' {
                let (end, token) = single_quote_end(text, index);
                push(&mut spans, index, end, token);
                index = end;
            } else if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    let mut end = end;
                    while text[end..].starts_with('.') {
                        let segment = identifier_end(text, end + 1);
                        if segment == end + 1 {
                            break;
                        }
                        end = segment;
                    }
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "def" || TYPE_DECLS.contains(&word))
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "def" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn dart(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "assert", "async", "await", "base", "break", "case", "catch",
        "class", "const", "continue", "covariant", "default", "deferred", "do", "else",
        "enum", "export", "extends", "extension", "external", "factory", "false", "final",
        "finally", "for", "get", "hide", "if", "implements", "import", "in", "interface",
        "is", "late", "library", "mixin", "new", "null", "on", "operator", "part",
        "required", "rethrow", "return", "sealed", "set", "show", "static", "super",
        "switch", "sync", "this", "throw", "true", "try", "typedef", "var", "when",
        "while", "with", "yield",
    ];
    const TYPES: &[&str] = &[
        "bool", "double", "dynamic", "int", "Never", "num", "Object", "String", "void",
        "Function", "Future", "Stream", "List", "Map", "Set",
    ];
    const TYPE_DECLS: &[&str] = &["class", "enum", "extension", "mixin", "typedef"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = rust_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if text[index..].starts_with("'''") {
                let end = skip_quoted(text, index, '\'', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    let mut end = end;
                    while text[end..].starts_with('.') {
                        let segment = identifier_end(text, end + 1);
                        if segment == end + 1 {
                            break;
                        }
                        end = segment;
                    }
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if TYPE_DECLS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word)
                    || word.chars().next().is_some_and(char::is_uppercase)
                {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn swift(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "actor", "any", "as", "associatedtype", "async", "await", "break", "case", "catch",
        "class", "continue", "convenience", "default", "defer", "deinit", "didSet", "do",
        "dynamic", "else", "enum", "extension", "fallthrough", "false", "fileprivate",
        "final", "for", "func", "guard", "if", "import", "in", "indirect", "infix", "init",
        "inout", "internal", "is", "lazy", "let", "mutating", "nil", "nonmutating", "open",
        "operator", "optional", "override", "postfix", "precedencegroup", "prefix", "private",
        "protocol", "public", "repeat", "required", "rethrows", "return", "self", "Self",
        "some", "static", "struct", "subscript", "super", "switch", "throw", "throws", "true",
        "try", "typealias", "unowned", "var", "weak", "where", "while", "willSet",
    ];
    const TYPE_DECLS: &[&str] = &["actor", "class", "enum", "extension", "protocol", "struct", "typealias"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = rust_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if let Some(end) = swift_raw_string_end(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    let mut end = end;
                    while text[end..].starts_with('.') {
                        let segment = identifier_end(text, end + 1);
                        if segment == end + 1 {
                            break;
                        }
                        end = segment;
                    }
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '#' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    index = end;
                } else {
                    index += 1;
                }
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "func" || TYPE_DECLS.contains(&word))
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "func" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn kotlin(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "actual", "annotation", "as", "break", "by", "catch", "class",
        "companion", "const", "constructor", "context", "continue", "crossinline", "data",
        "delegate", "do", "dynamic", "else", "enum", "expect", "external", "field", "file",
        "final", "finally", "for", "fun", "get", "if", "import", "in", "infix", "init",
        "inline", "inner", "interface", "internal", "is", "lateinit", "noinline", "object",
        "open", "operator", "out", "override", "package", "param", "private", "property",
        "protected", "public", "receiver", "reified", "return", "sealed", "set", "setparam",
        "super", "suspend", "tailrec", "this", "throw", "try", "typealias", "typeof", "val",
        "value", "var", "vararg", "when", "where", "while",
    ];
    const TYPE_DECLS: &[&str] = &["class", "interface", "object", "typealias"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = rust_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    let mut end = end;
                    while text[end..].starts_with('.') {
                        let segment = identifier_end(text, end + 1);
                        if segment == end + 1 {
                            break;
                        }
                        end = segment;
                    }
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = c_number_end(text, index, false);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "fun" || TYPE_DECLS.contains(&word))
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "fun" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn java_number_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index;
    if text[start..].starts_with("0x") || text[start..].starts_with("0X") {
        index = start + 2;
        while index < text.len()
            && (bytes[index].is_ascii_hexdigit()
                || bytes[index] == b'_'
                || bytes[index] == b'.')
        {
            index += 1;
        }
        if index < text.len() && matches!(bytes[index], b'p' | b'P') {
            let mut exponent = index + 1;
            if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
                exponent += 1;
            }
            let digits = exponent;
            while exponent < text.len()
                && (bytes[exponent].is_ascii_digit() || bytes[exponent] == b'_')
            {
                exponent += 1;
            }
            if exponent > digits {
                index = exponent;
            }
        }
    } else if text[start..].starts_with("0b") || text[start..].starts_with("0B") {
        index = start + 2;
        while index < text.len() && (matches!(bytes[index], b'0' | b'1') || bytes[index] == b'_')
        {
            index += 1;
        }
    } else {
        index = start;
        while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
            index += 1;
        }
        if index + 1 < text.len()
            && bytes[index] == b'.'
            && (bytes[index + 1].is_ascii_digit() || bytes[index + 1] == b'_')
        {
            index += 1;
            while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
                index += 1;
            }
        }
        if index < text.len() && matches!(bytes[index], b'e' | b'E') {
            let mut exponent = index + 1;
            if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
                exponent += 1;
            }
            let digits = exponent;
            while exponent < text.len()
                && (bytes[exponent].is_ascii_digit() || bytes[exponent] == b'_')
            {
                exponent += 1;
            }
            if exponent > digits {
                index = exponent;
            }
        }
    }
    while index < text.len()
        && matches!(
            bytes[index],
            b'f' | b'F' | b'd' | b'D' | b'l' | b'L' | b'_' | b'i' | b'I'
        )
    {
        index += 1;
    }
    index
}

fn java(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "assert", "break", "case", "catch", "class", "const", "continue",
        "default", "do", "else", "enum", "exports", "extends", "final", "finally", "for",
        "goto", "if", "implements", "import", "instanceof", "interface", "module", "native",
        "new", "non-sealed", "open", "opens", "package", "permits", "private", "protected",
        "provides", "public", "record", "requires", "return", "sealed", "static", "strictfp",
        "super", "switch", "synchronized", "this", "throw", "throws", "to", "transient",
        "try", "uses", "var", "volatile", "while", "with", "yield",
    ];
    const TYPES: &[&str] = &[
        "boolean", "byte", "char", "double", "float", "int", "long", "short", "void",
    ];
    const TYPE_DECLS: &[&str] = &["class", "enum", "interface", "record"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '@' {
                let end = identifier_end(text, index + 1);
                if end > index + 1 {
                    let mut end = end;
                    while text[end..].starts_with('.') {
                        let segment = identifier_end(text, end + 1);
                        if segment == end + 1 {
                            break;
                        }
                        end = segment;
                    }
                    push(&mut spans, index, end, SyntaxToken::Attribute);
                    index = end;
                } else {
                    index += 1;
                }
            } else if text[index..].starts_with("\"\"\"") {
                let end = skip_quoted(text, index, '"', true);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = java_number_end(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "null") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if TYPE_DECLS.contains(&word)
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(&mut spans, position, name_end, SyntaxToken::Type);
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word)
                    || word.chars().next().is_some_and(char::is_uppercase)
                {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('(') {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn go_raw_string_end(text: &str, start: usize) -> usize {
    text[start + 1..]
        .find('`')
        .map_or(text.len(), |offset| start + 1 + offset + 1)
}

fn go_number_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index;
    if text[start..].starts_with("0x") {
        index = start + 2;
        while index < text.len()
            && (bytes[index].is_ascii_hexdigit()
                || bytes[index] == b'_'
                || bytes[index] == b'.')
        {
            index += 1;
        }
    } else if text[start..].starts_with("0o") || text[start..].starts_with("0b") {
        index = start + 2;
        while index < text.len()
            && (bytes[index].is_ascii_digit() || bytes[index] == b'_')
        {
            index += 1;
        }
    } else {
        index = start;
        while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
            index += 1;
        }
        if index + 1 < text.len()
            && bytes[index] == b'.'
            && (bytes[index + 1].is_ascii_digit() || bytes[index + 1] == b'_')
        {
            index += 1;
            while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
                index += 1;
            }
        }
    }
    if index < text.len() && matches!(bytes[index], b'e' | b'E' | b'p' | b'P') {
        let mut exponent = index + 1;
        if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
            exponent += 1;
        }
        let digits = exponent;
        while exponent < text.len()
            && (bytes[exponent].is_ascii_digit() || bytes[exponent] == b'_')
        {
            exponent += 1;
        }
        if exponent > digits {
            index = exponent;
        }
    }
    while index < text.len()
        && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
    {
        index += 1;
    }
    index
}

fn go(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "break", "case", "chan", "const", "continue", "default", "defer", "else",
        "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
        "package", "range", "return", "select", "struct", "switch", "type", "var",
    ];
    const TYPES: &[&str] = &[
        "any", "bool", "byte", "complex64", "complex128", "error", "float32", "float64",
        "int", "int8", "int16", "int32", "int64", "rune", "string", "uint", "uint8",
        "uint16", "uint32", "uint64", "uintptr",
    ];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if ch == '`' {
                let end = go_raw_string_end(text, index);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if matches!(ch, '"' | '\'') {
                let end = skip_quoted(text, index, ch, false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && text[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()))
            {
                let end = go_number_end(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false" | "nil" | "iota") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "func" || word == "type")
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "func" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else {
                    let next = next_non_space_index(text, end);
                    let declaration = next.is_some_and(|(position, ch)| {
                        is_ident_start(ch) && {
                            let name_end = identifier_end(text, position);
                            matches!(&text[position..name_end], "struct" | "interface")
                        }
                    });
                    if next.is_some_and(|(_, ch)| ch == '(')
                        || declaration
                    {
                        push(
                            &mut spans,
                            index,
                            end,
                            if declaration {
                                SyntaxToken::Type
                            } else {
                                SyntaxToken::Function
                            },
                        );
                    }
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn rust_raw_string_end(text: &str, start: usize, prefix: usize) -> Option<usize> {
    let mut index = start + prefix;
    let mut hashes = 0usize;
    while text[index..].starts_with('#') {
        hashes += 1;
        index += 1;
    }
    if !text[index..].starts_with('"') {
        return None;
    }
    index += 1;
    while let Some(offset) = text[index..].find('"') {
        let quote = index + offset;
        let mut end = quote + 1;
        let mut matched = true;
        for _ in 0..hashes {
            if text.as_bytes().get(end) == Some(&b'#') {
                end += 1;
            } else {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(end);
        }
        index = quote + 1;
    }
    Some(text.len())
}

fn rust_raw_string_at(text: &str, index: usize) -> Option<usize> {
    let prefix = if text[index..].starts_with("br") {
        2
    } else if text[index..].starts_with('r') {
        1
    } else {
        return None;
    };
    rust_raw_string_end(text, index, prefix)
}

fn rust_char_end(text: &str, quote: usize) -> Option<usize> {
    let mut index = quote + 1;
    while index < text.len() {
        let ch = text[index..].chars().next()?;
        if ch == '\\' {
            index += 1;
            if let Some(escaped) = text[index..].chars().next() {
                index += escaped.len_utf8();
            }
            continue;
        }
        if ch == '\'' {
            return Some(index + 1);
        }
        if ch == '\n' || ch == '\r' {
            return None;
        }
        index += ch.len_utf8();
    }
    None
}

fn rust_block_comment_end(text: &str, start: usize) -> usize {
    let mut depth = 1usize;
    let mut index = start + 2;
    while index < text.len() {
        if text[index..].starts_with("/*") {
            depth += 1;
            index += 2;
        } else if text[index..].starts_with("*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

fn rust_number_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    if text[start..].starts_with("0x")
        || text[start..].starts_with("0o")
        || text[start..].starts_with("0b")
    {
        let mut index = start + 2;
        while index < text.len()
            && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        return index;
    }
    let mut index = start;
    while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
        index += 1;
    }
    if index + 1 < text.len()
        && bytes[index] == b'.'
        && (bytes[index + 1].is_ascii_digit() || bytes[index + 1] == b'_')
    {
        index += 1;
        while index < text.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
            index += 1;
        }
    }
    if index < text.len() && matches!(bytes[index], b'e' | b'E') {
        let mut exponent = index + 1;
        if exponent < text.len() && matches!(bytes[exponent], b'+' | b'-') {
            exponent += 1;
        }
        let digits = exponent;
        while exponent < text.len()
            && (bytes[exponent].is_ascii_digit() || bytes[exponent] == b'_')
        {
            exponent += 1;
        }
        if exponent > digits {
            index = exponent;
        }
    }
    while index < text.len()
        && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
    {
        index += 1;
    }
    index
}

fn rust(text: &str) -> Vec<Span> {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
        "crate", "do", "dyn", "else", "enum", "extern", "final", "fn", "for", "gen", "if",
        "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override",
        "priv", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "try", "type", "typeof", "union", "unsafe", "unsized", "use", "virtual", "where",
        "while", "yield",
    ];
    const TYPES: &[&str] = &[
        "bool", "char", "str", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize",
        "u8", "u16", "u32", "u64", "u128", "usize",
    ];
    const TYPE_DECLS: &[&str] = &["enum", "mod", "struct", "trait", "type", "union"];
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("/*") {
            let end = rust_block_comment_end(text, index);
            push(&mut spans, index, end, SyntaxToken::Comment);
            index = end;
        } else if text[index..].starts_with("#[") || text[index..].starts_with("#![") {
            let end = text[index..]
                .find(']')
                .map_or_else(
                    || {
                        text[index..]
                            .find('\n')
                            .map_or(text.len(), |offset| index + offset)
                    },
                    |offset| index + offset + 1,
                );
            push(&mut spans, index, end, SyntaxToken::Attribute);
            index = end;
        } else {
            let ch = text[index..].chars().next().unwrap_or('\0');
            if let Some(end) = rust_raw_string_at(text, index) {
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if text[index..].starts_with("b\"") {
                let end = skip_quoted(text, index + 1, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if text[index..].starts_with("b'") {
                if let Some(end) = rust_char_end(text, index + 1) {
                    push(&mut spans, index, end, SyntaxToken::String);
                    index = end;
                } else {
                    index += 2;
                }
            } else if ch == '"' {
                let end = skip_quoted(text, index, '"', false);
                push(&mut spans, index, end, SyntaxToken::String);
                index = end;
            } else if ch == '\'' {
                if let Some(end) = rust_char_end(text, index) {
                    push(&mut spans, index, end, SyntaxToken::String);
                    index = end;
                } else {
                    let end = identifier_end(text, index + 1);
                    if end > index + 1 {
                        push(&mut spans, index, end, SyntaxToken::Type);
                        index = end;
                    } else {
                        index += 1;
                    }
                }
            } else if ch.is_ascii_digit() {
                let end = rust_number_end(text, index);
                push(&mut spans, index, end, SyntaxToken::Number);
                index = end;
            } else if is_ident_start(ch) {
                let end = identifier_end(text, index);
                let word = &text[index..end];
                if matches!(word, "true" | "false") {
                    push(&mut spans, index, end, SyntaxToken::Number);
                } else if KEYWORDS.contains(&word) {
                    push(&mut spans, index, end, SyntaxToken::Keyword);
                    if (word == "fn" || TYPE_DECLS.contains(&word))
                        && let Some((position, ch)) = next_non_space_index(text, end)
                        && is_ident_start(ch)
                    {
                        let name_end = identifier_end(text, position);
                        push(
                            &mut spans,
                            position,
                            name_end,
                            if word == "fn" {
                                SyntaxToken::Function
                            } else {
                                SyntaxToken::Type
                            },
                        );
                        index = name_end;
                        continue;
                    }
                } else if TYPES.contains(&word)
                    || word.chars().next().is_some_and(char::is_uppercase)
                {
                    push(&mut spans, index, end, SyntaxToken::Type);
                } else if next_non_space(text, end) == Some('!')
                    || next_non_space(text, end) == Some('(')
                {
                    push(&mut spans, index, end, SyntaxToken::Function);
                }
                index = end;
            } else {
                index += ch.len_utf8();
            }
        }
    }
    spans
}

fn diff_metadata(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("diff ")
        || lower.starts_with("index ")
        || lower.starts_with("new file mode")
        || lower.starts_with("deleted file mode")
        || lower.starts_with("old mode")
        || lower.starts_with("new mode")
        || lower.starts_with("similarity index")
        || lower.starts_with("dissimilarity index")
        || lower.starts_with("rename from")
        || lower.starts_with("rename to")
        || lower.starts_with("copy from")
        || lower.starts_with("copy to")
        || lower.starts_with("binary files")
        || lower.starts_with("\\ no newline at end of file")
}

fn diff(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n');
        let token = if diff_metadata(trimmed) {
            Some(SyntaxToken::Comment)
        } else if trimmed.starts_with("@@") || trimmed.starts_with("+++") || trimmed.starts_with("---") {
            Some(SyntaxToken::Keyword)
        } else if trimmed.starts_with('+') {
            Some(SyntaxToken::Added)
        } else if trimmed.starts_with('-') {
            Some(SyntaxToken::Removed)
        } else {
            None
        };
        if let Some(token) = token {
            push(&mut spans, start, start + trimmed.len(), token);
        }
        start += line.len();
    }
    spans
}

fn markdown(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut fence: Option<char> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let line_start = start;
        start += line.len();
        if let Some(marker) = fence {
            let run = trimmed.chars().take_while(|ch| *ch == marker).count();
            if run >= 3 && trimmed[run..].trim().is_empty() {
                push(&mut spans, line_start + indent, line_start + indent + run, SyntaxToken::Keyword);
                fence = None;
            }
            continue;
        }
        let marker = trimmed.chars().next().unwrap_or('\0');
        if matches!(marker, '`' | '~') {
            let run = trimmed.chars().take_while(|ch| *ch == marker).count();
            if run >= 3 {
                let info_end = trimmed
                    .find(char::is_whitespace)
                    .unwrap_or(trimmed.trim_end().len());
                push(&mut spans, line_start + indent, line_start + indent + info_end, SyntaxToken::Keyword);
                fence = Some(marker);
                continue;
            }
        }
        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
            if trimmed[hashes..].starts_with(' ') || trimmed[hashes..].trim().is_empty() {
                push(&mut spans, line_start + indent, line_start + indent + hashes, SyntaxToken::Keyword);
            }
        }
        let content = trimmed.trim_end_matches(['\r', '\n']);
        if content.len() >= 3
            && content.chars().all(|ch| matches!(ch, '-' | '*' | '_'))
            && content.chars().next().is_some_and(|ch| content.chars().all(|other| other == ch))
        {
            push(&mut spans, line_start + indent, line_start + indent + content.len(), SyntaxToken::Keyword);
            continue;
        }
        let mut cursor = 0usize;
        let mut plain = 0usize;
        if let Some(rest) = trimmed.strip_prefix("> ") {
            push(&mut spans, line_start + indent, line_start + indent + 1, SyntaxToken::Keyword);
            plain = trimmed.len() - rest.len();
            cursor = plain;
        } else {
            let mut marker_end = 0usize;
            let first = trimmed.chars().next().unwrap_or('\0');
            if matches!(first, '-' | '*' | '+') && trimmed[1..].starts_with(' ') {
                marker_end = 1;
            } else {
                let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
                if digits > 0
                    && trimmed[digits..].starts_with(['.', ')'])
                    && trimmed[digits + 1..].starts_with(' ')
                {
                    marker_end = digits + 1;
                }
            }
            if marker_end > 0 {
                push(&mut spans, line_start + indent, line_start + indent + marker_end, SyntaxToken::Keyword);
                if let Some(rest) = trimmed[marker_end..].strip_prefix(' ')
                    && (rest.starts_with("[ ] ") || rest.starts_with("[x] ") || rest.starts_with("[X] "))
                {
                    let task_start = line_start + indent + marker_end + 1;
                    push(&mut spans, task_start, task_start + 3, SyntaxToken::Keyword);
                }
                plain = marker_end;
                cursor = plain;
            }
        }
        while cursor < trimmed.len() {
            if trimmed[cursor..].starts_with("**") || trimmed[cursor..].starts_with("__") {
                push(&mut spans, line_start + indent + cursor, line_start + indent + cursor + 2, SyntaxToken::Keyword);
                cursor += 2;
                continue;
            }
            if trimmed[cursor..].starts_with("~~") {
                push(&mut spans, line_start + indent + cursor, line_start + indent + cursor + 2, SyntaxToken::Keyword);
                cursor += 2;
                continue;
            }
            let emphasis = trimmed[cursor..].chars().next().unwrap_or('\0');
            if emphasis == '*'
                || (emphasis == '_'
                    && !trimmed[..cursor]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_alphanumeric))
            {
                push(&mut spans, line_start + indent + cursor, line_start + indent + cursor + 1, SyntaxToken::Keyword);
                cursor += 1;
                continue;
            }
            if trimmed[cursor..].starts_with('<')
                && trimmed[cursor + 1..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphabetic() || matches!(ch, '/' | '!'))
                && let Some(offset) = trimmed[cursor..].find('>')
            {
                let end = cursor + offset + 1;
                for span in html(&trimmed[cursor..end]) {
                    push(
                        &mut spans,
                        line_start + indent + cursor + span.start,
                        line_start + indent + cursor + span.end,
                        span.token,
                    );
                }
                cursor = end;
                continue;
            }
            if trimmed[cursor..].starts_with('`') {
                let ticks = trimmed[cursor..].chars().take_while(|ch| *ch == '`').count();
                let marker = "`".repeat(ticks);
                if let Some(offset) = trimmed[cursor + ticks..].find(&marker) {
                    let end = cursor + ticks + offset + ticks;
                    push(&mut spans, line_start + indent + cursor, line_start + indent + end, SyntaxToken::String);
                    cursor = end;
                    continue;
                }
            }
            if trimmed[cursor..].starts_with('[')
                && !trimmed[..cursor].ends_with('!')
                && let Some(close) = trimmed[cursor + 1..].find(']')
            {
                let close = cursor + 1 + close;
                if trimmed[close + 1..].starts_with('(')
                    && let Some(end) = trimmed[close + 2..].find(')')
                {
                    let end = close + 2 + end + 1;
                    push(&mut spans, line_start + indent + cursor, line_start + indent + close + 1, SyntaxToken::Function);
                    push(&mut spans, line_start + indent + close + 1, line_start + indent + end, SyntaxToken::String);
                    cursor = end;
                    continue;
                }
            }
            cursor += trimmed[cursor..].chars().next().map_or(1, char::len_utf8);
        }
        let _ = plain;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens<'a>(language: &str, text: &'a str) -> Vec<(SyntaxToken, &'a str)> {
        highlight(language, text)
            .into_iter()
            .map(|span| (span.token, &text[span.start..span.end]))
            .collect()
    }

    fn assert_well_formed(language: &str, text: &str) {
        let spans = highlight(language, text);
        let mut previous = 0;
        for span in spans {
            assert!(span.start >= previous, "{language}: overlapping spans");
            assert!(text.is_char_boundary(span.start));
            assert!(text.is_char_boundary(span.end));
            previous = span.end;
        }
    }

    #[test]
    fn aliases_cover_source_languages() {
        assert_eq!(language("js"), Some("javascript"));
        assert_eq!(language("TS extra"), Some("typescript"));
        assert_eq!(language(" sh "), Some("bash"));
        assert_eq!(language("yml"), Some("yaml"));
        assert_eq!(language("scss"), Some("css"));
        assert_eq!(language("unknown"), None);
    }

    #[test]
    fn inline_code_kind_matches_source_path_and_url_rules() {
        use InlineCodeKind::{Path, Url};
        for (text, expected) in [
            ("https://example.com/docs", Some(Url)),
            ("http://localhost:5173", Some(Url)),
            ("HTTPS://EXAMPLE.COM/docs", Some(Url)),
            ("src/main.rs", Some(Path)),
            ("./crates/vela-ui", Some(Path)),
            ("../vela-native/Cargo.toml", Some(Path)),
            ("Cargo.toml", Some(Path)),
            ("README", None),
            ("LICENSE", Some(Path)),
            (".gitignore", Some(Path)),
            ("Dockerfile.dev", Some(Path)),
            ("main.d.ts", Some(Path)),
            ("component.tsx", Some(Path)),
            ("query.graphql", Some(Path)),
            ("archive.tar.gz", None),
            ("C:\\Users\\vela\\main.go", Some(Path)),
            ("docs/", Some(Path)),
            ("hello", None),
            ("1.2", None),
            ("1.2.3", None),
            ("https", None),
            ("mailto:user@example.com", Some(Path)),
            ("/api", None),
            ("/", None),
            ("file:///tmp/x", None),
            ("note://open", None),
            ("vela://session", None),
            ("foo(bar)", None),
            ("foo(bar).rs", None),
            ("two words", None),
            ("two words.rs", None),
        ] {
            assert_eq!(inline_code_kind(text), expected, "{text}");
        }
    }

    #[test]
    fn json_marks_keys_strings_numbers_and_literals() {
        let text = r#"{"name": "Vela", "count": 2, "ok": true}"#;
        assert_eq!(
            tokens("json", text),
            vec![
                (SyntaxToken::Property, "\"name\""),
                (SyntaxToken::String, "\"Vela\""),
                (SyntaxToken::Property, "\"count\""),
                (SyntaxToken::Number, "2"),
                (SyntaxToken::Property, "\"ok\""),
                (SyntaxToken::Keyword, "true"),
            ]
        );
        assert_well_formed("json", text);
    }

    #[test]
    fn script_marks_comments_keywords_strings_numbers_and_functions() {
        let text = "const run = (count: number) => {\n  // note\n  return `value ${count}`;\n};";
        let tokens = tokens("typescript", text);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "const")));
        assert!(tokens.contains(&(SyntaxToken::Function, "run")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "number")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "// note")));
        assert!(tokens.iter().any(|(kind, text)| *kind == SyntaxToken::String && text.starts_with('`')));
        assert!(tokens.iter().filter(|(kind, _)| *kind == SyntaxToken::Keyword).count() >= 3);
        assert_well_formed("typescript", text);
    }

    #[test]
    fn html_doctype_cdata_and_processing_instructions_are_tokenized() {
        let source = "<!DOCTYPE html>\n<?xml version=\"1.0\"?>\n<root><![CDATA[if (a < b) & c]]></root>\n<!-- note --> &amp;";
        let tokens = tokens("html", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "<!DOCTYPE html>")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "<?xml version=\"1.0\"?>")));
        assert!(tokens.contains(&(SyntaxToken::String, "<![CDATA[if (a < b) & c]]>")));
        assert!(tokens.contains(&(SyntaxToken::Tag, "root")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "<!-- note -->")));
        assert!(tokens.contains(&(SyntaxToken::String, "&amp;")));
        assert!(
            !tokens.iter().any(|(_, text)| *text == "html" || *text == "xml"),
            "declaration internals are not re-tokenized as attributes"
        );
        assert_well_formed("html", source);
    }

    #[test]
    fn python_triple_quoted_and_prefixed_fstrings_tokenize_whole_strings() {
        let source = "doc = r\"\"\"line \\\"quote\"\nsecond\"\"\"\nmsg = f\"\"\"hi {name} and {1 + 2}\nbye\"\"\"\nplain = b'bytes'";
        let tokens = tokens("python", source);
        assert!(
            tokens.contains(&(
                SyntaxToken::String,
                "r\"\"\"line \\\"quote\"\nsecond\"\"\""
            )),
            "prefixed triple strings stay one string span: {tokens:?}"
        );
        assert!(tokens.contains(&(SyntaxToken::String, "f\"\"\"hi ")));
        assert!(tokens.contains(&(SyntaxToken::String, "\nbye\"\"\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1")));
        assert!(tokens.contains(&(SyntaxToken::Number, "2")));
        assert!(tokens.contains(&(SyntaxToken::String, "b'bytes'")));
        assert_well_formed("python", source);
    }

    #[test]
    fn php_tags_keywords_types_variables_attributes_strings_and_numbers() {
        let source = r##"<?php
declare(strict_types=1);

namespace App;

#[Attribute]
final class Greeter {
    public function __construct(private string $name) {}

    public function greet(bool $loud = false): string {
        // comment
        # hash comment
        $message = "hello {$this->name}";
        $label = 'x';
        $text = <<<TEXT
        hello
        TEXT;
        $count = 0x1f + 0b1010 + 1_000 + 1.5e3;
        $words = ['a', 'b'];
        echo strlen($message);
        return $loud ? strtoupper($message) : $message;
    }
}"##;
        assert_eq!(language("php"), Some("php"));
        let tokens = tokens("php", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "<?php")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "declare")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "namespace")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "final")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "public")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "echo")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "#[Attribute]")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Type, "string")));
        assert!(tokens.contains(&(SyntaxToken::Type, "bool")));
        assert!(tokens.contains(&(SyntaxToken::Function, "__construct")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet")));
        assert!(tokens.contains(&(SyntaxToken::Function, "strlen")));
        assert!(tokens.contains(&(SyntaxToken::Function, "strtoupper")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$name")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$message")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$label")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$count")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$words")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "# hash comment")));
        assert!(tokens.contains(&(
            SyntaxToken::String,
            "\"hello {$this->name}\""
        )));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::String, "'a'")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("<<<TEXT")
                && text.contains("hello")
                && text.trim_end().ends_with("TEXT")
        }));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "false")));
        assert_well_formed("php", source);
    }

    #[test]
    fn ruby_keywords_types_methods_symbols_percent_literals_heredocs_and_numbers() {
        let source = r##"=begin
block comment
=end
# comment
require "json"

module Demo
  class Greeter
    attr_reader :name

    def initialize(name)
      @name = name
    end

    def greet?(loud = false)
      message = "hello #{@name}"
      label = 'x'
      words = %w[a b c]
      text = <<~TEXT
        hello
      TEXT
      count = 0x1f + 0b1010 + 1_000 + 1.5e3
      puts message if loud
      defined?(name)
      :done
    end
  end
end"##;
        assert_eq!(language("ruby"), Some("ruby"));
        assert_eq!(language("rb"), Some("ruby"));
        let tokens = tokens("ruby", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("=begin") && text.contains("=end")
        }));
        assert!(tokens.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(tokens.contains(&(SyntaxToken::Function, "require")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "module")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "def")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "defined?")));
        assert!(tokens.contains(&(SyntaxToken::Function, "initialize")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet?")));
        assert!(tokens.contains(&(SyntaxToken::Function, "puts")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Demo")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Property, ":name")));
        assert!(tokens.contains(&(SyntaxToken::Property, ":done")));
        assert!(tokens.contains(&(SyntaxToken::Property, "@name")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"json\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"hello #{@name}\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::String, "%w[a b c]")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("<<~TEXT")
                && text.contains("hello")
                && text.trim_end().ends_with("TEXT")
        }));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "false")));
        assert_well_formed("ruby", source);
    }

    #[test]
    fn c_preprocessor_keywords_types_functions_strings_and_numbers_are_tokenized() {
        let source = r##"#include <stdio.h>
#define MAX(a, b) ((a) > (b) ? (a) : (b))

// comment
struct Point { int x; int y; };

static int add(int a, int b) {
    const char *name = "point";
    char mark = '\n';
    double ratio = 1.5e3;
    unsigned long mask = 0xFFu + 0b1010 + 0777;
    return a + b;
}"##;
        assert_eq!(language("c"), Some("c"));
        assert_eq!(language("h"), Some("c"));
        let tokens = tokens("c", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#include")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#define")));
        assert!(tokens.contains(&(SyntaxToken::String, "<stdio.h>")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "static")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "struct")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "const")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Point")));
        assert!(tokens.contains(&(SyntaxToken::Type, "int")));
        assert!(tokens.contains(&(SyntaxToken::Type, "double")));
        assert!(tokens.contains(&(SyntaxToken::Type, "unsigned")));
        assert!(tokens.contains(&(SyntaxToken::Function, "add")));
        assert!(tokens.contains(&(SyntaxToken::Function, "MAX")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"point\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'\\n'")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0xFFu")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0777")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert_well_formed("c", source);
    }

    #[test]
    fn cpp_namespaces_templates_classes_attributes_raw_strings_and_digit_separators() {
        let source = r##"#include <vector>

namespace demo {
template <typename T>
class Box {
public:
    explicit Box(T value) : value_(value) {}
    T get() const { return value_; }
private:
    T value_;
};

[[nodiscard]] int answer() { return 42; }
}

int main() {
    auto box = demo::Box<int>(42);
    std::vector<int> values{1, 2, 3};
    const char* raw = R"(line "quoted")";
    long long big = 1'000'000;
    bool ok = true;
    return 0;
}"##;
        assert_eq!(language("cpp"), Some("cpp"));
        assert_eq!(language("c++"), Some("cpp"));
        assert_eq!(language("cc"), Some("cpp"));
        let tokens = tokens("cpp", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#include")));
        assert!(tokens.contains(&(SyntaxToken::String, "<vector>")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "namespace")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "template")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "typename")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "public")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "private")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "explicit")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "auto")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "[[nodiscard]]")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Box")));
        assert!(tokens.contains(&(SyntaxToken::Type, "T")));
        assert!(tokens.contains(&(SyntaxToken::Type, "int")));
        assert!(tokens.contains(&(SyntaxToken::Type, "bool")));
        assert!(tokens.contains(&(SyntaxToken::Function, "main")));
        assert!(tokens.contains(&(SyntaxToken::Function, "get")));
        assert!(tokens.contains(&(SyntaxToken::Function, "answer")));
        assert!(tokens.contains(&(SyntaxToken::String, "R\"(line \"quoted\")\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1'000'000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert_well_formed("cpp", source);
    }

    #[test]
    fn csharp_keywords_types_methods_attributes_strings_and_numbers() {
        let source = r##"// comment
#region Demo
using System;

[Obsolete("old")]
public sealed class Greeter {
    private readonly string _name;

    public Greeter(string name) {
        _name = name;
    }

    public string Greet(bool loud = false) {
        var raw = @"raw ""quoted""";
        var message = $"hi {_name}";
        var mark = 'x';
        var count = 0x1f + 0b1010 + 1_000 + 1.5e3 + 42L;
        return loud ? message.ToUpper() : message;
    }
}
#endregion"##;
        assert_eq!(language("csharp"), Some("csharp"));
        assert_eq!(language("cs"), Some("csharp"));
        let tokens = tokens("csharp", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#region")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#endregion")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "using")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "public")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "sealed")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "private")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "readonly")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "var")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "[Obsolete(\"old\")]")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Type, "string")));
        assert!(tokens.contains(&(SyntaxToken::Type, "bool")));
        assert!(tokens.contains(&(SyntaxToken::Function, "Greet")));
        assert!(tokens.contains(&(SyntaxToken::Function, "ToUpper")));
        assert!(tokens.contains(&(SyntaxToken::String, "@\"raw \"\"quoted\"\"\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "$\"hi {_name}\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42L")));
        assert!(tokens.contains(&(SyntaxToken::Number, "false")));
        assert_well_formed("csharp", source);
    }

    #[test]
    fn r_comments_raw_strings_keywords_namespace_calls_and_constants_are_tokenized() {
        let source = r##"# comment
#' roxygen
values <- c(1, 2.5, 0x1f, 1e3, 2L, 3i)
flag <- TRUE
missing <- NA_real_
none <- NULL
frame <- data.frame(x = values)
mean_value <- mean(values, na.rm = TRUE)
model <- lm(y ~ x, data = df)
adder <- function(x) x + 1
if (is.null(missing)) {
  print("missing")
} else {
  library(dplyr)
  return(values)
}
ns <- dplyr::mutate
out <- pkg::compute(values)
raw <- r"(line "quoted")"
raw2 <- R'[bracket]'
"##;
        assert_eq!(language("r"), Some("r"));
        assert_eq!(language("Rprofile"), Some("r"));
        let tokens = tokens("r", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "#' roxygen")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "else")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "function")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "library")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Function, "c")));
        assert!(tokens.contains(&(SyntaxToken::Function, "data.frame")));
        assert!(tokens.contains(&(SyntaxToken::Function, "mean")));
        assert!(tokens.contains(&(SyntaxToken::Function, "lm")));
        assert!(tokens.contains(&(SyntaxToken::Function, "is.null")));
        assert!(tokens.contains(&(SyntaxToken::Function, "print")));
        assert!(tokens.contains(&(SyntaxToken::Function, "pkg::compute")));
        assert!(
            !tokens.contains(&(SyntaxToken::Function, "dplyr::mutate")),
            "a namespace reference without a call stays plain"
        );
        assert!(tokens.contains(&(SyntaxToken::String, "\"missing\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "r\"(line \"quoted\")\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "R'[bracket]'")));
        assert!(tokens.contains(&(SyntaxToken::Number, "TRUE")));
        assert!(tokens.contains(&(SyntaxToken::Number, "NA_real_")));
        assert!(tokens.contains(&(SyntaxToken::Number, "NULL")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "2L")));
        assert!(tokens.contains(&(SyntaxToken::Number, "3i")));
        assert_well_formed("r", source);
    }

    #[test]
    fn lua_comments_long_strings_keywords_functions_and_numbers_are_tokenized() {
        let source = r#"#!/usr/bin/lua
local M = {}
-- line comment
--[[ block
comment ]]
local text = [=[long
string]=]
local n = 0xff + 0x1p4 + 42 + 1.5e3
local t = { name = "Ada", [1] = 'x' }
local loaded = require("mod")
local function greet(name, ...)
  if not name then
    return nil
  end
  print("hello " .. name, math.abs(-1))
  return string.format("%s", name)
end
function M:run()
  local x = 1
  goto done
  ::done::
  return greet("world")
end
"#;
        assert_eq!(language("lua"), Some("lua"));
        let tokens = tokens("lua", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "#!/usr/bin/lua")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "-- line comment")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment
                && text.starts_with("--[[ block")
                && text.contains("comment ]]")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "[=[long\nstring]=]")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "local")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "function")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "not")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "then")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "end")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "goto")));
        assert!(tokens.contains(&(SyntaxToken::Function, "require")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet")));
        assert!(tokens.contains(&(SyntaxToken::Function, "M:run")));
        assert!(tokens.contains(&(SyntaxToken::Function, "print")));
        assert!(tokens.contains(&(SyntaxToken::Function, "math.abs")));
        assert!(tokens.contains(&(SyntaxToken::Function, "string.format")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Ada\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"hello \"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"%s\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"world\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0xff")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1p4")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "nil")));
        assert_well_formed("lua", source);
    }

    #[test]
    fn toml_keys_tables_strings_booleans_and_numbers_are_tokenized() {
        let source = r##"# comment
title = "Vela"
enabled = true
port = 8080
ratio = 1.5e3
literal = 'raw'

[server]
host = 'localhost'
ports = [3000, 3001]
nested = { key = "value" }"##;
        assert_eq!(language("toml"), Some("toml"));
        let tokens = tokens("toml", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(tokens.contains(&(SyntaxToken::Property, "title")));
        assert!(tokens.contains(&(SyntaxToken::Property, "enabled")));
        assert!(tokens.contains(&(SyntaxToken::Property, "port")));
        assert!(tokens.contains(&(SyntaxToken::Property, "ratio")));
        assert!(tokens.contains(&(SyntaxToken::Property, "server")));
        assert!(tokens.contains(&(SyntaxToken::Property, "host")));
        assert!(tokens.contains(&(SyntaxToken::Property, "ports")));
        assert!(tokens.contains(&(SyntaxToken::Property, "key")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Vela\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'raw'")));
        assert!(tokens.contains(&(SyntaxToken::String, "'localhost'")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"value\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert!(tokens.contains(&(SyntaxToken::Number, "8080")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "3000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "3001")));
        assert_well_formed("toml", source);
    }

    #[test]
    fn dockerfile_instructions_variables_strings_and_comments_are_tokenized() {
        let source = r##"# comment
FROM node:20 AS build
WORKDIR /app
copy package*.json ./
RUN npm ci && npm run build
ENV NODE_ENV=production PORT=$PORT
EXPOSE 3000
CMD ["node", "dist/index.js"]"##;
        assert_eq!(language("dockerfile"), Some("dockerfile"));
        assert_eq!(language("docker"), Some("dockerfile"));
        let tokens = tokens("dockerfile", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "FROM")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "AS")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "WORKDIR")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "copy")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "RUN")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "ENV")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "EXPOSE")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "CMD")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$PORT")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"node\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"dist/index.js\"")));
        assert_well_formed("dockerfile", source);
    }

    #[test]
    fn scala_keywords_types_functions_annotations_strings_symbols_and_numbers() {
        let source = r##"package demo

import scala.collection.mutable

/* block
   comment */
class Greeter(val name: String) extends App {
  // line comment
  @deprecated("old", "1.0")
  def greet(prefix: String = "hi"): String = {
    val raw = """line "quoted" text"""
    val ch = 'x'
    val symbol = 'ready
    val ok = true
    val n = 0xff + 0b1010 + 1_000 + 1.5e3 + 42L
    if (name.nonEmpty) s"$prefix $name" else "nobody"
  }
}
enum Color { case Red, Blue }
given Ordering[Int] = Ordering.Int
trait Show[A] { def show(a: A): String }
object Main { def main(args: Array[String]): Unit = println("hi") }
"##;
        assert_eq!(language("scala"), Some("scala"));
        assert_eq!(language("sc"), Some("scala"));
        let tokens = tokens("scala", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("/* block") && text.contains("comment */")
        }));
        assert!(tokens.contains(&(SyntaxToken::Comment, "// line comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@deprecated")));
        for keyword in [
            "package", "import", "class", "extends", "def", "val", "if", "else", "enum",
            "given", "trait", "object",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["Greeter", "String", "App", "Color", "Ordering", "Int", "Show", "A", "Main", "Array", "Unit", "Red", "Blue"] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["greet", "show", "main", "println"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"\"\"line \"quoted\" text\"\"\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::Property, "'ready")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"hi\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"$prefix $name\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"nobody\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0xff")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42L")));
        assert_well_formed("scala", source);
    }

    #[test]
    fn dart_keywords_types_functions_annotations_strings_and_numbers() {
        let source = r##"import 'dart:async';

/* block
   comment */
@immutable
class Greeter extends StatelessWidget {
  // line comment
  const Greeter({super.key, required this.name});
  final String name;

  String greet(String prefix) {
    final raw = r'raw \n text';
    final triple = """line
"quoted" text""";
    final value = 0xff + 0x1p4 + 1_000 + 1.5e3 + 42;
    final ok = true;
    final none = null;
    if (ok) {
      return '$prefix $name';
    }
    return 'nobody';
  }
}

void main() {
  runApp(const Greeter(name: 'Ada'));
}
"##;
        assert_eq!(language("dart"), Some("dart"));
        let tokens = tokens("dart", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("/* block") && text.contains("comment */")
        }));
        assert!(tokens.contains(&(SyntaxToken::Comment, "// line comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@immutable")));
        for keyword in [
            "import", "class", "extends", "const", "required", "final", "if", "return",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["Greeter", "StatelessWidget", "String", "void"] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["greet", "main", "runApp"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "'dart:async'")));
        assert!(tokens.contains(&(SyntaxToken::String, "'raw \\n text'")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("\"\"\"line")
                && text.contains("\"quoted\" text")
                && text.ends_with("\"\"\"")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "'$prefix $name'")));
        assert!(tokens.contains(&(SyntaxToken::String, "'nobody'")));
        assert!(tokens.contains(&(SyntaxToken::String, "'Ada'")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert!(tokens.contains(&(SyntaxToken::Number, "null")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0xff")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1p4")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42")));
        assert_well_formed("dart", source);
    }

    #[test]
    fn powershell_comments_here_strings_variables_cmdlets_types_and_numbers() {
        let source = r##"# line comment
<# block
comment #>
param(
  [string]$Name = "World",
  [int]$Count = 3
)

function Get-Greeting {
  [CmdletBinding()]
  param([string]$Prefix = 'Hi')
  $message = "$Prefix, $Name"
  $env:PATH
  $true; $false; $null
  $value = 0xff + 1_000 + 1.5e3 + 42
  if ($Count -gt 0) {
    Write-Host $message
  }
  return @{ Message = $message }
}

$here = @'
raw
text
'@

Get-Greeting -Prefix "Hello"
"##;
        assert_eq!(language("powershell"), Some("powershell"));
        assert_eq!(language("ps1"), Some("powershell"));
        assert_eq!(language("pwsh"), Some("powershell"));
        let tokens = tokens("powershell", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# line comment")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("<# block") && text.contains("comment #>")
        }));
        for keyword in ["param", "function", "if", "return"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        assert!(tokens.contains(&(SyntaxToken::Type, "[string]")));
        assert!(tokens.contains(&(SyntaxToken::Type, "[int]")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "[CmdletBinding()]")));
        for name in ["Get-Greeting", "Write-Host"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        for variable in [
            "$Name", "$Count", "$Prefix", "$message", "$env:PATH", "$value", "$here",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Property, variable)), "{variable}");
        }
        for value in ["$true", "$false", "$null"] {
            assert!(tokens.contains(&(SyntaxToken::Number, value)), "{value}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"World\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'Hi'")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"$Prefix, $Name\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello\"")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("@'\nraw")
                && text.ends_with("'@")
        }));
        for number in ["0xff", "1_000", "1.5e3", "42", "3", "0"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("powershell", source);
    }

    #[test]
    fn elixir_comments_atoms_sigils_heredocs_functions_and_numbers() {
        let source = r##"defmodule Demo do
  @moduledoc """
  Example module.
  """
  alias MyApp.{Greeter, Formatter}

  # line comment
  def greet(name, times \\ 1) do
    prefix = ~s(hello)
    regex = ~r/na+me/i
    flag = true
    values = [0xff, 0b1010, 1_000, 1.5e3, 42]
    case {name, times} do
      {nil, _} -> :missing
      {^name, n} when n > 0 -> "#{prefix} #{name}"
      _ -> :error
    end
  end

  defp helper(x), do: Enum.map(x, &(&1 * 2))
end
"##;
        assert_eq!(language("elixir"), Some("elixir"));
        assert_eq!(language("ex"), Some("elixir"));
        assert_eq!(language("exs"), Some("elixir"));
        let tokens = tokens("elixir", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# line comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@moduledoc")));
        for keyword in [
            "defmodule", "do", "alias", "def", "case", "when", "end", "defp",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["Demo", "MyApp", "Greeter", "Formatter"] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["greet", "helper", "Enum.map"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("\"\"\"\n  Example module.")
                && text.ends_with("\"\"\"")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "~s(hello)")));
        assert!(tokens.contains(&(SyntaxToken::String, "~r/na+me/i")));
        assert!(tokens.contains(&(SyntaxToken::Property, ":missing")));
        assert!(tokens.contains(&(SyntaxToken::Property, ":error")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"#{prefix} #{name}\"")));
        for number in ["true", "nil", "0xff", "0b1010", "1_000", "1.5e3", "42", "1", "0", "2"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("elixir", source);
    }

    #[test]
    fn haskell_pragmas_comments_declarations_functions_types_and_numbers() {
        let source = r##"{-# LANGUAGE OverloadedStrings #-}
module Demo where

import qualified Data.Map as Map
{- block
   comment -}
-- line comment

data Color = Red | Blue deriving (Show, Eq)

class Greeter a where
  greet :: a -> String

instance Greeter Color where
  greet Red = "red"
  greet Blue = "blue"

main :: IO ()
main = do
  let values = [0xff, 0o17, 1_000, 1.5e3, 42]
  let flag = True
  let ch = 'x'
  name <- getLine
  print (map (\x -> x * 2) values)
  putStrLn "done"
  pure ()
"##;
        assert_eq!(language("haskell"), Some("haskell"));
        assert_eq!(language("hs"), Some("haskell"));
        let tokens = tokens("haskell", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Keyword
                && text.starts_with("{-# LANGUAGE")
                && text.ends_with("#-}")
        }));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("{- block") && text.contains("comment -}")
        }));
        assert!(tokens.contains(&(SyntaxToken::Comment, "-- line comment")));
        for keyword in [
            "module", "where", "import", "qualified", "as", "data", "deriving", "class",
            "instance", "do", "let",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in [
            "Demo", "Data", "Map", "Color", "Red", "Blue", "Show", "Eq", "Greeter",
            "String", "IO",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["main", "greet", "getLine", "print", "map", "putStrLn", "pure"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"red\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"blue\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"done\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        for number in ["True", "0xff", "0o17", "1_000", "1.5e3", "42", "2"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("haskell", source);
    }

    #[test]
    fn clojure_keywords_symbols_def_forms_literals_and_characters() {
        let source = r##"(ns demo.core
  (:require [clojure.string :as str]))

; line comment
(defn greet
  [^String name]
  (let [message (str "Hello, " name)
        values [0xff 0o17 1_000 1.5e3 42]
        flag true
        ch \newline
        key :ready]
    (if flag
      (println message (first values))
      (println "none" nil))))

(defmulti render :type)
(defmethod render :text [item] (str item))
(defn -main [& args] (apply greet args))
"##;
        assert_eq!(language("clojure"), Some("clojure"));
        assert_eq!(language("clj"), Some("clojure"));
        assert_eq!(language("cljs"), Some("clojure"));
        let tokens = tokens("clojure", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "; line comment")));
        for keyword in ["ns", "defn", "let", "if", "defmulti", "defmethod"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["greet", "render", "-main", "str", "println", "first", "apply"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        for property in [":require", ":as", ":type", ":text", ":ready"] {
            assert!(tokens.contains(&(SyntaxToken::Property, property)), "{property}");
        }
        assert!(tokens.contains(&(SyntaxToken::Type, "String")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello, \"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"none\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\\newline")));
        for number in ["true", "nil", "0xff", "0o17", "1_000", "1.5e3", "42"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("clojure", source);
    }

    #[test]
    fn julia_comments_macros_declarations_functions_types_and_numbers() {
        let source = r##"module Demo

# line comment
#= block
comment =#
struct Point{T}
  x::T
  y::T
end

abstract type Shape end

function greet(name::String)
    message = "Hello, $name"
    values = [0xff, 0o17, 0b1010, 1_000, 1.5e3, 42]
    point = Point(1, 2)
    sym = :ready
    ch = 'x'
    @show message
    if isempty(values)
        return "none"
    end
    println(message, point)
end

greet("Ada")
"##;
        assert_eq!(language("julia"), Some("julia"));
        assert_eq!(language("jl"), Some("julia"));
        let tokens = tokens("julia", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# line comment")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("#= block") && text.contains("comment =#")
        }));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@show")));
        for keyword in ["module", "struct", "abstract", "type", "end", "function", "if", "return"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["Demo", "Point", "T", "Shape", "String"] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["greet", "isempty", "println"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::Property, ":ready")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello, $name\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"none\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        for number in ["0xff", "0o17", "0b1010", "1_000", "1.5e3", "42", "1", "2"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("julia", source);
    }

    #[test]
    fn ocaml_comments_declarations_let_functions_types_and_numbers() {
        let source = r##"(* block
   comment *)
let greeting = "hello"

let rec map f = function
  | [] -> []
  | x :: xs -> f x :: map f xs

type color = Red | Blue | Green

module Demo = struct
  let value = 42
  let add x y = x + y
  let hex = 0xff
  let bin = 0b1010
  let oct = 0o17
  let ratio = 1.5e3
  let flag = true
  let ch = 'x'
  let quoted = {|raw "text"|}
  let values = [1; 2; 3]
  let print_all () = List.iter print_endline ["a"; "b"]
end

let () = print_endline (string_of_int Demo.value)
"##;
        assert_eq!(language("ocaml"), Some("ocaml"));
        assert_eq!(language("ml"), Some("ocaml"));
        assert_eq!(language("mli"), Some("ocaml"));
        let tokens = tokens("ocaml", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("(* block") && text.contains("comment *)")
        }));
        for keyword in ["let", "rec", "function", "type", "module", "struct", "end"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in ["color", "Red", "Blue", "Green", "Demo"] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["map", "add", "print_all", "List.iter", "print_endline", "string_of_int"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(
            !tokens.contains(&(SyntaxToken::Function, "value")),
            "plain let bindings are not functions"
        );
        assert!(tokens.contains(&(SyntaxToken::String, "\"hello\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "{|raw \"text\"|}")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        for number in ["true", "42", "0xff", "0b1010", "0o17", "1.5e3", "1", "2", "3"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("ocaml", source);
    }

    #[test]
    fn erlang_attributes_functions_variables_types_strings_and_numbers() {
        let source = r##"-module(demo).
-export([greet/1, factorial/1]).

%% line comment
greet(Name) ->
    Message = "Hello, " ++ Name,
    Char = $a,
    State = 'ready',
    Values = [16#FF, 2#1010, 1_000, 1.5e3, 42],
    Flag = true,
    io:format("~s~n", [Message]),
    {ok, State, Char, Values, Flag}.

factorial(0) -> 1;
factorial(N) when N > 0 ->
    N * factorial(N - 1).

-record(user, {name, age = 0}).

-spec factorial(integer()) -> integer().
"##;
        assert_eq!(language("erlang"), Some("erlang"));
        assert_eq!(language("erl"), Some("erlang"));
        assert_eq!(language("hrl"), Some("erlang"));
        let tokens = tokens("erlang", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "%% line comment")));
        for attribute in ["-module", "-export", "-record", "-spec"] {
            assert!(tokens.contains(&(SyntaxToken::Attribute, attribute)), "{attribute}");
        }
        assert!(tokens.contains(&(SyntaxToken::Keyword, "when")));
        for name in ["greet", "factorial", "io:format"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::Type, "integer")));
        for variable in ["Name", "Message", "Char", "State", "Values", "Flag", "N"] {
            assert!(tokens.contains(&(SyntaxToken::Property, variable)), "{variable}");
        }
        assert!(tokens.contains(&(SyntaxToken::Property, "'ready'")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello, \"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"~s~n\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "$a")));
        for number in ["true", "16#FF", "2#1010", "1_000", "1.5e3", "42", "0", "1"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("erlang", source);
    }

    #[test]
    fn elm_comments_declarations_functions_module_chains_and_numbers() {
        let source = r##"module Demo exposing (main)

import Html exposing (Html, div, text)

{- block
   comment -}
-- line comment

type alias Model =
    { count : Int, name : String }

type Msg
    = Increment
    | Decrement

update : Msg -> Model -> Model
update msg model =
    case msg of
        Increment ->
            { model | count = model.count + 1 }

        Decrement ->
            { model | count = model.count - 1 }

view : Model -> Html Msg
view model =
    div [] [ text (String.fromInt model.count) ]

main =
    Html.beginnerProgram { model = { count = 0, name = "Ada" }, view = view, update = update }

values =
    [ 0xFF, 1_000, 1.5e3, 42 ]

flag = True
ch = 'x'
"##;
        assert_eq!(language("elm"), Some("elm"));
        let tokens = tokens("elm", source);
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("{- block") && text.contains("comment -}")
        }));
        assert!(tokens.contains(&(SyntaxToken::Comment, "-- line comment")));
        for keyword in ["module", "exposing", "import", "type", "alias", "case", "of"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in [
            "Demo", "Model", "Msg", "Int", "String", "Increment", "Decrement", "Html",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for name in ["update", "view", "main", "String.fromInt", "Html.beginnerProgram"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"Ada\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        for number in ["True", "0xFF", "1_000", "1.5e3", "42", "0", "1"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("elm", source);
    }

    #[test]
    fn hcl_blocks_attributes_references_heredocs_functions_and_numbers() {
        let source = r##"# line comment
// slash comment
/* block comment */

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

variable "region" {
  type    = string
  default = "us-east-1"
}

locals {
  enabled = true
  missing = null
  names   = [for s in var.names : upper(s)]
  label   = lookup(var.tags, "Name", "none")
}

resource "aws_instance" "web" {
  count         = 2
  ami           = data.aws_ami.ubuntu.id
  index         = count.index
  instance_type = "t3.micro"
  tags = {
    Name = "web-${count.index}"
  }
  user_data = <<EOF
#!/bin/bash
echo hello
EOF
}

output "ids" {
  value = aws_instance.web[*].id
}
"##;
        assert_eq!(language("hcl"), Some("hcl"));
        assert_eq!(language("terraform"), Some("hcl"));
        assert_eq!(language("tf"), Some("hcl"));
        let tokens = tokens("hcl", source);
        for comment in ["# line comment", "// slash comment", "/* block comment */"] {
            assert!(tokens.contains(&(SyntaxToken::Comment, comment)), "{comment}");
        }
        for keyword in [
            "terraform", "required_providers", "variable", "locals", "resource", "output",
            "for", "in",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for property in [
            "source", "version", "type", "default", "enabled", "missing", "names", "label",
            "count", "ami", "instance_type", "tags", "Name", "user_data", "value",
            "var.names", "var.tags", "data.aws_ami.ubuntu.id", "count.index",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Property, property)), "{property}");
        }
        for name in ["upper", "lookup"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        for string in [
            "\"hashicorp/aws\"", "\"~> 5.0\"", "\"us-east-1\"", "\"t3.micro\"",
            "\"web-${count.index}\"",
        ] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("<<EOF")
                && text.contains("echo hello")
                && text.ends_with("EOF")
        }));
        for number in ["true", "null", "2"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("hcl", source);
    }

    #[test]
    fn nix_comments_strings_attributes_let_inherit_rec_builtins_and_numbers() {
        let source = r##"{ pkgs, lib, ... }:

let
  # line comment
  /* block
     comment */
  greeting = "Hello, ${name}";
  indented = ''
    line one
    line two
  '';
  values = [ 1 0x1f 1_000 1.5e3 42 ];
  flag = true;
  nothing = null;
  nixpkgs = import <nixpkgs> {};
  inherit (pkgs) hello;
in rec
{
  environment.systemPackages = [ pkgs.git hello ];
  services.nginx.enable = flag;
  name = "demo";
  message = builtins.concatStringsSep ", " [ "a" "b" ];
  mapped = map (x: x + 1) values;
}
"##;
        assert_eq!(language("nix"), Some("nix"));
        let tokens = tokens("nix", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# line comment")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("/* block") && text.contains("comment */")
        }));
        for keyword in ["let", "inherit", "in", "rec"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for property in [
            "greeting", "indented", "values", "flag", "nothing", "nixpkgs",
            "environment.systemPackages", "services.nginx.enable", "name", "message",
            "mapped",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Property, property)), "{property}");
        }
        for name in ["import", "builtins.concatStringsSep", "map"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello, ${name}\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"demo\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\", \"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"a\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"b\"")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String
                && text.starts_with("''\n    line one")
                && text.contains("line two")
                && text.ends_with("''")
        }));
        for number in ["true", "null", "1", "0x1f", "1_000", "1.5e3", "42"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("nix", source);
    }

    #[test]
    fn puppet_comments_classes_resources_attributes_variables_and_numbers() {
        let source = r##"# line comment
/* block
   comment */

class nginx (
  String $version = 'latest',
  Boolean $enabled = true,
) inherits base {

  $config = "/etc/nginx/${version}.conf"

  package { 'nginx':
    ensure  => $version,
    require => Package['base'],
  }

  file { $config:
    ensure  => file,
    content => template('nginx/nginx.conf.erb'),
    notify  => Service['nginx'],
  }

  service { 'nginx':
    ensure => running,
    enable => $enabled,
  }
}

node 'web01' {
  include nginx
  $message = sprintf('Hello %s', $name)
  $count = 0x1f + 1.5e3 + 42
  $missing = undef
  notice($message)
}
"##;
        assert_eq!(language("puppet"), Some("puppet"));
        assert_eq!(language("pp"), Some("puppet"));
        let tokens = tokens("puppet", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# line comment")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("/* block") && text.contains("comment */")
        }));
        for keyword in ["class", "inherits", "node"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in [
            "String", "Boolean", "Package", "Service", "nginx", "package", "file",
            "service", "'web01'",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for property in [
            "ensure", "require", "content", "notify", "enable", "$version", "$enabled",
            "$config", "$message", "$name", "$count", "$missing",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Property, property)), "{property}");
        }
        for name in ["include", "template", "sprintf", "notice"] {
            assert!(tokens.contains(&(SyntaxToken::Function, name)), "{name}");
        }
        for string in [
            "'latest'", "\"/etc/nginx/${version}.conf\"", "'nginx'", "'base'",
            "'nginx/nginx.conf.erb'", "'Hello %s'",
        ] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["true", "undef", "0x1f", "1.5e3", "42"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("puppet", source);
    }

    #[test]
    fn graphql_operations_types_variables_directives_strings_and_numbers() {
        let source = r##"# query comment
query GetUser($id: ID!, $withPosts: Boolean = false) {
  user(id: $id) {
    id
    name
    email
    posts(first: 10) @include(if: $withPosts) {
      edges {
        node {
          ...PostFields
        }
      }
    }
  }
}

fragment PostFields on Post {
  id
  title
  body
  publishedAt
}

mutation CreatePost($input: CreatePostInput!) {
  createPost(input: $input) {
    id
    title
  }
}

type User implements Node {
  id: ID!
  name: String!
  posts: [Post!]!
}

enum Role {
  ADMIN
  MEMBER
}

input CreatePostInput {
  """A post body"""
  body: String
  rating: Int = 5
  score: Float = -1.5
  label: "A label"
}
"##;
        assert_eq!(language("graphql"), Some("graphql"));
        assert_eq!(language("gql"), Some("graphql"));
        let tokens = tokens("graphql", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# query comment")));
        for keyword in [
            "query", "fragment", "on", "mutation", "type", "implements", "enum", "input",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        for name in [
            "GetUser", "ID", "Boolean", "PostFields", "Post", "CreatePost",
            "CreatePostInput", "User", "Node", "String", "Role", "ADMIN", "MEMBER", "Int",
            "Float",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Type, name)), "{name}");
        }
        for variable in ["$id", "$withPosts", "$input"] {
            assert!(tokens.contains(&(SyntaxToken::Property, variable)), "{variable}");
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@include")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"A label\"")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String && *text == "\"\"\"A post body\"\"\""
        }));
        for number in ["false", "10", "5", "-1.5"] {
            assert!(tokens.contains(&(SyntaxToken::Number, number)), "{number}");
        }
        assert_well_formed("graphql", source);
    }

    #[test]
    fn vue_components_embed_html_script_style_directives_and_mustaches() {
        let source = r##"<template>
  <div :class="{ active: busy }" @click="toggle">
    {{ message }}
    <span v-if="busy">{{ label }}</span>
  </div>
</template>

<script setup lang="ts">
const message: string = "Hello";
const busy = true;
</script>

<style scoped>
.active { color: red; }
</style>
"##;
        assert_eq!(language("vue"), Some("vue"));
        let tokens = tokens("vue", source);
        for tag in ["template", "div", "span", "script", "style"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, ":class")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@click")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "v-if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "{{ message }}")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "{{ label }}")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "const")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello\"")));
        assert!(tokens.contains(&(SyntaxToken::Property, "color")));
        assert_well_formed("vue", source);
    }

    #[test]
    fn svelte_components_embed_script_style_special_tags_and_interpolation() {
        let source = r##"<script lang="ts">
let count = 0;
const items = ["a", "b"];
</script>

<button class:active={count > 0} on:click={() => count += 1} bind:value={name}>
  {count}
</button>

{#if count > 0}
  <p>{count} items</p>
{:else if count === 0}
  <p>Empty</p>
{:else}
  <p>Negative</p>
{/if}

{#each items as item, index (item)}
  <span>{index}: {item}</span>
{/each}

{@html raw}
{#await promise then value}
  {value}
{/await}

<style>
.active { color: red; }
</style>
"##;
        assert_eq!(language("svelte"), Some("svelte"));
        let tokens = tokens("svelte", source);
        for tag in ["script", "button", "p", "span", "style"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for attribute in ["class:active", "on:click", "bind:value"] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        for special in [
            "{#if",
            "{:else if",
            "{:else",
            "{/if",
            "{#each",
            "{/each",
            "{@html",
            "{#await",
            "{/await",
        ] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, special)), "{special}");
        }
        for keyword in ["const", "as"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"a\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0")));
        assert!(tokens.contains(&(SyntaxToken::Property, "color")));
        assert_well_formed("svelte", source);
    }

    #[test]
    fn astro_components_frontmatter_interpolation_scripts_and_styles() {
        let source = r##"---
const title: string = "Hello";
const items = [1, 2];
---
<html>
  <body>
    <h1 class="title">{title}</h1>
    <ul>
      <li>{items.length}</li>
    </ul>
    <MyComponent client:load count={items.length} />
    <script>
      const value = 42;
    </script>
    <style>
      .title { color: red; }
    </style>
  </body>
</html>
"##;
        assert_eq!(language("astro"), Some("astro"));
        let tokens = tokens("astro", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "---")));
        for keyword in ["const", "string"] {
            assert!(tokens.contains(&(SyntaxToken::Keyword, keyword)), "{keyword}");
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello\"")));
        for tag in ["html", "body", "h1", "ul", "li", "MyComponent", "script", "style"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for attribute in ["class", "client:load", "count"] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Property, "color")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42")));
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Keyword && *text == "{")
                .count(),
            3,
            "only template expressions paint braces, not frontmatter or style blocks"
        );
        assert_well_formed("astro", source);
    }

    #[test]
    fn zig_comments_strings_keywords_declarations_builtins_and_numbers() {
        let source = r##"const std = @import("std");

//! module docs
/// Point docs
const Point = struct {
    x: f32,
    y: f32,
    label: u8 = 'a',
    ready: bool = true,
};

pub fn add(a: Point, b: Point) Point {
    const sum = a.x + b.x;
    return .{ .x = sum, .y = b.y + a.y };
}

test "point" {
    const p = Point{ .x = 1.5, .y = 0x1.8p1 };
    const bits = 0b1010 + 0o755 + 1_000;
    const message =
        \\first line
        \\second line
    ;
    try expectEqual(3.5, p.x + p.y);
}
"##;
        assert_eq!(language("zig"), Some("zig"));
        let tokens = tokens("zig", source);
        for comment in ["//! module docs", "/// Point docs"] {
            assert!(
                tokens.contains(&(SyntaxToken::Comment, comment)),
                "{comment}"
            );
        }
        for keyword in [
            "const", "struct", "pub", "fn", "test", "try", "return", "f32", "u8",
            "bool",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for function in ["@import", "add", "expectEqual"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Type, "Point")));
        for string in ["\"std\"", "\"point\"", "'a'"] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in [
            "true", "1.5", "0x1.8p1", "0b1010", "0o755", "1_000", "3.5",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert!(
            tokens.iter().any(|(kind, text)| {
                *kind == SyntaxToken::String && text.starts_with("\\\\")
            }),
            "line-leading double backslashes form multiline strings"
        );
        assert_well_formed("zig", source);
    }

    #[test]
    fn solidity_comments_keywords_declarations_builtins_strings_and_numbers() {
        let source = r##"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/* block
 * comment */
import {IERC20} from "./IERC20.sol";

/// @notice A tiny counter.
contract Counter is Ownable {
    uint256 public value = 1_000;
    mapping(address => uint256) private balances;
    address constant OWNER = address(0x1234);

    event Updated(address indexed who, uint256 value);
    error NotOwner(address caller);

    modifier onlyOwner() {
        require(msg.sender == OWNER, "not owner");
        _;
    }

    function setValue(uint256 next) external onlyOwner returns (bool) {
        // TODO keep under limit
        if (next > 1e18) revert TooLarge(next);
        value = next;
        emit Updated(msg.sender, next);
        return true;
    }

    function balanceOf(address account) public view returns (uint256) {
        return balances[account];
    }
}
"##;
        assert_eq!(language("solidity"), Some("solidity"));
        assert_eq!(language("sol"), Some("solidity"));
        let tokens = tokens("solidity", source);
        assert!(
            tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::Comment && text.starts_with("/* block")),
            "multi-line block comments"
        );
        for keyword in [
            "pragma", "import", "contract", "is", "mapping", "private", "constant",
            "event", "error", "modifier", "function", "external", "returns", "if",
            "emit", "return", "public", "view",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for type_name in [
            "Counter", "Ownable", "IERC20", "Updated", "NotOwner", "uint256", "address",
            "bool",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Type, type_name)),
                "{type_name}"
            );
        }
        for function in ["setValue", "balanceOf", "onlyOwner", "require", "revert"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        for property in ["msg", "sender"] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        for string in ["\"./IERC20.sol\"", "\"not owner\""] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["1_000", "0x1234", "1e18", "true"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert_well_formed("solidity", source);
    }

    #[test]
    fn mdx_frontmatter_esm_jsx_expressions_and_fences_are_tokenized() {
        let source = r##"---
title: "Hello MDX"
draft: false
tags:
  - docs
---
import { Card } from "./Card";
export const count = 2;

# Hello {name}

<Card title="Hi" count={count}>
  **Bold** text with `code` and [link](https://example.com).
</Card>

{count > 1 ? <span>many</span> : <span>one</span>}

```ts
const ignored = { not: "an expression" };
```
"##;
        assert_eq!(language("mdx"), Some("mdx"));
        let tokens = tokens("mdx", source);
        assert!(tokens.contains(&(SyntaxToken::Property, "title")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Hello MDX\"")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "false")));
        for keyword in ["import", "from", "export", "const"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for tag in ["Card", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for attribute in ["title", "count"] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::String, "\"./Card\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "2")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "**")));
        assert!(tokens.contains(&(SyntaxToken::String, "`code`")));
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Keyword && *text == "{")
                .count(),
            3,
            "only MDX expressions paint braces, not frontmatter, ESM or fences"
        );
        assert_well_formed("mdx", source);
    }

    #[test]
    fn prisma_comments_blocks_fields_attributes_functions_strings_and_numbers() {
        let source = r##"/// Client generator
generator client {
  provider = "prisma-client-js"
}

/* datasource */
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

/// User model
model User {
  id        Int      @id @default(autoincrement())
  email     String   @unique @db.VarChar(255)
  name      String?
  active    Boolean  @default(true)
  priority  Int      @default(0)
  ratio     Float    @default(1.5)
  role      Role     @default(USER)
  posts     Post[]

  @@index([email])
}

enum Role {
  USER
  ADMIN
}

type UserWithPosts = User
"##;
        assert_eq!(language("prisma"), Some("prisma"));
        let tokens = tokens("prisma", source);
        for keyword in ["generator", "datasource", "model", "enum", "type"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for type_name in [
            "User", "Role", "Post", "UserWithPosts", "Int", "String", "Boolean", "Float",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Type, type_name)),
                "{type_name}"
            );
        }
        for property in [
            "provider",
            "url",
            "id",
            "email",
            "name",
            "active",
            "priority",
            "ratio",
            "role",
            "posts",
            "USER",
            "ADMIN",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        for attribute in [
            "@id",
            "@unique",
            "@db.VarChar",
            "@default",
            "@@index",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        for function in ["env", "autoincrement"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        for string in [
            "\"prisma-client-js\"",
            "\"postgresql\"",
            "\"DATABASE_URL\"",
        ] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["255", "true", "0", "1.5"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert_well_formed("prisma", source);
    }

    #[test]
    fn handlebars_frontmatter_blocks_comments_html_and_mustaches_are_tokenized() {
        let source = r##"---
title: "Handlebars"
enabled: true
---
<div class="page">
  {{!-- block comment --}}
  {{! inline comment }}
  {{#if user}}
    <h1>{{user.name}}</h1>
  {{else if guest}}
    <p>Guest</p>
  {{else}}
    {{{fallback}}}
  {{/if}}
  {{#each items as |item|}}
    <span>{{item.title}}</span>
  {{/each}}
  {{> footer year=2026}}
  {{formatDate date "yyyy-MM-dd"}}
</div>
"##;
        assert_eq!(language("handlebars"), Some("handlebars"));
        let tokens = tokens("handlebars", source);
        assert!(tokens.contains(&(SyntaxToken::Property, "title")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Handlebars\"")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "true")));
        for tag in ["div", "h1", "p", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, "class")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "{{!-- block comment --}}")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "{{! inline comment }}")));
        for keyword in ["else", "{{", "{{{"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for function in ["if", "each", "footer"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        for property in ["user.name", "fallback", "item.title", "formatDate"] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, "year")));
        assert!(tokens.contains(&(SyntaxToken::Number, "2026")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"yyyy-MM-dd\"")));
        assert_well_formed("handlebars", source);
    }

    #[test]
    fn liquid_comments_tags_objects_filters_blocks_and_embedded_content() {
        let source = r##"<div class="product">
  {% comment %}hidden {{ not_rendered }}{% endcomment %}
  {% if product.available %}
    <h1>{{ product.title | escape }}</h1>
  {% elsif product.preorder %}
    <p>Coming soon</p>
  {% else %}
    <p>{{ 'Sold out' | t }}</p>
  {% endif %}
  {% for item in collection.products limit: 3 %}
    <span>{{ forloop.index }} - {{ item.title }}</span>
  {% endfor %}
  {% assign discount = 10 %}
  {% render 'price', product: product %}
  {% raw %}{{ literal }}{% endraw %}
  {% style %}.price { color: red; }{% endstyle %}
  {% schema %}{"name":"price"}{% endschema %}
  {% javascript %}const price = 10;{% endjavascript %}
</div>
"##;
        assert_eq!(language("liquid"), Some("liquid"));
        let tokens = tokens("liquid", source);
        for tag in ["div", "h1", "p", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for keyword in [
            "if",
            "elsif",
            "else",
            "endif",
            "for",
            "endfor",
            "assign",
            "render",
            "raw",
            "endraw",
            "style",
            "endstyle",
            "schema",
            "endschema",
            "javascript",
            "endjavascript",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for filter in ["escape", "t"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, filter)),
                "{filter}"
            );
        }
        for attribute in ["limit:", "product:"] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        for property in [
            "product.available",
            "product.title",
            "collection.products",
            "forloop.index",
            "item.title",
            "discount",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        for string in ["'price'", "'Sold out'"] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["3", "10"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("{% comment %}")
        }));
        assert!(
            !tokens
                .iter()
                .any(|(_, text)| *text == "not_rendered" || *text == "literal"),
            "comment and raw blocks hide embedded liquid"
        );
        assert!(tokens.contains(&(SyntaxToken::Property, "color")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "const")));
        assert_well_formed("liquid", source);
    }

    #[test]
    fn blade_comments_directives_echoes_php_and_verbatim_are_tokenized() {
        let source = r##"<div class="card">
  {{-- Blade comment {{ $hidden }} --}}
  @if ($user->active)
    <h1>{{ $user->name }}</h1>
  @elseif ($user->pending)
    <p>{{ 'Pending' }}</p>
  @else
    <p>{!! $fallback !!}</p>
  @endif

  @foreach ($items as $item)
    <span>{{ $item->title }}</span>
  @endforeach

  @php
    $total = 42;
    $label = "Total";
  @endphp

  @include('partials.footer', ['year' => 2026])

  <?php echo $total; ?>
  @verbatim
    {{ not_blade }}
  @endverbatim
</div>
"##;
        assert_eq!(language("blade"), Some("blade"));
        let tokens = tokens("blade", source);
        for tag in ["div", "h1", "p", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for directive in [
            "@if", "@elseif", "@else", "@endif", "@foreach", "@endforeach", "@php",
            "@endphp", "@include", "@verbatim", "@endverbatim",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, directive)),
                "{directive}"
            );
        }
        for keyword in ["as", "echo"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for variable in ["$user", "$items", "$item", "$fallback", "$total", "$label"] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, variable)),
                "{variable}"
            );
        }
        for string in ["'Pending'", "\"Total\"", "'partials.footer'", "'year'"] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["42", "2026"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Keyword, "<?php")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("{{-- Blade comment")
        }));
        assert!(
            !tokens
                .iter()
                .any(|(_, text)| *text == "$hidden" || *text == "not_blade"),
            "comment and verbatim blocks hide embedded blade"
        );
        assert_well_formed("blade", source);
    }

    #[test]
    fn templ_declarations_go_control_components_html_and_expressions_are_tokenized() {
        let source = r##"package components

import "fmt"

// templ comment
templ Hello(name string) {
  <div class="hello">
    Hello, { name }!
  </div>
}

templ List(items []string) {
  if len(items) == 0 {
    <p>No items</p>
  } else {
    for _, item := range items {
      <li>{ item }</li>
    }
  }
}

templ Page() {
  <h1>Title</h1>
  @Layout() {
    <span>Nested</span>
  }
  @fmt.Println("x")
  {! someCall() }
  switch value {
    case 1:
      <p>one</p>
    default:
      <p>other</p>
  }
  <!-- html comment -->
}
"##;
        assert_eq!(language("templ"), Some("templ"));
        let tokens = tokens("templ", source);
        for keyword in [
            "templ", "package", "import", "if", "else", "for", "range", "switch",
            "case", "default",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for function in ["Hello", "List", "Page", "len", "someCall", "@Layout", "@fmt.Println"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Type, "string")));
        for tag in ["div", "p", "li", "h1", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, "class")));
        for string in ["\"fmt\"", "\"x\""] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["0", "1"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("<!-- html comment")
        }));
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Keyword && *text == "{")
                .count(),
            3,
            "only templ expressions paint braces"
        );
        assert_well_formed("templ", source);
    }

    #[test]
    fn pascal_comments_directives_keywords_functions_types_strings_and_numbers() {
        let source = r##"program Hello;

{$mode objfpc}

uses SysUtils;

type
  TPerson = class
  private
    FName: string;
    FAge: Integer;
  public
    constructor Create(const AName: string; AAge: Integer);
    function Greet(Prefix: string): string;
    property Name: string read FName write FName;
  end;

function Add(A, B: Integer): Integer;
begin
  Result := A + B;
  if Result > 10 then
    Exit(Result)
  else
    Result := 0;
  while Result < 5 do
    Inc(Result);
  for I := 1 to 3 do
    WriteLn(I);
end;

var
  Person: TPerson;
  Mask, CR: Integer;
begin
  // comment
  (* block comment *)
  { another comment }
  Mask := $FF;
  CR := #13;
  Person := TPerson.Create('Ada', 42);
  WriteLn('O''Brien');
  WriteLn(Person.Greet('Hello'));
  Assert(Person <> nil);
end.
"##;
        assert_eq!(language("pascal"), Some("pascal"));
        assert_eq!(language("pas"), Some("pascal"));
        let tokens = tokens("pascal", source);
        for keyword in [
            "program", "uses", "type", "class", "private", "public", "constructor",
            "function", "property", "read", "write", "end", "begin", "if", "then",
            "else", "while", "do", "for", "to", "var", "Assert", "Exit",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for function in ["Create", "Greet", "Add", "Name"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        for type_name in ["string", "Integer"] {
            assert!(
                tokens.contains(&(SyntaxToken::Type, type_name)),
                "{type_name}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Property, "Result")));
        for string in [
            "'Ada'",
            "'Hello'",
            "'O''Brien'",
            "#13",
            "{$mode objfpc}",
        ] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["10", "0", "5", "1", "3", "42", "$FF", "nil"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        for comment in ["// comment", "(* block comment *)", "{ another comment }"] {
            assert!(
                tokens.contains(&(SyntaxToken::Comment, comment)),
                "{comment}"
            );
        }
        assert_well_formed("pascal", source);
    }

    #[test]
    fn makefile_comments_assignments_directives_targets_recipes_and_variables() {
        let source = r##"# Makefile
CC := clang
CFLAGS ?= -Wall -O2
SRC = $(wildcard src/*.c)
OBJ = ${SRC:.c=.o}
ROOT := $(CURDIR)
TARGET = app

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $@ $^

%.o: %.c
	@echo "Building $<"
	$(CC) $(CFLAGS) -c $< -o $@

include config.mk

ifeq ($(DEBUG),1)
CFLAGS += -g
endif

clean:
	rm -f $(OBJ) $(TARGET)
"##;
        assert_eq!(language("makefile"), Some("makefile"));
        assert_eq!(language("make"), Some("makefile"));
        let tokens = tokens("makefile", source);
        for variable in [
            "CC", "CFLAGS", "SRC", "OBJ", "ROOT", "TARGET", "CURDIR", "DEBUG",
            "SRC:.c=.o", "$@", "$^",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, variable)),
                "{variable}"
            );
        }
        for directive in ["include", "ifeq", "endif"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, directive)),
                "{directive}"
            );
        }
        for operator in [":=", "?=", "+="] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, operator)),
                "{operator}"
            );
        }
        for target in [".PHONY", "all", "%.o", "clean"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, target)),
                "{target}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Function, "wildcard")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "# Makefile")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"Building $<\"")));
        assert_well_formed("makefile", source);
    }

    #[test]
    fn twig_comments_tags_filters_functions_hashes_and_verbatim_are_tokenized() {
        let source = r##"<div class="page">
  {# Twig comment {{ hidden }} #}
  {% if user.active %}
    <h1>{{ user.name | upper }}</h1>
  {% elseif user.pending %}
    <p>{{ 'Pending' }}</p>
  {% else %}
    <p>{{ fallback }}</p>
  {% endif %}
  {% for item in items %}
    <span>{{ loop.index }} - {{ item.title | escape }}</span>
  {% endfor %}
  {% set label = "Hello" %}
  {{ range(1, 3) | join(', ') }}
  {{ user.name starts with 'A' ? 'yes' : 'no' }}
  {{ include('x', { title: 'Hi' }) }}
  {% include 'x' %}
  {% verbatim %}{{ not_twig }}{% endverbatim %}
  <!-- html comment -->
</div>
"##;
        assert_eq!(language("twig"), Some("twig"));
        let tokens = tokens("twig", source);
        for tag in ["div", "h1", "p", "span"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for keyword in [
            "if", "elseif", "else", "endif", "for", "in", "endfor", "set", "include",
            "verbatim", "endverbatim", "starts", "with",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        for function in ["upper", "escape", "range", "join", "include"] {
            assert!(
                tokens.contains(&(SyntaxToken::Function, function)),
                "{function}"
            );
        }
        for property in [
            "user.active",
            "user.name",
            "user.pending",
            "fallback",
            "item",
            "items",
            "loop.index",
            "item.title",
            "label",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Attribute, "title:")));
        for string in [
            "'Pending'",
            "\"Hello\"",
            "', '",
            "'A'",
            "'yes'",
            "'no'",
            "'x'",
            "'Hi'",
        ] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        for number in ["1", "3"] {
            assert!(
                tokens.contains(&(SyntaxToken::Number, number)),
                "{number}"
            );
        }
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::Comment && text.starts_with("{# Twig comment")
        }));
        assert!(
            !tokens
                .iter()
                .any(|(_, text)| *text == "hidden" || *text == "not_twig"),
            "comment and verbatim blocks hide embedded twig"
        );
        assert_well_formed("twig", source);
    }

    #[test]
    fn pug_doctype_tags_attributes_flow_mixins_and_expressions_are_tokenized() {
        let source = r##"doctype html
html
  head
    title= pageTitle
  body
    //- unbuffered comment
    // buffered comment
    include partials/header
    h1#title.main-title Hello #{user.name}
    p(class="lead" data-count=count) Welcome !{user.name}
    if user.active
      span Active
    else if user.pending
      span Pending
    else
      span Inactive
    each item in items
      li= item.title
    mixin card(title)
      .card= title
    +card(user.name)
    - const total = 42;
    = total
    <!-- html comment -->
"##;
        assert_eq!(language("pug"), Some("pug"));
        assert_eq!(language("jade"), Some("pug"));
        let tokens = tokens("pug", source);
        for tag in ["html", "head", "title", "body", "h1", "p", "span", "li"] {
            assert!(tokens.contains(&(SyntaxToken::Tag, tag)), "{tag}");
        }
        for attribute in ["class", "data-count", "#title", ".main-title", ".card"] {
            assert!(
                tokens.contains(&(SyntaxToken::Attribute, attribute)),
                "{attribute}"
            );
        }
        for keyword in [
            "doctype", "if", "else", "each", "mixin", "+", "-", "=", "const", "in",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "{keyword}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#{")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "!{")));
        assert!(tokens.contains(&(SyntaxToken::Function, "card")));
        for string in ["partials/header", "\"lead\""] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        assert!(tokens.contains(&(SyntaxToken::Number, "42")));
        for comment in [
            "//- unbuffered comment",
            "// buffered comment",
            "<!-- html comment -->",
        ] {
            assert!(
                tokens.contains(&(SyntaxToken::Comment, comment)),
                "{comment}"
            );
        }
        assert_well_formed("pug", source);
    }

    #[test]
    fn ini_comments_sections_keys_and_strings_are_tokenized() {
        let source = r##"# comment
; other comment
[server]
host = "localhost"
port = 8080
path = 'C:\Program Files'
empty =
[client.ui]
theme = "dark"
"##;
        assert_eq!(language("ini"), Some("ini"));
        assert_eq!(language("properties"), Some("ini"));
        let tokens = tokens("ini", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "; other comment")));
        for section in ["server", "client.ui"] {
            assert!(tokens.contains(&(SyntaxToken::Type, section)), "{section}");
        }
        for property in ["host", "port", "path", "empty", "theme"] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, property)),
                "{property}"
            );
        }
        for string in ["\"localhost\"", "'C:\\Program Files'", "\"dark\""] {
            assert!(tokens.contains(&(SyntaxToken::String, string)), "{string}");
        }
        assert_well_formed("ini", source);
    }

    #[test]
    fn swift_keywords_types_functions_attributes_raw_strings_and_numbers() {
        let source = r##"// comment
@MainActor
final class Greeter {
    let name: String

    init(name: String) {
        self.name = name
    }

    func greet(loud: Bool = false) -> String {
        let raw = #"raw "text""#
        let block = """
            hello
        """
        let message = "hi \(name)"
        let count = 0x1f + 0b1010 + 1_000 + 1.5e3
        return if loud { message.uppercased() } else { message }
    }
}"##;
        assert_eq!(language("swift"), Some("swift"));
        let tokens = tokens("swift", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@MainActor")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "final")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "let")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "init")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "func")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "else")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Type, "String")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Bool")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet")));
        assert!(tokens.contains(&(SyntaxToken::Function, "uppercased")));
        assert!(tokens.contains(&(SyntaxToken::String, r##"#"raw "text""#"##)));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String && text.starts_with("\"\"\"") && text.ends_with("\"\"\"")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "\"hi \\(name)\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "false")));
        assert_well_formed("swift", source);
    }

    #[test]
    fn kotlin_keywords_types_functions_annotations_strings_and_numbers() {
        let source = r##"// comment
@Deprecated("old")
package demo

import kotlin.collections.List

data class Greeter(val name: String) {
    fun greet(loud: Boolean = false): String {
        val raw = """
            hello
        """
        val message = "hi $name"
        val mark = 'x'
        val count = 0x1f + 0b1010 + 1_000 + 1.5e3 + 42L
        return if (loud) message.uppercase() else message
    }
}"##;
        assert_eq!(language("kotlin"), Some("kotlin"));
        assert_eq!(language("kt"), Some("kotlin"));
        assert_eq!(language("kts"), Some("kotlin"));
        let tokens = tokens("kotlin", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@Deprecated")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "package")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "import")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "data")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "fun")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "val")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Type, "String")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Boolean")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet")));
        assert!(tokens.contains(&(SyntaxToken::Function, "uppercase")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String && text.starts_with("\"\"\"") && text.ends_with("\"\"\"")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "\"hi $name\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "42L")));
        assert!(tokens.contains(&(SyntaxToken::Number, "false")));
        assert_well_formed("kotlin", source);
    }

    #[test]
    fn java_keywords_types_annotations_strings_and_numbers_are_tokenized() {
        let source = r##"// comment
@Deprecated
package com.example;

import java.util.List;

public final class Greeter implements Runnable {
    private static final int MAX = 0x1f + 0b1010 + 1_000 + 1.5e3;

    public static void main(String[] args) {
        String block = """
            hello
            """;
        char mark = '\n';
        boolean ok = true;
        List<String> items = List.of("a");
        System.out.println(block, mark, ok, items);
    }
}"##;
        assert_eq!(language("java"), Some("java"));
        let tokens = tokens("java", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "@Deprecated")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "package")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "class")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "implements")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "public")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "final")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Greeter")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Runnable")));
        assert!(tokens.contains(&(SyntaxToken::Type, "String")));
        assert!(tokens.contains(&(SyntaxToken::Type, "int")));
        assert!(tokens.contains(&(SyntaxToken::Type, "void")));
        assert!(tokens.contains(&(SyntaxToken::Function, "main")));
        assert!(tokens.contains(&(SyntaxToken::Function, "println")));
        assert!(tokens.contains(&(SyntaxToken::Function, "of")));
        assert!(tokens.iter().any(|(kind, text)| {
            *kind == SyntaxToken::String && text.starts_with("\"\"\"") && text.ends_with("\"\"\"")
        }));
        assert!(tokens.contains(&(SyntaxToken::String, "'\\n'")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"a\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert_well_formed("java", source);
    }

    #[test]
    fn go_keywords_types_functions_comments_strings_runes_and_numbers_are_tokenized() {
        let source = r##"// comment
package main

import "fmt"

type Point struct {
    X, Y float64
    label string
}

func (p *Point) Move(dx, dy float64) Point {
    raw := `raw "text"
line`
    r := 'x'
    n := 0x1f + 0o17 + 0b1010 + 1_000 + 1.5e3 + 0x1p-2
    ok := true
    if len(raw) > 0 && ok {
        fmt.Println(raw, r, n)
    }
    return Point{}
}"##;
        assert_eq!(language("go"), Some("go"));
        assert_eq!(language("golang"), Some("go"));
        let tokens = tokens("go", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "package")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "struct")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "func")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "if")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Point")));
        assert!(tokens.contains(&(SyntaxToken::Type, "float64")));
        assert!(tokens.contains(&(SyntaxToken::Type, "string")));
        assert!(tokens.contains(&(SyntaxToken::Function, "Move")));
        assert!(tokens.contains(&(SyntaxToken::Function, "Println")));
        assert!(tokens.contains(&(SyntaxToken::Function, "len")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"fmt\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'x'")));
        assert!(tokens.contains(&(
            SyntaxToken::String,
            "`raw \"text\"\nline`"
        )));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0o17")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0b1010")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1_000")));
        assert!(tokens.contains(&(SyntaxToken::Number, "1.5e3")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1p-2")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert_well_formed("go", source);
    }

    #[test]
    fn rust_keywords_types_functions_macros_comments_and_literals_are_tokenized() {
        let source = r##"// comment
#[derive(Debug, Clone)]
struct Point { x: f64, y: f64 }
impl Point {
    fn new(x: f64, y: f64) -> Self {
        let hex = 0x1f_u32;
        let raw = r#"raw "quoted""#;
        let bytes = b"bytes";
        let ch = '\n';
        let life: &'static str = "text";
        let ok = true;
        println!("{}", hex);
        Self { x, y }
    }
}"##;
        assert_eq!(language("rust"), Some("rust"));
        assert_eq!(language("rs"), Some("rust"));
        let tokens = tokens("rust", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "// comment")));
        assert!(tokens.contains(&(
            SyntaxToken::Attribute,
            "#[derive(Debug, Clone)]"
        )));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "struct")));
        assert!(tokens.contains(&(SyntaxToken::Type, "Point")));
        assert!(tokens.contains(&(SyntaxToken::Type, "f64")));
        assert!(tokens.contains(&(SyntaxToken::Function, "new")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "Self")));
        assert!(tokens.contains(&(SyntaxToken::Number, "0x1f_u32")));
        assert!(tokens.contains(&(
            SyntaxToken::String,
            r##"r#"raw "quoted""#"##
        )));
        assert!(tokens.contains(&(SyntaxToken::String, "b\"bytes\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'\\n'")));
        assert!(tokens.contains(&(SyntaxToken::Type, "'static")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"text\"")));
        assert!(tokens.contains(&(SyntaxToken::Number, "true")));
        assert!(tokens.contains(&(SyntaxToken::Function, "println")));
        assert_well_formed("rust", source);
    }

    #[test]
    fn ansi_sgr_sequences_strip_escapes_and_carry_colors_and_decorations() {
        let source = "\x1b[31mred\x1b[0m plain \x1b[1;34mbold blue\x1b[22m \x1b[38;2;1;2;3mrgb\x1b[39m \x1b[4;9;3mstyle\x1b[0m";
        let (cleaned, spans) = ansi(source);
        assert_eq!(cleaned, "red plain bold blue rgb style");
        assert!(spans.iter().any(|span| {
            matches!(span.token, SyntaxToken::Ansi(style) if style.foreground == Some(0xcd3131))
                && &cleaned[span.start..span.end] == "red"
        }));
        assert!(spans.iter().any(|span| {
            matches!(span.token, SyntaxToken::Ansi(style) if style.bold
                && style.foreground == Some(0x2472c8))
                && &cleaned[span.start..span.end] == "bold blue"
        }));
        assert!(spans.iter().any(|span| {
            matches!(span.token, SyntaxToken::Ansi(style) if style.foreground == Some(0x010203))
                && &cleaned[span.start..span.end] == "rgb"
        }));
        assert!(spans.iter().any(|span| {
            matches!(span.token, SyntaxToken::Ansi(style) if style.italic && style.underline && style.strikethrough)
                && &cleaned[span.start..span.end] == "style"
        }));
        assert_well_formed_ansi(&spans, &cleaned);
    }

    fn assert_well_formed_ansi(spans: &[Span], text: &str) {
        let mut last = 0;
        for span in spans {
            assert!(span.start >= last);
            assert!(span.end <= text.len());
            assert!(text.is_char_boundary(span.start));
            assert!(text.is_char_boundary(span.end));
            last = span.end;
        }
    }

    #[test]
    fn html_namespaced_elements_and_unquoted_attributes_are_tokenized() {
        let source = "<svg:path data-id=abc aria-label=\"x\" disabled></svg:path>";
        let tokens = tokens("html", source);
        assert!(tokens.contains(&(SyntaxToken::Tag, "svg:path")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "data-id")));
        assert!(tokens.contains(&(SyntaxToken::String, "abc")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "aria-label")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"x\"")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "disabled")));
        assert_well_formed("html", source);
    }

    #[test]
    fn markdown_structure_inline_code_and_links_are_tokenized() {
        let source = "# Title\n\n> quote\n\n- item\n\n1. first\n\n- [x] done\n\n`code` [label](https://example.com)\n\n---\n\n```rust\nfn main() {}\n```";
        let tokens = tokens("markdown", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "#")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, ">")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "-")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "1.")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "[x]")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "---")));
        assert!(tokens.contains(&(SyntaxToken::String, "`code`")));
        assert!(tokens.contains(&(SyntaxToken::Function, "[label]")));
        assert!(tokens.contains(&(SyntaxToken::String, "(https://example.com)")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "```rust")));
        assert_well_formed("markdown", source);
    }

    #[test]
    fn markdown_emphasis_markers_and_inline_html_are_tokenized() {
        let source = "**bold** *italic* _under_ snake_case ~~gone~~ <span class=\"x\">hi</span>";
        let tokens = tokens("markdown", source);
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Keyword && matches!(*text, "**" | "*" | "~~"))
                .count(),
            6
        );
        assert!(tokens.contains(&(SyntaxToken::Keyword, "_")));
        assert!(tokens.contains(&(SyntaxToken::Tag, "span")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "class")));
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Keyword && *text == "_")
                .count(),
            1,
            "the underscore in snake_case stays plain: {tokens:?}"
        );
        assert_well_formed("markdown", source);
    }

    #[test]
    fn diff_metadata_lines_are_comments_while_hunks_and_changes_stay_colored() {
        let source = "diff --git a/a.rs b/a.rs\nindex 111..222 100644\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file\n";
        let tokens = tokens("diff", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "diff --git a/a.rs b/a.rs")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "index 111..222 100644")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "--- a/a.rs")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "+++ b/a.rs")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "@@ -1 +1 @@")));
        assert!(tokens.contains(&(SyntaxToken::Removed, "-old")));
        assert!(tokens.contains(&(SyntaxToken::Added, "+new")));
        assert!(tokens.contains(&(
            SyntaxToken::Comment,
            "\\ No newline at end of file"
        )));
        assert_well_formed("diff", source);
    }

    #[test]
    fn sql_operators_are_highlighted_without_breaking_comments_or_params() {
        let source = "select * from t where a = 1 and b <> 2 or c >= 3 and d || e -- note\nand f::int > 0 and g ->> 'k'";
        let tokens = tokens("sql", source);
        for operator in ["=", "<>", ">=", "||", "::", "->>", ">"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, operator)),
                "SQL operator {operator} is colored: {tokens:?}"
            );
        }
        assert!(tokens.contains(&(SyntaxToken::Comment, "-- note")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "int")));
        assert_well_formed("sql", source);
    }

    #[test]
    fn yaml_flow_mapping_keys_are_properties() {
        let source = "obj: { nested: true, other: false, \"quoted\": null }";
        let tokens = tokens("yaml", source);
        for key in ["nested", "other", "\"quoted\""] {
            assert!(
                tokens.contains(&(SyntaxToken::Property, key)),
                "flow mapping key {key} is colored: {tokens:?}"
            );
        }
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Property && *text == "obj")
                .count(),
            1
        );
        assert!(tokens.contains(&(SyntaxToken::Keyword, "true")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "null")));
        assert_well_formed("yaml", source);
    }

    #[test]
    fn yaml_quoted_keys_are_properties_but_quoted_values_stay_strings() {
        let source = "\"quoted key\": \"value\"\n'single key': 'value'\n{ nested: {\"flow key\": true} }";
        let tokens = tokens("yaml", source);
        assert!(tokens.contains(&(SyntaxToken::Property, "\"quoted key\"")));
        assert!(tokens.contains(&(SyntaxToken::Property, "'single key'")));
        assert!(tokens.contains(&(SyntaxToken::Property, "\"flow key\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "\"value\"")));
        assert!(tokens.contains(&(SyntaxToken::String, "'value'")));
        assert_well_formed("yaml", source);
    }

    #[test]
    fn yaml_booleans_null_document_markers_and_directives_are_tokenized() {
        let source = "%YAML 1.2\n---\nname: Vela\nenabled: true\nmissing: null\ntilde: ~\nitems:\n  - false\n  - yes\n...\n";
        let tokens = tokens("yaml", source);
        for keyword in ["%YAML", "---", "true", "null", "~", "false", "yes", "..."] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "YAML literal {keyword} is colored: {tokens:?}"
            );
        }
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Property && *text == "name")
                .count(),
            1
        );
        assert!(
            !tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::Keyword && *text == "name"),
            "keys are not literals"
        );
        assert_well_formed("yaml", source);
    }

    #[test]
    fn sql_parameters_dollar_quotes_functions_and_types_are_tokenized() {
        let source = "select count(*), price::numeric, $1, :name, @account, ?\nfrom orders where note = $$raw ' text$$ and tag = $fn$body$fn$ -- note";
        let tokens = tokens("sql", source);
        assert!(tokens.contains(&(SyntaxToken::Keyword, "select")));
        assert!(tokens.contains(&(SyntaxToken::Function, "count")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "numeric")));
        assert!(tokens.contains(&(SyntaxToken::Property, "$1")));
        assert!(tokens.contains(&(SyntaxToken::Property, ":name")));
        assert!(tokens.contains(&(SyntaxToken::Property, "@account")));
        assert!(tokens.contains(&(SyntaxToken::Property, "?")));
        assert!(tokens.contains(&(SyntaxToken::String, "$$raw ' text$$")));
        assert!(tokens.contains(&(SyntaxToken::String, "$fn$body$fn$")));
        assert!(tokens.contains(&(SyntaxToken::Comment, "-- note")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "::")));
        assert_well_formed("sql", source);
    }

    #[test]
    fn bash_heredocs_ansi_strings_assignments_functions_and_hash_words_are_tokenized() {
        let source = "#!/usr/bin/env bash\nname=\"world\"\ngreet() {\n  cat <<-EOF\n\thello $name\n\tEOF\n  echo \"hi ${name#pre}\" # note\n  printf foo#bar\n}\nvalue=$'ansi\\nstring'\n";
        let tokens = tokens("bash", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "#!/usr/bin/env bash")));
        assert!(tokens.contains(&(SyntaxToken::Property, "name")));
        assert!(tokens.contains(&(SyntaxToken::Property, "value")));
        assert!(tokens.contains(&(SyntaxToken::Function, "greet")));
        assert!(
            tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::String && text.contains("hello $name")),
            "heredoc bodies are one string span: {tokens:?}"
        );
        assert!(tokens.contains(&(SyntaxToken::Comment, "# note")));
        assert!(
            !tokens.iter().any(|(kind, text)| *kind == SyntaxToken::Comment && *text == "#bar"),
            "a hash inside a word is not a shell comment"
        );
        assert!(
            tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::String && text.starts_with("$'")),
            "ANSI-C quoted strings are string tokens"
        );
        assert_well_formed("bash", source);
    }

    #[test]
    fn css_selectors_pseudos_attributes_and_important_are_tokenized() {
        let source = ".card:hover, #main[data-state=\"open\"] {\n  color: #10b981;\n  background: url(https://example.com/a.png);\n  width: 2px !important;\n}\na:hover span { color: red; }";
        let tokens = tokens("css", source);
        assert!(tokens.contains(&(SyntaxToken::Tag, ".card")));
        assert!(tokens.contains(&(SyntaxToken::Tag, "hover")));
        assert!(tokens.contains(&(SyntaxToken::Tag, "#main")));
        assert!(tokens.contains(&(SyntaxToken::Attribute, "data-state")));
        assert!(tokens.contains(&(SyntaxToken::Property, "color")));
        assert!(tokens.contains(&(SyntaxToken::Number, "#10b981")));
        assert!(tokens.contains(&(SyntaxToken::Property, "background")));
        assert!(tokens.contains(&(SyntaxToken::Function, "url")));
        assert!(tokens.contains(&(SyntaxToken::Number, "2px")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "!important")));
        assert!(
            !tokens.iter().any(|(kind, text)| *kind == SyntaxToken::Property && *text == "https"),
            "URL schemes inside declaration values stay plain"
        );
        assert!(
            !tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::Property && *text == "a"),
            "nested element selectors are not mistaken for properties"
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Tag && *text == "hover")
                .count(),
            2
        );
        assert_well_formed("css", source);
    }

    #[test]
    fn script_shebang_decorators_private_fields_and_modern_keywords_are_tokenized() {
        let source = "#!/usr/bin/env node\n@sealed\n@Component({ selector: \"x\" })\nclass Box {\n  #count = 0;\n  abstract override accessor value: number;\n}\ntype Keys = keyof Box;\nconst usingValue = using;\n";
        let tokens = tokens("typescript", source);
        assert!(tokens.contains(&(SyntaxToken::Comment, "#!/usr/bin/env node")));
        assert!(tokens.contains(&(SyntaxToken::Function, "sealed")));
        assert!(tokens.contains(&(SyntaxToken::Function, "Component")));
        assert!(tokens.contains(&(SyntaxToken::Property, "#count")));
        for keyword in ["abstract", "override", "accessor", "keyof"] {
            assert!(
                tokens.contains(&(SyntaxToken::Keyword, keyword)),
                "modern TypeScript keyword {keyword} is colored: {tokens:?}"
            );
        }
        assert_well_formed("typescript", source);
    }

    #[test]
    fn python_decorators_and_fstrings_tokenize_interpolated_expressions() {
        let source = "@app.route\ndef run(count):\n    return f\"value {count + 1} {name.upper()}\" + rf\"{count}\n";
        let tokens = tokens("python", source);
        assert!(tokens.contains(&(SyntaxToken::Function, "app")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "def")));
        assert!(tokens.contains(&(SyntaxToken::Keyword, "return")));
        assert!(
            tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Number && *text == "1")
                .count()
                >= 1,
            "f-string interpolation expressions are tokenized: {tokens:?}"
        );
        assert!(
            tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::String && text.starts_with("f\"")),
            "the literal prefix stays part of the f-string token"
        );
        assert_well_formed("python", source);
    }

    #[test]
    fn scss_variables_html_entities_and_yaml_anchors_are_tokenized() {
        let scss = "$brand: #fff;\n.x { color: $brand; --gap: 4px; }\n@mixin card { }";
        let scss_tokens = tokens("css", scss);
        assert_eq!(
            scss_tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Property && *text == "$brand")
                .count(),
            2
        );
        assert!(scss_tokens.contains(&(SyntaxToken::Property, "--gap")));
        assert!(scss_tokens.contains(&(SyntaxToken::Keyword, "@mixin")));
        assert_well_formed("scss", scss);

        let less = "@brand-color: red;\n.x { color: @brand-color; }";
        let less_tokens = tokens("css", less);
        assert_eq!(
            less_tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Property && *text == "@brand-color")
                .count(),
            2
        );
        assert_well_formed("scss", less);

        let html_text = "<p title=\"a &amp; b\">A &amp; B &copy;</p>";
        let html_tokens = tokens("html", html_text);
        assert_eq!(
            html_tokens
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::String
                    && (*text == "&amp;" || *text == "&copy;"))
                .count(),
            2
        );
        assert_well_formed("html", html_text);

        let yaml_text = "defaults: &defaults\n  name: Vela\ncopy: *defaults\nversion: !!str 1\nliteral: |\n  &not-an-anchor";
        let yaml_tokens = tokens("yaml", yaml_text);
        assert!(yaml_tokens.contains(&(SyntaxToken::Property, "&defaults")));
        assert!(yaml_tokens.contains(&(SyntaxToken::Property, "*defaults")));
        assert!(yaml_tokens.contains(&(SyntaxToken::Keyword, "!!str")));
        assert!(
            !yaml_tokens
                .iter()
                .any(|(kind, text)| *kind == SyntaxToken::Property && *text == "&not-an-anchor"),
            "block scalar content stays literal"
        );
        assert_well_formed("yaml", yaml_text);
    }

    #[test]
    fn fixture_snippets_cover_numeric_boundaries_regex_variables_and_css_selectors() {
        let script = tokens(
            "javascript",
            "const diff = a-1; const re = /ab+c/gi;\nif (ok) return /x/;",
        );
        assert!(script.contains(&(SyntaxToken::Number, "1")));
        assert!(script.contains(&(SyntaxToken::String, "/ab+c/gi")));
        assert!(script.contains(&(SyntaxToken::String, "/x/")));
        assert_well_formed(
            "javascript",
            "const diff = a-1; const re = /ab+c/gi;\nif (ok) return /x/;",
        );

        let python = tokens("python", "value=x-1\n# note");
        assert!(python.contains(&(SyntaxToken::Number, "1")));
        assert!(python.contains(&(SyntaxToken::Comment, "# note")));
        assert_well_formed("python", "value=x-1\n# note");

        let bash = tokens("bash", "echo ${HOME} $# $?");
        assert!(bash.contains(&(SyntaxToken::Property, "${HOME}")));
        assert!(bash.contains(&(SyntaxToken::Property, "$#")));
        assert!(bash.contains(&(SyntaxToken::Property, "$?")));
        assert_well_formed("bash", "echo ${HOME} $# $?");

        let css = tokens("css", "a:hover { color: red; }");
        assert!(!css.contains(&(SyntaxToken::Property, "a")));
        assert!(css.contains(&(SyntaxToken::Property, "color")));
        assert_well_formed("css", "a:hover { color: red; }");

        let html = tokens("html", "<div data-id=\"x\">y</div>");
        assert!(html.contains(&(SyntaxToken::Attribute, "data-id")));
        assert_well_formed("html", "<div data-id=\"x\">y</div>");

        let template = tokens(
            "javascript",
            "const msg = `total ${count + 1} items ${format(name)}`;",
        );
        assert!(template.contains(&(SyntaxToken::Number, "1")));
        assert!(template.contains(&(SyntaxToken::Function, "format")));
        assert!(template.iter().any(|(kind, text)| *kind == SyntaxToken::String && text.starts_with('`')));
        assert_well_formed(
            "javascript",
            "const msg = `total ${count + 1} items ${format(name)}`;",
        );

        let custom = tokens(
            "css",
            ":root { --brand-color: #fff; color: var(--brand-color); }",
        );
        assert!(custom.contains(&(SyntaxToken::Property, "--brand-color")));
        assert_eq!(
            custom
                .iter()
                .filter(|(kind, text)| *kind == SyntaxToken::Property && *text == "--brand-color")
                .count(),
            1,
            "custom properties are marked on declarations, not var() references"
        );
        assert_well_formed(
            "css",
            ":root { --brand-color: #fff; color: var(--brand-color); }",
        );

        let embedded = tokens(
            "html",
            "<script>const n = 1;</script><style>body { color: red; }</style>",
        );
        assert!(embedded.contains(&(SyntaxToken::Keyword, "const")));
        assert!(embedded.contains(&(SyntaxToken::Number, "1")));
        assert!(embedded.contains(&(SyntaxToken::Property, "color")));
        assert_well_formed(
            "html",
            "<script>const n = 1;</script><style>body { color: red; }</style>",
        );
    }

    #[test]
    fn yaml_block_scalars_and_sql_quoted_identifiers_are_tokenized() {
        let yaml_text = "description: |\n  first # literal\n  count: 42\nnote: >-\n  folded\nname: next\n# comment";
        let yaml = tokens("yaml", yaml_text);
        assert!(yaml.contains(&(SyntaxToken::Property, "description")));
        assert!(yaml.contains(&(SyntaxToken::String, "|")));
        assert!(yaml.contains(&(SyntaxToken::String, "first # literal")));
        assert!(
            yaml.contains(&(SyntaxToken::String, "count: 42")),
            "block scalar content is one string span, not key/number tokens: {yaml:?}"
        );
        assert!(yaml.contains(&(SyntaxToken::String, ">-")));
        assert!(yaml.contains(&(SyntaxToken::String, "folded")));
        assert!(yaml.contains(&(SyntaxToken::Property, "name")));
        assert!(yaml.contains(&(SyntaxToken::Comment, "# comment")));
        assert!(
            !yaml.iter().any(|(kind, text)| *kind == SyntaxToken::Property && *text == "count"),
            "keys inside a block scalar stay content"
        );
        assert!(
            !yaml.iter().any(|(kind, text)| *kind == SyntaxToken::Number && *text == "42"),
            "numbers inside a block scalar stay content"
        );
        assert_well_formed("yaml", yaml_text);

        let sql_text = "select \"display\"\"name\", `tick``name`, [bracket]]name], 'it''s' from users";
        let sql = tokens("sql", sql_text);
        assert!(sql.contains(&(SyntaxToken::Keyword, "select")));
        assert!(sql.contains(&(SyntaxToken::Property, "\"display\"\"name\"")));
        assert!(sql.contains(&(SyntaxToken::Property, "`tick``name`")));
        assert!(sql.contains(&(SyntaxToken::Property, "[bracket]]name]")));
        assert!(sql.contains(&(SyntaxToken::String, "'it''s'")));
        assert!(sql.contains(&(SyntaxToken::Keyword, "from")));
        assert_well_formed("sql", sql_text);
    }

    #[test]
    fn css_html_python_bash_yaml_sql_and_diff_cover_common_tokens() {
        assert_eq!(
            tokens("css", "body { color: #fff; /* note */ }"),
            vec![
                (SyntaxToken::Property, "color"),
                (SyntaxToken::Number, "#fff"),
                (SyntaxToken::Comment, "/* note */")
            ]
        );
        let html = tokens("html", "<a href=\"https://example.com\">link</a>");
        assert!(html.contains(&(SyntaxToken::Tag, "a")));
        assert!(html.contains(&(SyntaxToken::Attribute, "href")));
        assert!(html.contains(&(SyntaxToken::String, "\"https://example.com\"")));
        let python = tokens("python", "def run(x):\n    # note\n    return len(x)");
        assert!(python.contains(&(SyntaxToken::Keyword, "def")));
        assert!(python.contains(&(SyntaxToken::Function, "run")));
        assert!(python.contains(&(SyntaxToken::Function, "len")));
        assert!(python.contains(&(SyntaxToken::Comment, "# note")));
        let bash = tokens("bash", "export PATH=$PATH\nprintf '%s' \"$PATH\"\n# note");
        assert!(bash.contains(&(SyntaxToken::Keyword, "export")));
        assert!(bash.contains(&(SyntaxToken::Function, "printf")));
        assert!(bash.iter().any(|(kind, _)| *kind == SyntaxToken::Property));
        let yaml = tokens("yaml", "name: Vela\ncount: 2");
        assert!(yaml.contains(&(SyntaxToken::Property, "name")));
        assert!(yaml.contains(&(SyntaxToken::Number, "2")));
        let sql = tokens("sql", "select * from users where id = 1 -- note");
        assert!(sql.contains(&(SyntaxToken::Keyword, "select")));
        assert!(sql.contains(&(SyntaxToken::Number, "1")));
        assert!(sql.contains(&(SyntaxToken::Comment, "-- note")));
        let diff = tokens("diff", "@@ -1 +1 @@\n-old\n+new");
        assert!(diff.contains(&(SyntaxToken::Keyword, "@@ -1 +1 @@")));
        assert!(diff.contains(&(SyntaxToken::Removed, "-old")));
        assert!(diff.contains(&(SyntaxToken::Added, "+new")));
        for (language, text) in [
            ("css", "body { color: #fff; /* note */ }"),
            ("html", "<a href=\"https://example.com\">link</a>"),
            ("python", "def run(x):\n    # note\n    return x"),
            ("bash", "export PATH=$PATH\n# note"),
            ("yaml", "name: Vela\ncount: 2"),
            ("sql", "select * from users where id = 1 -- note"),
            ("diff", "@@ -1 +1 @@\n-old\n+new"),
        ] {
            assert_well_formed(language, text);
        }
    }
}
