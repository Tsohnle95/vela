//! Bounded whole-word rename planning for open workspace documents.
//!
//! This is intentionally syntax-light: it validates a JavaScript-style
//! identifier and replaces whole-word matches in open buffers. It is not a
//! language-server rename.

use std::ops::Range;

use vela_core::document::DocumentKey;

pub(super) const MAX_RENAME_OCCURRENCES: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RenameError {
    Empty,
    Invalid,
    Unchanged,
    TooMany,
    NoSymbol,
}

impl RenameError {
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "Enter a new name.".into(),
            Self::Invalid => {
                "Names must start with a letter, _ or $ and contain only letters, numbers, _ or $."
                    .into()
            }
            Self::Unchanged => "Enter a name different from the current name.".into(),
            Self::TooMany => format!(
                "This rename affects more than {MAX_RENAME_OCCURRENCES} occurrences; narrow the workspace first."
            ),
            Self::NoSymbol => "Place the caret on an identifier to rename.".into(),
        }
    }
}

pub(super) type RenameFile = (DocumentKey, Vec<Range<usize>>);

pub(super) fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_alphabetic() || matches!(first, '_' | '$'))
        && chars.all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
}

/// Whole-word occurrences across the supplied open documents. Documents with
/// no matches are omitted; the combined result is bounded.
pub(super) fn occurrences(
    files: &[(DocumentKey, String)],
    word: &str,
) -> Result<Vec<RenameFile>, RenameError> {
    if word.is_empty() {
        return Err(RenameError::NoSymbol);
    }
    let mut targets = Vec::new();
    let mut total = 0usize;
    for (key, text) in files {
        let ranges =
            crate::highlights::whole_word_ranges(text, word, MAX_RENAME_OCCURRENCES + 1 - total);
        if ranges.is_empty() {
            continue;
        }
        total += ranges.len();
        if total > MAX_RENAME_OCCURRENCES {
            return Err(RenameError::TooMany);
        }
        targets.push((key.clone(), ranges));
    }
    if targets.is_empty() {
        return Err(RenameError::NoSymbol);
    }
    Ok(targets)
}

pub(super) fn plan(
    files: &[(DocumentKey, String)],
    word: &str,
    new_name: &str,
) -> Result<Vec<RenameFile>, RenameError> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(RenameError::Empty);
    }
    if !valid_identifier(new_name) {
        return Err(RenameError::Invalid);
    }
    if new_name == word {
        return Err(RenameError::Unchanged);
    }
    occurrences(files, word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> DocumentKey {
        DocumentKey::new("/ws".into(), name.into())
    }

    fn files() -> Vec<(DocumentKey, String)> {
        vec![
            (
                key("a.js"),
                "function foo() {}\nfoo();\nconst foobar = 1;\n".into(),
            ),
            (key("b.js"), "foo();\n// foo\n".into()),
        ]
    }

    #[test]
    fn identifiers_allow_unicode_letters_digits_and_underscore_dollar_rules() {
        for name in ["bar", "_bar", "$bar", "café", "a1", "A_B$9"] {
            assert!(valid_identifier(name), "{name}");
        }
        for name in ["", "1bar", "foo-bar", "foo bar", "🌍", "foo.bar"] {
            assert!(!valid_identifier(name), "{name}");
        }
    }

    #[test]
    fn rename_planning_is_whole_word_cross_file_and_bounded() {
        let planned = plan(&files(), "foo", "bar").unwrap();
        assert_eq!(planned.len(), 2);
        assert_eq!(planned[0].0, key("a.js"));
        assert_eq!(planned[0].1, vec![9..12, 18..21]);
        assert_eq!(planned[1].0, key("b.js"));
        assert_eq!(planned[1].1, vec![0..3, 10..13]);
        assert_eq!(plan(&files(), "foo", "foo"), Err(RenameError::Unchanged));
        assert_eq!(plan(&files(), "foo", ""), Err(RenameError::Empty));
        assert_eq!(plan(&files(), "foo", "1x"), Err(RenameError::Invalid));
        assert_eq!(plan(&files(), "missing", "bar"), Err(RenameError::NoSymbol));
    }

    #[test]
    fn excessive_matches_refuse_the_rename() {
        let text = (0..=MAX_RENAME_OCCURRENCES)
            .map(|_| "foo")
            .collect::<Vec<_>>()
            .join(" ");
        let files = vec![(key("many.js"), text)];
        assert_eq!(plan(&files, "foo", "bar"), Err(RenameError::TooMany));
    }
}
