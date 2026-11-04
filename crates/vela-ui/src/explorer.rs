//! Virtualized file rows retain scroll and selection independently per workspace.
use super::*;
use crate::scroll::{AutoScroll, AutoScrollHandle};
use gpui::{AvailableSpace, Pixels, canvas};

/// Ancestor selections subsume their children; a drop never moves an entry
/// into itself or attempts a no-op in its current parent.
pub(super) fn compact_paths(paths: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let unique = paths
        .iter()
        .filter(|path| !path.is_empty() && seen.insert(path.as_str()))
        .collect::<Vec<_>>();
    unique
        .into_iter()
        .filter(|path| {
            !path
                .match_indices('/')
                .any(|(index, _)| seen.contains(&path[..index]))
        })
        .cloned()
        .collect()
}
fn movable_paths(paths: &[String], parent: &str) -> Vec<String> {
    compact_paths(paths)
        .into_iter()
        .filter(|path| {
            parent != path.as_str()
                && !parent
                    .strip_prefix(path.as_str())
                    .is_some_and(|tail| tail.starts_with('/'))
                && path
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .unwrap_or("")
                    != parent
        })
        .collect()
}

#[derive(Clone)]
struct DraggedEntries {
    workspace: String,
    paths: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}
struct DragPreview {
    label: String,
}
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(5.))
            .bg(theme.selected)
            .text_color(theme.text)
            .text_size(px(12.5))
            .child(self.label.clone())
    }
}
fn drop_surface(
    row: gpui::Stateful<gpui::Div>,
    workspace: String,
    parent: String,
    listener: impl Fn(&DraggedEntries, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let accepted_workspace = workspace.clone();
    let accepted_parent = parent.clone();
    let highlight_workspace = workspace.clone();
    let highlight_parent = parent.clone();
    row.can_drop(move |value, _, _| {
        if let Some(paths) = value.downcast_ref::<gpui::ExternalPaths>() {
            return !paths.paths().is_empty();
        }
        value.downcast_ref::<DraggedEntries>().is_some_and(|drag| {
            drag.workspace == accepted_workspace
                && !movable_paths(&drag.paths.borrow(), &accepted_parent).is_empty()
        })
    })
    .drag_over::<DraggedEntries>(move |style, drag, _, _| {
        if drag.workspace == highlight_workspace
            && !movable_paths(&drag.paths.borrow(), &highlight_parent).is_empty()
        {
            style.bg(Theme::default().selected)
        } else {
            style
        }
    })
    .drag_over::<gpui::ExternalPaths>(|style, _, _, _| style.bg(Theme::default().selected))
    .on_drop(listener)
}
fn drop_target(
    row: gpui::Stateful<gpui::Div>,
    workspace: String,
    parent: String,
    weak: gpui::WeakEntity<VelaApp>,
) -> gpui::Stateful<gpui::Div> {
    let external_weak = weak.clone();
    let external_workspace = workspace.clone();
    let external_parent = parent.clone();
    drop_surface(
        row,
        workspace.clone(),
        parent.clone(),
        move |drag, _, cx| {
            let _ = weak.update(cx, |app, cx| {
                if drag.workspace != workspace || app.workspace_directory() != workspace {
                    return;
                }
                let Some(session) = app.active_id.clone() else {
                    return;
                };
                let paths = movable_paths(&drag.paths.borrow(), &parent);
                if paths.is_empty() {
                    return;
                }
                if let Some(tree) = app.file_trees.get_mut(&workspace) {
                    tree.selected.clear();
                    tree.anchor = None;
                }
                for rel in paths {
                    app.file_request += 1;
                    app.rename_waiting.insert(
                        app.file_request,
                        vela_engine::files::RenameEntryRequest {
                            id: app.file_request,
                            session: session.clone(),
                            workspace: workspace.clone(),
                            name: rel.rsplit('/').next().unwrap().to_owned(),
                            rel,
                            target_parent: Some(parent.clone()),
                        },
                    );
                }
                app.pump_entry_renames(cx);
                cx.notify();
            });
        },
    )
    .on_drop(move |paths: &gpui::ExternalPaths, _, cx| {
        cx.stop_propagation();
        let _ = external_weak.update(cx, |app, cx| {
            app.start_explorer_import(
                external_workspace.clone(),
                external_parent.clone(),
                paths.paths().to_vec(),
                cx,
            )
        });
    })
}

pub(super) struct Tree {
    scroll: AutoScrollHandle,
    paths: Vec<String>,
    selected: HashSet<String>,
    anchor: Option<String>,
    action_focus: HashMap<String, gpui::FocusHandle>,
    focus: Option<gpui::FocusHandle>,
    open: bool,
}
impl Default for Tree {
    fn default() -> Self {
        Self {
            scroll: AutoScrollHandle::default(),
            paths: Vec::new(),
            selected: HashSet::new(),
            anchor: None,
            action_focus: HashMap::new(),
            focus: None,
            open: true,
        }
    }
}
impl Tree {
    pub(super) fn clear_selection(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }
    pub(super) fn remove_prefix(&mut self, prefix: &str) {
        let within = |rel: &str| {
            rel == prefix
                || rel
                    .strip_prefix(prefix)
                    .is_some_and(|tail| tail.starts_with('/'))
        };
        self.paths.retain(|path| !within(path));
        self.selected.retain(|path| !within(path));
        self.action_focus.retain(|path, _| !within(path));
        if self.anchor.as_ref().is_some_and(|path| within(path)) {
            self.anchor = None;
        }
    }
    pub(super) fn drag_paths(&self, grabbed: &str) -> Vec<String> {
        if !self.selected.contains(grabbed) {
            return vec![grabbed.to_owned()];
        }
        let mut paths = self
            .paths
            .iter()
            .filter(|path| self.selected.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        // Collapsing a directory must not silently drop its selected children
        // from a subsequent drag of the retained selection.
        let mut hidden = self
            .selected
            .iter()
            .filter(|path| !paths.contains(path))
            .cloned()
            .collect::<Vec<_>>();
        hidden.sort();
        paths.extend(hidden);
        paths
    }
    pub(super) fn show(&mut self) {
        self.open = true;
    }
    pub(super) fn relocate(&mut self, from: &str, to: &str) {
        let remap = |path: &str| {
            if path == from
                || path
                    .strip_prefix(from)
                    .is_some_and(|tail| tail.starts_with('/'))
            {
                format!("{to}{}", &path[from.len()..])
            } else {
                path.to_owned()
            }
        };
        self.paths = self.paths.iter().map(|path| remap(path)).collect();
        self.selected = self.selected.iter().map(|path| remap(path)).collect();
        self.anchor = self.anchor.as_ref().map(|path| remap(path));
        self.action_focus = std::mem::take(&mut self.action_focus)
            .into_iter()
            .map(|(path, focus)| (remap(&path), focus))
            .collect();
    }
    fn reconcile(&mut self, paths: Vec<String>) {
        self.paths = paths;
    }
    fn select(&mut self, path: &str, shift: bool, additive: bool) {
        if shift {
            let start = self
                .anchor
                .as_ref()
                .and_then(|anchor| self.paths.iter().position(|p| p == anchor));
            let end = self.paths.iter().position(|p| p == path);
            if !additive {
                self.selected.clear();
            }
            if let Some((start, end)) = start.zip(end) {
                self.selected
                    .extend(self.paths[start.min(end)..=start.max(end)].iter().cloned());
            } else {
                self.selected.insert(path.to_owned());
            }
        } else {
            self.anchor = Some(path.to_owned());
            if additive {
                if !self.selected.remove(path) {
                    self.selected.insert(path.to_owned());
                }
            } else {
                self.selected = HashSet::from([path.to_owned()]);
            }
        }
    }
}

impl VelaApp {
    fn explorer_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace = self.workspace_directory();
        let Some(tree) = self.file_trees.get_mut(&workspace) else {
            return;
        };
        if !tree
            .focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window))
        {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => tree.clear_selection(),
            "delete" | "backspace" => {
                let rel = tree
                    .paths
                    .iter()
                    .find(|path| tree.selected.contains(*path))
                    .cloned()
                    .or_else(|| tree.selected.iter().min().cloned());
                if let Some(rel) = rel {
                    self.start_remove_entry(workspace, rel, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn render_single_file_body(
        &mut self,
        directory: String,
        rel: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let name = rel.rsplit('/').next().unwrap_or(&rel).to_owned();
        let title = format!("{directory}/{rel}");
        let unsaved = self
            .documents
            .borrow()
            .unsaved_keys()
            .into_iter()
            .any(|key| key.workspace == self.document_workspace() && key.rel == rel);
        let changed = self
            .engine
            .observed_files(&directory)
            .iter()
            .any(|file| file.rel == rel);
        let click_rel = rel.clone();
        div()
            .id("file-explorer")
            .role(Role::Group)
            .aria_label("Workspace files")
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("single-file-title")
                    .h(px(30.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .role(Role::Group)
                    .aria_label(title.clone())
                    .child(
                        div()
                            .size(px(5.))
                            .flex_shrink_0()
                            .rounded(px(999.))
                            .bg(theme.accent),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(crate::typography::TrackedText::new(
                                name.clone(),
                                px(-0.125),
                            )),
                    ),
            )
            .child(
                div()
                    .h(px(26.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.faint)
                    .child("FILE"),
            )
            .child(
                div()
                    .id("single-file-entry")
                    .role(Role::Button)
                    .aria_label(SharedString::from(title))
                    .h(px(28.))
                    .mx(px(4.))
                    .pl(px(8.))
                    .pr(px(8.))
                    .rounded(px(if self.is_codex() { 10. } else { 999. }))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .cursor_pointer()
                    .bg(theme.selected)
                    .hover(|style| style.bg(theme.inset))
                    .on_click(
                        cx.listener(move |app, event: &gpui::ClickEvent, window, cx| {
                            app.open_file_tab(
                                click_rel.clone(),
                                event.click_count() >= 2,
                                window,
                                cx,
                            );
                        }),
                    )
                    .child(crate::tree_icons::glyph(&name))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(crate::typography::TrackedText::new(
                                name.clone(),
                                px(-0.125),
                            )),
                    )
                    .when(unsaved, |row| {
                        row.child(
                            tree_status_dot(
                                "Unsaved changes",
                                theme::rgba(0xf0, 0xa6, 0x4b, 1.),
                                7.,
                            )
                            .mx(px(3.)),
                        )
                    })
                    .when(changed, |row| {
                        row.child(tree_status_dot("Observed change", theme.accent, 6.))
                    }),
            )
            .into_any_element()
    }

    pub(super) fn render_files_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let directory = self.workspace_directory();
        if directory.is_empty() {
            return div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .id("explorer-open-workspace")
                        .debug_selector(|| "explorer-open-workspace".into())
                        .role(Role::Button)
                        .aria_label("Open Workspace")
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(13.))
                        .cursor_pointer()
                        .text_size(px(13.))
                        .text_color(theme.text)
                        .hover(|style| style.bg(theme.selected))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.choose_workspace(cx);
                        }))
                        .child(
                            div()
                                .size(px(18.))
                                .rounded(px(3.))
                                .border_1()
                                .border_color(theme.border_strong)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(14.))
                                .font_family("codicon")
                                .text_color(theme.dim)
                                .child(icon("ea73")),
                        )
                        .child("Open Workspace"),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .line_height(px(16.))
                        .text_color(theme.faint)
                        .child("Choose a folder to start a session."),
                )
                .into_any_element();
        }
        if let Some(rel) = self.single_file() {
            return self.render_single_file_body(directory, rel, cx);
        }
        let root_name = Path::new(&directory)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| directory.clone());
        let rows = self.flattened_files();
        let tree = self.file_trees.entry(directory.clone()).or_default();
        tree.reconcile(rows.iter().map(|(entry, _)| entry.rel.clone()).collect());
        let tree_focus = tree
            .focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let action_focus = std::iter::once(String::new())
            .chain(rows.iter().map(|(entry, _)| entry.rel.clone()))
            .map(|rel| {
                let focus = tree
                    .action_focus
                    .entry(rel.clone())
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                (rel, focus)
            })
            .collect::<HashMap<_, _>>();
        let root_action_focus = action_focus[""].clone();
        let open = tree.open;
        let selected = tree.selected.clone();
        let state = tree.scroll.clone();
        let observed = self
            .engine
            .observed_files(&directory)
            .iter()
            .map(|file| file.rel.clone())
            .collect::<HashSet<_>>();
        let dirty = self
            .documents
            .borrow()
            .unsaved_keys()
            .into_iter()
            .filter(|key| key.workspace == self.document_workspace())
            .map(|key| key.rel)
            .collect::<HashSet<_>>();
        let active = self.active_document_key().map(|key| key.rel);
        let expanded = self.expanded.clone();
        let codex = self.is_codex();
        let weak = cx.weak_entity();
        let row_directory = directory.clone();
        let root_directory = directory.clone();
        let root_weak = cx.weak_entity();
        let empty = rows.is_empty();
        let heights = rows
            .iter()
            .map(|(_, depth)| px(if *depth == 0 { 30. } else { 29. }))
            .collect();
        let list = known_height_rows(state, heights, move |index, _, _| {
            let Some((entry, depth)) = rows.get(index) else {
                return div().into_any_element();
            };
            let selected = selected.contains(&entry.rel);
            let active = active.as_deref() == Some(entry.rel.as_str());
            let is_dir = entry.is_dir;
            let changed = observed.contains(&entry.rel);
            let unsaved = dirty.contains(&entry.rel);
            let open = expanded.contains(&entry.rel);
            let rel = entry.rel.clone();
            let directory = row_directory.clone();
            let weak = weak.clone();
            let create_weak = weak.clone();
            let menu_weak = weak.clone();
            let menu_directory = directory.clone();
            let menu_rel = rel.clone();
            let click_weak = weak.clone();
            let click_workspace = directory.clone();
            let click_rel = rel.clone();
            let drag_weak = weak.clone();
            let drag_rel = rel.clone();
            let drag_workspace = directory.clone();
            let drop_weak = weak.clone();
            let drop_workspace = directory.clone();
            let drop_parent = if is_dir { rel.clone() } else { String::new() };
            let selection = if active { 0.28 } else { 0.22 };
            // Nested block rows collapse adjacent 1px vertical margins in the
            // reference; top-level workspace children are flex items (30px).
            let mut row = div()
                .relative()
                .h(px(if *depth == 0 { 30. } else { 29. }))
                .w_full()
                .flex_shrink_0();
            for ancestor in 0..*depth {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(14. + ancestor as f32 * 15.))
                        .top_0()
                        .bottom_0()
                        .w(px(1.))
                        .bg(theme.border),
                );
            }
            row.child(
                div()
                    .id(SharedString::from(format!("file-{}", entry.rel)))
                    .role(gpui::Role::Button)
                    .aria_label(SharedString::from(entry.rel.clone()))
                    .when(is_dir, |row| row.aria_expanded(open))
                    .on_drag(
                        DraggedEntries {
                            workspace: drag_workspace.clone(),
                            paths: Default::default(),
                        },
                        move |drag, _, _, cx| {
                            let paths = drag_weak
                                .update(cx, |app, _| {
                                    app.file_trees
                                        .get(&drag_workspace)
                                        .map(|tree| tree.drag_paths(&drag_rel))
                                        .unwrap_or_else(|| vec![drag_rel.clone()])
                                })
                                .unwrap_or_else(|_| vec![drag_rel.clone()]);
                            *drag.paths.borrow_mut() = paths;
                            let paths = drag.paths.borrow();
                            let label = if paths.len() == 1 {
                                paths[0].rsplit('/').next().unwrap().to_owned()
                            } else {
                                format!("{} items", paths.len())
                            };
                            cx.new(|_| DragPreview { label })
                        },
                    )
                    .when(is_dir, |row| {
                        drop_target(row, drop_workspace, drop_parent, drop_weak)
                    })
                    .when(!is_dir, |row| {
                        row.can_drop(|_, _, _| false)
                            .on_drop(|_: &DraggedEntries, _, _| {})
                    })
                    .on_click(move |event, window, cx| {
                        if !event.modifiers().shift
                            && !event.modifiers().platform
                            && !event.modifiers().control
                        {
                            let _ = click_weak.update(cx, |app, cx| {
                                app.file_trees
                                    .entry(click_workspace.clone())
                                    .or_default()
                                    .select(&click_rel, false, false);
                                if is_dir {
                                    app.toggle_dir(click_rel.clone(), cx);
                                } else {
                                    app.open_file_tab(
                                        click_rel.clone(),
                                        event.click_count() >= 2,
                                        window,
                                        cx,
                                    );
                                }
                                cx.notify();
                            });
                        }
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        cx.stop_propagation();
                        let _ = menu_weak.update(cx, |app, cx| {
                            app.open_explorer_menu(
                                menu_directory.clone(),
                                menu_rel.clone(),
                                is_dir,
                                event.position,
                                window,
                                cx,
                            )
                        });
                    })
                    .ml(px(*depth as f32 * 15.))
                    .mt(px(1.))
                    .h(px(28.))
                    .pl(px(4.))
                    .pr(px(8.))
                    .rounded(px(if codex { 10. } else { 999. }))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .cursor_pointer()
                    .bg(if selected {
                        mix(theme.accent, theme.panel, selection)
                    } else if active {
                        theme.selected
                    } else {
                        theme.panel
                    })
                    .when(selected, |row| {
                        row.shadow(vec![gpui::BoxShadow {
                            inset: true,
                            offset: point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: px(1.),
                            color: theme.accent.opacity(0.42).into(),
                        }])
                    })
                    .hover(move |style| {
                        style.bg(if selected {
                            mix(theme.accent, theme.panel, 0.28)
                        } else {
                            theme.selected
                        })
                    })
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let _ = weak.update(cx, |app, cx| {
                            let tree = app.file_trees.entry(directory.clone()).or_default();
                            if event.modifiers.shift
                                || event.modifiers.platform
                                || event.modifiers.control
                                || !tree.selected.contains(&rel)
                            {
                                tree.select(
                                    &rel,
                                    event.modifiers.shift,
                                    event.modifiers.platform || event.modifiers.control,
                                );
                            }
                            if let Some(focus) = &tree.focus {
                                window.focus(focus, cx);
                            }
                            cx.notify();
                        });
                    })
                    .child(if is_dir {
                        div()
                            .size(px(16.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .text_color(theme.faint)
                            .child(icon(if open { "eab4" } else { "eab6" }))
                            .into_any_element()
                    } else {
                        crate::tree_icons::glyph(&entry.name).into_any_element()
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(crate::typography::TrackedText::new(
                                entry.name.clone(),
                                px(-0.125),
                            )),
                    )
                    .when(is_dir, |row| {
                        row.child(super::explorer_create::create_action(
                            row_directory.clone(),
                            entry.rel.clone(),
                            false,
                            create_weak.clone(),
                        ))
                        .child(super::explorer_create::create_action(
                            row_directory.clone(),
                            entry.rel.clone(),
                            true,
                            create_weak.clone(),
                        ))
                    })
                    .child(super::explorer_rename::menu_action(
                        row_directory.clone(),
                        entry.rel.clone(),
                        is_dir,
                        action_focus[&entry.rel].clone(),
                        create_weak.clone(),
                    ))
                    .when(unsaved, |row| {
                        row.child(
                            tree_status_dot(
                                "Unsaved changes",
                                theme::rgba(0xf0, 0xa6, 0x4b, 1.),
                                7.,
                            )
                            .mx(px(3.)),
                        )
                    })
                    .when(changed, |row| {
                        row.child(tree_status_dot("Observed change", theme.accent, 6.))
                    }),
            )
            .into_any_element()
        })
        .flex_1()
        .min_h_0()
        .w_full();
        div()
            .id("file-explorer")
            .role(Role::Group)
            .aria_label("Workspace files")
            .track_focus(&tree_focus)
            .on_key_down(cx.listener(Self::explorer_key_down))
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("explorer-root")
                    .map(|row| {
                        drop_target(
                            row,
                            root_directory.clone(),
                            String::new(),
                            root_weak.clone(),
                        )
                    })
                    .role(gpui::Role::Button)
                    .aria_label("Toggle workspace files")
                    .aria_expanded(open)
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|app, event: &gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            app.open_explorer_menu(
                                app.workspace_directory(),
                                String::new(),
                                true,
                                event.position,
                                window,
                                cx,
                            );
                        }),
                    )
                    .h(px(30.))
                    .flex_shrink_0()
                    .pt(px(4.))
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        let tree = app.file_trees.entry(directory.clone()).or_default();
                        tree.open = !tree.open;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(22.))
                            .flex_shrink_0()
                            .text_center()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .text_color(theme.faint)
                            .child(icon(if open { "eab4" } else { "eab6" })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .px(px(4.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(SharedString::from(root_name)),
                    )
                    .child(super::explorer_create::create_action(
                        root_directory.clone(),
                        String::new(),
                        false,
                        root_weak.clone(),
                    ))
                    .child(super::explorer_create::create_action(
                        root_directory.clone(),
                        String::new(),
                        true,
                        root_weak.clone(),
                    ))
                    .child(super::explorer_rename::menu_action(
                        root_directory,
                        String::new(),
                        true,
                        root_action_focus,
                        root_weak,
                    )),
            )
            .when_some(self.render_create_name(cx), |root, input| root.child(input))
            .when_some(self.file_error.clone(), |root, error| {
                root.child(
                    div()
                        .px(px(12.))
                        .text_size(px(12.))
                        .text_color(theme.red)
                        .child(error),
                )
            })
            .when(open, |root| {
                root.child(
                    div()
                        .id("explorer-drop-area")
                        .map(|row| {
                            drop_target(
                                row,
                                self.workspace_directory(),
                                String::new(),
                                cx.weak_entity(),
                            )
                        })
                        .px(px(6.))
                        .pt(px(2.))
                        .pb(px(14.))
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .child(if empty {
                            div()
                                .p(px(12.))
                                .text_size(px(13.))
                                .text_color(theme.faint)
                                .child("No files yet")
                                .into_any_element()
                        } else {
                            list.into_any_element()
                        }),
                )
            })
            .into_any_element()
    }
}

/// File rows have exact heights. Keep their full geometry in the native scroll
/// container while laying out only visible rows. GPUI's variable-height List
/// discards even explicit height hints on width changes, so it cannot provide a
/// reliable scroll range here. No filesystem row needs rendering to measure it.
fn known_height_rows(
    scroll: AutoScrollHandle,
    heights: Vec<Pixels>,
    mut render: impl FnMut(usize, &mut Window, &mut App) -> AnyElement + 'static,
) -> impl IntoElement + Styled {
    let mut offsets = Vec::with_capacity(heights.len() + 1);
    offsets.push(px(0.));
    for height in heights {
        offsets.push(*offsets.last().unwrap() + height);
    }
    // The reference's collapsed blank-drop target still has two transparent
    // 1px borders. Include that space in the scroll extent, after the final row.
    let height = *offsets.last().unwrap() + px(2.);
    let viewport_scroll = scroll.native_handle();
    div()
        .flex()
        .flex_col()
        .child(AutoScroll::new(scroll, move |scroll, gutter| {
            div()
                .id("file-rows-viewport")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .scrollbar_width(gutter)
                .child(
                    canvas(
                        move |bounds, window, cx| {
                            let viewport = window
                                .content_mask()
                                .bounds
                                .intersect(&viewport_scroll.bounds())
                                .intersect(&bounds);
                            let top = (viewport.top() - bounds.top()).max(px(0.));
                            let bottom = (viewport.bottom() - bounds.top()).max(top);
                            let start = offsets
                                .partition_point(|offset| *offset <= top)
                                .saturating_sub(1);
                            let end = offsets
                                .partition_point(|offset| *offset < bottom)
                                .min(offsets.len() - 1);
                            let mut items = Vec::with_capacity(end.saturating_sub(start));
                            for index in start..end {
                                let mut item = render(index, window, cx);
                                item.layout_as_root(
                                    size(
                                        AvailableSpace::Definite(bounds.size.width),
                                        AvailableSpace::Definite(
                                            offsets[index + 1] - offsets[index],
                                        ),
                                    ),
                                    window,
                                    cx,
                                );
                                item.prepaint_at(
                                    bounds.origin + point(px(0.), offsets[index]),
                                    window,
                                    cx,
                                );
                                items.push(item);
                            }
                            items
                        },
                        |_, mut items, window, cx| {
                            for item in &mut items {
                                item.paint(window, cx);
                            }
                        },
                    )
                    .w_full()
                    .h(height)
                    .flex_shrink_0(),
                )
                .into_any_element()
        }))
}

fn mix(ink: gpui::Rgba, paper: gpui::Rgba, amount: f32) -> gpui::Rgba {
    gpui::Rgba {
        r: ink.r * amount + paper.r * (1. - amount),
        g: ink.g * amount + paper.g * (1. - amount),
        b: ink.b * amount + paper.b * (1. - amount),
        a: 1.,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn move_selection_compacts_ancestors_and_skips_self_descendants_and_current_parent() {
        let paths = [
            "folder/child",
            "folder",
            "other",
            "other",
            "target/already",
            "",
        ]
        .map(str::to_owned);
        assert_eq!(movable_paths(&paths, "target"), vec!["folder", "other"]);
        assert_eq!(
            movable_paths(&paths, "folder/sub"),
            vec!["other", "target/already"]
        );
        assert_eq!(movable_paths(&paths, ""), vec!["target/already"]);
        assert_eq!(
            movable_paths(&["folder".into(), "folderish/file".into()], "folderish"),
            vec!["folder"]
        );
    }
    #[test]
    fn drag_retains_selected_hidden_children_and_unselected_grabs_use_only_their_own_path() {
        let mut tree = Tree::default();
        tree.reconcile(vec!["folder/child".into(), "other".into()]);
        tree.select("folder/child", false, false);
        tree.select("other", false, true);
        tree.reconcile(vec!["folder".into(), "other".into()]);
        assert_eq!(tree.drag_paths("other"), vec!["other", "folder/child"]);
        assert_eq!(tree.drag_paths("folder"), vec!["folder"]);
        tree.select("other", false, false);
        assert_eq!(tree.drag_paths("other"), vec!["other"]);
    }
    struct MoveTargets {
        destination: String,
        target_workspace: String,
        received: Rc<RefCell<Vec<String>>>,
    }
    impl Render for MoveTargets {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let received = self.received.clone();
            let root = drop_surface(
                div().id("root-target").size_full().flex().flex_col(),
                "workspace".into(),
                "".into(),
                move |_, _, _| received.borrow_mut().push("root".into()),
            );
            let received = self.received.clone();
            let source = div()
                .id("drag-source")
                .h(px(40.))
                .w_full()
                .flex_shrink_0()
                .on_drag(
                    DraggedEntries {
                        workspace: "workspace".into(),
                        paths: Rc::new(RefCell::new(vec!["folder/sub/file.txt".into()])),
                    },
                    |_, _, _, cx| {
                        cx.new(|_| DragPreview {
                            label: "file.txt".into(),
                        })
                    },
                )
                .can_drop(|_, _, _| false)
                .on_drop(|_: &DraggedEntries, _, _| {});
            let folder = drop_surface(
                div()
                    .id("folder-target")
                    .h(px(40.))
                    .w_full()
                    .flex_shrink_0(),
                self.target_workspace.clone(),
                self.destination.clone(),
                move |_, _, _| received.borrow_mut().push("folder".into()),
            );
            let file = div()
                .id("file-target")
                .h(px(40.))
                .w_full()
                .flex_shrink_0()
                .can_drop(|_, _, _| false)
                .on_drop(|_: &DraggedEntries, _, _| {});
            root.child(source).child(folder).child(file)
        }
    }
    #[gpui::test]
    fn nested_invalid_and_file_drop_targets_never_fall_through_to_workspace_root(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| MoveTargets {
            destination: "folder/sub".into(),
            target_workspace: "workspace".into(),
            received: Rc::default(),
        });
        cx.simulate_resize(size(px(240.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(240.), px(200.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        let drag = |cx: &mut gpui::VisualTestContext, y: f32| {
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(20.), px(20.)),
                ..Default::default()
            });
            cx.simulate_event(gpui::MouseDownEvent {
                position: point(px(20.), px(20.)),
                button: MouseButton::Left,
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(30.), px(25.)),
                pressed_button: Some(MouseButton::Left),
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(20.), px(y)),
                pressed_button: Some(MouseButton::Left),
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseUpEvent {
                position: point(px(20.), px(y)),
                button: MouseButton::Left,
                ..Default::default()
            });
            draw(cx);
        };
        drag(cx, 60.); // current parent: invalid, even though root would accept
        drag(cx, 100.); // a file is never a directory destination
        cx.update(|_, cx| assert!(view.read(cx).received.borrow().is_empty()));
        view.update(cx, |state, cx| {
            state.destination = "destination".into();
            cx.notify();
        });
        drag(cx, 60.);
        cx.update(|_, cx| assert_eq!(*view.read(cx).received.borrow(), vec!["folder"]));
        view.update(cx, |state, cx| {
            state.target_workspace = "other-workspace".into();
            cx.notify();
        });
        drag(cx, 60.);
        cx.update(|_, cx| assert_eq!(*view.read(cx).received.borrow(), vec!["folder"]));
        drag(cx, 160.);
        cx.update(|_, cx| assert_eq!(*view.read(cx).received.borrow(), vec!["folder", "root"]));
    }
    struct ExternalTargets {
        received: Rc<RefCell<Vec<String>>>,
    }
    impl Render for ExternalTargets {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let root_received = self.received.clone();
            let folder_received = self.received.clone();
            let root = drop_surface(
                div().id("external-root").size_full().flex().flex_col(),
                "workspace".into(),
                "".into(),
                |_, _, _| {},
            )
            .on_drop(move |paths: &gpui::ExternalPaths, _, _| {
                root_received
                    .borrow_mut()
                    .push(format!("root:{}", paths.paths()[0].display()))
            });
            let source = div()
                .id("external-source")
                .h(px(40.))
                .w_full()
                .flex_shrink_0()
                .on_drag(
                    gpui::ExternalPaths(vec![std::path::PathBuf::from("/external/a")].into()),
                    |_, _, _, cx| {
                        cx.new(|_| DragPreview {
                            label: "external".into(),
                        })
                    },
                );
            let folder = drop_surface(
                div()
                    .id("external-folder")
                    .h(px(40.))
                    .w_full()
                    .flex_shrink_0(),
                "workspace".into(),
                "folder".into(),
                |_, _, _| {},
            )
            .on_drop(move |paths: &gpui::ExternalPaths, _, _| {
                folder_received
                    .borrow_mut()
                    .push(format!("folder:{}", paths.paths()[0].display()))
            });
            root.child(source).child(folder)
        }
    }
    #[gpui::test]
    fn external_paths_drop_once_at_the_folder_or_blank_root_surface(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| ExternalTargets {
            received: Rc::default(),
        });
        cx.simulate_resize(size(px(240.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(240.), px(200.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        for y in [60., 160.] {
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(20.), px(20.)),
                ..Default::default()
            });
            cx.simulate_event(gpui::MouseDownEvent {
                position: point(px(20.), px(20.)),
                button: MouseButton::Left,
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(30.), px(25.)),
                pressed_button: Some(MouseButton::Left),
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseMoveEvent {
                position: point(px(20.), px(y)),
                pressed_button: Some(MouseButton::Left),
                ..Default::default()
            });
            draw(cx);
            cx.simulate_event(gpui::MouseUpEvent {
                position: point(px(20.), px(y)),
                button: MouseButton::Left,
                ..Default::default()
            });
            draw(cx);
        }
        cx.update(|_, cx| {
            assert_eq!(
                *view.read(cx).received.borrow(),
                ["folder:/external/a", "root:/external/a"]
            )
        });
    }
    struct LargeTree {
        tree: Tree,
        painted: Rc<RefCell<Vec<usize>>>,
        row_width: Rc<std::cell::Cell<Pixels>>,
    }
    impl Render for LargeTree {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.painted.borrow_mut().clear();
            let painted = self.painted.clone();
            let row_width = self.row_width.clone();
            let paths = self.tree.paths.clone();
            known_height_rows(
                self.tree.scroll.clone(),
                (0..paths.len())
                    .map(|index| px(if index % 2 == 0 { 30. } else { 29. }))
                    .collect(),
                move |index, _, _| {
                    painted.borrow_mut().push(index);
                    let row_width = row_width.clone();
                    div()
                        .relative()
                        .h(px(if index % 2 == 0 { 30. } else { 29. }))
                        .w_full()
                        .child(
                            canvas(
                                move |bounds, _, _| row_width.set(bounds.size.width),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        )
                        .child(SharedString::from(paths[index].clone()))
                        .into_any_element()
                },
            )
            .size_full()
        }
    }

    #[gpui::test]
    fn large_tree_scroll_reaches_last_file_and_new_batches_without_painting_every_row(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| {
            let mut tree = Tree::default();
            tree.reconcile(
                (0..1205)
                    .map(|index| format!("file-{index:04}.txt"))
                    .collect(),
            );
            LargeTree {
                tree,
                painted: Rc::default(),
                row_width: Rc::default(),
            }
        });
        cx.simulate_resize(size(px(229.), px(640.)));
        let draw = |cx: &mut gpui::VisualTestContext, width: f32, height: f32| {
            cx.draw(
                point(px(0.), px(0.)),
                size(px(width), px(height)),
                |_, _| view.clone().into_any_element(),
            );
        };
        let scroll = |cx: &mut gpui::VisualTestContext| {
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: point(px(100.), px(200.)),
                delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-100_000.))),
                ..Default::default()
            });
        };
        draw(cx, 229., 640.);
        scroll(cx);
        draw(cx, 229., 640.);
        cx.update(|_, cx| {
            let state = view.read(cx);
            assert!(
                state.painted.borrow().contains(&1204),
                "last file unreachable: scroll={:?}, painted={:?}",
                state.tree.scroll.native_handle().offset(),
                state.painted.borrow()
            );
            assert!(
                state.painted.borrow().len() < 100,
                "offscreen files must stay virtualized"
            );
            assert_eq!(
                state.row_width.get(),
                px(221.),
                "overflow reserves the source eight-point gutter"
            );
        });
        view.update(cx, |state, cx| {
            state.tree.reconcile(
                (0..1405)
                    .map(|index| format!("file-{index:04}.txt"))
                    .collect(),
            );
            cx.notify();
        });
        draw(cx, 229., 640.);
        scroll(cx);
        draw(cx, 229., 640.);
        cx.update(|_, cx| {
            let state = view.read(cx);
            assert!(
                state.painted.borrow().contains(&1404),
                "newly inserted files need scroll-height estimates too"
            );
            assert!(state.painted.borrow().len() < 100);
        });
        // Width changes used to erase all height hints and collapse the range.
        cx.simulate_resize(size(px(155.), px(340.)));
        draw(cx, 155., 340.);
        scroll(cx);
        draw(cx, 155., 340.);
        cx.update(|_, cx| {
            let state = view.read(cx);
            assert!(state.painted.borrow().contains(&1404));
            assert!(state.painted.borrow().len() < 20);
            assert_eq!(state.row_width.get(), px(147.));
            assert_eq!(
                state.tree.scroll.native_handle().max_offset().y,
                px(41_450. - 340.)
            );
        });
        // Removing files while at the bottom must clamp to real remaining rows.
        view.update(cx, |state, cx| {
            state
                .tree
                .reconcile((0..5).map(|index| format!("file-{index:04}.txt")).collect());
            cx.notify();
        });
        draw(cx, 155., 340.);
        draw(cx, 155., 340.);
        cx.update(|_, cx| {
            let state = view.read(cx);
            assert_eq!(*state.painted.borrow(), vec![0, 1, 2, 3, 4]);
            assert_eq!(state.tree.scroll.native_handle().offset().y, px(0.));
            assert_eq!(
                state.row_width.get(),
                px(155.),
                "short trees recover the full width"
            );
        });
    }
}
