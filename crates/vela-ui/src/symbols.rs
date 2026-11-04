//! Bounded document symbols for go-to-definition and find-references.
//!
//! This is intentionally local and syntax-light: declarations are recognized
//! from common `function`/`class`/`const`/`let`/`var`/type-style lines and
//! references are whole-identifier occurrences in the active buffer.

use std::ops::Range;
use vela_core::document::DocumentKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SymbolLocation {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub preview: String,
}

fn word_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$')
}

pub(super) fn word_at(text: &str, caret: usize) -> Option<(Range<usize>, String)> {
    let range = crate::completion::word_range_at(text, caret);
    (range.start < range.end).then(|| (range.clone(), text[range].to_owned()))
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        text.bytes()
            .enumerate()
            .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
    );
    starts
}

fn location(text: &str, starts: &[usize], offset: usize) -> SymbolLocation {
    let line_index = starts
        .partition_point(|start| *start <= offset)
        .saturating_sub(1);
    let line = line_index + 1;
    let line_start = starts.get(line_index).copied().unwrap_or(0);
    let column = text[line_start..offset].chars().count() + 1;
    let line_end = starts
        .get(line_index + 1)
        .map(|next| next.saturating_sub(1))
        .unwrap_or(text.len());
    let mut preview = text[line_start..line_end].trim().to_owned();
    if preview.chars().count() > 96 {
        preview = preview.chars().take(95).collect::<String>() + "…";
    }
    SymbolLocation {
        offset,
        line,
        column,
        preview,
    }
}

fn declaration_name(line: &str) -> Option<(usize, &str)> {
    let mut rest = line.trim_start();
    let mut base = line.len() - rest.len();
    loop {
        let mut stripped = None;
        for prefix in ["export ", "default ", "async "] {
            if let Some(tail) = rest.strip_prefix(prefix) {
                stripped = Some((prefix.len(), tail.trim_start()));
                break;
            }
        }
        if let Some((prefix_len, tail)) = stripped {
            base += prefix_len + (rest[prefix_len..].len() - tail.len());
            rest = tail;
        } else {
            break;
        }
    }
    let mut keyword_len = 0;
    for keyword in [
        "function",
        "class",
        "const",
        "let",
        "var",
        "interface",
        "type",
        "enum",
        "def",
        "fn",
        "struct",
        "trait",
    ] {
        if rest.starts_with(keyword)
            && rest[keyword.len()..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_whitespace() || ch == '*')
        {
            keyword_len = keyword.len();
            break;
        }
    }
    if keyword_len == 0 {
        return None;
    }
    let mut tail = &rest[keyword_len..];
    let mut removed = keyword_len;
    while tail.starts_with('*') || tail.starts_with(char::is_whitespace) {
        let ch = tail.chars().next()?;
        tail = &tail[ch.len_utf8()..];
        removed += ch.len_utf8();
    }
    let name_len = tail
        .char_indices()
        .take_while(|(_, ch)| word_char(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    let name = &tail[..name_len];
    if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_alphabetic() || ch == '_' || ch == '$')
    {
        Some((base + removed, name))
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Declaration {
    pub name: String,
    pub location: SymbolLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WorkspaceSymbol {
    pub key: DocumentKey,
    pub name: String,
    pub location: SymbolLocation,
}

pub(super) fn declarations(text: &str) -> Vec<Declaration> {
    let starts = line_starts(text);
    let mut found = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts
            .get(index + 1)
            .map(|next| next.saturating_sub(1))
            .unwrap_or(text.len());
        if let Some((offset, name)) = declaration_name(&text[*start..end]) {
            found.push(Declaration {
                name: name.to_owned(),
                location: location(text, &starts, start + offset),
            });
        }
    }
    found
}

pub(super) fn definitions(text: &str, name: &str) -> Vec<SymbolLocation> {
    declarations(text)
        .into_iter()
        .filter(|declaration| declaration.name == name)
        .map(|declaration| declaration.location)
        .collect()
}

/// Whole-word references across open workspace documents, file-qualified and
/// ordered by path/offset. The limit applies after sorting.
pub(super) fn workspace_references(
    files: &[(DocumentKey, String)],
    name: &str,
    limit: usize,
) -> Vec<WorkspaceSymbol> {
    let mut found = Vec::new();
    for (key, text) in files {
        for location in references(text, name) {
            found.push(WorkspaceSymbol {
                key: key.clone(),
                name: name.to_owned(),
                location,
            });
        }
    }
    found.sort_by(|left, right| {
        left.key
            .directory
            .cmp(&right.key.directory)
            .then_with(|| left.key.rel.cmp(&right.key.rel))
            .then_with(|| left.location.offset.cmp(&right.location.offset))
    });
    found.truncate(limit);
    found
}

/// Exact-name declarations from open workspace documents, used as a cross-file
/// go-to-definition fallback.
pub(super) fn workspace_definitions(
    files: &[(DocumentKey, String)],
    name: &str,
) -> Vec<WorkspaceSymbol> {
    workspace_symbols(files, name, 100)
        .into_iter()
        .filter(|symbol| symbol.name == name)
        .collect()
}

/// Index declarations from open workspace documents. The query matches the
/// symbol name case-insensitively by prefix; an empty query lists everything.
pub(super) fn workspace_symbols(
    files: &[(DocumentKey, String)],
    query: &str,
    limit: usize,
) -> Vec<WorkspaceSymbol> {
    let query = query.to_lowercase();
    let mut found = Vec::new();
    for (key, text) in files {
        for declaration in declarations(text) {
            if !query.is_empty() && !declaration.name.to_lowercase().starts_with(&query) {
                continue;
            }
            found.push(WorkspaceSymbol {
                key: key.clone(),
                name: declaration.name,
                location: declaration.location,
            });
        }
    }
    found.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.key.directory.cmp(&right.key.directory))
            .then_with(|| left.key.rel.cmp(&right.key.rel))
            .then_with(|| left.location.offset.cmp(&right.location.offset))
    });
    found.truncate(limit);
    found
}

pub(super) fn references(text: &str, name: &str) -> Vec<SymbolLocation> {
    let starts = line_starts(text);
    let mut found = Vec::new();
    let mut iter = text.char_indices().peekable();
    while let Some((offset, ch)) = iter.next() {
        if !word_char(ch) {
            continue;
        }
        let start = offset;
        let mut end = offset + ch.len_utf8();
        while let Some((next, ch)) = iter.peek().copied() {
            if !word_char(ch) {
                break;
            }
            end = next + ch.len_utf8();
            iter.next();
        }
        if &text[start..end] == name {
            found.push(location(text, &starts, start));
            if found.len() >= 100 {
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations_cover_common_javascript_and_type_forms() {
        let text = "export function alpha() {}\nasync function beta() {}\nconst gamma = 1;\nlet delta = 2;\nvar epsilon = 3;\nclass Zeta {}\ninterface Eta {}\ntype Theta = string;\nfunction* star() {}";
        assert_eq!(definitions(text, "alpha")[0].line, 1);
        assert_eq!(definitions(text, "alpha")[0].column, 17);
        assert_eq!(definitions(text, "beta")[0].line, 2);
        assert_eq!(definitions(text, "gamma")[0].line, 3);
        assert_eq!(definitions(text, "delta")[0].line, 4);
        assert_eq!(definitions(text, "epsilon")[0].line, 5);
        assert_eq!(definitions(text, "Zeta")[0].line, 6);
        assert_eq!(definitions(text, "Eta")[0].line, 7);
        assert_eq!(definitions(text, "Theta")[0].line, 8);
        assert_eq!(definitions(text, "star")[0].line, 9);
        assert!(definitions(text, "notThere").is_empty());
    }

    #[test]
    fn references_are_whole_identifier_matches_in_source_order() {
        let text = "function foo() {}\nfoo();\nconst foobar = foo;\n// foo\n";
        let found = references(text, "foo");
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].offset, 9);
        assert_eq!(found[0].line, 1);
        assert_eq!(found[0].column, 10);
        assert_eq!(found[1].line, 2);
        assert_eq!(found[2].line, 3);
        assert!(found.windows(2).all(|pair| pair[0].offset < pair[1].offset));
        assert!(found.iter().all(|location| !location.preview.is_empty()));
        assert!(
            references(text, "foobar")
                .iter()
                .all(|location| location.line == 3)
        );
        assert!(references(text, "fo").is_empty());
    }

    #[test]
    fn word_queries_use_the_identifier_around_the_caret() {
        let text = "function alpha() {\n  return alpha;\n}";
        let (range, word) = word_at(text, text.find("return ").unwrap() + 9).unwrap();
        assert_eq!(word, "alpha");
        assert_eq!(
            range,
            text.find("return alpha").unwrap() + 7..text.find("return alpha").unwrap() + 12
        );
        assert!(word_at(text, text.len()).is_none());
        assert!(word_at("  ", 1).is_none());

        let unicode = "const café = 1;\nreturn café;";
        let caret = unicode.rfind("café").unwrap() + 2;
        assert_eq!(word_at(unicode, caret).unwrap().1, "café");
    }

    #[test]
    fn workspace_symbols_filter_sort_and_stay_file_qualified() {
        let a = DocumentKey::new("/ws".into(), "a.js".into());
        let b = DocumentKey::new("/ws".into(), "src/b.ts".into());
        let files = vec![
            (
                a.clone(),
                "function beta() {}\nconst alpha = 1;\nclass AlphaTwo {}".into(),
            ),
            (
                b.clone(),
                "type Alpha = string;\nfunction zeta() {}\nzeta;".into(),
            ),
        ];
        let all = workspace_symbols(&files, "", 10);
        assert_eq!(
            all.iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "Alpha", "AlphaTwo", "beta", "zeta"]
        );
        assert_eq!(all[0].key, a);
        assert_eq!(all[0].location.line, 2);
        let alpha = workspace_symbols(&files, "ALP", 10);
        assert_eq!(alpha.len(), 3);
        assert!(
            alpha
                .iter()
                .all(|symbol| symbol.name.to_lowercase().starts_with("alp"))
        );
        assert_eq!(workspace_symbols(&files, "bet", 10)[0].key, a);
        assert!(workspace_symbols(&files, "nomatch", 10).is_empty());
        assert_eq!(workspace_symbols(&files, "", 2).len(), 2);
        assert_eq!(workspace_symbols(&files, "alpha", 0).len(), 0);
        assert_eq!(all[0].location.column, 7);

        let exact = workspace_definitions(&files, "Alpha");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].key, b);
        assert_eq!(exact[0].location.line, 1);
        assert!(workspace_definitions(&files, "ALPHA").is_empty());
        assert_eq!(workspace_definitions(&files, "alpha").len(), 1);

        let references = workspace_references(&files, "zeta", 10);
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].key, b);
        assert_eq!(references[1].key, b);
        assert_eq!(references[0].location.line, 2);
        assert_eq!(references[1].location.line, 3);
        assert_eq!(workspace_references(&files, "zeta", 1).len(), 1);
        assert!(workspace_references(&files, "missing", 10).is_empty());
    }
}
