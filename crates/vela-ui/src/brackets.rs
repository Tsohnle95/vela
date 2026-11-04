//! Matching-bracket lookup for the caret.
//!
//! Brackets inside strings and comments are ignored; matching is depth-based
//! for the bracket family at the caret, so mixed nesting works.

use std::ops::Range;

fn bracket(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}')
}

fn family(ch: char) -> Option<(char, char, bool)> {
    match ch {
        '(' => Some(('(', ')', true)),
        '[' => Some(('[', ']', true)),
        '{' => Some(('{', '}', true)),
        ')' => Some(('(', ')', false)),
        ']' => Some(('[', ']', false)),
        '}' => Some(('{', '}', false)),
        _ => None,
    }
}

fn pair_for(
    masked: &str,
    index: usize,
    ch: char,
) -> Option<(Range<usize>, Range<usize>)> {
    if masked[index..].chars().next()? != ch {
        return None;
    }
    let (open, close, forward) = family(ch)?;
    let partner = if forward {
        let mut depth = 0i32;
        let mut found = None;
        for (offset, ch) in masked[index..].char_indices() {
            let at = index + offset;
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    found = Some(at);
                    break;
                }
            }
        }
        found?
    } else {
        let mut depth = 0i32;
        let mut found = None;
        for (offset, ch) in masked[..=index].char_indices().rev() {
            if ch == close {
                depth += 1;
            } else if ch == open {
                depth -= 1;
                if depth == 0 {
                    found = Some(offset);
                    break;
                }
            }
        }
        found?
    };
    let (start, end) = if index < partner {
        (index, partner)
    } else {
        (partner, index)
    };
    Some((start..start + 1, end..end + 1))
}

/// Ranges of the bracket at/near the caret and its partner, in source order.
pub(super) fn matching_pair(text: &str, caret: usize) -> Option<(Range<usize>, Range<usize>)> {
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut candidate = None;
    if let Some((index, ch)) = text[..caret.min(text.len())].char_indices().next_back()
        && bracket(ch)
    {
        candidate = Some((index, ch));
    }
    if candidate.is_none() && caret < text.len() {
        let ch = text[caret..].chars().next()?;
        if bracket(ch) {
            candidate = Some((caret, ch));
        }
    }
    let (index, ch) = candidate?;
    pair_for(&masked, index, ch)
}

/// Monaco's `selectToBracket` pair: the bracket at/near the caret, otherwise
/// the innermost enclosing pair, otherwise the pair of the next structural
/// bracket.
pub(super) fn select_target(
    text: &str,
    caret: usize,
) -> Option<(Range<usize>, Range<usize>)> {
    bracket_target(text, caret, true)
}

pub(super) fn remove_target(
    text: &str,
    caret: usize,
) -> Option<(Range<usize>, Range<usize>)> {
    bracket_target(text, caret, false)
}

fn bracket_target(
    text: &str,
    caret: usize,
    allow_next: bool,
) -> Option<(Range<usize>, Range<usize>)> {
    let caret = caret.min(text.len());
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut candidate = None;
    if caret < text.len() {
        let ch = text[caret..].chars().next()?;
        if bracket(ch) {
            candidate = Some((caret, ch));
        }
    }
    if candidate.is_none()
        && caret > 0
        && let Some((index, ch)) = text[..caret].char_indices().next_back()
        && bracket(ch)
    {
        candidate = Some((index, ch));
    }
    if let Some((index, ch)) = candidate
        && let Some(pair) = pair_for(&masked, index, ch)
    {
        return Some(pair);
    }

    let mut stack = Vec::new();
    let mut enclosing = None;
    for (index, ch) in masked.char_indices() {
        match ch {
            '(' | '[' | '{' => stack.push((index, ch)),
            ')' | ']' | '}' => {
                let Some((open_index, open_ch)) = stack.pop() else {
                    continue;
                };
                if family(open_ch).is_some_and(|(_, close, _)| close == ch)
                    && open_index < caret
                    && caret <= index
                    && enclosing.is_none()
                {
                    enclosing = Some((open_index, index));
                }
            }
            _ => {}
        }
    }
    if let Some((open, close)) = enclosing {
        return Some((open..open + 1, close..close + 1));
    }
    if !allow_next {
        return None;
    }
    let (next, ch) = masked[caret..]
        .char_indices()
        .find(|(_, ch)| bracket(*ch))?;
    pair_for(&masked, caret + next, ch)
}

/// Monaco's `jumpToBracket` target: the partner of the bracket at the caret,
/// otherwise the closing bracket of the innermost enclosing pair, otherwise
/// the next structural bracket.
pub(super) fn jump_target(text: &str, caret: usize) -> Option<usize> {
    let caret = caret.min(text.len());
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut candidate = None;
    if caret < text.len() {
        let ch = text[caret..].chars().next()?;
        if bracket(ch) {
            candidate = Some((caret, ch));
        }
    }
    if candidate.is_none() && caret > 0
        && let Some((index, ch)) = text[..caret].char_indices().next_back()
        && bracket(ch)
    {
        candidate = Some((index, ch));
    }
    if let Some((index, ch)) = candidate
        && let Some((first, second)) = pair_for(&masked, index, ch)
    {
        return Some(if index == first.start {
            second.start
        } else {
            first.start
        });
    }

    let mut stack = Vec::new();
    let mut enclosing = None;
    for (index, ch) in masked.char_indices() {
        match ch {
            '(' | '[' | '{' => stack.push((index, ch)),
            ')' | ']' | '}' => {
                let Some((open_index, open_ch)) = stack.pop() else {
                    continue;
                };
                if family(open_ch).is_some_and(|(_, close, _)| close == ch)
                    && open_index < caret
                    && caret <= index
                    && enclosing.is_none()
                {
                    enclosing = Some(index);
                }
            }
            _ => {}
        }
    }
    if enclosing.is_some() {
        return enclosing;
    }
    masked[caret..]
        .char_indices()
        .find(|(_, ch)| bracket(*ch))
        .map(|(offset, _)| caret + offset)
}

/// Bracket spans with their nesting depth for colorization. Strings and
/// comments are masked first, so only structural brackets are returned.
pub(super) fn colorized(text: &str) -> Vec<(Range<usize>, usize)> {
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut depth = 0usize;
    let mut spans = Vec::new();
    for (index, ch) in masked.char_indices() {
        match ch {
            '(' | '[' | '{' => {
                spans.push((index..index + 1, depth));
                depth += 1;
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                spans.push((index..index + 1, depth));
            }
            _ => {}
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_target_uses_partner_enclosing_and_next_pair() {
        let text = "call(a[b]{c})";
        assert_eq!(select_target(text, 4), Some((4..5, 12..13)));
        assert_eq!(select_target(text, 5), Some((4..5, 12..13)));
        assert_eq!(select_target(text, 6), Some((6..7, 8..9)));
        assert_eq!(select_target(text, 0), Some((4..5, 12..13)));
        assert_eq!(select_target("a(b", 2), None);
    }

    #[test]
    fn remove_target_requires_a_matching_or_enclosing_pair() {
        assert_eq!(remove_target("call(a)", 4), Some((4..5, 6..7)));
        assert_eq!(remove_target("call(a)", 5), Some((4..5, 6..7)));
        assert_eq!(remove_target("call(a)", 0), None);
    }

    #[test]
    fn jump_target_uses_partner_enclosing_and_next_structural_bracket() {
        let text = "call(a[b]{c})";
        assert_eq!(jump_target(text, 4), Some(12));
        assert_eq!(jump_target(text, 5), Some(12));
        assert_eq!(jump_target(text, 6), Some(8));
        assert_eq!(jump_target(text, 12), Some(4));
        assert_eq!(jump_target(text, 0), Some(4));
        assert_eq!(jump_target("a(b", 2), None);
        assert_eq!(jump_target("a(b", 1), Some(1));
        assert_eq!(jump_target("// (\n(", 0), Some(5));
    }

    #[test]
    fn matching_uses_the_caret_family_and_mixed_nesting() {
        let text = "call(a[b]{c})";
        let (open, close) = matching_pair(text, 4).unwrap();
        assert_eq!(&text[open.clone()], "(");
        assert_eq!(&text[close.clone()], ")");
        assert_eq!(open.start, 4);
        assert_eq!(close.start, 12);

        let (open, close) = matching_pair(text, 13).unwrap();
        assert_eq!((open.start, close.start), (4, 12));

        let (open, close) = matching_pair(text, 6).unwrap();
        assert_eq!((&text[open], &text[close]), ("[", "]"));
    }

    #[test]
    fn caret_after_opening_or_before_closing_matches() {
        let text = "value = (1 + 2);";
        let after_open = text.find('(').unwrap() + 1;
        assert_eq!(matching_pair(text, after_open).unwrap().0.start, 8);
        let before_close = text.find(')').unwrap();
        assert_eq!(matching_pair(text, before_close).unwrap().1.start, 14);
    }

    #[test]
    fn strings_comments_and_unbalanced_brackets_are_ignored() {
        let text = "const s = \"([{\";\n// ([\ncall(1);";
        let string_open = text.find("\"([").unwrap() + 1;
        assert!(matching_pair(text, string_open).is_none());
        let comment_open = text.find("// (").unwrap() + 3;
        assert!(matching_pair(text, comment_open).is_none());
        assert!(matching_pair("call(a(b)", 5).is_none());
        assert!(matching_pair("plain text", 4).is_none());
    }

    #[test]
    fn colorized_depths_alternate_and_skip_strings_and_comments() {
        let text = "a(b[c]{d}) // (ignored)\n\"([{ignored}\")\nfunction f() {\n  return {k: 1};\n}";
        let spans = colorized(text);
        let depth_at = |index: usize| {
            spans
                .iter()
                .find(|(range, _)| range.start == index)
                .map(|(_, depth)| *depth)
                .unwrap()
        };
        assert_eq!(depth_at(1), 0, "outer paren");
        assert_eq!(depth_at(3), 1, "nested bracket");
        assert_eq!(depth_at(5), 1, "closing bracket shares its pair depth");
        assert_eq!(depth_at(6), 1, "brace after the closed bracket");
        assert_eq!(depth_at(8), 1);
        assert_eq!(depth_at(9), 0, "outer paren closes to depth zero");
        // String/comment brackets are excluded from the color list.
        assert_eq!(
            spans.len(),
            text.matches(['(', ')', '[', ']', '{', '}']).count() - 6
        );
        assert_well_formed(&spans, text);
    }

    fn assert_well_formed(spans: &[(Range<usize>, usize)], text: &str) {
        let mut last = 0;
        for (range, _) in spans {
            assert!(range.start >= last);
            assert_eq!(range.end - range.start, 1);
            assert!(text.is_char_boundary(range.start));
            assert!(text.is_char_boundary(range.end));
            last = range.end;
        }
        assert!(last <= text.len());
    }

    #[test]
    fn multiline_braces_and_escaped_quotes_match_outside_strings() {
        let text = "function f() {\n  const s = \"}\\\"{\";\n}\n";
        let close = text.rfind('}').unwrap();
        let (open, close_range) = matching_pair(text, close).unwrap();
        assert_eq!(open.start, 13);
        assert_eq!(close_range.start, close);
    }
}
