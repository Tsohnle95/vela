//! Editor Find uses Unicode-safe byte ranges, literal search by default and
//! an explicit regular-expression mode. Highlight limits never limit navigation
//! or replacement of the document's remaining text.
use regex::{Regex, RegexBuilder};
use std::ops::Range;

pub const HIGHLIGHT_LIMIT: usize = 19_999;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub match_case: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub preserve_case: bool,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    regex: Option<Regex>,
    options: Options,
    scope: Option<Range<usize>>,
}

#[derive(Debug, Default)]
pub struct Highlights {
    pub ranges: Vec<Range<usize>>,
    pub more: bool,
}

impl Pattern {
    pub fn new(query: &str, options: Options) -> Result<Self, String> {
        Self::new_in(query, options, None)
    }

    pub fn new_in(
        query: &str,
        options: Options,
        scope: Option<Range<usize>>,
    ) -> Result<Self, String> {
        let regex = if query.is_empty() {
            None
        } else {
            let source = if options.regex {
                query.to_owned()
            } else {
                regex::escape(query)
            };
            Some(
                RegexBuilder::new(&source)
                    .case_insensitive(!options.match_case)
                    .build()
                    .map_err(|error| error.to_string())?,
            )
        };
        Ok(Self {
            regex,
            options,
            scope,
        })
    }

    pub fn matches<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
        self.regex
            .iter()
            .flat_map(move |regex| regex.find_iter(text))
            .map(|found| found.range())
            .filter(move |range| self.in_scope(range))
            .filter(move |range| !self.options.whole_word || whole_word(text, range))
    }

    fn in_scope(&self, range: &Range<usize>) -> bool {
        self.scope
            .as_ref()
            .is_none_or(|scope| range.start >= scope.start && range.end <= scope.end)
    }

    pub fn highlights(&self, text: &str) -> Highlights {
        let mut matches = self.matches(text);
        let ranges = matches.by_ref().take(HIGHLIGHT_LIMIT).collect();
        Highlights {
            ranges,
            more: matches.next().is_some(),
        }
    }

    pub fn next(&self, text: &str, from: usize) -> Option<Range<usize>> {
        self.matches(text)
            .find(|range| range.start >= from)
            .or_else(|| self.matches(text).next())
    }

    pub fn previous(&self, text: &str, before: usize) -> Option<Range<usize>> {
        self.matches(text)
            .take_while(|range| range.start < before)
            .last()
            .or_else(|| self.matches(text).last())
    }

    /// Expand one matched range using regular-expression captures, or return
    /// the replacement unchanged in literal mode. Preserve-case adjusts the
    /// expansion using the casing pattern of the matched text.
    pub fn expand(&self, text: &str, range: &Range<usize>, replacement: &str) -> String {
        let expanded = if !self.options.regex {
            replacement.to_owned()
        } else {
            let Some(regex) = &self.regex else {
                return replacement.to_owned();
            };
            let Some(captures) = regex.captures_at(text, range.start) else {
                return replacement.to_owned();
            };
            if captures.get(0).map(|found| found.range()) != Some(range.clone()) {
                return replacement.to_owned();
            }
            let mut expanded = String::new();
            captures.expand(replacement, &mut expanded);
            expanded
        };
        if self.options.preserve_case {
            preserve_case(&text[range.clone()], &expanded)
        } else {
            expanded
        }
    }

    /// Replacement is literal in literal-search mode, including dollar signs.
    /// The caller installs this as one document edit so Undo is atomic.
    pub fn replace_all(&self, text: &str, replacement: &str) -> (String, usize) {
        let mut result = String::with_capacity(text.len());
        let (mut cursor, mut count) = (0, 0);
        for range in self.matches(text) {
            result.push_str(&text[cursor..range.start]);
            result.push_str(&self.expand(text, &range, replacement));
            cursor = range.end;
            count += 1;
        }
        result.push_str(&text[cursor..]);
        (result, count)
    }
}

fn separator(ch: char) -> bool {
    ch.is_whitespace() || "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?".contains(ch)
}

fn preserve_case(matched: &str, replacement: &str) -> String {
    let mut letters = matched.chars().filter(|ch| ch.is_alphabetic());
    let Some(first) = letters.next() else {
        return replacement.to_owned();
    };
    let rest = letters.collect::<String>();
    if matched.chars().filter(|ch| ch.is_alphabetic()).all(|ch| ch.is_lowercase()) {
        return replacement.to_lowercase();
    }
    if matched.chars().filter(|ch| ch.is_alphabetic()).all(|ch| ch.is_uppercase()) {
        return replacement.to_uppercase();
    }
    if first.is_uppercase() && rest.chars().all(|ch| ch.is_lowercase()) {
        let mut chars = replacement.chars();
        return chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect())
            .unwrap_or_default();
    }
    replacement.to_owned()
}

fn whole_word(text: &str, range: &Range<usize>) -> bool {
    let found = &text[range.clone()];
    let left = range.start == 0
        || text[..range.start]
            .chars()
            .next_back()
            .is_some_and(separator)
        || found.chars().next().is_some_and(separator);
    let right = range.end == text.len()
        || text[range.end..].chars().next().is_some_and(separator)
        || found.chars().next_back().is_some_and(separator);
    left && right
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_unicode_matches_preserve_byte_ranges_and_case_options() {
        let text = "界🌍 Foo foo FOO f.o [x]";
        let pattern = Pattern::new("foo", Options::default()).unwrap();
        assert_eq!(
            pattern.matches(text).map(|r| &text[r]).collect::<Vec<_>>(),
            ["Foo", "foo", "FOO"]
        );
        let pattern = Pattern::new(
            "foo",
            Options {
                match_case: true,
                whole_word: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pattern.matches(text).count(), 1);
        assert_eq!(
            Pattern::new("f.o", Options::default())
                .unwrap()
                .matches(text)
                .count(),
            1
        );
        assert_eq!(
            Pattern::new("[x]", Options::default())
                .unwrap()
                .matches(text)
                .count(),
            1
        );
        assert_eq!(
            Pattern::new("🌍", Options::default())
                .unwrap()
                .next(text, 0),
            Some(3..7)
        );
        assert_eq!(
            Pattern::new("", Options::default())
                .unwrap()
                .matches(text)
                .count(),
            0
        );
    }
    #[test]
    fn whole_words_follow_editor_separators_and_navigation_wraps() {
        let text = "food foo foo_bar $foo foo.";
        let pattern = Pattern::new(
            "foo",
            Options {
                whole_word: true,
                ..Default::default()
            },
        )
        .unwrap();
        let ranges = pattern.matches(text).collect::<Vec<_>>();
        assert_eq!(ranges, [5..8, 18..21, 22..25]);
        assert_eq!(pattern.next(text, text.len()), Some(5..8));
        assert_eq!(pattern.previous(text, 0), Some(22..25));
        assert_eq!(pattern.next(text, 8), Some(18..21));
    }
    #[test]
    fn highlight_limit_does_not_truncate_navigation_or_replace_all_and_undo() {
        let text = "界x ".repeat(HIGHLIGHT_LIMIT + 2);
        let pattern = Pattern::new("x", Options::default()).unwrap();
        let highlights = pattern.highlights(&text);
        assert_eq!(highlights.ranges.len(), HIGHLIGHT_LIMIT);
        assert!(highlights.more);
        assert_eq!(
            pattern.previous(&text, text.len()),
            Some(text.len() - 2..text.len() - 1)
        );
        let (replaced, count) = pattern.replace_all(&text, "$1🌍");
        assert_eq!(count, HIGHLIGHT_LIMIT + 2);
        let mut buffer = crate::text::TextBuffer::new(text.clone());
        buffer.replace(0..text.len(), &replaced);
        assert!(buffer.text().ends_with("界$1🌍 "));
        buffer.undo();
        assert_eq!(buffer.text(), text);
    }

    #[test]
    fn regular_expressions_support_captures_whole_words_and_invalid_patterns() {
        let text = "foo-12 food-9 foo-7";
        let pattern = Pattern::new(
            r"foo-(\d+)",
            Options {
                regex: true,
                whole_word: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            pattern.matches(text).map(|r| &text[r]).collect::<Vec<_>>(),
            ["foo-12", "foo-7"]
        );
        let (replaced, count) = pattern.replace_all(text, "<$1>");
        assert_eq!(count, 2);
        assert_eq!(replaced, "<12> food-9 <7>");
        assert_eq!(
            pattern.expand(text, &(0..6), "[$1]"),
            "[12]"
        );
        let whole = Pattern::new(
            r"foo-\d+",
            Options {
                regex: true,
                whole_word: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(whole.matches("xfoo-1 foo-2").count(), 1);
        assert!(Pattern::new(
            "(",
            Options {
                regex: true,
                ..Default::default()
            }
        )
        .is_err());
        let literal = Pattern::new(
            "(",
            Options {
                regex: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(literal.matches("a ( b").count(), 1);
    }

    #[test]
    fn zero_width_regex_matches_do_not_stall_replacement() {
        let pattern = Pattern::new(
            r"\b",
            Options {
                regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let (replaced, count) = pattern.replace_all("one two", "|");
        assert_eq!(count, 4);
        assert_eq!(replaced, "|one| |two|");
    }

    #[test]
    fn selection_scope_limits_navigation_and_replacement_but_preserves_outside_text() {
        let text = "needle outside needle inside needle";
        let scope = 10..25;
        let pattern = Pattern::new_in("needle", Options::default(), Some(scope)).unwrap();
        assert_eq!(pattern.matches(text).collect::<Vec<_>>(), vec![15..21]);
        assert_eq!(pattern.next(text, 0), Some(15..21));
        assert_eq!(pattern.previous(text, text.len()), Some(15..21));
        let (replaced, count) = pattern.replace_all(text, "X");
        assert_eq!(count, 1);
        assert_eq!(replaced, "needle outside X inside needle");
    }

    #[test]
    fn preserve_case_follows_lower_upper_and_capitalized_matches() {        let pattern = Pattern::new(
            "foo",
            Options {
                match_case: false,
                preserve_case: true,
                ..Default::default()
            },
        )
        .unwrap();
        let (replaced, count) = pattern.replace_all("foo FOO Foo fOo", "Bar");
        assert_eq!(count, 4);
        assert_eq!(replaced, "bar BAR Bar Bar");
        let literal = Pattern::new(
            "price",
            Options {
                preserve_case: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(literal.replace_all("Price price", "<$1>").0, "<$1> <$1>");
    }
}
