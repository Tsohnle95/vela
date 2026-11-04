//! Document occurrence highlights for the word at the caret.
//!
//! This mirrors Monaco's default occurrences highlight: the identifier under a
//! collapsed caret is matched as a whole word throughout the buffer, bounded so
//! pathological identifiers cannot paint thousands of ranges.

use std::ops::Range;

pub(super) const MAX_OCCURRENCES: usize = 100;

fn word_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$')
}

/// Whole-word occurrences of the identifier at `caret`, in source order. The
/// caret may sit anywhere inside the identifier. Returns empty for whitespace,
/// punctuation or a non-collapsed selection handled by the caller.
pub(super) fn word_occurrences(text: &str, caret: usize, limit: usize) -> Vec<Range<usize>> {
    if limit == 0 {
        return Vec::new();
    }
    let range = crate::completion::word_range_at(text, caret);
    if range.start == range.end {
        return Vec::new();
    }
    whole_word_ranges(text, &text[range], limit)
}

/// Whole-word ranges for an explicit identifier, shared by occurrence
/// highlighting and rename planning.
pub(super) fn whole_word_ranges(text: &str, word: &str, limit: usize) -> Vec<Range<usize>> {
    if word.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (start, _) in text.match_indices(word) {
        let end = start + word.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if before.is_some_and(word_char) || after.is_some_and(word_char) {
            continue;
        }
        found.push(start..end);
        if found.len() >= limit {
            break;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_whole_word_occurrences_in_source_order() {
        let text = "foo bar foo foobar barfoo foo";
        assert_eq!(
            word_occurrences(text, 1, MAX_OCCURRENCES),
            vec![0..3, 8..11, 26..29]
        );
        assert_eq!(word_occurrences(text, 5, MAX_OCCURRENCES), vec![4..7]);
    }

    #[test]
    fn handles_unicode_identifiers_and_caret_edges() {
        let text = "const café = 1;\nreturn café;";
        let caret = text.rfind("café").unwrap() + 2;
        let occurrences = word_occurrences(text, caret, MAX_OCCURRENCES);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(&text[occurrences[0].clone()], "café");
        assert_eq!(&text[occurrences[1].clone()], "café");

        assert!(word_occurrences("  \n\t", 1, MAX_OCCURRENCES).is_empty());
        assert!(word_occurrences("word ", 5, MAX_OCCURRENCES).is_empty());
        assert!(word_occurrences("word", 0, 0).is_empty());
    }

    #[test]
    fn occurrence_limits_are_bounded() {
        let text = (0..150).map(|_| "x").collect::<Vec<_>>().join(" ");
        let found = word_occurrences(&text, 0, 10);
        assert_eq!(found.len(), 10);
        assert_eq!(found[0], 0..1);
        assert_eq!(found[9], 18..19);
    }
}
