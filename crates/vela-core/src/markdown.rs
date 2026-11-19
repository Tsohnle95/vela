//! Streaming-friendly Markdown block boundary detection.
//!
//! Vela renders a transcript as rows. Parsing every historical message again
//! on every streamed token would make cost grow with conversation length, so
//! this module only finds top-level block boundaries. Completed blocks keep
//! stable identities; a streaming block is reparsed from its start.

pub use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Tag, TagEnd};
use pulldown_cmark::CowStr;
use pulldown_cmark::{Options, Parser};
use std::ops::Range;
use std::collections::HashMap;
use std::sync::Arc;

/// An immutable, presentation-independent Markdown syntax tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Container(Tag<'static>, Vec<Node>),
    Leaf(Event<'static>),
    /// Source-backed overflow presentation. It carries no parsed links or
    /// provisional repairs and never allocates a second copy of the source.
    PlainText(Arc<str>),
}

pub fn tree(events: &mut impl Iterator<Item = Event<'static>>) -> Vec<Node> {
    let mut nodes = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::Start(tag) => {
                let children = tree(events);
                let children = if matches!(tag, Tag::Item) {
                    list_item_paragraphs(children)
                } else {
                    children
                };
                nodes.push(Node::Container(tag, children));
            }
            Event::End(_) => break,
            event => nodes.push(Node::Leaf(event)),
        }
    }
    nodes
}

// CommonMark omits paragraph tags in tight lists. Keep each inline run in
// one paragraph so spans, whitespace and punctuation share a line layout.
fn list_item_paragraphs(children: Vec<Node>) -> Vec<Node> {
    let mut blocks = Vec::with_capacity(children.len());
    let mut inline = Vec::new();
    for child in children {
        if matches!(
            child,
            Node::Container(Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }, _)
                | Node::Leaf(Event::Text(_) | Event::Code(_) | Event::InlineMath(_)
                    | Event::InlineHtml(_) | Event::FootnoteReference(_)
                    | Event::SoftBreak | Event::HardBreak)
        ) {
            inline.push(child);
        } else {
            if !inline.is_empty() {
                blocks.push(Node::Container(Tag::Paragraph, std::mem::take(&mut inline)));
            }
            blocks.push(child);
        }
    }
    if !inline.is_empty() {
        blocks.push(Node::Container(Tag::Paragraph, inline));
    }
    blocks
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
}

/// Balanced CommonMark/GFM events for one independently cached Markdown block.
/// Soft breaks remain explicit so the UI can match Marked's `breaks: true`.
pub fn events(text: &str) -> Vec<Event<'static>> {
    let protected = protect_autolinks(text);
    let parsed = Parser::new_ext(&protected, options())
        .map(Event::into_static)
        .collect();
    autolink_events(decode_text_entities(parsed))
}

#[cfg(test)]
#[test]
fn tight_list_keeps_inline_code_punctuation_and_nested_blocks_in_order() {
    let source = "- `projects.html` — listing\n- Case studies: `dinobase`, `luno`\n  - nested";
    for streaming in [true, false] {
        let doc = PreparedDocument::parse(Arc::from(source), streaming);
        let [Node::Container(Tag::List(_), items)] = doc.nodes() else {
            panic!("expected one list");
        };
        let Node::Container(Tag::Item, first) = &items[0] else {
            panic!("expected first item");
        };
        assert!(matches!(first.as_slice(),
            [Node::Container(Tag::Paragraph, spans)] if matches!(spans.as_slice(),
                [Node::Leaf(Event::Code(_)), Node::Leaf(Event::Text(_))])));
        let Node::Container(Tag::Item, second) = &items[1] else {
            panic!("expected second item");
        };
        assert!(matches!(second.as_slice(),
            [Node::Container(Tag::Paragraph, spans), Node::Container(Tag::List(_), _)]
                if spans.len() == 4));
    }
}

fn bounded_events(text: &str, budget: usize) -> Option<Vec<Event<'static>>> {
    let protected = protect_autolinks(text);
    let mut parsed = Vec::new();
    let mut remaining = budget;
    for event in Parser::new_ext(&protected, options()) {
        remaining = remaining.checked_sub(
            std::mem::size_of::<Node>().saturating_add(event_bytes(&event)),
        )?;
        parsed.push(event.into_static());
    }
    Some(autolink_events(decode_text_entities(parsed)))
}

fn decode_text_entities(events: Vec<Event<'static>>) -> Vec<Event<'static>> {
    let mut out = Vec::with_capacity(events.len());
    let mut code_blocks = 0usize;
    for event in events {
        match &event {
            Event::Start(Tag::CodeBlock(_)) => {
                code_blocks += 1;
                out.push(event);
            }
            Event::End(TagEnd::CodeBlock) => {
                code_blocks = code_blocks.saturating_sub(1);
                out.push(event);
            }
            Event::Text(text) if code_blocks == 0 => {
                out.push(Event::Text(CowStr::from(decode_entity_references(text))));
            }
            _ => out.push(event),
        }
    }
    out
}

fn decode_entity_references(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;
    while let Some(offset) = text[index..].find('&') {
        let start = index + offset;
        out.push_str(&text[index..start]);
        let Some(close) = text[start + 1..].find(';') else {
            out.push('&');
            index = start + 1;
            continue;
        };
        let entity = &text[start + 1..start + 1 + close];
        if !entity.is_empty()
            && entity.len() <= 32
            && !entity.chars().any(char::is_whitespace)
            && let Some(ch) = decode_entity(entity)
        {
            out.push(ch);
            index = start + 1 + close + 1;
            continue;
        }
        out.push('&');
        index = start + 1;
    }
    out.push_str(&text[index..]);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    if let Some(value) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
        return Some(numeric_entity(u32::from_str_radix(value, 16).ok()?));
    }
    if let Some(value) = entity.strip_prefix('#') {
        return Some(numeric_entity(value.parse().ok()?));
    }
    Some(match entity {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{00A0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "laquo" => '«',
        "raquo" => '»',
        "times" => '×',
        "divide" => '÷',
        "plusmn" => '±',
        "degree" => '°',
        "micro" => 'µ',
        "para" => '¶',
        "sect" => '§',
        "middot" => '·',
        "bull" => '•',
        "dagger" => '†',
        "Dagger" => '‡',
        "permil" => '‰',
        "prime" => '′',
        "Prime" => '″',
        "infin" => '∞',
        "ne" => '≠',
        "le" => '≤',
        "ge" => '≥',
        "larr" => '←',
        "rarr" => '→',
        "uarr" => '↑',
        "darr" => '↓',
        "harr" => '↔',
        "lArr" => '⇐',
        "rArr" => '⇒',
        "hArr" => '⇔',
        "forall" => '∀',
        "exist" => '∃',
        "empty" => '∅',
        "isin" => '∈',
        "notin" => '∉',
        "sub" => '⊂',
        "sup" => '⊃',
        "sube" => '⊆',
        "supe" => '⊇',
        "and" => '∧',
        "or" => '∨',
        "cap" => '∩',
        "cup" => '∪',
        "sum" => '∑',
        "prod" => '∏',
        "int" => '∫',
        "radic" => '√',
        "asymp" => '≈',
        "equiv" => '≡',
        "there4" => '∴',
        "sim" => '∼',
        "cong" => '≅',
        "alpha" => 'α',
        "beta" => 'β',
        "gamma" => 'γ',
        "delta" => 'δ',
        "epsilon" => 'ε',
        "lambda" => 'λ',
        "mu" => 'μ',
        "pi" => 'π',
        "sigma" => 'σ',
        "tau" => 'τ',
        "phi" => 'φ',
        "omega" => 'ω',
        "Alpha" => 'Α',
        "Beta" => 'Β',
        "Gamma" => 'Γ',
        "Delta" => 'Δ',
        "Pi" => 'Π',
        "Sigma" => 'Σ',
        "Omega" => 'Ω',
        "euro" => '€',
        "pound" => '£',
        "yen" => '¥',
        "cent" => '¢',
        "frac12" => '½',
        "frac14" => '¼',
        "frac34" => '¾',
        "sup1" => '¹',
        "sup2" => '²',
        "sup3" => '³',
        _ => return None,
    })
}

fn numeric_entity(value: u32) -> char {
    char::from_u32(value)
        .filter(|ch| *ch != '\0')
        .unwrap_or('\u{FFFD}')
}

/// Escape Markdown emphasis delimiters inside bare URLs/emails before parsing.
/// Without this, an address like `https://example.com/a_b` is split into
/// separate Text/Emphasis events before the post-parse autolinker can see the
/// complete URL. Code spans/fences, HTML tags and explicit link destinations
/// are copied verbatim.
fn protect_autolinks(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let trimmed = content.trim_start();
        if let Some((marker, length)) = fence {
            output.push_str(line);
            if trimmed.starts_with(&marker.to_string().repeat(length))
                && trimmed[length..].trim().is_empty()
            {
                fence = None;
            }
            continue;
        }
        let indent = content.len() - trimmed.len();
        if indent <= 3 && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            let marker = trimmed.chars().next().unwrap_or('`');
            let length = trimmed.chars().take_while(|ch| *ch == marker).count();
            fence = Some((marker, length));
            output.push_str(line);
            continue;
        }
        output.push_str(&protect_line(line));
    }
    output
}

fn protect_line(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut index = 0usize;
    let mut square_depth = 0usize;
    let has_email = line.contains('@');
    while index < line.len() {
        if line[index..].starts_with('`') {
            let ticks = line[index..].chars().take_while(|ch| *ch == '`').count();
            let marker = "`".repeat(ticks);
            let rest = &line[index + ticks..];
            if let Some(offset) = rest.find(&marker) {
                let end = index + ticks + offset + ticks;
                output.push_str(&line[index..end]);
                index = end;
                continue;
            }
            output.push('`');
            index += 1;
            continue;
        }
        if line[index..].starts_with('<') {
            if let Some(offset) = line[index..].find('>') {
                let end = index + offset + 1;
                output.push_str(&line[index..end]);
                index = end;
                continue;
            }
            output.push('<');
            index += 1;
            continue;
        }
        if line[index..].starts_with("\\(") {
            if let Some(offset) = line[index + 2..].find("\\)") {
                let content_end = index + 2 + offset;
                output.push('$');
                output.push_str(&line[index + 2..content_end]);
                output.push('$');
                index = content_end + 2;
                continue;
            }
            output.push('\\');
            output.push('(');
            index += 2;
            continue;
        }
        // Email detection scans a local part. Trying it at every character of
        // a growing plain word makes each stream frame quadratic. Match the
        // autolinker boundary and skip email scans entirely without an '@'.
        let email_boundary = !line[..index].chars().next_back()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '/' | '_' | '@'));
        let candidate = bare_url(line, index)
            .or_else(|| (has_email && email_boundary).then(|| bare_email(line, index)).flatten());
        if let Some((end, _)) = candidate {
            let previous = line[..index].chars().next_back();
            let explicit_destination =
                previous == Some('(') && line[..index.saturating_sub(1)].ends_with(']');
            let reference = line[..index].trim_start().starts_with('[')
                && line[..index].contains("]:");
            if square_depth > 0 || explicit_destination || reference {
                output.push_str(&line[index..end]);
            } else {
                for ch in line[index..end].chars() {
                    if matches!(ch, '\\' | '`' | '*' | '_' | '~' | '[' | ']') {
                        output.push('\\');
                    }
                    output.push(ch);
                }
            }
            index = end;
            continue;
        }
        let ch = line[index..].chars().next().unwrap_or('\0');
        if ch == '[' {
            square_depth += 1;
        } else if ch == ']' {
            square_depth = square_depth.saturating_sub(1);
        }
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}

/// Match Marked's GFM URL/email autolinking on plain text while leaving code
/// spans, code blocks, images and explicit links untouched.
fn autolink_events(events: Vec<Event<'static>>) -> Vec<Event<'static>> {
    let mut out = Vec::with_capacity(events.len());
    let mut opaque = 0usize;
    let mut pending = String::new();
    for event in events {
        match &event {
            Event::Text(text) if opaque == 0 => {
                pending.push_str(text);
                continue;
            }
            Event::Start(Tag::Link { .. } | Tag::Image { .. } | Tag::CodeBlock(_)) => {
                opaque += 1;
            }
            Event::End(TagEnd::Link | TagEnd::Image | TagEnd::CodeBlock) => {
                opaque = opaque.saturating_sub(1);
            }
            _ => {}
        }
        if !pending.is_empty() {
            out.extend(autolink_text(&pending));
            pending.clear();
        }
        out.push(event);
    }
    if !pending.is_empty() {
        out.extend(autolink_text(&pending));
    }
    out
}

fn link_event(dest: String, label: &str) -> [Event<'static>; 3] {
    [
        Event::Start(Tag::Link {
            link_type: LinkType::Autolink,
            dest_url: CowStr::from(dest),
            title: CowStr::from(""),
            id: CowStr::from(""),
        }),
        Event::Text(CowStr::from(label.to_string())),
        Event::End(TagEnd::Link),
    ]
}

fn autolink_text(text: &str) -> Vec<Event<'static>> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut emitted = 0usize;
    let mut linked = false;
    while cursor < text.len() {
        if !text.is_char_boundary(cursor) {
            cursor += 1;
            continue;
        }
        let previous = text[..cursor].chars().next_back();
        if previous.is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '/' | '_' | '@')) {
            cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let link = bare_url(text, cursor).or_else(|| {
            bare_email(text, cursor).map(|(end, address)| (end, format!("mailto:{address}")))
        });
        let Some((display_end, dest)) = link else {
            cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            continue;
        };
        if cursor > emitted {
            out.push(Event::Text(CowStr::from(text[emitted..cursor].to_string())));
        }
        let label = &text[cursor..display_end];
        out.extend(link_event(dest, label));
        emitted = display_end;
        cursor = display_end;
        linked = true;
    }
    if !linked {
        out.push(Event::Text(CowStr::from(text.to_string())));
    } else if emitted < text.len() {
        out.push(Event::Text(CowStr::from(text[emitted..].to_string())));
    }
    out
}

fn bare_url(text: &str, start: usize) -> Option<(usize, String)> {
    let rest = &text[start..];
    // An ASCII prefix cannot straddle a UTF-8 character. Never lowercase the
    // whole suffix when an eight-byte probe ends inside a multibyte character.
    let starts = |prefix: &str| rest.get(..prefix.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(prefix));
    let (prefix_len, www) = if starts("https://") {
        (8, false)
    } else if starts("http://") {
        (7, false)
    } else if starts("ftp://") {
        (6, false)
    } else if starts("www.") {
        (4, true)
    } else {
        return None;
    };
    let mut end = start + prefix_len;
    if end >= text.len() {
        return None;
    }
    while end < text.len() {
        let ch = text[end..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '`') {
            break;
        }
        end += ch.len_utf8();
    }
    end = trim_url_end(text, start, end);
    if end <= start + prefix_len {
        return None;
    }
    let display = &text[start..end];
    let href = if www {
        format!("http://{display}")
    } else {
        display.to_string()
    };
    Some((end, href))
}

fn trim_url_end(text: &str, start: usize, mut end: usize) -> usize {
    loop {
        let Some(ch) = text[start..end].chars().next_back() else {
            return end;
        };
        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?' | '*' | '_' | '~' | '\'' | '"' | '&') {
            end -= ch.len_utf8();
            continue;
        }
        let (open, close) = match ch {
            ')' => ('(', ')'),
            ']' => ('[', ']'),
            '}' => ('{', '}'),
            _ => return end,
        };
        let opens = text[start..end].matches(open).count();
        let closes = text[start..end].matches(close).count();
        if closes > opens {
            end -= ch.len_utf8();
            continue;
        }
        return end;
    }
}

fn bare_email(text: &str, start: usize) -> Option<(usize, String)> {
    let mut index = start;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '+' | '-') {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    if index == start || !text[index..].starts_with('@') {
        return None;
    }
    let local = &text[start..index];
    if local.starts_with('.') || local.ends_with('.') {
        return None;
    }
    index += 1;
    let domain_start = index;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or('\0');
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    let domain = text[domain_start..index].trim_end_matches('.');
    let domain_end = domain_start + domain.len();
    let (name, tld) = domain.rsplit_once('.')?;
    if name.is_empty()
        || tld.len() < 2
        || !tld.chars().all(|ch| ch.is_ascii_alphabetic())
        || domain.starts_with('-')
        || name.ends_with('-')
    {
        return None;
    }
    Some((domain_end, text[start..domain_end].to_string()))
}

/// Find complete top-level containers, retaining nested paragraphs in lists,
/// quotes and tables. Parser offsets preserve code indentation and fence length.
pub fn split_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    start = if matches!(tag, Tag::CodeBlock(CodeBlockKind::Indented)) {
                        text[..range.start].rfind('\n').map_or(0, |index| index + 1)
                    } else {
                        range.start
                    };
                }
                depth += 1;
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    push_range(&mut ranges, text, start..range.end);
                }
            }
            _ if depth == 0 => push_range(&mut ranges, text, range),
            _ => {}
        }
    }
    ranges
}

fn push_range(ranges: &mut Vec<Range<usize>>, text: &str, mut range: Range<usize>) {
    // Keep indentation and trailing spaces (hard line breaks and code), but
    // exclude the container's trailing newline from layout and Copy labels.
    range.end -=
        text[range.clone()].len() - text[range.clone()].trim_end_matches(['\r', '\n']).len();
    if range.start < range.end {
        ranges.push(range);
    }
}

/// Convenience for tests and non-streaming callers.
pub fn split_blocks(text: &str) -> Vec<&str> {
    split_ranges(text)
        .into_iter()
        .map(|range| &text[range])
        .collect()
}

/// Close incomplete streaming Markdown the way Source's `remend` projection
/// does for unterminated bold/italic/code/strike spans and open fences.
pub fn heal_streaming_markdown(text: &str) -> String {
    let mut in_fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches([' ', '\t']);
        let Some(rest) = trimmed.strip_prefix("```").or_else(|| trimmed.strip_prefix("~~~")) else {
            continue;
        };
        let mark = trimmed.chars().next().unwrap_or('`');
        let run = trimmed.chars().take_while(|c| *c == mark).count();
        if run < 3 {
            continue;
        }
        match in_fence {
            None => in_fence = Some((mark, run)),
            Some((open, open_run)) if open == mark && run >= open_run => in_fence = None,
            _ => {}
        }
        let _ = rest;
    }
    if let Some((mark, run)) = in_fence {
        let mut healed = text.to_owned();
        if !healed.ends_with('\n') {
            healed.push('\n');
        }
        healed.extend(std::iter::repeat_n(mark, run));
        return healed;
    }

    // Protect inline code runs; never heal emphasis inside them.
    let mut code_spans = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' || (index > 0 && bytes[index - 1] == b'\\') {
            index += 1;
            continue;
        }
        let run = bytes[index..].iter().take_while(|b| **b == b'`').count();
        let mut close = index + run;
        let mut found = None;
        while close < bytes.len() {
            if bytes[close] == b'`' {
                let close_run = bytes[close..].iter().take_while(|b| **b == b'`').count();
                if close_run == run {
                    found = Some(close + run);
                    break;
                }
                close += close_run;
            } else {
                close += 1;
            }
        }
        match found {
            Some(end) => {
                code_spans.push(index..end);
                index = end;
            }
            None => {
                let mut healed = text.to_owned();
                healed.extend(std::iter::repeat_n('`', run));
                return healed;
            }
        }
    }
    let mut healed = text.to_owned();
    let mut outside_code = text.as_bytes().to_vec();
    for range in code_spans {
        outside_code[range].fill(b' ');
    }
    let outside_code = String::from_utf8_lossy(&outside_code);
    let outside_bytes = outside_code.as_bytes();
    // A destination is actionable only after its closing delimiter arrives.
    // Keep the visible label while withholding partial URLs and image loads.
    if let Some(destination) = outside_code.rfind("](")
        && !outside_code[destination + 2..].contains(')')
        && let Some(open) = outside_code[..destination].rfind('[')
        && (open == 0 || outside_bytes[open - 1] != b'\\')
    {
        let from = if open > 0 && outside_bytes[open - 1] == b'!' { open - 1 } else { open };
        return format!("{}{}", &text[..from], &text[open + 1..destination]);
    }
    let count = |marker: &str| {
        let mut n = 0;
        let mut offset = 0;
        while let Some(found) = outside_code[offset..].find(marker) {
            let at = offset + found;
            if at > 0 && outside_bytes[at - 1] == b'\\' {
                offset = at + marker.len();
                continue;
            }
            // CommonMark treats word-internal underscores as literal text.
            if marker == "_"
                && outside_code[..at].chars().next_back().is_some_and(char::is_alphanumeric)
                && outside_code[at + 1..].chars().next().is_some_and(char::is_alphanumeric)
            {
                offset = at + 1;
                continue;
            }
            n += 1;
            offset = at + marker.len();
        }
        n
    };
    for marker in ["***", "**", "~~", "*", "_"] {
        if count(marker) % 2 == 1 {
            healed.push_str(marker);
            break;
        }
    }
    healed
}


/// A parsed section keeps its original source and provisional formatting apart.
/// UI keys derive from source position, never the repaired text.
#[derive(Clone, Debug)]
pub struct PreparedSection {
    pub range: Range<usize>,
    pub source: Arc<str>,
    pub streaming: bool,
    pub nodes: Arc<[Node]>,
    display: Arc<str>,
    syntax_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedDocument {
    pub source: Arc<str>,
    pub streaming: bool,
    pub sections: Arc<[PreparedSection]>,
    references: Arc<str>,
    deferred: bool,
}

impl PreparedDocument {
    pub fn parse(source: Arc<str>, streaming: bool) -> Self {
        Self::prepare(source, streaming, Arc::from(""), None)
    }

    /// Cheap, immutable presentation for a source outside the rich cache's
    /// admission budget. Preserve all raw text without parsing on GPUI or
    /// making incomplete/synthetic destinations actionable.
    pub fn deferred(source: Arc<str>, streaming: bool) -> Self {
        let section = PreparedSection {
            range: 0..source.len(),
            source: source.clone(),
            streaming,
            nodes: vec![Node::PlainText(source.clone())].into(),
            display: source.clone(),
            syntax_bytes: std::mem::size_of::<Node>(),
        };
        Self {
            source,
            streaming,
            sections: vec![section].into(),
            references: Arc::from(""),
            deferred: true,
        }
    }

    pub fn is_deferred(&self) -> bool {
        self.deferred
    }

    fn prepare(
        source: Arc<str>,
        streaming: bool,
        references: Arc<str>,
        previous: Option<&Self>,
    ) -> Self {
        let keep_sections = previous.is_some_and(|old| {
            old.sections.len() > 1 && source.starts_with(old.source.as_ref())
        });
        let ranges = if streaming || keep_sections { split_ranges(&source) } else { Vec::new() };
        let ranges = if ranges.len() > 1 { ranges } else { std::iter::once(0..source.len()).collect() };
        let last = ranges.len().saturating_sub(1);
        let mut sections = Vec::new();
        let mut remaining = PreparedMarkdown::MAX_BYTES.saturating_sub(source.len() + references.len());
        for (index, range) in ranges.into_iter().enumerate() {
            let section_streaming = streaming && index == last;
            let text = &source[range.clone()];
            let display: Arc<str> = if section_streaming {
                Arc::from(heal_streaming_markdown(text))
            } else {
                Arc::from(text)
            };
            let old = previous.filter(|old| !old.deferred).and_then(|old| old.sections.get(index));
            let (nodes, syntax_bytes) = if let Some(old) = old.filter(|old| {
                old.display == display && previous.is_some_and(|p| p.references == references)
            }) {
                (old.nodes.clone(), old.syntax_bytes)
            } else {
                let parse_source = if references.is_empty() {
                    display.to_string()
                } else {
                    format!("{display}\n\n{references}")
                };
                let Some(parsed) = bounded_events(&parse_source, remaining) else {
                    return Self::deferred(source, streaming);
                };
                let nodes = tree(&mut parsed.into_iter());
                let bytes = node_bytes(&nodes);
                (Arc::from(nodes), bytes)
            };
            let Some(next) = remaining.checked_sub(std::mem::size_of::<PreparedSection>()
                .saturating_add(text.len()).saturating_add(display.len())
                .saturating_add(syntax_bytes)) else {
                return Self::deferred(source, streaming);
            };
            remaining = next;
            sections.push(PreparedSection { range, source: Arc::from(text), streaming: section_streaming, nodes, display, syntax_bytes });
        }
        Self { source, streaming, references, sections: sections.into(), deferred: false }
    }

    pub fn nodes(&self) -> &[Node] {
        self.sections.first().map_or(&[], |section| &section.nodes)
    }

    /// Conservative retained syntax/storage estimate. Shared source/reference
    /// strings may be counted more than once; this is an admission budget for
    /// derived documents, not a limit on authoritative source or process RSS.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(self.source.len())
            .saturating_add(self.references.len())
            .saturating_add(self.sections.len().saturating_mul(std::mem::size_of::<PreparedSection>()))
            .saturating_add(self.sections.iter().map(|section| {
                section.source.len()
                    .saturating_add(section.display.len())
                    .saturating_add(section.syntax_bytes)
            }).sum::<usize>())
    }
}

fn tag_bytes(tag: &Tag<'_>) -> usize {
    match tag {
        Tag::Heading { id, classes, attrs, .. } => {
            id.as_ref().map_or(0, |id| id.len())
                .saturating_add(classes.capacity().saturating_mul(std::mem::size_of::<CowStr<'_>>()))
                .saturating_add(classes.iter().map(|class| class.len()).sum::<usize>())
                .saturating_add(attrs.capacity().saturating_mul(std::mem::size_of::<(CowStr<'_>, Option<CowStr<'_>>)>() ))
                .saturating_add(attrs.iter().map(|(key, value)| key.len().saturating_add(value.as_ref().map_or(0, |value| value.len()))).sum::<usize>())
        }
        Tag::Link { dest_url, title, id, .. } | Tag::Image { dest_url, title, id, .. } => {
            dest_url.len().saturating_add(title.len()).saturating_add(id.len())
        }
        Tag::CodeBlock(CodeBlockKind::Fenced(info)) | Tag::FootnoteDefinition(info) => info.len(),
        Tag::Table(alignments) => alignments.capacity().saturating_mul(std::mem::size_of::<Alignment>()),
        _ => 0,
    }
}

fn event_bytes(event: &Event<'_>) -> usize {
    match event {
        Event::Start(tag) => tag_bytes(tag),
        Event::Text(text) | Event::Code(text) | Event::InlineMath(text)
        | Event::DisplayMath(text) | Event::Html(text) | Event::InlineHtml(text)
        | Event::FootnoteReference(text) => text.len(),
        _ => 0,
    }
}

fn node_bytes(nodes: &[Node]) -> usize {
    nodes.iter().map(|node| std::mem::size_of::<Node>() + match node {
        Node::Container(tag, children) => tag_bytes(tag)
            .saturating_add(node_bytes(children))
            .saturating_add(children.capacity().saturating_sub(children.len()).saturating_mul(std::mem::size_of::<Node>())),
        Node::Leaf(event) => event_bytes(event),
        // This Arc aliases PreparedDocument::source, already counted above.
        Node::PlainText(_) => 0,
    }).sum()
}

/// Immutable engine-prepared Markdown for a source transcript revision.
/// Snapshot reads and lookups never invoke a parser.
#[derive(Clone, Debug, Default)]
pub struct PreparedMarkdown {
    pub revision: u64,
    documents: HashMap<Arc<str>, Arc<PreparedDocument>>,
    source_index: HashMap<Arc<str>, Vec<Arc<PreparedDocument>>>,
    pub estimated_bytes: usize,
    reference_sources: HashMap<Arc<str>, (Arc<str>, Arc<str>)>,
}

impl PreparedMarkdown {
    pub const MAX_DOCUMENTS: usize = 8192;
    pub const MAX_BYTES: usize = 32 * 1024 * 1024;

    pub fn get(&self, id: &str, source: &str, streaming: bool) -> Option<Arc<PreparedDocument>> {
        self.documents.get(id).filter(|document| document.source.as_ref() == source && document.streaming == streaming).cloned()
    }

    /// Reference-definition changes can affect an unchanged source row (for
    /// example a reference image that resolves later). Include this in its
    /// layout stamp; it is never a render identity.
    pub fn dependency_fingerprint(&self, id: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let Some(document) = self.documents.get(id) else { return 0; };
        if document.references.is_empty() {
            return if document.deferred { 2 } else { 1 };
        }
        let mut hash = std::hash::DefaultHasher::new();
        document.deferred.hash(&mut hash);
        document.references.hash(&mut hash);
        hash.finish()
    }

    /// Used by non-block UI helpers. Ambiguous source strings with distinct
    /// reference contexts are deliberately not conflated.
    pub fn find(&self, source: &str, streaming: bool) -> Option<Arc<PreparedDocument>> {
        let documents = self.source_index.get(source)?;
        let mut matches = documents.iter().filter(|document| document.streaming == streaming);
        let document = matches.next()?;
        matches.all(|other| other.references == document.references).then(|| document.clone())
    }

    /// Capture only raw parts that can carry cross-block references. This is
    /// a source-copy operation, not parsing, so the engine can release its
    /// transcript lock before doing syntax work.
    pub fn reference_sources(
        previous: &Self,
        messages: &[crate::transcript::Message],
    ) -> Vec<(Arc<str>, Arc<str>)> {
        messages.iter().flat_map(|message| &message.parts).filter_map(|part| {
            let text = match &part.kind {
                crate::transcript::PartKind::Text { text, .. }
                    | crate::transcript::PartKind::Reasoning { text, .. } => text,
                _ => return None,
            };
            if !text.contains("]:") { return None; }
            let source = previous.reference_sources.get(part.id.as_str())
                .filter(|(source, _)| source.as_ref() == text)
                .map(|(source, _)| source.clone()).unwrap_or_else(|| Arc::from(text.as_str()));
            Some((Arc::from(part.id.as_str()), source))
        }).collect()
    }

    pub fn refresh(
        previous: &Self,
        revision: u64,
        blocks: &[crate::transcript::Block],
        messages: &[crate::transcript::Message],
    ) -> Self {
        let sources = Self::reference_sources(previous, messages);
        Self::refresh_sources(previous, revision, blocks, &sources)
    }

    pub fn refresh_sources(
        previous: &Self,
        revision: u64,
        blocks: &[crate::transcript::Block],
        sources: &[(Arc<str>, Arc<str>)],
    ) -> Self {
        Self::refresh_sources_with_reference_budget(previous, revision, blocks, sources, Self::MAX_BYTES / 4)
    }

    fn refresh_sources_with_reference_budget(
        previous: &Self,
        revision: u64,
        blocks: &[crate::transcript::Block],
        sources: &[(Arc<str>, Arc<str>)],
        reference_budget: usize,
    ) -> Self {
        let mut references = HashMap::<&str, Arc<str>>::new();
        let mut reference_sources = HashMap::new();
        let mut reference_bytes = 0usize;
        for (id, text) in sources {
            if let Some((source, definitions)) = previous.reference_sources.get(id)
                .filter(|(source, _)| source == text)
            {
                references.insert(id, definitions.clone());
                let bytes = reference_entry_bytes(id, source, definitions);
                if reference_bytes.saturating_add(bytes) <= reference_budget {
                    reference_bytes += bytes;
                    reference_sources.insert(id.clone(), (source.clone(), definitions.clone()));
                }
                continue;
            }
            let parser = Parser::new_ext(text, options());
            let mut spans = parser.reference_definitions().iter()
                .map(|(_, definition)| definition.span.clone()).collect::<Vec<_>>();
            spans.sort_by_key(|span| span.start);
            let definitions: Arc<str> = Arc::from(spans.iter().map(|span| &text[span.clone()]).collect::<Vec<_>>().join("\n"));
            references.insert(id, definitions.clone());
            let bytes = reference_entry_bytes(id, text, &definitions);
            if reference_bytes.saturating_add(bytes) <= reference_budget {
                reference_bytes += bytes;
                reference_sources.insert(id.clone(), (text.clone(), definitions));
            }
        }
        let mut snapshot = Self { revision, reference_sources, estimated_bytes: reference_bytes, ..Self::default() };
        // Prioritize the live tail when an exceptionally long transcript hits
        // the bounded derived cache. Evicted content remains authoritative.
        for block in blocks.iter().rev().filter(|block| !block.text.is_empty() && !matches!(block.kind, crate::transcript::BlockKind::Tool | crate::transcript::BlockKind::Shell)) {
            if snapshot.documents.len() >= Self::MAX_DOCUMENTS { break; }
            if snapshot.estimated_bytes.saturating_add(block.text.len().saturating_mul(4)) > Self::MAX_BYTES { continue; }
            let references = references.get(block.part_id.as_ref()).cloned().unwrap_or_else(|| Arc::from(""));
            let old = previous.documents.get(&block.id);
            let document = if let Some(old) = old.filter(|old| old.source == block.text && old.streaming == block.streaming && old.references == references) {
                old.clone()
            } else {
                Arc::new(PreparedDocument::prepare(block.text.clone(), block.streaming, references, old.map(Arc::as_ref)))
            };
            let bytes = document.estimated_bytes();
            if snapshot.documents.len() >= Self::MAX_DOCUMENTS || snapshot.estimated_bytes.saturating_add(bytes) > Self::MAX_BYTES {
                continue;
            }
            snapshot.estimated_bytes += bytes;
            snapshot.estimated_bytes += std::mem::size_of::<Arc<PreparedDocument>>();
            snapshot.source_index.entry(block.text.clone()).or_default().push(document.clone());
            // Reasoning bodies use virtual paragraph rows. Index their already
            // parsed sections too, so those rows do not need a UI parser pass.
            // Section entries are retained memory as well; charge them to the
            // same derived budget instead of letting source_index grow past it.
            if document.sections.len() > 1 {
                for section in document.sections.iter() {
                    let mut section = section.clone();
                    section.range = 0..section.source.len();
                    let section_document = Arc::new(PreparedDocument {
                        source: section.source.clone(), streaming: section.streaming,
                        references: document.references.clone(), sections: vec![section].into(),
                        deferred: false,
                    });
                    // Two index slots (source and section source) plus the
                    // section document's own retained struct/source estimate.
                    let bytes = section_document.estimated_bytes().saturating_add(
                        std::mem::size_of::<Arc<PreparedDocument>>().saturating_mul(2));
                    if snapshot.estimated_bytes.saturating_add(bytes) > Self::MAX_BYTES {
                        break;
                    }
                    snapshot.estimated_bytes += bytes;
                    snapshot.source_index.entry(section_document.source.clone()).or_default().push(section_document);
                }
            }
            snapshot.documents.insert(block.id.clone(), document);
        }
        snapshot
    }
}

fn reference_entry_bytes(id: &str, source: &str, definitions: &str) -> usize {
    std::mem::size_of::<(Arc<str>, (Arc<str>, Arc<str>))>()
        .saturating_add(id.len())
        .saturating_add(source.len())
        .saturating_add(definitions.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_snapshot(
        previous: &PreparedMarkdown,
        source: &str,
        streaming: bool,
    ) -> (PreparedMarkdown, crate::transcript::Transcript) {
        use crate::transcript::{Message, PartKind, Role, Transcript};
        let mut message = Message::text("message", Role::Assistant, 1, source);
        if let PartKind::Text { streaming: state, .. } = &mut message.parts[0].kind { *state = streaming; }
        let mut transcript = Transcript::new();
        transcript.hydrate(vec![message]);
        let blocks = transcript.blocks().to_vec();
        let snapshot = PreparedMarkdown::refresh(previous, previous.revision + 1, &blocks, transcript.messages());
        (snapshot, transcript)
    }

    #[test]
    fn prepared_markdown_reuses_unchanged_blocks_and_replaces_only_growing_tail() {
        let (first, mut transcript) = prepared_snapshot(&PreparedMarkdown::default(), "Settled paragraph.\n\nGrowing", true);
        let blocks = transcript.blocks().to_vec();
        let head = first.get(&blocks[0].id, &blocks[0].text, blocks[0].streaming).unwrap();
        let tail = first.get(&blocks[1].id, &blocks[1].text, blocks[1].streaming).unwrap();
        let (second, mut updated) = prepared_snapshot(&first, "Settled paragraph.\n\nGrowing tail", true);
        let blocks = updated.blocks().to_vec();
        assert!(Arc::ptr_eq(&head, &second.get(&blocks[0].id, &blocks[0].text, blocks[0].streaming).unwrap()));
        assert!(!Arc::ptr_eq(&tail, &second.get(&blocks[1].id, &blocks[1].text, blocks[1].streaming).unwrap()));
    }

    #[test]
    fn prepared_markdown_resolves_and_invalidates_late_reference_definitions() {
        let (first, mut transcript) = prepared_snapshot(&PreparedMarkdown::default(), "Read [the docs][guide].\n\n[guide]: https://first.example/docs", false);
        let block = transcript.blocks()[0].clone();
        let document = first.get(&block.id, &block.text, false).unwrap();
        assert!(format!("{:?}", document.nodes()).contains("https://first.example/docs"));
        let (second, mut transcript) = prepared_snapshot(&first, "Read [the docs][guide].\n\n[guide]: https://second.example/docs", false);
        let block = transcript.blocks()[0].clone();
        let corrected = second.get(&block.id, &block.text, false).unwrap();
        assert!(!Arc::ptr_eq(&document, &corrected));
        assert!(format!("{:?}", corrected.nodes()).contains("https://second.example/docs"));
    }

    #[test]
    fn prepared_markdown_keeps_raw_source_and_reuses_settled_sections() {
        let first = PreparedDocument::parse(Arc::from("First.\n\nThis is **bold"), true);
        assert_eq!(first.source.as_ref(), "First.\n\nThis is **bold");
        assert_eq!(first.sections.len(), 2);
        let second = PreparedDocument::prepare(Arc::from("First.\n\nThis is **bold text"), true, Arc::from(""), Some(&first));
        assert!(Arc::ptr_eq(&first.sections[0].nodes, &second.sections[0].nodes));
        assert!(!Arc::ptr_eq(&first.sections[1].nodes, &second.sections[1].nodes));
        assert!(format!("{:?}", second.sections[1].nodes).contains("Strong"));
    }

    #[test]
    fn provisional_markdown_withholds_incomplete_destinations_and_preserves_literal_underscores() {
        assert_eq!(heal_streaming_markdown("Read [the docs](https://exam"), "Read the docs");
        assert_eq!(heal_streaming_markdown("![preview](https://exam"), "preview");
        assert_eq!(heal_streaming_markdown("`[the docs](https://exam`"), "`[the docs](https://exam`");
        assert_eq!(heal_streaming_markdown("snake_case and another_value"), "snake_case and another_value");
        assert_eq!(heal_streaming_markdown("_italic words"), "_italic words_");
    }

    #[test]
    fn section_index_entries_are_charged_to_the_derived_budget() {
        use crate::transcript::{Message, Part, PartKind, Role, Transcript};
        let mut message = Message::text("message", Role::Assistant, 1, "");
        message.parts = vec![Part {
            id: "reasoning".into(),
            ordinal: 0,
            kind: PartKind::Reasoning {
                text: "First paragraph.\n\nSecond paragraph.".into(),
                streaming: true,
            },
        }];
        let mut transcript = Transcript::new();
        transcript.hydrate(vec![message]);
        let blocks = transcript.blocks().to_vec();
        let snapshot = PreparedMarkdown::refresh(
            &PreparedMarkdown::default(), 1, &blocks, transcript.messages());
        let document = snapshot.documents.get("reasoning").unwrap();
        assert!(document.sections.len() > 1, "the reasoning body is split into sections");
        let documents = snapshot.documents.values()
            .map(|document| document.estimated_bytes()).sum::<usize>();
        assert!(snapshot.estimated_bytes > documents,
            "section-index documents must be charged to the retained estimate");
    }

    #[test]
    fn prepared_markdown_lookup_rejects_stale_source_and_streaming_state() {
        let (snapshot, mut transcript) = prepared_snapshot(&PreparedMarkdown::default(), "Original", true);
        let block = transcript.blocks()[0].clone();
        assert!(snapshot.get(&block.id, "Correction", true).is_none());
        assert!(snapshot.get(&block.id, &block.text, false).is_none());
        assert!(snapshot.find("Correction", true).is_none());
    }

    #[test]
    fn retained_syntax_estimate_counts_repeated_reference_destination_allocations() {
        let destination = format!("https://fixture.test/{}", "a".repeat(1_024));
        let source: Arc<str> = "[label][guide] ".repeat(128).into();
        let references: Arc<str> = format!("[guide]: {destination}").into();
        let document = PreparedDocument::prepare(source.clone(), false, references, None);
        assert!(document.estimated_bytes() >= destination.len() * 128,
            "each owned link destination must count against the retained syntax budget");
        assert!(document.estimated_bytes() > source.len() * 20,
            "small source can expand into much larger syntax through reference links");
    }

    #[test]
    fn reference_expansion_stops_at_the_owned_event_budget() {
        let destination = format!("https://fixture.test/{}", "a".repeat(4096));
        let source = format!("{}\n\n[guide]: {destination}", "[label][guide] ".repeat(64));
        assert!(source.len() < 64 * 1024);
        assert!(bounded_events(&source, 64 * 1024).is_none());
        assert!(bounded_events(&source, 256 * 1024).is_some());
    }

    #[test]
    fn settling_a_stream_keeps_completed_markdown_sections_and_reuses_their_syntax() {
        let source: Arc<str> = "First paragraph.\n\nLast **unfinished".into();
        let streaming = PreparedDocument::parse(source.clone(), true);
        let settled = PreparedDocument::prepare(source, false, Arc::from(""), Some(&streaming));
        assert_eq!(settled.sections.len(), 2);
        assert!(Arc::ptr_eq(&streaming.sections[0].nodes, &settled.sections[0].nodes));
        assert!(!settled.sections[1].streaming);
        assert!(!Arc::ptr_eq(&streaming.sections[1].nodes, &settled.sections[1].nodes));
    }

    #[test]
    fn oversized_source_has_an_exact_source_backed_parse_free_representation() {
        let source: Arc<str> = format!("**raw {}", "x".repeat(PreparedMarkdown::MAX_BYTES / 4)).into();
        let (_, mut transcript) = prepared_snapshot(&PreparedMarkdown::default(), "seed", false);
        let mut block = transcript.blocks()[0].clone();
        block.text = source.clone();
        let snapshot = PreparedMarkdown::refresh_sources(&PreparedMarkdown::default(), 1, std::slice::from_ref(&block), &[]);
        assert!(snapshot.get(&block.id, &source, false).is_none(),
            "oversized syntax must not bypass the rich cache's admission budget");
        for streaming in [true, false] {
            let document = PreparedDocument::deferred(source.clone(), streaming);
            assert!(document.is_deferred());
            assert!(Arc::ptr_eq(&document.source, &source));
            assert_eq!(document.sections[0].range, 0..source.len());
            let [Node::PlainText(text)] = document.nodes() else {
                panic!("overflow content must use a plain source node, never a GPUI parser fallback");
            };
            assert!(Arc::ptr_eq(text, &source));
            assert!(Arc::ptr_eq(&document.sections[0].display, &source));
        }
    }

    #[test]
    fn document_limit_preserves_tail_reuse_and_omitted_sources_remain_available() {
        let (_, mut transcript) = prepared_snapshot(&PreparedMarkdown::default(), "History", false);
        let seed = transcript.blocks()[0].clone();
        let blocks = (0..PreparedMarkdown::MAX_DOCUMENTS + 2).map(|index| {
            let mut block = seed.clone();
            block.id = format!("history-{index}").into();
            block
        }).collect::<Vec<_>>();
        let first = PreparedMarkdown::refresh_sources(&PreparedMarkdown::default(), 1, &blocks, &[]);
        assert_eq!(first.documents.len(), PreparedMarkdown::MAX_DOCUMENTS);
        assert!(first.get(&blocks[0].id, &blocks[0].text, false).is_none());
        let last = blocks.last().unwrap();
        let tail = first.get(&last.id, &last.text, false).unwrap();
        let second = PreparedMarkdown::refresh_sources(&first, 2, &blocks, &[]);
        assert!(Arc::ptr_eq(&tail, &second.get(&last.id, &last.text, false).unwrap()));
        let omitted = PreparedDocument::deferred(blocks[0].text.clone(), false);
        assert!(Arc::ptr_eq(&omitted.source, &blocks[0].text));
        assert_eq!(omitted.source.as_ref(), "History");
    }

    #[test]
    fn deferred_nodes_cannot_be_reused_as_if_they_were_rich_syntax() {
        let source: Arc<str> = "**formatted**".into();
        let plain = PreparedDocument::deferred(source.clone(), false);
        let rich = PreparedDocument::prepare(source, false, Arc::from(""), Some(&plain));
        assert!(!rich.is_deferred());
        assert!(matches!(rich.nodes().first(), Some(Node::Container(Tag::Paragraph, _))));
        assert!(!Arc::ptr_eq(&plain.sections[0].nodes, &rich.sections[0].nodes));
    }

    #[test]
    fn reused_reference_sources_obey_the_same_retention_budget_as_new_sources() {
        let source = |id: &str| -> (Arc<str>, Arc<str>) {
            (id.into(), format!("[{id}]: https://fixture.test/{id}").into())
        };
        let a = source("a");
        let b = source("b");
        let c = source("c");
        let budget = reference_entry_bytes(&a.0, &a.1, &a.1) * 2;
        let first = PreparedMarkdown::refresh_sources_with_reference_budget(
            &PreparedMarkdown::default(), 1, &[], &[a.clone(), b.clone()], budget,
        );
        assert_eq!(first.reference_sources.len(), 2);
        let second = PreparedMarkdown::refresh_sources_with_reference_budget(
            &first, 2, &[], &[c, a, b], budget,
        );
        assert!(second.estimated_bytes <= budget,
            "new definitions followed by reused definitions must not accumulate past the budget");
        assert_eq!(second.reference_sources.len(), 2);
        assert!(second.reference_sources.contains_key("c"));
        assert!(second.reference_sources.contains_key("a"));
        assert!(!second.reference_sources.contains_key("b"));
    }


    #[test]
    fn splits_paragraphs_and_headings() {
        let text = "# Title\n\nFirst paragraph.\n\n## Next\nSecond line\ncontinues.";
        assert_eq!(
            split_blocks(text),
            vec![
                "# Title",
                "First paragraph.",
                "## Next",
                "Second line\ncontinues."
            ]
        );
    }

    #[test]
    fn keeps_fenced_code_whole_even_with_blank_lines() {
        let text = "Intro\n\n```rust\nfn main() {\n\n    println!(\"hi\");\n}\n```\n\nDone";
        let blocks = split_blocks(text);
        assert_eq!(blocks.len(), 3);
        assert!(blocks[1].starts_with("```rust"));
        assert!(blocks[1].ends_with("```"));
    }

    #[test]
    fn handles_unclosed_fence_while_streaming() {
        let text = "Before\n\n```rust\nfn main() {";
        let blocks = split_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1], "```rust\nfn main() {");
    }

    #[test]
    fn trims_blank_edges() {
        let ranges = split_ranges("\n\n  hello  \n\n");
        assert_eq!(&"\n\n  hello  \n\n"[ranges[0].clone()], "hello  ");
    }

    #[test]
    fn nested_containers_tables_and_long_fences_keep_their_structure() {
        let text = "- first\n\n  second paragraph\n\n  - nested\n\n> quote\n>\n> continuation\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n````rust\n```\n\n# code\n````\n\n    indented code\n";
        let blocks = split_blocks(text);
        assert_eq!(
            blocks.len(),
            5,
            "lists, quotes, tables and fences are indivisible containers"
        );
        assert!(blocks[0].contains("second paragraph") && blocks[0].contains("nested"));
        assert!(blocks[1].contains("continuation"));
        assert!(blocks[2].contains("| 1 | 2 |"));
        assert!(blocks[3].contains("# code"));
        assert!(matches!(
            events(blocks[4])[0],
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented))
        ));
    }

    #[test]
    fn inline_events_preserve_nested_styles_links_entities_and_literal_delimiters() {
        let parsed = events(
            "**bold *and italic*** &amp; \\*literal\\* [label](https://example.com) ~~gone~~ `**code**`",
        );
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Start(Tag::Strong)))
        );
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Start(Tag::Emphasis)))
        );
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Start(Tag::Strikethrough)))
        );
        assert!(parsed.iter().any(|event| matches!(event, Event::Start(Tag::Link { dest_url, .. }) if dest_url.as_ref() == "https://example.com")));
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Code(text) if text.as_ref() == "**code**"))
        );
    }

    #[test]
    fn text_entities_decode_in_prose_but_not_code_or_raw_html() {
        let parsed = events(
            "Tom &amp; Jerry &lt;3 &#35; &#x1F600; &bogus; `&amp;` \
             <span title=\"&amp;\">x</span>\n\n```text\n&amp;\n```",
        );
        let mut prose = String::new();
        let mut code = String::new();
        let mut raw = String::new();
        let mut in_code = false;
        for event in &parsed {
            match event {
                Event::Start(Tag::CodeBlock(_)) => in_code = true,
                Event::End(TagEnd::CodeBlock) => in_code = false,
                Event::Text(text) if in_code => code.push_str(text),
                Event::Text(text) => prose.push_str(text),
                Event::Code(text) => code.push_str(text),
                Event::Html(text) | Event::InlineHtml(text) => raw.push_str(text),
                _ => {}
            }
        }
        assert!(
            prose.contains("Tom & Jerry <3 # 😀 &bogus;"),
            "{prose}"
        );
        assert_eq!(code.matches("&amp;").count(), 2, "{code}");
        assert!(raw.contains("&amp;"), "{raw}");
    }

    #[test]
    fn long_streaming_words_keep_text_and_following_email_semantics() {
        for prefix in ["x".repeat(16_000), "🌍é文".repeat(4_000)] {
            for source in [prefix.clone(), format!("{prefix} local_name+tag@example.com")] {
                let parsed = events(&source);
                let text = parsed.iter().filter_map(|event| match event {
                    Event::Text(text) => Some(text.as_ref()), _ => None,
                }).collect::<String>();
                assert_eq!(text, source);
                assert_eq!(parsed.iter().any(|event| matches!(event,
                    Event::Start(Tag::Link { dest_url, .. }) if dest_url.as_ref() == "mailto:local_name+tag@example.com")), source.contains('@'));
            }
        }
    }

    #[test]
    fn bare_urls_and_emails_autolink_like_marked_gfm() {
        let parsed = events(
            "Visit https://example.com/a?b=1, www.example.org and mail me@example.com. Code `https://code.example` stays plain.",
        );
        let links = parsed
            .iter()
            .filter_map(|event| match event {
                Event::Start(Tag::Link { dest_url, .. }) => Some(dest_url.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            links,
            vec![
                "https://example.com/a?b=1",
                "http://www.example.org",
                "mailto:me@example.com"
            ]
        );
        assert!(
            !links.iter().any(|link| link.contains("code.example")),
            "inline code never autolinks"
        );
    }

    #[test]
    fn autolink_skips_explicit_links_images_code_blocks_and_balanced_punctuation() {
        let parsed = events(
            "[explicit](https://explicit.example) ![alt https://image.example](https://image.example/a.png) `https://code.example`\n\n```text\nhttps://fence.example\n```\n\n(https://paren.example/a(b)) done",
        );
        let links = parsed
            .iter()
            .filter_map(|event| match event {
                Event::Start(Tag::Link { dest_url, .. }) => Some(dest_url.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            links,
            vec![
                "https://explicit.example",
                "https://paren.example/a(b)"
            ],
            "only the bare balanced URL is autolinked"
        );
    }

    #[test]
    fn source_inline_math_delimiters_convert_outside_code() {
        let parsed = events(
            "before \\(x_i^2 + \\alpha\\) and `\\(code\\)` after\n\n```text\n\\(fence\\)\n```",
        );
        let math = parsed
            .iter()
            .filter_map(|event| match event {
                Event::InlineMath(text) => Some(text.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(math, vec![r"x_i^2 + \alpha"]);
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Code(text) if text.contains("\\(code\\)")))
        );
        assert!(
            parsed
                .iter()
                .any(|event| matches!(event, Event::Text(text) if text.contains("(fence)")))
                || parsed
                    .iter()
                    .any(|event| matches!(event, Event::Text(text) if text.contains("\\(fence\\)")))
        );
    }

    #[test]
    fn autolink_protection_keeps_markdown_punctuation_inside_urls() {
        let parsed = events(
            "See https://example.com/a_b*c and first_last@example.com, plus [ex](https://explicit.example/x_y) and `<https://code.example/a_b>`.\n\n```text\nhttps://fence.example/a_b\n```",
        );
        let links = parsed
            .iter()
            .filter_map(|event| match event {
                Event::Start(Tag::Link { dest_url, .. }) => Some(dest_url.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            links,
            vec![
                "https://example.com/a_b*c",
                "mailto:first_last@example.com",
                "https://explicit.example/x_y"
            ]
        );
        assert!(
            parsed.iter().any(|event| matches!(event, Event::Text(text) if text.contains("a_b*c"))),
            "displayed autolink text keeps underscores and asterisks"
        );
    }

    #[test]
    fn autolink_protection_leaves_html_attributes_verbatim() {
        let parsed = events("<a href=\"https://example.com/a_b\">link</a>");
        let html = parsed
            .iter()
            .filter_map(|event| match event {
                Event::Html(text) | Event::InlineHtml(text) => Some(text.to_string()),
                _ => None,
            })
            .collect::<String>();
        assert!(html.contains("https://example.com/a_b"));
        assert!(!html.contains("\\_"));
    }

    #[test]
    fn a_long_paragraph_is_one_block() {
        let text = "word ".repeat(10_000);
        assert_eq!(split_blocks(&text).len(), 1);
    }
}
