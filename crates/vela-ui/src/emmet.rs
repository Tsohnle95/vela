//! Native Emmet-style abbreviation expansion for HTML and stylesheets.
//!
//! The editor only has a compact snippet model: tab stops are flattened on
//! insertion and the caret lands on the final `$0` stop. Expansion syntax covers
//! the reference cases used by the desktop editor: doctype, tags, ids/classes,
//! text, child/sibling chains, `*N` repetition and common CSS number shorthands.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EmmetExpansion {
    pub abbr: String,
    /// Byte offset of the abbreviation start within the scanned line prefix.
    pub start: usize,
    pub snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EmmetInsertion {
    pub text: String,
    /// Byte offset of the caret within `text`.
    pub caret: usize,
}

const HTML_TAGS: &[&str] = &[
    "body",
    "head",
    "html",
    "address",
    "blockquote",
    "dd",
    "div",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "nav",
    "menu",
    "dl",
    "dt",
    "fieldset",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "iframe",
    "noframes",
    "object",
    "ol",
    "p",
    "ul",
    "applet",
    "center",
    "dir",
    "hr",
    "pre",
    "a",
    "abbr",
    "acronym",
    "area",
    "b",
    "base",
    "basefont",
    "bdo",
    "big",
    "br",
    "button",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "del",
    "dfn",
    "em",
    "font",
    "i",
    "img",
    "input",
    "ins",
    "isindex",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "map",
    "meta",
    "noscript",
    "optgroup",
    "option",
    "param",
    "q",
    "s",
    "samp",
    "script",
    "select",
    "small",
    "span",
    "strike",
    "strong",
    "style",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "tt",
    "u",
    "var",
    "canvas",
    "main",
    "figure",
    "plaintext",
    "figcaption",
    "hgroup",
    "details",
    "summary",
    "lorem",
];

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const HTML_SKELETON: &str = "<!DOCTYPE html>\n<html>\n<head>\n\t<meta charset=\"UTF-8\">\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\t<title>${1:Document}</title>\n</head>\n<body>\n\t${0}\n</body>\n</html>";

fn is_abbr_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '!' | '.'
                | '-'
                | '_'
                | ':'
                | '$'
                | '#'
                | '@'
                | '%'
                | '^'
                | '*'
                | '+'
                | '>'
                | '~'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '='
                | '/'
                | '|'
                | ','
                | '\\'
        )
}

fn language_kind(language: &str) -> Option<bool> {
    match language {
        "html" => Some(false),
        "css" | "scss" | "less" | "sass" => Some(true),
        _ => None,
    }
}

pub(super) fn emmet_expansion_at(before_caret: &str, language: &str) -> Option<EmmetExpansion> {
    let stylesheet = language_kind(language)?;
    let mut start = before_caret.len();
    while start > 0 {
        let ch = before_caret[..start].chars().next_back()?;
        if !is_abbr_char(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let abbr = &before_caret[start..];
    if abbr.is_empty() {
        return None;
    }
    let snippet = if stylesheet {
        expand_stylesheet(abbr)?
    } else {
        expand_markup(abbr)?
    };
    Some(EmmetExpansion {
        abbr: abbr.to_owned(),
        start,
        snippet,
    })
}

#[derive(Clone, Debug)]
struct Node {
    tag: String,
    id: Option<String>,
    classes: Vec<String>,
    text: Option<String>,
    repeat: usize,
    children: Vec<Node>,
}

fn read_ident(chars: &[char], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < chars.len()
        && (chars[*pos].is_ascii_alphanumeric() || matches!(chars[*pos], '-' | '_'))
    {
        *pos += 1;
    }
    (*pos > start).then(|| chars[start..*pos].iter().collect())
}

fn read_text(chars: &[char], pos: &mut usize) -> Option<String> {
    let mut text = String::new();
    while *pos < chars.len() {
        match chars[*pos] {
            '}' => {
                *pos += 1;
                return Some(text);
            }
            '\\' if *pos + 1 < chars.len() => {
                *pos += 1;
                text.push(chars[*pos]);
                *pos += 1;
            }
            ch => {
                text.push(ch);
                *pos += 1;
            }
        }
    }
    None
}

fn parse_element(chars: &[char], pos: &mut usize) -> Option<Node> {
    let start = *pos;
    while *pos < chars.len() && (chars[*pos].is_ascii_alphanumeric() || chars[*pos] == '-') {
        *pos += 1;
    }
    let mut tag = chars[start..*pos].iter().collect::<String>();
    if tag.is_empty() {
        if matches!(chars.get(*pos), Some('.') | Some('#')) {
            tag = "div".into();
        } else {
            return None;
        }
    }
    let lower = tag.to_lowercase();
    if !HTML_TAGS.contains(&lower.as_str()) {
        return None;
    }
    let mut node = Node {
        tag: lower,
        id: None,
        classes: Vec::new(),
        text: None,
        repeat: 1,
        children: Vec::new(),
    };
    loop {
        match chars.get(*pos) {
            Some('#') => {
                *pos += 1;
                node.id = Some(read_ident(chars, pos)?);
            }
            Some('.') => {
                *pos += 1;
                node.classes.push(read_ident(chars, pos)?);
            }
            Some('{') => {
                *pos += 1;
                node.text = Some(read_text(chars, pos)?);
            }
            Some('*') => {
                *pos += 1;
                let digits = read_ident(chars, pos)?;
                node.repeat = digits.parse().ok().filter(|repeat| *repeat > 0)?;
            }
            _ => break,
        }
    }
    Some(node)
}

fn parse_node(chars: &[char], pos: &mut usize) -> Option<Node> {
    let mut node = parse_element(chars, pos)?;
    if chars.get(*pos) == Some(&'>') {
        *pos += 1;
        node.children = parse_list(chars, pos)?;
    }
    Some(node)
}

fn parse_list(chars: &[char], pos: &mut usize) -> Option<Vec<Node>> {
    let mut nodes = vec![parse_node(chars, pos)?];
    while chars.get(*pos) == Some(&'+') {
        *pos += 1;
        nodes.push(parse_node(chars, pos)?);
    }
    Some(nodes)
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push('\t');
    }
}

fn escape_text(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        if ch == '$' {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn render_slot(out: &mut String, slots: &mut Vec<()>) {
    out.push('\u{0}');
    out.push_str(&slots.len().to_string());
    out.push('\u{0}');
    slots.push(());
}

fn render_attrs(node: &Node, out: &mut String) {
    if let Some(id) = &node.id {
        out.push_str(" id=\"");
        out.push_str(id);
        out.push('"');
    }
    if !node.classes.is_empty() {
        out.push_str(" class=\"");
        out.push_str(&node.classes.join(" "));
        out.push('"');
    }
}

fn render_single(node: &Node, depth: usize, out: &mut String, slots: &mut Vec<()>) {
    out.push('<');
    out.push_str(&node.tag);
    render_attrs(node, out);
    if node.tag == "a" && node.id.is_none() && node.classes.is_empty() {
        out.push_str(" href=\"${1}\"");
    }
    out.push('>');
    if VOID_TAGS.contains(&node.tag.as_str()) {
        return;
    }
    if let Some(text) = &node.text {
        if node.tag == "title" {
            out.push_str("${1:");
            out.push_str(&escape_text(text));
            out.push_str("}</title>");
            return;
        }
        out.push_str(&escape_text(text));
        out.push_str("</");
        out.push_str(&node.tag);
        out.push('>');
        render_slot(out, slots);
        return;
    }
    if node.children.is_empty() {
        render_slot(out, slots);
    } else {
        for child in &node.children {
            for _ in 0..child.repeat {
                out.push('\n');
                indent(out, depth + 1);
                render_single(child, depth + 1, out, slots);
            }
        }
        out.push('\n');
        indent(out, depth);
    }
    out.push_str("</");
    out.push_str(&node.tag);
    out.push('>');
}

fn finalize(mut out: String, slots: usize) -> String {
    for index in 0..slots {
        let token = format!("\u{0}{index}\u{0}");
        let replacement = if index + 1 == slots {
            "${0}".to_owned()
        } else {
            format!("${{{}}}", index + 1)
        };
        out = out.replace(&token, &replacement);
    }
    if slots == 0 && !out.contains("${") {
        out.push_str("${0}");
    }
    out
}

fn expand_markup(abbr: &str) -> Option<String> {
    if abbr == "!" {
        return Some(HTML_SKELETON.to_owned());
    }
    if abbr.starts_with('!') {
        return None;
    }
    let chars = abbr.chars().collect::<Vec<_>>();
    let mut pos = 0;
    let nodes = parse_list(&chars, &mut pos)?;
    if pos != chars.len() || nodes.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut slots = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        for _ in 0..node.repeat {
            render_single(node, 0, &mut out, &mut slots);
            out.push('\n');
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    Some(finalize(out, slots.len()))
}

fn property_for(abbr: &str) -> Option<(&'static str, bool)> {
    match abbr {
        "m" => Some(("margin", false)),
        "mt" => Some(("margin-top", false)),
        "mr" => Some(("margin-right", false)),
        "mb" => Some(("margin-bottom", false)),
        "ml" => Some(("margin-left", false)),
        "p" => Some(("padding", false)),
        "pt" => Some(("padding-top", false)),
        "pr" => Some(("padding-right", false)),
        "pb" => Some(("padding-bottom", false)),
        "pl" => Some(("padding-left", false)),
        "w" => Some(("width", false)),
        "h" => Some(("height", false)),
        "mw" => Some(("max-width", false)),
        "mh" => Some(("max-height", false)),
        "fs" => Some(("font-size", false)),
        "lh" => Some(("line-height", true)),
        "fw" => Some(("font-weight", true)),
        "br" => Some(("border-radius", false)),
        "c" => Some(("color", false)),
        "bg" => Some(("background", false)),
        "d" => Some(("display", true)),
        "z" => Some(("z-index", true)),
        "op" => Some(("opacity", true)),
        "g" => Some(("gap", false)),
        _ => None,
    }
}

fn expand_stylesheet(abbr: &str) -> Option<String> {
    let mut split = None;
    for (index, ch) in abbr.char_indices() {
        if ch.is_ascii_digit() {
            split = Some(index);
            break;
        }
    }
    let index = split?;
    let property = property_for(&abbr[..index])?;
    let value = &abbr[index..];
    let number_end = value
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-'))
        .unwrap_or(value.len());
    let number = &value[..number_end];
    if number.is_empty() || number == "." || number.parse::<f64>().is_err() {
        return None;
    }
    let unit = &value[number_end..];
    if !unit.is_empty()
        && !matches!(
            unit,
            "px" | "em" | "rem" | "%" | "vh" | "vw" | "vmin" | "vmax"
        )
    {
        return None;
    }
    let suffix = if unit.is_empty() && !property.1 {
        "px"
    } else {
        unit
    };
    Some(format!("{}: {number}{suffix};${{0}}", property.0))
}

/// Flatten snippet tab stops for insertion. `${0}` becomes the caret; numbered
/// stops insert their default text (or nothing) in order. Escaped dollars become
/// literal dollars.
pub(super) fn emmet_insertion(snippet: &str) -> EmmetInsertion {
    let chars = snippet.chars().collect::<Vec<_>>();
    let mut text = String::new();
    let mut caret = None;
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '\\' if chars.get(index + 1) == Some(&'$') => {
                text.push('$');
                index += 2;
            }
            '$' if chars.get(index + 1) == Some(&'{') => {
                let mut end = index + 2;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end >= chars.len() {
                    text.push('$');
                    index += 1;
                    continue;
                }
                let inner = chars[index + 2..end].iter().collect::<String>();
                let (number, default) = inner
                    .split_once(':')
                    .map_or((inner.as_str(), None), |(number, default)| {
                        (number, Some(default))
                    });
                if number == "0" {
                    caret = Some(text.len());
                } else if let Some(default) = default {
                    text.push_str(default);
                }
                index = end + 1;
            }
            ch => {
                text.push(ch);
                index += 1;
            }
        }
    }
    let caret = caret.unwrap_or(text.len());
    EmmetInsertion { text, caret }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(before: &str, language: &str) -> Option<String> {
        emmet_expansion_at(before, language).map(|found| found.snippet)
    }

    #[test]
    fn expands_the_reference_html_cases() {
        let doctype = emmet_expansion_at("!", "html").unwrap();
        assert_eq!(doctype.abbr, "!");
        assert_eq!(doctype.start, 0);
        assert!(doctype.snippet.contains("<!DOCTYPE html>"));
        assert!(doctype.snippet.contains("<title>${1:Document}</title>"));
        assert!(doctype.snippet.contains("<body>\n\t${0}\n</body>"));

        let found = emmet_expansion_at("    ul>li*3", "html").unwrap();
        assert_eq!(found.abbr, "ul>li*3");
        assert_eq!(found.start, 4);
        assert_eq!(
            found.snippet,
            "<ul>\n\t<li>${1}</li>\n\t<li>${2}</li>\n\t<li>${0}</li>\n</ul>"
        );
        assert!(
            snippet("title{Hello}", "html")
                .unwrap()
                .contains("<title>${1:Hello}</title>")
        );
        assert_eq!(snippet("div", "html").unwrap(), "<div>${0}</div>");
        assert_eq!(snippet("a", "html").unwrap(), "<a href=\"${1}\">${0}</a>");
        assert_eq!(snippet("p", "html").unwrap(), "<p>${0}</p>");
        assert_eq!(
            snippet("p.red", "html").unwrap(),
            "<p class=\"red\">${0}</p>"
        );
        assert_eq!(
            snippet("div>p", "html").unwrap(),
            "<div>\n\t<p>${0}</p>\n</div>"
        );
        assert_eq!(
            snippet("ul>li", "html").unwrap(),
            "<ul>\n\t<li>${0}</li>\n</ul>"
        );
    }

    #[test]
    fn rejects_noise_existing_tags_and_unknown_languages() {
        for before in [
            "mydiv",
            "hello",
            "abc.",
            "xdiv",
            "<main>",
            "<div>",
            "  <section>",
            "!x",
            "title=\"foo",
        ] {
            assert_eq!(snippet(before, "html"), None, "{before:?}");
        }
        assert_eq!(snippet("div", "plaintext"), None);
        assert_eq!(snippet("!", "javascript"), None);
        assert_eq!(snippet("!", "typescript"), None);
    }

    #[test]
    fn expands_reference_stylesheet_shorthands() {
        assert_eq!(snippet("m10", "css").unwrap(), "margin: 10px;${0}");
        assert_eq!(snippet("p10", "scss").unwrap(), "padding: 10px;${0}");
        assert_eq!(snippet("w50%", "less").unwrap(), "width: 50%;${0}");
        assert_eq!(snippet("lh1.5", "css").unwrap(), "line-height: 1.5;${0}");
        assert_eq!(snippet("hello", "css"), None);
        assert_eq!(snippet("abc:d", "less"), None);
        assert_eq!(snippet("zz10", "css"), None);
    }

    #[test]
    fn escapes_text_dollars_and_scans_only_the_abbreviation() {
        let found = emmet_expansion_at("div{a\\$b}", "html").unwrap();
        assert!(found.snippet.contains("<div>a\\$b</div>"));
        let found = emmet_expansion_at("x div", "html").unwrap();
        assert_eq!(found.abbr, "div");
        assert_eq!(found.start, 2);
    }

    #[test]
    fn insertion_flattens_tab_stops_and_places_the_caret() {
        let div = emmet_insertion("<div>${0}</div>");
        assert_eq!(div.text, "<div></div>");
        assert_eq!(div.caret, 5);

        let list =
            emmet_insertion("<ul>\n\t<li>${1}</li>\n\t<li>${2}</li>\n\t<li>${0}</li>\n</ul>");
        assert_eq!(
            list.text,
            "<ul>\n\t<li></li>\n\t<li></li>\n\t<li></li>\n</ul>"
        );
        assert_eq!(list.caret, list.text.rfind("<li>").unwrap() + 4);

        let link = emmet_insertion("<a href=\"${1}\">${0}</a>");
        assert_eq!(link.text, "<a href=\"\"></a>");
        assert_eq!(link.caret, "<a href=\"\">".len());

        let doctype = emmet_insertion(HTML_SKELETON);
        assert!(doctype.text.contains("<title>Document</title>"));
        assert_eq!(&doctype.text[doctype.caret..doctype.caret + 4], "\n</b");

        let escaped = emmet_insertion("<div>a\\$b</div>");
        assert_eq!(escaped.text, "<div>a$b</div>");
    }
}
