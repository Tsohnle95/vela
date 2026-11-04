//! Slash-command argument completion (workspace file paths).
use crate::theme::{self, Theme};
use gpui::accesskit::Role;
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, MouseButton, SharedString, Window, div, px};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ArgumentTrigger {
    pub command: String,
    /// Byte offset where the partial argument starts.
    pub start: usize,
    pub query: String,
}

/// Detect `/command partial` at the caret. A trailing argument must be
/// non-empty and whitespace-free; the command name must be non-empty and not
/// contain a slash.
pub(super) fn argument_at(text: &str, caret: usize) -> Option<ArgumentTrigger> {
    let caret = caret.min(text.len());
    let before = &text[..caret];
    let rest = before.strip_prefix('/')?;
    let whitespace = rest.find(char::is_whitespace)?;
    let command = &rest[..whitespace];
    if command.is_empty() || command.contains('/') {
        return None;
    }
    let after = &rest[whitespace..];
    let trimmed = after.trim_start();
    let spaces = after.len().saturating_sub(trimmed.len());
    let start = 1 + whitespace + spaces;
    let query = &text[start..caret];
    if query.is_empty() || query.chars().any(char::is_whitespace) {
        return None;
    }
    if command.eq_ignore_ascii_case("compact") || command.eq_ignore_ascii_case("compress") {
        return None;
    }
    Some(ArgumentTrigger {
        command: command.to_owned(),
        start,
        query: query.to_owned(),
    })
}

/// Escape only the characters this composer inserts itself: literal spaces and
/// backslashes. Other backslashes typed by the user are left intact so command
/// arguments such as regexes are not rewritten.
pub(super) fn escape_argument(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | ' ') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

pub(super) fn unescape_arguments(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '\\' && matches!(chars.get(index + 1), Some(' ') | Some('\\')) {
            out.push(chars[index + 1]);
            index += 2;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// Replace the partial argument with an escaped path (plus one trailing space)
/// and return the new composer text and caret byte offset.
pub(super) fn insert_argument(
    text: &str,
    start: usize,
    caret: usize,
    path: &str,
) -> (String, usize) {
    let start = start.min(text.len()).min(caret);
    let caret = caret.min(text.len()).max(start);
    let escaped = escape_argument(path);
    let mut next = String::with_capacity(text.len() + escaped.len() + 1);
    next.push_str(&text[..start]);
    next.push_str(&escaped);
    next.push(' ');
    next.push_str(&text[caret..]);
    let cursor = start + escaped.len() + 1;
    (next, cursor)
}

pub(super) fn filter_paths(paths: &[String], query: &str, limit: usize) -> Vec<String> {
    let query = query.to_lowercase();
    let mut found = paths
        .iter()
        .filter(|path| query.is_empty() || path.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    found.sort_by(|left, right| {
        let left_prefix = left.to_lowercase().starts_with(&query);
        let right_prefix = right.to_lowercase().starts_with(&query);
        right_prefix
            .cmp(&left_prefix)
            .then_with(|| left.len().cmp(&right.len()))
            .then_with(|| left.cmp(right))
    });
    found.truncate(limit);
    found
}

/// Escape hides the derived slash menus until the user edits the composer
/// again. Keeping this state explicit lets the command and argument menus share
/// one dismissal rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct MenuDismissal {
    dismissed: bool,
}

impl MenuDismissal {
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub fn edited(&mut self) {
        self.dismissed = false;
    }

    pub fn visible(&self) -> bool {
        !self.dismissed
    }
}

pub(super) type SelectFn = Rc<dyn Fn(usize, &mut Window, &mut gpui::App)>;

/// Shared rendering for the composer's file-argument menu. The callback runs on
/// pointer selection and is supplied by the app so tests can exercise the same
/// production rows without constructing the full application.
pub(super) fn render_menu(items: &[String], selected: usize, on_select: SelectFn) -> AnyElement {
    let theme = Theme::default();
    let selected = selected.min(items.len().saturating_sub(1));
    let mut menu = div()
        .id("composer-slash-arguments")
        .debug_selector(|| "composer-slash-arguments".into())
        .role(Role::Menu)
        .aria_label("Slash command arguments")
        .w_full()
        .max_h(px(220.))
        .overflow_y_scroll()
        .rounded(px(10.))
        .border_1()
        .border_color(theme.border_strong)
        .bg(theme.panel)
        .p(px(4.))
        .flex()
        .flex_col()
        .gap(px(1.))
        .child(
            div()
                .px(px(8.))
                .py(px(3.))
                .text_size(px(9.5))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.faint)
                .child("FILE ARGUMENTS"),
        );
    for (index, path) in items.iter().enumerate() {
        let id = format!("slash-argument-{index}");
        let selector = id.clone();
        let value = path.clone();
        let callback = on_select.clone();
        menu = menu.child(
            div()
                .id(SharedString::from(id))
                .debug_selector(move || selector.clone())
                .role(Role::MenuItem)
                .aria_label(value.clone())
                .aria_selected(index == selected)
                .h(px(30.))
                .px(px(8.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .bg(if index == selected {
                    theme.selected
                } else {
                    theme::rgba(0, 0, 0, 0.)
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme.inset))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    callback(index, window, cx);
                    cx.stop_propagation();
                })
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(theme.text)
                        .child(SharedString::from(value)),
                ),
        );
    }
    menu.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_trigger_requires_a_command_and_a_partial_token() {
        assert_eq!(
            argument_at("/review src/lib.rs", "/review src/lib.rs".len()),
            Some(ArgumentTrigger {
                command: "review".into(),
                start: 8,
                query: "src/lib.rs".into()
            })
        );
        assert_eq!(
            argument_at("/review  sr", "/review  sr".len())
                .unwrap()
                .query,
            "sr"
        );
        assert_eq!(argument_at("/review ", "/review ".len()), None);
        assert_eq!(
            argument_at("/review src file", "/review src file".len()),
            None
        );
        assert_eq!(argument_at("/", 1), None);
        assert_eq!(argument_at("review src", "review src".len()), None);
        assert_eq!(
            argument_at("/compact anything", "/compact anything".len()),
            None
        );
        assert_eq!(argument_at("/compress x", "/compress x".len()), None);
        assert_eq!(
            argument_at("/review src", 5),
            None,
            "the caret before the argument is still inside the command name"
        );
    }

    #[test]
    fn dismissal_hides_menus_until_the_next_edit() {
        let mut dismissal = MenuDismissal::default();
        assert!(dismissal.visible());
        dismissal.dismiss();
        assert!(!dismissal.visible());
        dismissal.dismiss();
        assert!(!dismissal.visible());
        dismissal.edited();
        assert!(dismissal.visible());
    }

    #[test]
    fn paths_escape_spaces_backslashes_and_round_trip_into_arguments() {
        assert_eq!(escape_argument("src/lib.rs"), "src/lib.rs");
        assert_eq!(escape_argument("src/my file.rs"), "src/my\\ file.rs");
        assert_eq!(escape_argument("a\\b.rs"), "a\\\\b.rs");
        assert_eq!(unescape_arguments("src/my\\ file.rs"), "src/my file.rs");
        assert_eq!(unescape_arguments("a\\\\b.rs"), "a\\b.rs");
        assert_eq!(unescape_arguments(r"\d+\s?x"), r"\d+\s?x");
        let (text, caret) = insert_argument("/review sr", 8, 10, "src/my file.rs");
        assert_eq!(text, "/review src/my\\ file.rs ");
        assert_eq!(caret, text.len());
        assert_eq!(unescape_arguments(text[8..].trim()), "src/my file.rs");
    }

    #[test]
    fn path_filter_prefers_prefix_matches_and_is_bounded() {
        let paths = vec![
            "src/main.rs".to_owned(),
            "tests/main_test.rs".to_owned(),
            "src/lib.rs".to_owned(),
            "docs/main.md".to_owned(),
        ];
        let filtered = filter_paths(&paths, "main", 3);
        assert_eq!(
            filtered,
            vec![
                "src/main.rs".to_owned(),
                "docs/main.md".to_owned(),
                "tests/main_test.rs".to_owned()
            ]
        );
        assert_eq!(filter_paths(&paths, "zzz", 3).len(), 0);
        assert_eq!(filter_paths(&paths, "src", 10).len(), 2);
    }

    struct MenuProbe {
        items: Vec<String>,
        selected: usize,
        chosen: Rc<std::cell::Cell<Option<usize>>>,
    }

    impl gpui::Render for MenuProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let chosen = self.chosen.clone();
            div().size_full().child(render_menu(
                &self.items,
                self.selected,
                Rc::new(move |index, _, _| chosen.set(Some(index))),
            ))
        }
    }

    #[gpui::test]
    fn argument_menu_renders_rows_and_dispatches_selection(cx: &mut gpui::TestAppContext) {
        let chosen = Rc::new(std::cell::Cell::new(None));
        let (probe, cx) = cx.add_window_view(|_, _| MenuProbe {
            items: vec![
                "src/main.rs".into(),
                "src/lib.rs".into(),
                "src/my file.rs".into(),
            ],
            selected: 0,
            chosen: chosen.clone(),
        });
        cx.simulate_resize(gpui::size(gpui::px(320.), gpui::px(240.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                gpui::point(gpui::px(0.), gpui::px(0.)),
                gpui::size(gpui::px(320.), gpui::px(240.)),
                |_, _| probe.clone().into_any_element(),
            );
        };
        draw(cx);
        assert!(cx.debug_bounds("composer-slash-arguments").is_some());
        assert!(cx.debug_bounds("slash-argument-0").is_some());
        assert!(
            cx.debug_bounds("slash-argument-2").is_some(),
            "paths containing spaces remain selectable rows"
        );
        let second = cx.debug_bounds("slash-argument-1").unwrap().center();
        cx.simulate_click(second, gpui::Modifiers::default());
        assert_eq!(chosen.take(), Some(1));
    }
}
