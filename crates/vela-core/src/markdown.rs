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
        let candidate = bare_url(line, index).or_else(|| bare_email(line, index));
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
    let lower = rest.get(..8).unwrap_or(rest).to_ascii_lowercase();
    let (prefix_len, www) = if lower.starts_with("https://") {
        (8, false)
    } else if lower.starts_with("http://") {
        (7, false)
    } else if lower.starts_with("ftp://") {
        (6, false)
    } else if lower.starts_with("www.") {
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

#[cfg(test)]
mod tests {
    use super::*;

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
