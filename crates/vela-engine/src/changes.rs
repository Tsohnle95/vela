//! Context-owned review state. Observation remains physical-root-owned; clearing
//! a review never edits a file, changes its baseline, or clears sibling contexts.
use crate::observation::{Baseline, ObservedFile};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Review {
    files: BTreeMap<String, ObservedFile>,
    hidden: BTreeSet<String>,
}
impl Review {
    pub fn receive(&mut self, changes: &[ObservedFile]) {
        for change in changes {
            let clean = match &change.baseline {
                Baseline::Unknown => false,
                Baseline::Known { content, existed } => {
                    (!change.deleted && change.content.as_deref() == Some(content.as_ref()))
                        || (change.deleted && !existed)
                }
            };
            if clean {
                self.files.remove(&change.rel);
            } else {
                self.files.insert(change.rel.clone(), change.clone());
            }
        }
    }
    pub fn files(&self) -> impl Iterator<Item = &ObservedFile> {
        self.files
            .values()
            .filter(|file| !self.hidden.iter().any(|prefix| within(&file.rel, prefix)))
    }
    pub fn get(&self, path: &str) -> Option<&ObservedFile> {
        self.files().find(|file| file.rel == path)
    }
    pub fn dismiss(&mut self, path: &str) {
        self.files.remove(path);
    }
    pub fn clear(&mut self) {
        self.files.clear();
    }
    pub fn hide(&mut self, path: String) {
        self.hidden.insert(path);
    }
    pub fn unhide(&mut self, path: &str) {
        self.hidden.remove(path);
    }

    pub fn relocate(&mut self, from: &str, to: &str) {
        self.files = std::mem::take(&mut self.files)
            .into_iter()
            .map(|(path, mut file)| {
                if within(&path, from) {
                    let path = format!("{to}{}", &path[from.len()..]);
                    file.rel = path.clone();
                    (path, file)
                } else {
                    (path, file)
                }
            })
            .collect();
    }
}
fn within(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub directory: bool,
    pub path: String,
    pub label: String,
    pub depth: usize,
}
#[derive(Default)]
struct Directory {
    directories: BTreeMap<String, Directory>,
    files: BTreeSet<String>,
}
pub fn tree_rows<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    collapsed: &BTreeSet<String>,
) -> Vec<TreeRow> {
    let mut root = Directory::default();
    for path in paths {
        let mut parts = path.split('/').peekable();
        let mut node = &mut root;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                node.files.insert(part.to_owned());
            } else {
                node = node.directories.entry(part.to_owned()).or_default();
            }
        }
    }
    fn walk(
        node: &Directory,
        prefix: &str,
        depth: usize,
        collapsed: &BTreeSet<String>,
        rows: &mut Vec<TreeRow>,
    ) {
        let join = |name: &str| {
            if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            }
        };
        for (name, child) in {
            let mut directories = node.directories.iter().collect::<Vec<_>>();
            directories.sort_by(|(left, _), (right, _)| {
                left.to_lowercase()
                    .cmp(&right.to_lowercase())
                    .then_with(|| left.cmp(right))
            });
            directories
        } {
            let mut label = name.clone();
            let mut path = join(name);
            let mut child = child;
            while child.files.is_empty() && child.directories.len() == 1 {
                let (name, next) = child.directories.first_key_value().unwrap();
                label.push('/');
                label.push_str(name);
                path.push('/');
                path.push_str(name);
                child = next;
            }
            rows.push(TreeRow {
                directory: true,
                path: path.clone(),
                label,
                depth,
            });
            if !collapsed.contains(&path) {
                walk(child, &path, depth + 1, collapsed, rows);
            }
        }
        for name in {
            let mut files = node.files.iter().collect::<Vec<_>>();
            files.sort_by(|left, right| {
                left.to_lowercase()
                    .cmp(&right.to_lowercase())
                    .then_with(|| left.cmp(right))
            });
            files
        } {
            rows.push(TreeRow {
                directory: false,
                path: join(name),
                label: name.clone(),
                depth,
            });
        }
    }
    let mut rows = Vec::new();
    walk(&root, "", 0, collapsed, &mut rows);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(path: &str, before: Option<(&str, bool)>, after: Option<&str>) -> ObservedFile {
        ObservedFile {
            rel: path.into(),
            baseline: before
                .map(|(content, existed)| Baseline::Known {
                    content: content.into(),
                    existed,
                })
                .unwrap_or(Baseline::Unknown),
            content: after.map(Into::into),
            deleted: after.is_none(),
            moved_from: None,
            write_id: None,
        }
    }
    #[test]
    fn clean_known_baselines_disappear_but_unknown_and_real_deletions_remain() {
        let mut review = Review::default();
        review.receive(&[
            file("same", Some(("a", true)), Some("a")),
            file("new-gone", Some(("", false)), None),
            file("deleted", Some(("a", true)), None),
            file("unknown", None, Some("a")),
        ]);
        assert_eq!(
            review.files().map(|f| f.rel.as_str()).collect::<Vec<_>>(),
            ["deleted", "unknown"]
        );
        review.receive(&[file("deleted", Some(("a", true)), Some("a"))]);
        assert!(review.get("deleted").is_none());
    }
    #[test]
    fn dismissal_is_local_and_only_a_new_event_reintroduces_a_change() {
        let snapshot = vec![file("a", Some(("old", true)), Some("new"))];
        let mut a = Review::default();
        let mut b = Review::default();
        a.receive(&snapshot);
        b.receive(&snapshot);
        a.dismiss("missing");
        a.dismiss("a");
        assert!(a.files().next().is_none());
        assert!(b.get("a").is_some());
        // Navigation reads the retained review; it does not reseed snapshots.
        assert!(a.files().next().is_none());
        a.receive(&[file("a", Some(("old", true)), Some("newer"))]);
        assert_eq!(a.get("a").unwrap().content.as_deref(), Some("newer"));
        a.clear();
        assert!(a.files().next().is_none());
        assert!(b.get("a").is_some());
        assert_eq!(snapshot[0].content.as_deref(), Some("new"));
    }
    #[test]
    fn hidden_prefixes_and_relocations_do_not_capture_similarly_named_siblings() {
        let mut review = Review::default();
        review.receive(&[
            file("src/vendor/a", None, Some("a")),
            file("src/vendor-2/b", None, Some("b")),
            file("old/界🌍", Some(("x", true)), Some("y")),
        ]);
        review.hide("src/vendor".into());
        review.relocate("old", "new");
        assert_eq!(
            review.files().map(|f| f.rel.as_str()).collect::<Vec<_>>(),
            ["new/界🌍", "src/vendor-2/b"]
        );
        review.receive(&[file("src/vendor/late", None, Some("late"))]);
        assert!(review.get("src/vendor/late").is_none());
        review.unhide("src/vendor-2");
        assert!(review.get("src/vendor/late").is_none());
        review.unhide("src/vendor");
        assert!(review.get("src/vendor/late").is_some());
    }
    #[test]
    fn tree_sorts_case_insensitively_without_losing_stable_case_order() {
        let rows = tree_rows(
            ["Zeta/a", "alpha/a", "Beta/a", "beta/b", "Alpha/c"],
            &BTreeSet::new(),
        );
        let directories = rows
            .iter()
            .filter(|row| row.directory)
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(directories, ["Alpha", "alpha", "Beta", "beta", "Zeta"]);
        let files = rows
            .iter()
            .filter(|row| !row.directory)
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(files, ["c", "a", "a", "b", "a"]);
    }

    #[test]
    fn tree_matches_source_directory_first_compaction_and_collapse_examples() {        let rows = tree_rows(
            ["src/a.ts", "src/nested/b.ts", "readme.md"],
            &BTreeSet::new(),
        );
        assert_eq!(
            rows.iter()
                .map(|r| (r.directory, r.label.as_str(), r.depth))
                .collect::<Vec<_>>(),
            [
                (true, "src", 0),
                (true, "nested", 1),
                (false, "b.ts", 2),
                (false, "a.ts", 1),
                (false, "readme.md", 0)
            ]
        );
        let rows = tree_rows(["a/b/c/d.ts"], &BTreeSet::new());
        assert_eq!(
            rows[0],
            TreeRow {
                directory: true,
                label: "a/b/c".into(),
                path: "a/b/c".into(),
                depth: 0
            }
        );
        assert_eq!(
            tree_rows(["a/b/c/d.ts"], &BTreeSet::from(["a/b/c".into()])).len(),
            1
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Equal,
    Added,
    Removed,
}
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub text: std::sync::Arc<str>,
    /// Byte range inside `text` that changed within a replaced line pair.
    pub emphasis: Option<std::ops::Range<usize>>,
}
#[derive(Default)]
pub struct LineDiff {
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub deletions: usize,
}
/// Linear-space Myers bisection. Call off the render thread, only for sections
/// the user reveals. No line-count truncation and no quadratic trace storage.
pub fn line_diff(before: &str, after: &str) -> LineDiff {
    let a = before.split_inclusive('\n').collect::<Vec<_>>();
    let b = after.split_inclusive('\n').collect::<Vec<_>>();
    let mut kinds = Vec::new();
    fn emit(a: &[&str], b: &[&str], out: &mut Vec<LineKind>) {
        let prefix = a.iter().zip(b).take_while(|(a, b)| a == b).count();
        out.extend(std::iter::repeat_n(LineKind::Equal, prefix));
        let (a, b) = (&a[prefix..], &b[prefix..]);
        let suffix = a
            .iter()
            .rev()
            .zip(b.iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let (a, b) = (&a[..a.len() - suffix], &b[..b.len() - suffix]);
        if a.is_empty() {
            out.extend(std::iter::repeat_n(LineKind::Added, b.len()));
        } else if b.is_empty() {
            out.extend(std::iter::repeat_n(LineKind::Removed, a.len()));
        } else if let Some((x, y)) = bisect(a, b)
            && (x > 0 || y > 0)
            && (x < a.len() || y < b.len())
        {
            emit(&a[..x], &b[..y], out);
            emit(&a[x..], &b[y..], out);
        } else {
            out.extend(std::iter::repeat_n(LineKind::Removed, a.len()));
            out.extend(std::iter::repeat_n(LineKind::Added, b.len()));
        }
        out.extend(std::iter::repeat_n(LineKind::Equal, suffix));
    }
    fn bisect(a: &[&str], b: &[&str]) -> Option<(usize, usize)> {
        let (n, m) = (a.len() as isize, b.len() as isize);
        let max = (n + m + 1) / 2;
        let offset = max + 1;
        let mut front = vec![-1; (2 * max + 3) as usize];
        let mut back = front.clone();
        front[(offset + 1) as usize] = 0;
        back[(offset + 1) as usize] = 0;
        let delta = n - m;
        for d in 0..=max {
            for k in (-d..=d).step_by(2) {
                let at = (offset + k) as usize;
                let mut x = if k == -d || (k != d && front[at - 1] < front[at + 1]) {
                    front[at + 1]
                } else {
                    front[at - 1] + 1
                };
                let mut y = x - k;
                while x >= 0 && y >= 0 && x < n && y < m && a[x as usize] == b[y as usize] {
                    x += 1;
                    y += 1;
                }
                front[at] = x;
                let reverse = offset + delta - k;
                if delta % 2 != 0
                    && reverse >= 0
                    && (reverse as usize) < back.len()
                    && back[reverse as usize] >= 0
                    && x + back[reverse as usize] >= n
                    && x >= 0
                    && x <= n
                    && y >= 0
                    && y <= m
                {
                    return Some((x as usize, y as usize));
                }
            }
            for k in (-d..=d).step_by(2) {
                let at = (offset + k) as usize;
                let mut x = if k == -d || (k != d && back[at - 1] < back[at + 1]) {
                    back[at + 1]
                } else {
                    back[at - 1] + 1
                };
                let mut y = x - k;
                while x >= 0
                    && y >= 0
                    && x < n
                    && y < m
                    && a[(n - x - 1) as usize] == b[(m - y - 1) as usize]
                {
                    x += 1;
                    y += 1;
                }
                back[at] = x;
                let forward = offset + delta - k;
                if delta % 2 == 0
                    && forward >= 0
                    && (forward as usize) < front.len()
                    && front[forward as usize] >= 0
                    && x + front[forward as usize] >= n
                    && x >= 0
                    && x <= n
                    && y >= 0
                    && y <= m
                {
                    return Some(((n - x) as usize, (m - y) as usize));
                }
            }
        }
        None
    }
    emit(&a, &b, &mut kinds);
    let mut result = LineDiff::default();
    let (mut old, mut new) = (0, 0);
    for kind in kinds {
        let (text, old_number, new_number) = match kind {
            LineKind::Equal => {
                let text = a[old];
                old += 1;
                new += 1;
                (text, Some(old), Some(new))
            }
            LineKind::Removed => {
                let text = a[old];
                old += 1;
                result.deletions += 1;
                (text, Some(old), None)
            }
            LineKind::Added => {
                let text = b[new];
                new += 1;
                result.additions += 1;
                (text, None, Some(new))
            }
        };
        result.lines.push(DiffLine {
            kind,
            old: old_number,
            new: new_number,
            text: text.into(),
            emphasis: None,
        });
    }
    mark_intraline(&mut result.lines);
    result
}

/// Mark the changed middle of replaced line pairs so the renderer can
/// emphasise the actual edit instead of painting the whole line. Pairs are
/// positional inside a removed/added run; unmatched lines keep no emphasis.
fn mark_intraline(lines: &mut [DiffLine]) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].kind == LineKind::Equal {
            index += 1;
            continue;
        }
        let kind = lines[index].kind;
        let start = index;
        while index < lines.len() && lines[index].kind == kind {
            index += 1;
        }
        let end = index;
        let opposite = match kind {
            LineKind::Removed => LineKind::Added,
            LineKind::Added => LineKind::Removed,
            LineKind::Equal => continue,
        };
        let pair_start = index;
        while index < lines.len() && lines[index].kind == opposite {
            index += 1;
        }
        let pair_len = (index - pair_start).min(end - start);
        for offset in 0..pair_len {
            let (old_index, new_index) = if kind == LineKind::Removed {
                (start + offset, pair_start + offset)
            } else {
                (pair_start + offset, start + offset)
            };
            let pair = emphasis_ranges(&lines[old_index].text, &lines[new_index].text);
            if let Some((old_range, new_range)) = pair {
                lines[old_index].emphasis = Some(old_range);
                lines[new_index].emphasis = Some(new_range);
            }
        }
    }
}

fn emphasis_ranges(
    old_text: &str,
    new_text: &str,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let old = old_text.strip_suffix('\n').unwrap_or(old_text);
    let new = new_text.strip_suffix('\n').unwrap_or(new_text);
    let old = old.strip_suffix('\r').unwrap_or(old);
    let new = new.strip_suffix('\r').unwrap_or(new);
    let prefix = old
        .char_indices()
        .zip(new.chars())
        .take_while(|((_, left), right)| left == right)
        .map(|((index, left), _)| index + left.len_utf8())
        .last()
        .unwrap_or(0);
    let suffix = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(left, _)| left.len_utf8())
        .sum::<usize>()
        .min(old.len() - prefix)
        .min(new.len() - prefix);
    if prefix < old.len() - suffix || prefix < new.len() - suffix {
        Some((prefix..old.len() - suffix, prefix..new.len() - suffix))
    } else {
        None
    }
}

#[cfg(test)]
mod diff_tests {
    use super::*;
    fn check(a: &str, b: &str) -> LineDiff {
        let diff = line_diff(a, b);
        assert_eq!(
            diff.lines
                .iter()
                .filter(|l| l.kind != LineKind::Added)
                .map(|l| l.text.as_ref())
                .collect::<String>(),
            a
        );
        assert_eq!(
            diff.lines
                .iter()
                .filter(|l| l.kind != LineKind::Removed)
                .map(|l| l.text.as_ref())
                .collect::<String>(),
            b
        );
        diff
    }
    #[test]
    fn intraline_emphasis_marks_only_the_changed_middle() {
        let diff = line_diff("let value = 1;\n", "let value = 2;\n");
        let removed = diff
            .lines
            .iter()
            .find(|line| line.kind == LineKind::Removed)
            .unwrap();
        let added = diff
            .lines
            .iter()
            .find(|line| line.kind == LineKind::Added)
            .unwrap();
        let old = removed.emphasis.clone().unwrap();
        let new = added.emphasis.clone().unwrap();
        assert_eq!(&removed.text[old], "1");
        assert_eq!(&added.text[new], "2");

        let diff = line_diff("name = \"café 🌍\";\n", "name = \"café 🌎\";\n");
        for line in diff.lines.iter().filter(|line| line.kind != LineKind::Equal) {
            let range = line.emphasis.clone().unwrap();
            assert!(line.text.is_char_boundary(range.start));
            assert!(line.text.is_char_boundary(range.end));
            assert!(range.start < range.end);
        }

        let diff = line_diff("same\n", "same\n");
        assert!(diff.lines.iter().all(|line| line.emphasis.is_none()));
    }

    #[test]
    fn intraline_emphasis_handles_multi_line_replacement_runs() {
        let diff = line_diff("a = 1\nb = 22\n", "a = 3\nb = 44\n");
        let changed = diff
            .lines
            .iter()
            .filter(|line| line.kind != LineKind::Equal)
            .collect::<Vec<_>>();
        assert_eq!(changed.len(), 4);
        assert_eq!(
            changed
                .iter()
                .filter_map(|line| line.emphasis.clone())
                .count(),
            4
        );
        assert!(changed.iter().all(|line| {
            let range = line.emphasis.clone().unwrap();
            line.text.is_char_boundary(range.start) && line.text.is_char_boundary(range.end)
        }));
    }

    #[test]
    fn source_line_count_examples_and_exact_newlines() {
        let d = check("a\nb\n", "a\nc\n");
        assert_eq!((d.additions, d.deletions), (1, 1));
        let d = check("a\nb\n", "");
        assert_eq!((d.additions, d.deletions), (0, 2));
        let d = check("a\n", "a");
        assert_eq!((d.additions, d.deletions), (1, 1));
        check("界\r\n🌍\nlast", "界\r\nreplacement\nlast");
        check("", "");
        check("", "new\n");
    }
    #[test]
    fn exhaustive_small_sequences_are_minimal_and_reconstruct_both_sides() {
        let sequences = (0..127usize)
            .map(|code| {
                let len = (usize::BITS - code.leading_zeros()).saturating_sub(1) as usize;
                (0..len)
                    .map(|bit| if code & (1 << bit) == 0 { "a\n" } else { "b\n" })
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        for a in &sequences {
            for b in &sequences {
                let diff = check(a, b);
                let (a, b) = (a.lines().collect::<Vec<_>>(), b.lines().collect::<Vec<_>>());
                let mut lcs = vec![vec![0; b.len() + 1]; a.len() + 1];
                for i in 0..a.len() {
                    for j in 0..b.len() {
                        lcs[i + 1][j + 1] = if a[i] == b[j] {
                            lcs[i][j] + 1
                        } else {
                            lcs[i][j + 1].max(lcs[i + 1][j])
                        };
                    }
                }
                assert_eq!(
                    diff.additions + diff.deletions,
                    a.len() + b.len() - 2 * lcs[a.len()][b.len()]
                );
            }
        }
    }
    #[test]
    fn line_counts_match_actual_reference_diff_library_fixtures() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/changes-diff-source.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let diff = check(
                case["before"].as_str().unwrap(),
                case["after"].as_str().unwrap(),
            );
            assert_eq!(
                diff.additions,
                case["additions"].as_u64().unwrap() as usize,
                "{case}"
            );
            assert_eq!(
                diff.deletions,
                case["deletions"].as_u64().unwrap() as usize,
                "{case}"
            );
        }
    }
    #[test]
    fn large_sparse_change_retains_all_lines_and_numbers() {
        let before = (0..25000)
            .map(|n| format!("line {n}\n"))
            .collect::<String>();
        let after = before.replacen("line 20000\n", "changed 🌍\n", 1);
        let d = check(&before, &after);
        assert_eq!((d.additions, d.deletions), (1, 1));
        assert_eq!(d.lines.last().unwrap().new, Some(25000));
    }
}
