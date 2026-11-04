use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::accesskit::Role;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, CursorStyle, Focusable, Font, FontStyle, FontWeight, Hsla,
    IntoElement, KeyDownEvent, ListAlignment, ListOffset, ListState, MouseButton, MouseDownEvent,
    MouseMoveEvent, PathPromptOptions, Pixels, Render, SharedString, StyledText, TextRun, Window,
    WindowBounds, WindowControlArea, WindowOptions, div, list, point, px, relative, size,
};

use vela_browser::{BrowserEvent, BrowserHost, BrowserProfile};
use vela_core::document::{DocumentKey, Documents, ExternalChange, FileRequest};
use vela_core::event::{PermissionRequest, TodoItem};
use vela_core::session::{SessionPhase, SessionSummary, SessionUsage};
use vela_core::transcript::{Block, BlockKind, ToolFile, ToolStatus, ToolSubCall};
use vela_engine::runtime::RuntimeCapabilities;
use vela_engine::{ConnectionState, Engine, EngineUpdate, FileEntry, GitEntry, Settings};
use vela_opencode::client::PermissionReply;
use vela_opencode::{AgentInfo, CommandOption, InboxItem, ModelInfo};
#[cfg(test)]
use vela_terminal::emulator::Emulator;
use vela_terminal::emulator::{Cell, CellColor, UnderlineKind};

use crate::editor::{CodeEditor, Edited, OpenSymbol, SaveRequested};
use crate::icons::{IconAssets, glyph, vela_mark};
use crate::input::{Changed, TextInput};
use crate::providers::{
    ProviderCatalog, ProviderForm, ProviderFormEvent, ProviderFormHandle, ProviderPending,
    ProviderScope,
};
use crate::settings::{setting_row, settings_group_title, settings_list, tracked_label};
use crate::surface::{RoundedSurface, codex_card};
use crate::theme::{self, Theme};

use crate::theme::{AGENT_WIDTH, PANEL_RADIUS, RAIL_WIDTH, SIDEBAR_WIDTH, TITLEBAR_HEIGHT};

const CODICON_FONT: &[u8] = include_bytes!("../assets/codicon.ttf");

/// Fraction of the viewport reserved for simultaneous agent panels. The
/// single-panel layout keeps its fixed source width; tiled mode needs more room
/// for legible side-by-side transcripts.
const TILED_PANEL_FRACTION: f32 = 0.68;
/// Height reserved for the browser download shelf while it is open.
const BROWSER_DOWNLOADS_HEIGHT: f32 = 108.0;

fn validation_summary_label(errors: usize, warnings: usize) -> String {
    match (errors, warnings) {
        (0, 0) => "No problems".into(),
        (errors, 0) => format!("{errors} error{}", if errors == 1 { "" } else { "s" }),
        (0, warnings) => format!("{warnings} warning{}", if warnings == 1 { "" } else { "s" }),
        (errors, warnings) => format!(
            "{errors} error{}, {warnings} warning{}",
            if errors == 1 { "" } else { "s" },
            if warnings == 1 { "" } else { "s" }
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValidationTarget {
    W3c,
    Scss,
    Unsupported,
}

fn validation_target(path: &str) -> ValidationTarget {
    let lower = path.to_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") || lower.ends_with(".css") {
        ValidationTarget::W3c
    } else if lower.ends_with(".scss") || lower.ends_with(".sass") {
        ValidationTarget::Scss
    } else {
        ValidationTarget::Unsupported
    }
}

fn track_validation_request(
    requests: &mut HashMap<u64, DocumentKey>,
    request_id: u64,
    key: DocumentKey,
) {
    requests.retain(|_, pending| pending != &key);
    requests.insert(request_id, key);
}

fn pending_validation(requests: &HashMap<u64, DocumentKey>, key: &DocumentKey) -> Option<u64> {
    requests
        .iter()
        .filter(|(_, pending)| *pending == key)
        .map(|(request, _)| *request)
        .max()
}

fn validation_action_label(pending: bool, failed: bool, has_summary: bool) -> &'static str {
    if pending {
        "Cancel validation"
    } else if failed {
        "Retry validation"
    } else if has_summary {
        "Validate again"
    } else {
        "Validate"
    }
}

fn validation_diagnostics(
    target: ValidationTarget,
    diagnostics: &[vela_engine::validation::Diagnostic],
) -> Vec<crate::diagnostics::Diagnostic> {
    let source = if target == ValidationTarget::Scss {
        "Sass"
    } else {
        "W3C"
    };
    diagnostics
        .iter()
        .map(|diagnostic| crate::diagnostics::Diagnostic {
            source,
            kind: crate::diagnostics::DiagnosticKind::Validation,
            line: diagnostic.line as usize,
            column: diagnostic.column as usize,
            message: diagnostic.message.clone(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SassMenuMetadata {
    label: &'static str,
    description: String,
    disabled: bool,
}

fn sass_menu_metadata(
    status: vela_engine::SassStatus,
    enabled: bool,
    error: Option<&str>,
    pending: bool,
    directory: &str,
) -> SassMenuMetadata {
    let label = match status {
        vela_engine::SassStatus::Starting => "Starting Sass…",
        vela_engine::SassStatus::Error => "Sass: Error",
        _ if enabled => "Sass: On",
        _ => "Watch Sass",
    };
    // Mirrors the reference title: the error text is read from Sass Output,
    // while the normal hint describes the toggle action and target directory.
    let description = if let Some(error) = error {
        format!(
            "{error}\nSee Sass Output in the terminal panel. Click to {}",
            if enabled {
                "stop watching"
            } else {
                "retry starting"
            }
        )
    } else {
        format!(
            "{} Sass watching in {directory}; saved Sass files compile to CSS on disk",
            if enabled { "Stop" } else { "Start" }
        )
    };
    SassMenuMetadata {
        label,
        description,
        disabled: pending,
    }
}

fn apply_sass_disabled(node: &mut gpui::accesskit::Node, disabled: bool) {
    if disabled {
        node.set_disabled();
    }
}

fn resized_split_fraction(fraction: f32, delta: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return fraction;
    }
    (fraction + delta / width).clamp(0.2, 0.8)
}

fn executable_file(path: &Path) -> bool {
    let Some(metadata) = path.metadata().ok() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Resolve the Agent TUI program before creating a PTY tab. A configured
/// binary wins; a bare name is searched on PATH so a missing CLI fails with a
/// useful toast instead of leaving an empty terminal behind.
fn assign_editor_group_tab(
    groups: &mut [Option<DocumentKey>],
    key: &DocumentKey,
    index: usize,
) -> bool {
    let target = if index == 0 {
        None
    } else {
        let Some(target) = index.checked_sub(1) else {
            return false;
        };
        if target >= groups.len() {
            return false;
        }
        Some(target)
    };
    for group in groups.iter_mut() {
        if group.as_ref() == Some(key) {
            *group = None;
        }
    }
    if let Some(target) = target {
        groups[target] = Some(key.clone());
    }
    true
}

fn tui_exit_notice(code: Option<i32>) -> String {
    match code {
        Some(0) | None => "Agent TUI exited.".into(),
        Some(code) => format!("Agent TUI exited with code {code}."),
    }
}

fn find_tui_program(
    configured: Option<&std::ffi::OsStr>,
    path_var: Option<&std::ffi::OsStr>,
) -> Option<std::path::PathBuf> {
    let configured = configured
        .map(|value| value.to_string_lossy().trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "opencode".into());
    let candidate = PathBuf::from(&configured);
    if configured.contains('/') || configured.contains('\\') {
        return executable_file(&candidate).then_some(candidate);
    }
    for directory in std::env::split_paths(path_var?) {
        let candidate = directory.join(&configured);
        if executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[path = "attachments.rs"]
mod attachments;
#[path = "close.rs"]
mod close;
#[path = "composer.rs"]
mod composer;
#[path = "terminal_composition.rs"]
mod terminal_composition;
#[path = "terminal_selection.rs"]
mod terminal_selection;
#[path = "terminal_state.rs"]
mod terminal_state;
#[path = "terminal_tray.rs"]
mod terminal_tray;
use terminal_state::View as TerminalView;
#[path = "terminal_input.rs"]
mod terminal_input;
use terminal_input::encode as encode_terminal_key;
#[path = "conflict_banner.rs"]
mod conflict_banner;
#[path = "explorer.rs"]
mod explorer;
#[path = "explorer_create.rs"]
mod explorer_create;
#[path = "explorer_detach.rs"]
mod explorer_detach;
#[path = "explorer_import.rs"]
mod explorer_import;
#[path = "explorer_remove.rs"]
mod explorer_remove;
#[path = "explorer_rename.rs"]
mod explorer_rename;
#[path = "forms.rs"]
mod forms;
#[path = "inbox.rs"]
mod inbox;
#[path = "inline_flow.rs"]
mod inline_flow;
#[path = "markdown_images.rs"]
mod markdown_images;
#[path = "markdown_view.rs"]
mod markdown_view;
#[path = "model_picker.rs"]
mod model_picker;
#[path = "model_settings.rs"]
mod model_settings;
#[path = "preview_settings.rs"]
mod preview_settings;
#[path = "sass_output.rs"]
mod sass_output;
#[path = "session_sidebar.rs"]
mod session_sidebar;
#[path = "shimmer.rs"]
mod shimmer;
#[path = "tabs.rs"]
mod tabs;
#[path = "tool_view.rs"]
mod tool_view;
#[path = "transcript_view.rs"]
mod transcript_view;
#[path = "workspace_close.rs"]
mod workspace_close;

/// Source only interprets `/name` as a command when the runtime advertises the
/// command capability; otherwise the same text is an ordinary prompt.
fn prompt_command(text: &str, commands_enabled: bool) -> Option<(String, String)> {
    commands_enabled
        .then(|| parse_slash_command(text))
        .flatten()
}

fn parse_slash_command(text: &str) -> Option<(String, String)> {
    let rest = text.strip_prefix('/')?;
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next()?.trim();
    if name.is_empty() || name.contains('/') {
        return None;
    }
    Some((
        name.to_owned(),
        parts.next().unwrap_or_default().trim().to_owned(),
    ))
}

fn usage_count(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.)
    } else {
        value.to_string()
    }
}

fn usage_cache_summary(usage: &SessionUsage) -> String {
    let cache = usage
        .cache_read_tokens
        .saturating_add(usage.cache_write_tokens);
    let total = usage.total_tokens();
    if cache == 0 || total == 0 {
        usage_count(cache)
    } else {
        format!(
            "{} · {}%",
            usage_count(cache),
            ((cache as f64 / total as f64) * 100.).round() as u64
        )
    }
}

fn usage_cost(value: f64) -> String {
    if value >= 1. {
        format!("${value:.2}")
    } else if value > 0. {
        format!("${value:.4}")
    } else {
        "$0.00".into()
    }
}

fn icon(code: &str) -> SharedString {
    let value = u32::from_str_radix(code, 16).unwrap_or(0xea60);
    SharedString::from(char::from_u32(value).unwrap_or('\u{ea60}').to_string())
}

fn launch_paths_from_urls(urls: Vec<String>) -> Vec<std::path::PathBuf> {
    urls.into_iter()
        .filter_map(|raw| {
            if let Ok(url) = url::Url::parse(&raw) {
                if url.scheme() != "file" {
                    return None;
                }
                return url.to_file_path().ok();
            }
            let path = std::path::PathBuf::from(raw);
            path.is_absolute().then_some(path)
        })
        .collect()
}

/// Source `PendingOpenPaths`: trims and de-duplicates renderer-ready paths and
/// retains only the first ten, preserving launch order across second-instance
/// events that arrive before the UI can consume them.
const MAX_PENDING_PATHS: usize = 10;

#[derive(Default)]
struct PendingOpenPaths {
    paths: Vec<std::path::PathBuf>,
}

fn classify_launch_file(
    path: &Path,
    workspace: &str,
    token: &str,
) -> Option<Result<DocumentKey, String>> {
    if !path.is_file() {
        return None;
    }
    let file = path.canonicalize().ok()?;
    let directory = Path::new(workspace).canonicalize().ok();
    if let Some(directory) = directory
        && let Ok(relative) = file.strip_prefix(&directory)
        && let Some(relative) = relative.to_str()
    {
        return Some(Ok(DocumentKey::in_workspace(
            token.to_owned(),
            workspace.to_owned(),
            relative.replace(std::path::MAIN_SEPARATOR, "/"),
        )));
    }
    Some(Err(file.to_string_lossy().into_owned()))
}

impl PendingOpenPaths {
    fn push(&mut self, items: impl IntoIterator<Item = std::path::PathBuf>) {
        for item in items {
            let item = match item.to_str() {
                Some(value) => {
                    let value = value.trim();
                    if value.is_empty() {
                        continue;
                    }
                    std::path::PathBuf::from(value)
                }
                None => item,
            };
            if self.paths.contains(&item) {
                continue;
            }
            if self.paths.len() >= MAX_PENDING_PATHS {
                return;
            }
            self.paths.push(item);
        }
    }

    fn take(&mut self) -> Vec<std::path::PathBuf> {
        std::mem::take(&mut self.paths)
    }
}

pub fn run(engine: Engine, settings: Settings, workspace_hint: Option<String>) {
    theme::set_appearance(&settings.appearance);
    let explicit_origin = std::env::var("VELA_WINDOW_ORIGIN").ok().and_then(|value| {
        let (x, y) = value.split_once(',')?;
        Some(point(
            px(x.trim().parse::<f32>().ok()?),
            px(y.trim().parse::<f32>().ok()?),
        ))
    });
    let window_size = std::env::var("VELA_WINDOW_SIZE")
        .ok()
        .and_then(|value| {
            let (width, height) = value.split_once('x')?;
            let width = width.parse::<f32>().ok()?;
            let height = height.parse::<f32>().ok()?;
            (width.is_finite() && height.is_finite() && width >= 200. && height >= 200.)
                .then_some(size(px(width), px(height)))
        })
        .unwrap_or_else(|| {
            let saved = engine
                .data_dir()
                .as_deref()
                .and_then(vela_engine::window_state::WindowSize::load)
                .unwrap_or_default();
            size(px(saved.width as f32), px(saved.height as f32))
        });
    let app = gpui_platform::application().with_assets(IconAssets);
    let pending_open_paths: Rc<RefCell<PendingOpenPaths>> =
        Rc::new(RefCell::new(PendingOpenPaths::default()));
    app.on_open_urls({
        let pending = pending_open_paths.clone();
        move |urls| pending.borrow_mut().push(launch_paths_from_urls(urls))
    });
    app.run(move |cx| {
        let quitting_engine = engine.clone();
        cx.on_app_quit(move |_| {
            // GPUI's platform quit path need not return to main(). Cancel and
            // terminate owned children before yielding to its bounded wait.
            quitting_engine.shutdown();
            let engine = quitting_engine.clone();
            async move { engine.shutdown_and_wait().await }
        })
        .detach();
        crate::platform_menu::sync_appearance(Theme::default().dark);
        crate::input::install_menu(cx);
        // Vela quits after its last window closes on every platform. Unsaved
        // documents must approve that close before this notification fires.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let (origin, display_id) = if let Some(origin) = explicit_origin {
            (origin, None)
        } else if std::env::var("VELA_WINDOW_SCREEN").as_deref() != Ok("main") {
            let primary = cx.primary_display().map(|display| display.id());
            let target = cx
                .displays()
                .into_iter()
                .filter(|display| Some(display.id()) != primary)
                .max_by(|left, right| {
                    let left = left.visible_bounds().size;
                    let right = right.visible_bounds().size;
                    (f32::from(left.width) * f32::from(left.height))
                        .total_cmp(&(f32::from(right.width) * f32::from(right.height)))
                });
            if let Some(display) = target {
                let bounds = display.visible_bounds();
                let width = f32::from(window_size.width);
                let height = f32::from(window_size.height);
                let x = f32::from(bounds.origin.x)
                    + ((f32::from(bounds.size.width) - width) / 2.0).max(0.0);
                let y = f32::from(bounds.origin.y)
                    + ((f32::from(bounds.size.height) - height) / 2.0).max(0.0);
                (point(px(x), px(y)), Some(display.id()))
            } else {
                (Bounds::centered(None, window_size, cx).origin, None)
            }
        } else {
            (Bounds::centered(None, window_size, cx).origin, None)
        };
        let _ = cx
            .text_system()
            .add_fonts(vec![std::borrow::Cow::Borrowed(CODICON_FONT)]);
        theme::initialize_source_font(cx);
        let bounds = Bounds {
            origin,
            size: window_size,
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(200.0), px(200.0))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Vela".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.0), px(15.0))),
            }),
            app_owns_titlebar_drag: true,
            display_id,
            app_id: Some("dev.vela.native".into()),
            ..Default::default()
        };
        let engine = engine.clone();
        let settings = settings.clone();
        let hint = workspace_hint.clone();
        let pending_open_paths = pending_open_paths.clone();
        cx.open_window(options, move |window, cx| {
            let settings = settings.clone();
            let hint = hint.clone();
            let engine = engine.clone();
            let pending_open_paths = pending_open_paths.clone();
            let app = cx.new(|cx| VelaApp::new(engine, settings, hint, pending_open_paths, cx));
            let close_owner = app.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                close_owner
                    .update(cx, |app, cx| {
                        app.request_close(close::Intent::Window, window, cx)
                    })
                    .unwrap_or(true)
            });
            let quit_owner = app.downgrade();
            let owner_window = window.window_handle();
            // App menu actions must also work while another app is active.
            cx.on_action(|_: &crate::input::BringWindowsToFront, cx| {
                cx.activate(true);
                cx.defer(|cx| {
                    for handle in cx.windows() {
                        if let Err(error) = handle.update(cx, |_, window, _| {
                            window.activate_window();
                            window.refresh();
                        }) {
                            tracing::error!(%error, "could not bring Vela window to front");
                        }
                    }
                });
            });
            // AppKit sheets have no GPUI window handle. Menu actions must
            // still reach the document owner while a native sheet is key.
            let menu_quit_owner = app.downgrade();
            cx.on_action(move |_: &crate::input::RequestQuit, cx| {
                if menu_quit_owner.upgrade().is_none() {
                    cx.quit();
                    return;
                }
                close::defer_owner(
                    owner_window,
                    menu_quit_owner.clone(),
                    cx,
                    |app, window, cx| app.quit_if_allowed(window, cx),
                );
            });
            let mut quit_cx = cx.to_async();
            crate::platform_quit::install(move || {
                if quit_owner.upgrade().is_none() {
                    return true;
                }
                close::update_owner(
                    owner_window,
                    &quit_owner,
                    &mut quit_cx,
                    |app, window, cx| app.request_close(close::Intent::Quit, window, cx),
                )
                .unwrap_or_else(|error| {
                    tracing::error!(%error, "could not route AppKit Quit to the document owner");
                    false
                })
            });
            app.update(cx, |app, cx| {
                app.last_window_size = crate::window_state::normal_size(window);
                cx.observe_window_bounds(window, |app, window, cx| {
                    let next = crate::window_state::normal_size(window);
                    if next.is_none() {
                        app.window_size_save = None;
                        return;
                    }
                    if next == app.last_window_size {
                        return;
                    }
                    app.last_window_size = next;
                    app.window_size_save = Some(cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(400))
                            .await;
                        let _ = this.update_in(cx, |app, window, _| {
                            if let Some(size) = crate::window_state::normal_size(window) {
                                app.engine.save_window_size(size);
                            }
                        });
                    }));
                })
                .detach();
            });
            window.focus(&app.read(cx).composer_focus.clone(), cx);
            app
        })
        .expect("open Vela window");
        cx.activate(true);
    });
}

/// Per-panel composer state for inactive tiled agents. Source keeps each
/// panel's attachments, file-reference menu and slash menus independent.
#[derive(Default)]
struct TileComposer {
    attachments: Vec<attachments::StagedAttachment>,
    reference_files: Vec<String>,
    /// Outstanding file-reference search and the byte offset of its `@`.
    reference_pending: Option<(u64, usize)>,
    reference_selected: usize,
    slash_selected: usize,
    slash_dismissed: bool,
    slash_arg_files: Vec<String>,
    slash_arg_pending: Option<u64>,
    slash_arg_query: String,
    slash_arg_selected: usize,
}

struct VelaApp {
    closing: Option<close::Request>,
    close_sequence: u64,
    close_approved: Option<close::Intent>,
    close_deadline: Option<gpui::Task<()>>,
    toasts: gpui::Entity<crate::toasts::Toasts>,
    last_window_size: Option<vela_engine::window_state::WindowSize>,
    window_size_save: Option<gpui::Task<()>>,
    engine: Engine,
    settings: Settings,
    sessions: Arc<Vec<SessionSummary>>,
    blocks: Arc<Vec<Block>>,
    permissions: Vec<PermissionRequest>,
    todos: Vec<TodoItem>,
    usage: Option<SessionUsage>,
    active_id: Option<String>,
    #[cfg(test)]
    test_workspace_directory: Option<String>,
    panels: crate::panels::Panels,
    panel_inputs: HashMap<String, gpui::Entity<TextInput>>,
    panel_focuses: HashMap<String, gpui::FocusHandle>,
    panel_composers: HashMap<String, TileComposer>,
    panel_form_docks: HashMap<String, gpui::Entity<forms::FormDock>>,
    /// Active splitter drag: boundary panel index and last pointer x.
    panel_resize: Option<(usize, f32)>,
    attachments: Vec<attachments::StagedAttachment>,
    attachment_drag_over: bool,
    attachment_drag_lifetime: attachments::DragLifetime,
    composer_focus_pending: bool,
    composer: String,
    composer_input: gpui::Entity<TextInput>,
    composer_focus: gpui::FocusHandle,
    transcript_list: ListState,
    transcript_rows: Arc<Vec<transcript_view::Row>>,
    transcript_ui: transcript_view::TranscriptUi,
    session_list: ListState,
    session_filter: String,
    session_projects_open: bool,
    session_projects_scroll: crate::scroll::AutoScrollHandle,
    session_project_focus: RefCell<HashMap<String, gpui::FocusHandle>>,
    expanded_session_projects: HashSet<String>,
    session_recents_open: bool,
    session_search_open: bool,
    workspace_chooser_pending: bool,
    saved_workspace_menu: Option<(String, gpui::Point<gpui::Pixels>)>,
    saved_workspace_menu_focus: gpui::FocusHandle,
    session_context_menu: Option<(String, gpui::Point<gpui::Pixels>)>,
    session_context_focus: [gpui::FocusHandle; 2],
    session_delete_confirmation: Option<u64>,
    session_deletions: HashMap<String, (u64, String)>,
    session_filter_focus: gpui::FocusHandle,
    session_search_input: gpui::Entity<TextInput>,
    filtered_sessions: Arc<Vec<usize>>,
    pinned_session_list: ListState,
    session_pin_focus: Rc<RefCell<HashMap<String, gpui::FocusHandle>>>,
    filtered_pinned_sessions: Arc<Vec<usize>>,
    file_trees: HashMap<String, explorer::Tree>,
    entry_name: gpui::Entity<TextInput>,
    pending_create: Option<explorer_create::CreateForm>,
    create_requests: HashMap<u64, vela_engine::files::CreateEntryRequest>,
    import_requests: HashMap<u64, vela_engine::imports::Request>,
    detach_confirmation: Option<u64>,
    detach_waiting: HashMap<u64, vela_engine::detach::Request>,
    detach_requests: HashMap<u64, vela_engine::detach::Request>,
    workspace_closures: HashMap<String, workspace_close::Request>,
    workspace_closed: bool,
    create_blur: Option<gpui::Subscription>,
    created_files_to_open: Vec<DocumentKey>,
    review_file_to_open: Option<DocumentKey>,
    reveal_requests: HashSet<u64>,
    terminal_directory_requests: HashMap<u64, terminal_state::DirectoryRequest>,
    removal_batches: HashMap<u64, Vec<(vela_engine::files::RemoveEntryRequest, bool, bool)>>,
    remove_waiting: HashMap<u64, vela_engine::files::RemoveEntryRequest>,
    remove_requests: HashMap<u64, vela_engine::files::RemoveEntryRequest>,
    rename_waiting: HashMap<u64, vela_engine::files::RenameEntryRequest>,
    rename_requests: HashMap<u64, vela_engine::files::RenameEntryRequest>,
    relocation_saves: HashMap<DocumentKey, String>,
    explorer_menu: Option<explorer_rename::Menu>,
    explorer_menu_focus: [gpui::FocusHandle; 8],
    tab_strip: tabs::Strip,
    phase: SessionPhase,
    connection: ConnectionState,
    recovering_from_disconnect: bool,
    error: Option<String>,
    stick: bool,
    revision: u64,
    pending_reset: bool,
    bootstrapped: bool,
    workspace_hint: Option<String>,
    pending_launch_paths: VecDeque<std::path::PathBuf>,
    launch_busy: bool,
    launch_directory: Option<String>,
    open_path_request: u64,
    single_files: HashMap<String, String>,
    file_workspace_to_open: Option<(String, String, String)>,
    external_files_to_open: Vec<String>,
    recovery_records: Vec<vela_engine::recovery::RecoveryRecord>,
    recovery_scroll: crate::scroll::AutoScrollHandle,
    revert_requests: HashMap<u64, (String, Option<String>)>,
    staged_reverts: HashSet<String>,
    commit_revert_requests: HashMap<u64, String>,
    clear_revert_requests: HashMap<u64, String>,
    usage_open: bool,
    usage_focus: gpui::FocusHandle,
    usage_anchor: Option<Bounds<gpui::Pixels>>,
    provider_usage: Vec<vela_engine::provider_usage::ProviderUsageResult>,
    provider_usage_loading: bool,
    provider_usage_request: Option<u64>,
    scroll_offsets: HashMap<String, ListOffset>,
    focused_once: bool,
    loading: bool,
    terminals: terminal_state::Tabs,
    tray: vela_core::tray::Tray,
    terminal_focus_pending: bool,
    automation_done: bool,
    sidebar_tab: SidebarTab,
    files: Vec<FileEntry>,
    children: HashMap<String, Vec<FileEntry>>,
    expanded: HashSet<String>,
    file_request: u64,
    directory_requests: HashMap<u64, DocumentKey>,
    documents: Rc<RefCell<Documents>>,
    document_sessions: HashMap<String, String>,
    document_scopes: HashMap<String, String>,
    change_reviews: HashMap<String, gpui::Entity<crate::changes_view::ChangesView>>,
    editor_views: HashMap<DocumentKey, gpui::Entity<CodeEditor>>,
    pending_editor_jump: Option<(DocumentKey, usize)>,
    /// Extra editor groups after the primary pane. `None` renders an empty
    /// group; up to three extras complete the four-group limit.
    /// Extra editor-group assignments keyed by activation-owned document
    /// workspace. Panels opening the same physical root must not share panes.
    editor_groups: HashMap<String, Vec<Option<DocumentKey>>>,
    editor_group_fraction_x: f32,
    editor_group_fraction_y: f32,
    editor_group_bounds: Option<Bounds<Pixels>>,
    editor_group_drag: Option<EditorGroupDrag>,
    close_after_save: HashSet<DocumentKey>,
    editor_focus_pending: bool,
    file_error: Option<String>,
    browser: Option<Rc<BrowserHost>>,
    browser_profile: BrowserProfile,
    browser_input: gpui::Entity<TextInput>,
    browser_editing: bool,
    browser_open: bool,
    browser_url: String,
    browser_address: String,
    browser_tabs: Vec<BrowserTab>,
    browser_active: usize,
    browser_focus: gpui::FocusHandle,
    browser_nav_focus: [gpui::FocusHandle; 3],
    browser_close_focus: gpui::FocusHandle,
    browser_downloads_focus: gpui::FocusHandle,
    browser_error: Option<String>,
    browser_focus_pending: bool,
    browser_downloads: Vec<crate::browser_downloads::BrowserDownload>,
    browser_downloads_open: bool,
    browser_download_actions_focus: [gpui::FocusHandle; 8],
    changes: Vec<GitEntry>,
    diff: Option<DiffView>,
    problems: Vec<vela_engine::validation::Diagnostic>,
    problem_path: Option<String>,
    validation_requests: HashMap<u64, DocumentKey>,
    validation_summary: Option<(usize, usize)>,
    validation_failed: bool,
    validation_key: Option<DocumentKey>,
    git_request: u64,
    status_request: Option<(u64, String)>,
    diff_request: Option<(u64, DocumentKey)>,
    nav_pending: Option<(String, std::time::Instant)>,
    nav_samples: Vec<f64>,
    nav_frame_samples: Vec<f64>,
    models: Vec<ModelInfo>,
    agents: Vec<AgentInfo>,
    selected_model: Option<ModelInfo>,
    settings_model_bounds: Rc<std::cell::Cell<Option<Bounds<gpui::Pixels>>>>,
    settings_model_focus: gpui::FocusHandle,
    runtime_probe: Option<u64>,
    runtime_probe_visible: bool,
    runtime_cli_version: Option<String>,
    runtime_capabilities: RuntimeCapabilities,
    runtime_confirmation: Option<u64>,
    runtime_action: Option<(u64, vela_engine::runtime::RuntimeAction)>,
    runtime_feedback: String,
    selected_agent: Option<String>,
    catalog_session: Option<String>,
    provider_catalog: ProviderCatalog,
    provider_forms: HashMap<String, ProviderFormHandle>,
    provider_oauth_forms: HashMap<String, crate::provider_oauth::OAuthHandle>,
    provider_forms_scope: Option<ProviderScope>,
    provider_pending: HashMap<u64, ProviderPending>,
    provider_removals: HashMap<u64, crate::providers::PendingRemoval>,
    provider_remove_errors: HashMap<String, String>,
    provider_remove_confirmation: Option<u64>,
    settings_scroll: crate::scroll::AutoScrollHandle,
    appearance_grid: crate::settings::ThemeGrid,
    picker: Option<PickerKind>,
    picker_anchors: composer::PickerAnchors,
    composer_bounds: Option<Bounds<gpui::Pixels>>,
    strength_scroll: crate::scroll::AutoScrollHandle,
    agent_picker_scroll: crate::scroll::AutoScrollHandle,
    model_picker_scroll: crate::scroll::AutoScrollHandle,
    model_menu_settings: bool,
    model_collapsed: HashSet<String>,
    model_search: String,
    model_search_input: gpui::Entity<TextInput>,
    settings_open: bool,
    inbox_scroll: crate::scroll::AutoScrollHandle,
    inbox: Vec<InboxItem>,
    inbox_session: Option<String>,
    forms: gpui::Entity<forms::FormDock>,
    forms_session: Option<String>,
    form_requests: HashMap<u64, (String, String)>,
    slash_requests: HashMap<u64, String>,
    command_catalog: Vec<CommandOption>,
    command_session: Option<String>,
    slash_selected: usize,
    slash_menu: crate::slash_args::MenuDismissal,
    slash_arg_files: Vec<String>,
    slash_arg_pending: Option<u64>,
    slash_arg_query: String,
    slash_arg_selected: usize,
    reference_files: Vec<String>,
    reference_pending: Option<(u64, String, usize)>,
    reference_selected: usize,
    agent_panel_open: bool,
    sidebar_open: bool,
    ui_shell: UiShell,
    terminal_panel: TerminalPanel,
    status_menu_open: bool,
    sass_pending: Option<(uuid::Uuid, String)>,
    status_menu_focus: gpui::FocusHandle,
    status_menu_anchor: Option<Bounds<gpui::Pixels>>,
    status_action_focus: [gpui::FocusHandle; 3],
    sass: vela_engine::SassWatchState,
    sass_scroll: crate::scroll::AutoScrollHandle,
    sass_output: gpui::Entity<sass_output::SassOutput>,
    settings_section: SettingsSection,
    changes_open: bool,
    preview_servers: Vec<vela_engine::preview::PreviewServer>,
    integrations: preview_settings::IntegrationInventory,
    integration_request: Option<u64>,
    preview_pending: HashMap<u64, Option<String>>,
    preview_notice: Option<(String, String)>,
    preview_menu: Option<gpui::Point<gpui::Pixels>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSection {
    Appearance,
    Providers,
    Voice,
    Model,
    Servers,
    Mobile,
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorGroupAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
struct EditorGroupDrag {
    axis: EditorGroupAxis,
    last: f32,
}

#[derive(Clone, Default)]
struct BrowserTab {
    host: Option<Rc<BrowserHost>>,
    title: String,
    loading: bool,
    can_go_back: bool,
    can_go_forward: bool,
    url: String,
    address: String,
    error: Option<String>,
    loading_started: Option<std::time::Instant>,
}

impl BrowserTab {
    fn begin_load(&mut self, url: String) {
        self.url = url;
        self.loading = true;
        self.loading_started = Some(std::time::Instant::now());
        self.error = None;
    }

    fn finish_load(&mut self, url: String) {
        self.url = url;
        self.loading = false;
        self.loading_started = None;
        self.error = None;
    }

    fn fail(&mut self, message: String) {
        self.loading = false;
        self.loading_started = None;
        self.error = Some(message);
    }

    fn check_timeout(&mut self, now: std::time::Instant, timeout: std::time::Duration) -> bool {
        if self.loading
            && self
                .loading_started
                .is_some_and(|started| now.duration_since(started) >= timeout)
        {
            self.fail("The page took too long to load.".into());
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalPanel {
    Terminal,
    Problems,
    Sass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiShell {
    Codex,
    Classic,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerKind {
    Model,
    Agent,
    Strength,
}

#[derive(Clone, Copy)]
enum ConflictAction {
    Reload,
    Merge,
    Overwrite,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Sessions,
    Files,
    Changes,
}

struct DiffView {
    rel: String,
    content: String,
    full: Option<String>,
}

type DiffRow = (usize, String, bool, bool);

impl VelaApp {
    fn new(
        engine: Engine,
        settings: Settings,
        workspace_hint: Option<String>,
        pending_open_paths: Rc<RefCell<PendingOpenPaths>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut updates = engine.subscribe();
        cx.spawn(async move |this, cx| {
            loop {
                match updates.recv().await {
                    Ok(update) => {
                        let mut pending = Some(update);
                        if this
                            .update_in(cx, |app, window, cx| {
                                app.on_update(pending.take().expect("pending engine update"), cx);
                                app.advance_close(window, cx);
                            })
                            .is_err()
                        {
                            // Initial events can precede the root's first
                            // window association. Retain their state normally.
                            let Some(update) = pending else {
                                break;
                            };
                            if this
                                .update(cx, |app, cx| app.on_update(update, cx))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let ready = this
                    .update(cx, |app, _| app.connection == ConnectionState::Ready)
                    .unwrap_or(false);
                if !ready {
                    continue;
                }
                let paths = pending_open_paths.borrow_mut().take();
                if paths.is_empty() {
                    continue;
                }
                if this
                    .update(cx, |app, cx| app.open_launch_paths(paths, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        if let Ok(output) = std::env::var("VELA_NAV_OUT") {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(900))
                    .await;
                let mut ready = false;
                for _ in 0..60 {
                    ready = this
                        .update(cx, |app, _| {
                            app.engine.connection() == ConnectionState::Ready
                        })
                        .unwrap_or(false);
                    if ready {
                        break;
                    }
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(200))
                        .await;
                }
                if !ready {
                    let _ =
                        std::fs::write(&output, "{\"error\":\"OpenCode did not become ready\"}");
                    return;
                }
                let requested = std::env::var("VELA_NAV_IDS").ok();
                let ids = match this.update(cx, |app, _| match &requested {
                    Some(list) => list
                        .split(',')
                        .map(str::trim)
                        .filter(|id| !id.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                    None => app
                        .sessions
                        .iter()
                        .take(6)
                        .map(|session| session.id.clone())
                        .collect::<Vec<_>>(),
                }) {
                    Ok(ids) => ids,
                    Err(_) => return,
                };
                if ids.len() < 2 {
                    let _ = std::fs::write(&output, "{\"error\":\"need at least two sessions\"}");
                    return;
                }
                // Preload each session once so the measured phase only exercises
                // warm promotion and retained rendering.
                for id in ids.iter().cycle().take(ids.len() + 1) {
                    let id = id.clone();
                    let _ = this.update(cx, |app, cx| app.open_session(id, cx));
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(900))
                        .await;
                }
                for round in 0..72 {
                    let id = ids[round % ids.len()].clone();
                    let target = id.clone();
                    let _ = this.update(cx, move |app, cx| {
                        app.nav_pending = Some((target, std::time::Instant::now()));
                        app.open_session(id, cx);
                    });
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(120))
                        .await;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(400))
                    .await;
                let _ = this.update(cx, |app, _| app.write_nav_results(&output));
            })
            .detach();
        }

        let sessions = Arc::new(engine.sessions());
        let initial_filtered = sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| !settings.pinned_sessions.contains(&session.id))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let initial_pinned = sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| settings.pinned_sessions.contains(&session.id))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let connection = engine.connection();
        let transcript_list = ListState::new(0, ListAlignment::Top, px(420.0));
        {
            let weak = cx.weak_entity();
            let scroll_list = transcript_list.clone();
            transcript_list.set_scroll_handler(move |event, _window, cx| {
                let weak = weak.clone();
                crate::scroll::observe_list_scroll(
                    &scroll_list,
                    event,
                    cx,
                    move |following, cx| {
                        let _ = weak.update(cx, |app, cx| {
                            app.stick = following;
                            cx.notify();
                        });
                    },
                );
            });
        }
        let ui_shell = if settings.shell == "classic" {
            UiShell::Classic
        } else {
            UiShell::Codex
        };
        let composer_input = cx.new(|cx| {
            let mut input =
                TextInput::new(true, "Ask about this repository or describe a change…", cx);
            input.submit_on_enter = true;
            input
        });
        let composer_focus = composer_input.read(cx).focus_handle(cx);
        cx.subscribe(&composer_input, |this, _, change: &Changed, cx| {
            this.composer = change.0.clone();
            this.slash_selected = 0;
            this.slash_menu.edited();
            this.update_reference_search(cx);
            this.update_slash_arguments(cx);
            if let Some(id) = &this.active_id {
                this.engine.set_draft(id.clone(), this.composer.clone());
            }
            cx.notify();
        })
        .detach();
        let browser_input = cx.new(|cx| TextInput::new(false, "Search or enter a URL", cx));
        let browser_focus = browser_input.read(cx).focus_handle(cx);
        cx.subscribe(&browser_input, |this, _, change: &Changed, cx| {
            this.browser_address = change.0.clone();
            this.browser_editing = true;
            cx.notify();
        })
        .detach();
        let entry_name = cx.new(|cx| TextInput::new(false, "Entry name", cx));
        let session_search_input = cx.new(|cx| TextInput::new(false, "Filter sessions…", cx));
        cx.subscribe(&session_search_input, |app, _, change: &Changed, cx| {
            app.session_filter = change.0.clone();
            app.refresh_session_filter();
            cx.notify();
        })
        .detach();
        let model_search_input = cx.new(|cx| TextInput::new(false, "Search models", cx));
        cx.subscribe(&model_search_input, |app, _, change: &Changed, cx| {
            app.model_search = change.0.clone();
            app.model_picker_scroll
                .native_handle()
                .set_offset(point(px(0.), px(0.)));
            cx.notify();
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |app, cx| app.poll_browser(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let forms = cx.new(forms::FormDock::new);
        let sass_output = cx.new(sass_output::SassOutput::new);
        cx.subscribe(&forms, |app, _, event: &forms::Event, cx| match event {
            forms::Event::Submit {
                form_id,
                session_id: form_session_id,
                answers,
            } => {
                let Some(session) = app.active_id.clone() else {
                    return;
                };
                app.file_request += 1;
                let request = app.file_request;
                app.form_requests
                    .insert(request, (session.clone(), form_id.clone()));
                app.engine.reply_form(
                    request,
                    session,
                    form_session_id.clone(),
                    form_id.clone(),
                    answers.clone(),
                );
                cx.notify();
            }
            forms::Event::Dismiss {
                form_id,
                session_id: form_session_id,
            } => {
                let Some(session) = app.active_id.clone() else {
                    return;
                };
                app.file_request += 1;
                let request = app.file_request;
                app.form_requests
                    .insert(request, (session.clone(), form_id.clone()));
                app.engine
                    .cancel_form(request, session, form_session_id.clone(), form_id.clone());
                cx.notify();
            }
            forms::Event::OpenExternal(url) => cx.open_url(url),
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                if this
                    .update(cx, |app, _| {
                        if let Some(id) = app.active_id.clone() {
                            app.engine.refresh_forms(id);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let preview_servers = engine.preview_servers();
        let browser_downloads = engine
            .data_dir()
            .map(|directory| crate::browser_downloads::load(&directory))
            .unwrap_or_default();
        Self {
            closing: None,
            close_sequence: 0,
            close_approved: None,
            close_deadline: None,
            composer_focus,
            toasts: cx.new(|_| crate::toasts::Toasts::default()),
            last_window_size: None,
            window_size_save: None,
            composer_input,
            engine,
            settings,
            sessions,
            blocks: Arc::new(Vec::new()),
            permissions: Vec::new(),
            todos: Vec::new(),
            usage: None,
            active_id: None,
            #[cfg(test)]
            test_workspace_directory: None,
            panels: Default::default(),
            panel_inputs: HashMap::new(),
            panel_focuses: HashMap::new(),
            panel_composers: HashMap::new(),
            panel_form_docks: HashMap::new(),
            panel_resize: None,
            attachments: Vec::new(),
            attachment_drag_over: false,
            attachment_drag_lifetime: attachments::DragLifetime::default(),
            composer_focus_pending: false,
            composer: String::new(),
            transcript_list,
            transcript_rows: Arc::new(Vec::new()),
            transcript_ui: Default::default(),
            session_list: ListState::new(initial_filtered.len(), ListAlignment::Top, px(200.0)),
            session_filter: String::new(),
            session_projects_open: true,
            session_projects_scroll: Default::default(),
            session_project_focus: Default::default(),
            expanded_session_projects: HashSet::new(),
            session_recents_open: true,
            session_search_open: false,
            workspace_chooser_pending: false,
            saved_workspace_menu: None,
            saved_workspace_menu_focus: cx.focus_handle().tab_stop(true),
            session_context_menu: None,
            session_context_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            session_delete_confirmation: None,
            session_deletions: HashMap::new(),
            session_filter_focus: cx.focus_handle().tab_stop(true),
            session_search_input,
            filtered_sessions: Arc::new(initial_filtered),
            session_pin_focus: Rc::new(RefCell::new(HashMap::new())),
            pinned_session_list: ListState::new(initial_pinned.len(), ListAlignment::Top, px(200.)),
            filtered_pinned_sessions: Arc::new(initial_pinned),
            file_trees: HashMap::new(),
            entry_name,
            pending_create: None,
            create_requests: HashMap::new(),
            import_requests: HashMap::new(),
            detach_confirmation: None,
            detach_waiting: HashMap::new(),
            detach_requests: HashMap::new(),
            workspace_closures: HashMap::new(),
            workspace_closed: false,
            create_blur: None,
            created_files_to_open: Vec::new(),
            review_file_to_open: None,
            reveal_requests: HashSet::new(),
            terminal_directory_requests: HashMap::new(),
            removal_batches: HashMap::new(),
            remove_waiting: HashMap::new(),
            remove_requests: HashMap::new(),
            rename_waiting: HashMap::new(),
            rename_requests: HashMap::new(),
            relocation_saves: HashMap::new(),
            explorer_menu: None,
            explorer_menu_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            tab_strip: tabs::Strip::default(),
            phase: SessionPhase::Idle,
            connection,
            recovering_from_disconnect: false,
            error: None,
            stick: true,
            revision: u64::MAX,
            pending_reset: true,
            bootstrapped: false,
            workspace_hint,
            pending_launch_paths: VecDeque::new(),
            launch_busy: false,
            launch_directory: None,
            open_path_request: 0,
            single_files: HashMap::new(),
            file_workspace_to_open: None,
            external_files_to_open: Vec::new(),
            recovery_records: Vec::new(),
            recovery_scroll: Default::default(),
            revert_requests: HashMap::new(),
            staged_reverts: HashSet::new(),
            commit_revert_requests: HashMap::new(),
            clear_revert_requests: HashMap::new(),
            usage_open: false,
            usage_focus: cx.focus_handle().tab_stop(true),
            usage_anchor: None,
            provider_usage: Vec::new(),
            provider_usage_loading: false,
            provider_usage_request: None,
            scroll_offsets: HashMap::new(),
            focused_once: false,
            loading: false,
            terminals: terminal_state::Tabs::default(),
            tray: vela_core::tray::Tray::default(),
            terminal_focus_pending: false,
            automation_done: false,
            sidebar_tab: SidebarTab::Sessions,
            files: Vec::new(),
            children: HashMap::new(),
            expanded: HashSet::new(),
            file_request: 0,
            directory_requests: HashMap::new(),
            documents: Rc::new(RefCell::new(Documents::default())),
            document_sessions: HashMap::new(),
            document_scopes: HashMap::new(),
            change_reviews: HashMap::new(),
            editor_views: HashMap::new(),
            pending_editor_jump: None,
            editor_groups: HashMap::new(),
            editor_group_fraction_x: 0.5,
            editor_group_fraction_y: 0.5,
            editor_group_bounds: None,
            editor_group_drag: None,
            close_after_save: HashSet::new(),
            editor_focus_pending: false,
            file_error: None,
            browser: None,
            browser_profile: BrowserProfile::default(),
            browser_input,
            browser_editing: false,
            browser_open: false,
            browser_url: String::new(),
            browser_address: String::new(),
            browser_tabs: Vec::new(),
            browser_active: 0,
            browser_focus,
            browser_nav_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            browser_close_focus: cx.focus_handle().tab_stop(true),
            browser_downloads_focus: cx.focus_handle().tab_stop(true),
            browser_error: None,
            browser_focus_pending: false,
            browser_downloads,
            browser_downloads_open: false,
            browser_download_actions_focus: std::array::from_fn(|_| {
                cx.focus_handle().tab_stop(true)
            }),
            changes: Vec::new(),
            diff: None,
            problems: Vec::new(),
            problem_path: None,
            validation_requests: HashMap::new(),
            validation_summary: None,
            validation_failed: false,
            validation_key: None,
            git_request: 0,
            status_request: None,
            diff_request: None,
            nav_pending: None,
            nav_samples: Vec::new(),
            nav_frame_samples: Vec::new(),
            models: Vec::new(),
            agents: Vec::new(),
            selected_model: None,
            settings_model_bounds: Rc::new(std::cell::Cell::new(None)),
            settings_model_focus: cx.focus_handle().tab_stop(false),
            runtime_probe: None,
            runtime_probe_visible: false,
            runtime_cli_version: None,
            runtime_capabilities: RuntimeCapabilities::default(),
            runtime_confirmation: None,
            runtime_action: None,
            runtime_feedback: String::new(),
            selected_agent: None,
            catalog_session: None,
            provider_catalog: ProviderCatalog::default(),
            provider_forms: HashMap::new(),
            provider_oauth_forms: HashMap::new(),
            provider_forms_scope: None,
            provider_pending: HashMap::new(),
            provider_removals: HashMap::new(),
            provider_remove_errors: HashMap::new(),
            provider_remove_confirmation: None,
            settings_scroll: crate::scroll::AutoScrollHandle::default(),
            appearance_grid: crate::settings::ThemeGrid::default(),
            picker: None,
            picker_anchors: composer::PickerAnchors::new(cx),
            composer_bounds: None,
            strength_scroll: crate::scroll::AutoScrollHandle::default(),
            agent_picker_scroll: crate::scroll::AutoScrollHandle::default(),
            model_picker_scroll: crate::scroll::AutoScrollHandle::default(),
            model_menu_settings: false,
            model_collapsed: HashSet::new(),
            model_search: String::new(),
            model_search_input,
            settings_open: false,
            inbox_scroll: crate::scroll::AutoScrollHandle::default(),
            inbox: Vec::new(),
            inbox_session: None,
            forms,
            forms_session: None,
            form_requests: HashMap::new(),
            slash_requests: HashMap::new(),
            command_catalog: Vec::new(),
            command_session: None,
            slash_selected: 0,
            slash_menu: Default::default(),
            slash_arg_files: Vec::new(),
            slash_arg_pending: None,
            slash_arg_query: String::new(),
            slash_arg_selected: 0,
            reference_files: Vec::new(),
            reference_pending: None,
            reference_selected: 0,
            agent_panel_open: true,
            sidebar_open: true,
            ui_shell,
            terminal_panel: TerminalPanel::Terminal,
            status_menu_open: false,
            sass_pending: None,
            status_menu_focus: cx.focus_handle().tab_stop(true),
            status_menu_anchor: None,
            status_action_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            sass: Default::default(),
            sass_scroll: Default::default(),
            sass_output,
            settings_section: SettingsSection::Appearance,
            changes_open: false,
            preview_servers,
            integrations: Default::default(),
            integration_request: None,
            preview_pending: HashMap::new(),
            preview_notice: None,
            preview_menu: None,
        }
    }

    fn on_update(&mut self, update: EngineUpdate, cx: &mut Context<Self>) {
        self.note_close_result(&update);
        self.note_workspace_close_result(&update);
        tracing::debug!(?update, "ui engine update");
        match update {
            EngineUpdate::WorkspaceFiles {
                directory,
                generation,
                changes,
                tree_changed,
                git_changed,
                error,
            } => {
                if !self.engine.observation_is_current(&directory, generation) {
                    return;
                }
                self.ensure_changes_view(cx);
                for review in self.change_reviews.values() {
                    if review.read(cx).directory == directory {
                        review.update(cx, |review, cx| review.receive(&changes, cx));
                    }
                }
                let mut moved_targets = HashSet::new();
                for change in changes.iter().filter(|change| change.moved_from.is_some()) {
                    let old_rel = change.moved_from.as_deref().expect("rename source");
                    let keys = self.documents.borrow().keys_at_path(&directory, old_rel);
                    for old in keys {
                        let new = DocumentKey::in_workspace(
                            old.workspace.clone(),
                            directory.clone(),
                            change.rel.clone(),
                        );
                        let owned_relocation = self.documents.borrow().relocation_contains(&old);
                        if self.documents.borrow_mut().relocate(&old, new.clone()) {
                            moved_targets.insert(new.clone());
                            if self.close_after_save.remove(&old) && owned_relocation {
                                self.close_after_save.insert(new.clone());
                            }
                            if let Some(view) = self.editor_views.remove(&old) {
                                view.update(cx, |editor, cx| {
                                    editor.key = new.clone();
                                    cx.notify();
                                });
                                self.editor_views.insert(new.clone(), view);
                            }
                            for group in self.editor_group_layout_mut(&old.workspace) {
                                if group.as_ref() == Some(&old) {
                                    *group = Some(new.clone());
                                }
                            }
                        }
                    }
                }
                for change in changes.iter() {
                    let keys = self
                        .documents
                        .borrow()
                        .keys_at_path(&directory, &change.rel);
                    for key in keys {
                        let external = ExternalChange {
                            content: change.content.as_ref().map(|content| content.to_string()),
                            deleted: change.deleted,
                            ..Default::default()
                        };
                        let changed = if moved_targets.contains(&key) {
                            self.documents
                                .borrow_mut()
                                .observe_relocation(&key, external, true)
                        } else {
                            self.documents
                                .borrow_mut()
                                .observe(&key, external, change.write_id)
                        };
                        if changed && let Some(view) = self.editor_views.get(&key) {
                            view.update(cx, |_, cx| cx.notify());
                        }
                    }
                }
                if directory == self.workspace_directory() {
                    if let Some(error) = error {
                        self.file_error = Some(error);
                    }
                    if tree_changed && self.sidebar_tab == SidebarTab::Files && !self.settings_open
                    {
                        self.refresh_files(cx);
                    }
                    if (tree_changed || git_changed)
                        && self.sidebar_tab == SidebarTab::Changes
                        && !self.settings_open
                    {
                        self.refresh_changes(cx);
                    }
                    if !self.browser_open
                        && let Some(diff) = &self.diff
                        && (git_changed || changes.iter().any(|change| change.rel == diff.rel))
                    {
                        self.open_diff(diff.rel.clone(), cx);
                    }
                }
                cx.notify();
            }
            EngineUpdate::Previews => {
                self.preview_servers = self.engine.preview_servers();
                cx.notify();
            }
            EngineUpdate::PreviewFinished {
                request_id,
                session_id,
                result,
            } => {
                if let Some(owner) = self.preview_pending.remove(&request_id)
                    && owner == session_id
                {
                    if let Some(owner) = owner {
                        let notice = match result {
                            Ok(Some(server)) => {
                                cx.open_url(&server.url);
                                String::new()
                            }
                            Ok(None) => "Vite server stopped".into(),
                            Err(error) => error,
                        };
                        if self.active_id.as_ref() == Some(&owner) {
                            self.preview_notice = Some((owner, notice));
                        }
                    } else if let Err(error) = result {
                        self.toasts
                            .update(cx, |toasts, cx| toasts.push(error, true, cx));
                    }
                    cx.notify();
                }
            }
            EngineUpdate::RuntimeFinished { request_id, result } => {
                if let Some((pending, action)) = self.runtime_action
                    && pending == request_id
                {
                    self.runtime_action = None;
                    self.runtime_probe = None;
                    self.runtime_feedback = match result {
                        Ok(result) => {
                            self.catalog_session = None;
                            match action {
                                vela_engine::runtime::RuntimeAction::Sync => {
                                    format!("Vela is synced to OpenCode {}.", result.version)
                                }
                                vela_engine::runtime::RuntimeAction::Update
                                    if result.cli_updated =>
                                {
                                    format!(
                                        "Updated OpenCode from {} to {}. Vela is synced.",
                                        result.previous_version, result.version
                                    )
                                }
                                vela_engine::runtime::RuntimeAction::Update => format!(
                                    "OpenCode {} is already current. Vela is synced.",
                                    result.version
                                ),
                            }
                        }
                        Err(error) => error,
                    };
                    self.file_request += 1;
                    self.runtime_probe = Some(self.file_request);
                    self.engine.load_runtime_info(self.file_request);
                    cx.notify();
                }
            }
            EngineUpdate::SettingsApplied { error } => {
                if let Some(error) = error {
                    self.toasts
                        .update(cx, |toasts, cx| toasts.push(error.clone(), true, cx));
                    self.error = Some(error);
                    cx.notify();
                }
            }
            EngineUpdate::RuntimeInfo {
                request_id,
                cli_version,
                capabilities,
            } => {
                if self.runtime_probe == Some(request_id) {
                    self.runtime_cli_version = cli_version;
                    self.runtime_capabilities = capabilities;
                    cx.notify();
                }
            }
            EngineUpdate::Connection => {
                let connection = self.engine.connection();
                let error = self.engine.last_error();
                self.apply_connection_state(connection, error, cx);
            }
            EngineUpdate::Sessions => {
                self.sessions = Arc::new(self.engine.sessions());
                let ids = self
                    .panels
                    .entries()
                    .iter()
                    .map(|panel| panel.session.clone())
                    .collect::<Vec<_>>();
                for id in ids {
                    let title = self.panel_title(&id);
                    self.panels.replace_title(&id, title);
                }
                self.refresh_session_filter();
                if !self.bootstrapped && self.connection == ConnectionState::Ready {
                    self.bootstrap(cx);
                }
            }
            EngineUpdate::WorkspaceOpened { directory, result } => {
                if self.workspace_hint.as_deref() == Some(directory.as_str()) {
                    self.loading = false;
                    let launch = self.launch_busy
                        && self.launch_directory.as_deref() == Some(directory.as_str());
                    match result {
                        Ok(id) => {
                            self.open_session(id, cx);
                            self.refresh_active();
                            self.ensure_changes_view(cx);
                            self.refresh_files(cx);
                        }
                        Err(error) => {
                            self.error = Some(format!("Could not open workspace: {error}"));
                            self.toasts.update(cx, |toasts, cx| {
                                toasts.push(format!("Could not open workspace: {error}"), true, cx);
                            });
                        }
                    }
                    if launch {
                        self.launch_busy = false;
                        self.launch_directory = None;
                        self.pump_launch_paths(cx);
                    }
                    cx.notify();
                }
            }
            EngineUpdate::SessionRemoved(id) => self.retire_session_ui(&id, cx),
            EngineUpdate::SessionDeleteFinished {
                request_id,
                session_id,
                error,
            } => self.finish_session_deletion(request_id, &session_id, error, cx),
            EngineUpdate::Session(id) => {
                if self.active_id.is_none() && !self.workspace_closed {
                    self.active_id = self.engine.active_session();
                }
                if self.active_id.as_deref() == Some(id.as_str()) {
                    self.refresh_active();
                    self.ensure_changes_view(cx);
                    if self.sidebar_tab == SidebarTab::Files && self.files.is_empty() {
                        self.refresh_files(cx);
                    }
                    if self.sidebar_tab == SidebarTab::Changes {
                        self.refresh_changes(cx);
                    }
                    if self.catalog_session.as_deref() != Some(id.as_str()) {
                        self.catalog_session = Some(id.clone());
                        self.engine.load_catalog(id.clone());
                    }
                    if self.inbox_session.as_deref() != Some(id.as_str()) {
                        self.inbox_session = Some(id.clone());
                        self.engine.refresh_inbox(id.clone());
                    }
                    if self.forms_session.as_deref() != Some(id.as_str()) {
                        self.engine.refresh_forms(id.clone());
                    }
                    cx.notify();
                }
            }
            EngineUpdate::Permissions => {
                if let Some(id) = &self.active_id {
                    self.permissions = self.engine.permissions(id);
                    cx.notify();
                }
            }
            EngineUpdate::PermissionAsked(request) => {
                // Match the reference's event-time policy: changing the mode
                // does not retroactively approve already pending requests, and
                // a closed/background warm session is not an open model panel.
                // Every open tile counts, not only the focused one.
                let open = self.panels.contains(&request.session_id)
                    || self.active_id.as_deref() == Some(request.session_id.as_str());
                if self.settings.auto_approve_tools && open {
                    self.engine.reply_permission(
                        request.session_id,
                        request.id,
                        PermissionReply::Allow,
                    );
                }
            }
            EngineUpdate::Todo(_) => {}
            EngineUpdate::TerminalData { id, data } => {
                let Some(terminal) = self.terminals.get_mut(&id) else {
                    return;
                };
                let replies = terminal.emulator.feed(&data);
                if !replies.is_empty() {
                    self.engine.terminal_input(id, replies);
                }
                cx.notify();
            }
            EngineUpdate::TerminalExit { id, code } => {
                self.handle_terminal_exit(&id, code, cx);
            }
            EngineUpdate::Catalog {
                session_id,
                models,
                agents,
                default_model,
                error,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    if let Some(error) = error {
                        self.toasts
                            .update(cx, |toasts, cx| toasts.push(error, true, cx));
                        cx.notify();
                        return;
                    }
                    self.models = models;
                    self.agents = agents;
                    if self.selected_model.is_none() {
                        self.selected_model = default_model;
                    }
                    if self.selected_agent.is_none() {
                        self.selected_agent = self
                            .active_id
                            .as_ref()
                            .and_then(|id| self.engine.view(id))
                            .and_then(|view| view.summary().agent)
                            .or_else(|| self.agents.first().map(|agent| agent.id.clone()));
                    }
                }
                cx.notify();
            }
            EngineUpdate::Integrations {
                request_id,
                session_id,
                mcp_servers,
                plugins,
                skills,
                error,
            } => {
                if self.integration_request == Some(request_id)
                    && self.active_id.as_deref() == Some(session_id.as_str())
                {
                    self.integration_request = None;
                    self.integrations
                        .finish(session_id, mcp_servers, plugins, skills, error);
                    cx.notify();
                }
            }
            EngineUpdate::ProviderUsage {
                request_id,
                results,
            } => {
                if self.provider_usage_request == Some(request_id) {
                    self.provider_usage_request = None;
                    self.provider_usage_loading = false;
                    self.provider_usage = results;
                    cx.notify();
                }
            }
            EngineUpdate::Providers {
                request_id,
                session_id,
                providers,
                error,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    let scope = ProviderScope {
                        session: session_id,
                        directory: self.workspace_directory(),
                    };
                    if self
                        .provider_catalog
                        .finish(request_id, &scope, providers, error)
                    {
                        cx.notify();
                    }
                }
            }
            EngineUpdate::ProviderConnected {
                request_id,
                session_id,
                integration_id,
                error,
            } => {
                if let Some(pending) = self.provider_pending.remove(&request_id)
                    && pending.scope.session == session_id
                    && pending.provider_id == integration_id
                    && self.provider_scope().as_ref() == Some(&pending.scope)
                {
                    if let Some(form) = pending.form.upgrade()
                        && self
                            .provider_forms
                            .get(&integration_id)
                            .is_some_and(|current| current.form.entity_id() == form.entity_id())
                    {
                        if error.is_none() {
                            form.update(cx, |form, cx| form.clear_credentials(cx));
                            self.provider_forms.remove(&integration_id);
                        } else {
                            form.update(cx, |form, cx| form.set_result(error.clone(), cx));
                        }
                    }
                    if error.is_none() {
                        self.provider_catalog.invalidate();
                        self.engine.load_catalog(session_id);
                    }
                    cx.notify();
                }
            }
            EngineUpdate::ProviderOAuth {
                flow_id,
                session_id,
                integration_id,
                state,
            } => {
                let current = self.provider_scope();
                if let Some(handle) = self.provider_oauth_forms.get(&integration_id)
                    && handle.scope.session == session_id
                    && current.as_ref() == Some(&handle.scope)
                    && self.settings_open
                    && self.settings_section == SettingsSection::Providers
                {
                    let finished = handle
                        .form
                        .update(cx, |form, cx| form.apply(flow_id, state, cx));
                    if finished {
                        self.provider_oauth_forms.remove(&integration_id);
                        self.provider_catalog.invalidate();
                        self.engine.load_catalog(session_id);
                    }
                    cx.notify();
                }
            }
            EngineUpdate::ProviderCredentialRemoved {
                request_id,
                session_id,
                integration_id,
                credential_id,
                error,
            } => {
                if let Some(pending) = self.provider_removals.remove(&request_id)
                    && pending.scope.session == session_id
                    && pending.target.provider == integration_id
                    && pending.target.credential == credential_id
                    && self.provider_scope().as_ref() == Some(&pending.scope)
                {
                    if let Some(form) = self.provider_forms.get(&integration_id) {
                        form.form
                            .update(cx, |form, cx| form.set_result(error.clone(), cx));
                    }
                    if let Some(error) = error {
                        self.provider_remove_errors.insert(integration_id, error);
                    } else {
                        self.provider_remove_errors.remove(&integration_id);
                        self.provider_catalog.invalidate();
                        self.engine.load_catalog(session_id);
                    }
                    cx.notify();
                }
            }
            EngineUpdate::Selection {
                session_id,
                model,
                agent,
            } => {
                if self.active_id.as_ref() != Some(&session_id) {
                    return;
                }
                if model.is_some() {
                    self.selected_model = model;
                }
                if agent.is_some() {
                    self.selected_agent = agent;
                }
                cx.notify();
            }
            EngineUpdate::Validation {
                request_id,
                session_id,
                path,
                diagnostics,
                error,
            } => {
                if let Some(key) = self.validation_requests.remove(&request_id)
                    && key.workspace == self.document_workspace()
                    && self.active_id.as_deref() == Some(session_id.as_str())
                {
                    self.problem_path = Some(path);
                    let failed = error.is_some();
                    let errors = diagnostics
                        .iter()
                        .filter(|problem| problem.severity == "error")
                        .count();
                    let warnings = diagnostics
                        .iter()
                        .filter(|problem| problem.severity == "warning")
                        .count();
                    let target = validation_target(&key.rel);
                    let editor_diagnostics = validation_diagnostics(target, &diagnostics);
                    if let Some(editor) = self.editor_views.get(&key) {
                        editor.update(cx, |editor, cx| {
                            editor.set_validation_diagnostics(editor_diagnostics, cx)
                        });
                    }
                    self.problems = diagnostics;
                    self.validation_summary = (!failed).then_some((errors, warnings));
                    self.validation_failed = failed;
                    self.validation_key = Some(key.clone());
                    if let Some(error) = error {
                        self.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("Validation failed: {error}"), true, cx)
                        });
                    } else {
                        self.tray.show();
                        self.terminal_panel = TerminalPanel::Problems;
                    }
                    cx.notify();
                }
            }
            EngineUpdate::GitStatus {
                request_id,
                entries,
                error,
            } => {
                if self.status_request.as_ref() != Some(&(request_id, self.workspace_directory())) {
                    return;
                }
                self.status_request = None;
                self.changes = entries;
                self.file_error = error;
                cx.notify();
            }
            EngineUpdate::GitDiff {
                request_id,
                rel,
                content,
                full,
                error,
            } => {
                let expected = DocumentKey::new(self.workspace_directory(), rel.clone());
                if self.diff_request.as_ref() != Some(&(request_id, expected)) {
                    return;
                }
                self.diff_request = None;
                if let Some(content) = content {
                    self.diff = Some(DiffView { rel, content, full });
                    self.file_error = None;
                } else {
                    self.file_error = error;
                }
                cx.notify();
            }
            EngineUpdate::Directory {
                request_id,
                rel,
                entries,
                error,
            } => {
                let owner = self.directory_requests.remove(&request_id);
                if owner != Some(DocumentKey::new(self.workspace_directory(), rel.clone())) {
                    return;
                }
                if !rel.is_empty() && !self.expanded.contains(&rel) {
                    return;
                }
                self.file_error = error;
                let directories = entries
                    .iter()
                    .filter(|entry| entry.is_dir)
                    .map(|entry| entry.rel.clone())
                    .collect::<HashSet<_>>();
                let removed = self
                    .expanded
                    .iter()
                    .filter(|path| {
                        Path::new(path)
                            .parent()
                            .and_then(Path::to_str)
                            .unwrap_or("")
                            == rel
                            && !directories.contains(*path)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                self.expanded.retain(|path| {
                    !removed
                        .iter()
                        .any(|removed| path == removed || path.starts_with(&format!("{removed}/")))
                });
                self.children.retain(|path, _| {
                    !removed
                        .iter()
                        .any(|removed| path == removed || path.starts_with(&format!("{removed}/")))
                });
                let refresh = directories
                    .into_iter()
                    .filter(|path| self.expanded.contains(path))
                    .collect::<Vec<_>>();
                if rel.is_empty() {
                    self.files = entries;
                } else {
                    self.children.insert(rel, entries);
                }
                for directory in refresh {
                    self.request_directory(directory);
                }
                cx.notify();
            }
            EngineUpdate::EntriesImported {
                request,
                outcomes,
                error,
            } => {
                self.finish_explorer_import(request, outcomes, error, cx);
            }
            EngineUpdate::EntryResolved {
                request_id,
                path,
                error,
            } => {
                if let Some(request) = self.terminal_directory_requests.remove(&request_id) {
                    if request.current(
                        &self.terminals,
                        &self.workspace_directory(),
                        self.active_id.as_deref(),
                    ) {
                        if let Some(error) = error {
                            self.toasts
                                .update(cx, |toasts, cx| toasts.push(error, true, cx));
                        } else if let Some(path) = path {
                            self.tray.show();
                            self.create_terminal(Some(path.to_string_lossy().into_owned()), cx);
                            if let Some(view) = self.terminals.active_mut() {
                                view.name = request.name;
                            }
                        }
                    }
                } else if self.reveal_requests.remove(&request_id) {
                    if let Some(error) = error {
                        self.toasts
                            .update(cx, |toasts, cx| toasts.push(error, true, cx));
                    } else if let Some(path) = path {
                        cx.reveal_path(&path);
                    }
                }
            }
            EngineUpdate::EntryRemoved { request, error } => {
                self.finish_entry_removal(request, error, cx)
            }
            EngineUpdate::EntryDetached {
                request,
                path: _,
                error,
            } => self.finish_entry_detachment(request, error, cx),
            EngineUpdate::EntryRenamed {
                request,
                new_rel,
                snapshots,
                error,
            } => self.finish_entry_rename(request, new_rel, snapshots, error, cx),
            EngineUpdate::EntryCreated {
                request,
                rel,
                error,
            } => self.finish_create_entry(request, rel, error, cx),
            EngineUpdate::FileContent {
                request_id,
                rel,
                content,
                error,
                deleted,
            } => {
                let owner = self.documents.borrow_mut().finish_load(
                    request_id,
                    &rel,
                    if deleted {
                        Some(String::new())
                    } else {
                        content
                    },
                );
                if deleted && let Some(key) = &owner {
                    self.documents.borrow_mut().observe(
                        key,
                        ExternalChange {
                            deleted: true,
                            ..Default::default()
                        },
                        None,
                    );
                }
                if let Some(view) = owner.as_ref().and_then(|key| self.editor_views.get(key)) {
                    view.update(cx, |_, cx| cx.notify());
                }
                if owner
                    .as_ref()
                    .is_some_and(|key| key.workspace == self.document_workspace())
                {
                    self.file_error = error;
                }
                cx.notify();
            }
            EngineUpdate::FileSaved {
                request_id,
                rel,
                error,
                conflict,
            } => {
                let has_conflict = conflict.is_some();
                let failed = error.is_some() || has_conflict;
                let result = self.documents.borrow_mut().complete_save(
                    request_id,
                    &rel,
                    error.is_none(),
                    conflict,
                );
                if let Some((owner, next)) = result {
                    if let Some(error) = error.as_ref() {
                        self.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("Failed to save {}: {error}", owner.rel), true, cx);
                        });
                    }
                    if owner.workspace == self.document_workspace() {
                        self.file_error = if has_conflict { None } else { error };
                    }
                    if let Some(next) = next {
                        self.dispatch_save(next);
                    } else if self.close_after_save.remove(&owner) {
                        let clean = self
                            .documents
                            .borrow()
                            .get(&owner)
                            .is_some_and(|d| !d.is_modified());
                        if clean {
                            self.documents.borrow_mut().close(&owner);
                            self.editor_views.remove(&owner);
                            for group in self.editor_group_layout_mut(&owner.workspace) {
                                if group.as_ref() == Some(&owner) {
                                    *group = None;
                                }
                            }
                            self.editor_focus_pending =
                                owner.workspace == self.document_workspace();
                        }
                    }
                }
                self.release_unused_document_workspaces();
                if failed && let Some(id) = self.active_id.clone() {
                    self.engine.load_recovery(id);
                }
                cx.notify();
            }
            EngineUpdate::Recovery {
                session_id,
                records,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.recovery_records = records;
                    cx.notify();
                }
            }
            EngineUpdate::RecoveryFailed { session_id, error } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.toasts.update(cx, |toasts, cx| {
                        toasts.push(format!("Could not load recovery files: {error}"), true, cx)
                    });
                    cx.notify();
                }
            }
            EngineUpdate::Inbox {
                session_id,
                items,
                error,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.inbox = items;
                    if let Some(error) = error {
                        self.file_error = Some(error);
                    }
                }
                cx.notify();
            }
            EngineUpdate::Forms {
                session_id,
                forms,
                error: _,
            } => {
                self.panels.update(&session_id, |panel| {
                    panel.forms = forms.clone();
                });
                if let Some(dock) = self.panel_form_docks.get(&session_id).cloned() {
                    dock.update(cx, |dock, cx| dock.set_forms(forms.clone(), cx));
                    cx.notify();
                }
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.forms_session = Some(session_id);
                    self.forms.update(cx, |dock, cx| dock.set_forms(forms, cx));
                    cx.notify();
                }
            }
            EngineUpdate::FormFinished {
                request_id,
                session_id,
                form_id,
                error,
            } => {
                if let Some((owner, pending_form)) = self.form_requests.remove(&request_id)
                    && owner == session_id
                    && pending_form == form_id
                    && let Some(error) = error
                {
                    self.forms.update(cx, |dock, cx| dock.failed(&form_id, cx));
                    if let Some(dock) = self.panel_form_docks.get(&session_id).cloned() {
                        dock.update(cx, |dock, cx| dock.failed(&form_id, cx));
                    }
                    self.toasts.update(cx, |toasts, cx| {
                        toasts.push(format!("Could not submit form: {error}"), true, cx)
                    });
                    cx.notify();
                }
            }
            EngineUpdate::LocalQueue(_) => cx.notify(),
            EngineUpdate::SessionSettled {
                session_id,
                stalled,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.refresh_active();
                    self.ensure_changes_view(cx);
                    if stalled {
                        self.toasts.update(cx, |toasts, cx| {
                            toasts.push("Turn stalled — recovered", false, cx)
                        });
                    }
                    cx.notify();
                }
            }
            EngineUpdate::RevertStaged { session_id } => {
                self.staged_reverts.insert(session_id);
                cx.notify();
            }
            EngineUpdate::RevertFinished {
                request_id,
                session_id,
                error,
            } => {
                if let Some(owner) = self.commit_revert_requests.remove(&request_id) {
                    if owner == session_id {
                        if error.is_none() {
                            self.staged_reverts.remove(&session_id);
                        } else if let Some(error) = error {
                            self.toasts.update(cx, |toasts, cx| {
                                toasts.push(
                                    format!("Could not commit staged undo: {error}"),
                                    true,
                                    cx,
                                )
                            });
                        }
                        cx.notify();
                    }
                } else if let Some(owner) = self.clear_revert_requests.remove(&request_id) {
                    if owner == session_id {
                        self.staged_reverts.remove(&session_id);
                        if let Some(error) = error {
                            self.toasts.update(cx, |toasts, cx| {
                                toasts.push(
                                    format!("Could not discard staged undo: {error}"),
                                    true,
                                    cx,
                                )
                            });
                        }
                        cx.notify();
                    }
                } else if let Some((owner, prompt)) = self.revert_requests.remove(&request_id)
                    && owner == session_id
                {
                    if let Some(error) = error {
                        self.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("Could not undo message: {error}"), true, cx)
                        });
                    } else {
                        self.staged_reverts.remove(&session_id);
                        if let Some(prompt) = prompt {
                            self.composer = prompt;
                            let text = self.composer.clone();
                            self.composer_input
                                .update(cx, |input, cx| input.set_text(text, cx));
                            if let Some(id) = self.active_id.clone() {
                                self.engine.set_draft(id, self.composer.clone());
                            }
                            self.composer_focus_pending = true;
                        }
                    }
                    cx.notify();
                }
            }
            EngineUpdate::SlashCommandFinished {
                request_id,
                session_id,
                error,
            } => {
                if self.slash_requests.remove(&request_id).as_deref() == Some(session_id.as_str())
                    && let Some(error) = error
                {
                    self.toasts.update(cx, |toasts, cx| {
                        toasts.push(format!("Command failed: {error}"), true, cx)
                    });
                    cx.notify();
                }
            }
            EngineUpdate::CommandCatalog {
                session_id,
                commands,
                error: _,
            } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.command_session = Some(session_id);
                    self.command_catalog = commands;
                    self.slash_selected = 0;
                    cx.notify();
                }
            }
            EngineUpdate::References {
                request_id,
                session_id,
                files,
                error: _,
            } => {
                if self.slash_arg_pending == Some(request_id)
                    && self.active_id.as_deref() == Some(session_id.as_str())
                {
                    self.slash_arg_pending = None;
                    self.slash_arg_files =
                        crate::slash_args::filter_paths(&files, &self.slash_arg_query, 8);
                    self.slash_arg_selected = 0;
                    cx.notify();
                }
                if self.active_id.as_deref() == Some(session_id.as_str())
                    && self
                        .reference_pending
                        .as_ref()
                        .is_some_and(|(pending, _, _)| *pending == request_id)
                {
                    self.reference_files = files.clone();
                    self.reference_selected = 0;
                    cx.notify();
                }
                for (session, state) in &mut self.panel_composers {
                    if session != &session_id {
                        continue;
                    }
                    if state
                        .reference_pending
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == request_id)
                    {
                        state.reference_files = files.clone();
                        state.reference_selected = 0;
                        cx.notify();
                    }
                    if state.slash_arg_pending == Some(request_id) {
                        state.slash_arg_pending = None;
                        state.slash_arg_files =
                            crate::slash_args::filter_paths(&files, &state.slash_arg_query, 8);
                        state.slash_arg_selected = 0;
                        cx.notify();
                    }
                }
            }
            EngineUpdate::Sass { session_id, state } => {
                if self.active_id.as_deref() == Some(session_id.as_str()) {
                    self.sass = state;
                }
                cx.notify();
            }
            EngineUpdate::Error(message) => {
                self.toasts
                    .update(cx, |toasts, cx| toasts.push(message.clone(), true, cx));
                self.error = Some(message);
                self.connection = self.engine.connection();
                cx.notify();
            }
        }
        if self.panels.len() > 1 {
            self.save_active_panel_state(cx);
            self.refresh_panel_snapshots();
        }
    }

    fn apply_connection_state(
        &mut self,
        connection: ConnectionState,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let was_ready = self.connection == ConnectionState::Ready;
        self.connection = connection;
        self.error = error;
        self.provider_catalog.invalidate();
        if connection == ConnectionState::Ready {
            if self.bootstrapped && self.recovering_from_disconnect {
                self.recovering_from_disconnect = false;
                self.reload_interface(cx);
            } else {
                self.bootstrap(cx);
            }
        } else if self.bootstrapped || was_ready {
            // Match the reference's renderer reload on backend interruption:
            // the next Ready transition rebuilds UI state from the engine.
            self.recovering_from_disconnect = true;
        }
        cx.notify();
    }

    /// Native equivalent of Source's renderer reload after a backend restart.
    /// Engine transcripts and drafts are retained; UI-owned catalogs, panels and
    /// caches are rebuilt from the reconnected engine.
    fn reload_interface(&mut self, cx: &mut Context<Self>) {
        self.sessions = Arc::new(self.engine.sessions());
        self.refresh_session_filter();
        self.catalog_session = None;
        self.models.clear();
        self.agents.clear();
        self.selected_model = None;
        self.selected_agent = None;
        self.provider_catalog.invalidate();
        self.provider_usage.clear();
        self.provider_usage_loading = false;
        self.provider_usage_request = None;
        self.command_catalog.clear();
        self.command_session = None;
        self.slash_requests.clear();
        self.reference_files.clear();
        self.reference_pending = None;
        self.preview_servers = self.engine.preview_servers();
        self.forms
            .update(cx, |dock, cx| dock.set_forms(Vec::new(), cx));
        self.forms_session = None;
        self.form_requests.clear();
        self.inbox.clear();
        self.inbox_session = None;
        self.runtime_cli_version = None;
        if self.connection == ConnectionState::Ready {
            let active = self.active_id.clone().or_else(|| {
                (!self.workspace_closed)
                    .then(|| self.engine.active_session())
                    .flatten()
            });
            if let Some(id) = active {
                self.save_view_state();
                self.open_session(id, cx);
            }
            if !self.workspace_closed && !self.workspace_directory().is_empty() {
                self.refresh_files(cx);
                self.refresh_changes(cx);
            }
        } else {
            self.revision = u64::MAX;
        }
        cx.notify();
    }

    fn render_connection_recovery(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.connection == ConnectionState::Ready {
            return div().into_any_element();
        }
        let reload = div()
            .id("connection-reload")
            .debug_selector(|| "connection-reload".into())
            .role(Role::Button)
            .aria_label("Reload interface")
            .focusable()
            .tab_stop(true)
            .h(px(22.))
            .px(px(9.))
            .rounded(px(5.))
            .border_1()
            .bg(Theme::default().inset)
            .text_size(px(10.))
            .text_color(Theme::default().text)
            .cursor_pointer()
            .hover(|style| style.bg(Theme::default().elevated))
            .on_click(cx.listener(|app, _, _, cx| app.reload_interface(cx)))
            .child("Reload");
        connection_recovery_element(self.connection, self.error.as_deref(), reload.into_any_element())
    }

    fn bootstrap(&mut self, cx: &mut Context<Self>) {
        if self.bootstrapped {
            return;
        }
        self.bootstrapped = true;
        self.connection = self.engine.connection();
        tracing::info!(connected = ?self.connection, sessions = self.sessions.len(), hint = ?self.workspace_hint, "ui bootstrap");
        // Match the reference's fresh-launch behavior: a previous conversation is
        // not reopened automatically. It remains available from Recents, while
        // an explicit launch path is honored immediately.
        let Some(path) = self.workspace_hint.clone() else {
            return;
        };
        if Path::new(&path).is_file() {
            self.start_file_workspace(path, cx);
            return;
        }
        if let Some(existing) = self
            .sessions
            .iter()
            .find(|session| session.directory == path)
        {
            let id = existing.id.clone();
            self.open_session(id, cx);
        } else {
            tracing::info!(%path, "opening workspace from hint");
            self.loading = true;
            self.engine.open_directory(path);
            cx.notify();
        }
    }

    fn start_file_workspace(&mut self, path: String, cx: &mut Context<Self>) {
        self.open_path_request += 1;
        let request = self.open_path_request;
        self.loading = true;
        self.error = None;
        let engine = self.engine.clone();
        cx.spawn(async move |this, cx| {
            let result = engine.open_file_workspace(path.clone()).await;
            let _ = this.update(cx, |app, cx| {
                if app.open_path_request != request {
                    if app.launch_busy && app.launch_directory.is_none() {
                        app.launch_busy = false;
                        app.pump_launch_paths(cx);
                    }
                    return;
                }
                app.loading = false;
                match result {
                    Ok(file) => {
                        app.workspace_hint = None;
                        app.single_files
                            .insert(file.session_id.clone(), file.path.clone());
                        app.file_workspace_to_open = Some((
                            file.session_id.clone(),
                            file.directory.clone(),
                            file.path.clone(),
                        ));
                        app.open_session(file.session_id, cx);
                        app.sidebar_tab = SidebarTab::Files;
                        app.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("Opened {}", file.path), false, cx)
                        });
                    }
                    Err(error) => {
                        app.error = Some(format!("Could not open file: {error}"));
                        app.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("Could not open file: {error}"), true, cx)
                        });
                    }
                }
                if app.launch_busy && app.launch_directory.is_none() {
                    app.launch_busy = false;
                    app.pump_launch_paths(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn open_launch_paths(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        let activate = !paths.is_empty();
        self.pending_launch_paths.extend(paths);
        self.pump_launch_paths(cx);
        if activate {
            // Finder/second-instance delivery returns to the running app rather
            // than leaving the new request behind an inactive window.
            cx.activate(true);
            for handle in cx.windows() {
                let _ = handle.update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                });
            }
        }
    }

    /// Source awaits each launch path before starting the next one. Directory
    /// activation is asynchronous here, so serialize the batch: files after a
    /// directory must attach to the context that directory just opened instead
    /// of racing it as standalone workspaces.
    fn pump_launch_paths(&mut self, cx: &mut Context<Self>) {
        if self.launch_busy {
            return;
        }
        while let Some(path) = self.pending_launch_paths.pop_front() {
            let Some(path) = path.to_str().map(str::to_owned) else {
                continue;
            };
            if Path::new(&path).is_dir() {
                self.launch_busy = true;
                self.launch_directory = Some(path.clone());
                self.workspace_hint = Some(path.clone());
                self.loading = true;
                self.engine.open_directory(path);
                return;
            }
            if Path::new(&path).is_file() {
                if self.active_id.is_none() {
                    self.launch_busy = true;
                    self.launch_directory = None;
                    self.start_file_workspace(path, cx);
                    return;
                }
                let workspace = self.workspace_directory();
                match classify_launch_file(
                    Path::new(&path),
                    &workspace,
                    &self.document_workspace(),
                ) {
                    Some(Ok(key)) => {
                        self.sidebar_tab = SidebarTab::Files;
                        self.created_files_to_open.push(key);
                        cx.notify();
                    }
                    Some(Err(file)) => {
                        self.sidebar_tab = SidebarTab::Files;
                        self.external_files_to_open.push(file);
                        cx.notify();
                    }
                    None => {
                        self.toasts.update(cx, |toasts, cx| {
                            toasts.push(format!("{path} is not available"), true, cx)
                        });
                    }
                }
                continue;
            }
            self.toasts.update(cx, |toasts, cx| {
                toasts.push(format!("{path} is not available"), true, cx)
            });
        }
    }

    fn single_file(&self) -> Option<String> {
        self.active_id
            .as_ref()
            .and_then(|id| self.single_files.get(id))
            .cloned()
    }

    fn panel_title(&self, id: &str) -> String {
        self.sessions
            .iter()
            .find(|session| session.id == id)
            .map(|session| {
                if session.title.is_empty() {
                    session.id.clone()
                } else {
                    session.title.clone()
                }
            })
            .unwrap_or_else(|| id.to_owned())
    }

    /// Parent id/title when the reference renders its agent-header back button.
    /// A parent missing from the cached inventory still gets a usable label;
    /// navigation is by id and must not depend on the summary list.
    fn session_parent(&self, id: &str) -> Option<(String, String)> {
        let parent = self
            .sessions
            .iter()
            .find(|session| session.id == id)?
            .parent_id
            .clone()?;
        let title = self
            .sessions
            .iter()
            .find(|session| session.id == parent)
            .map(|session| {
                if session.title.is_empty() {
                    session.id.clone()
                } else {
                    session.title.clone()
                }
            })
            .unwrap_or_else(|| "parent session".into());
        Some((parent, title))
    }

    fn capture_panel_state(&self) -> Option<crate::panels::PanelState> {
        let session = self.active_id.clone()?;
        let mut state = self
            .panels
            .active()
            .filter(|state| state.session == session)
            .cloned()
            .unwrap_or_else(|| {
                crate::panels::PanelState::new(session.clone(), self.panel_title(&session))
            });
        state.title = self.panel_title(&session);
        state.composer = if self.panels.len() > 1 {
            self.panels
                .active()
                .map(|panel| panel.composer.clone())
                .unwrap_or_else(|| self.composer.clone())
        } else {
            self.composer.clone()
        };
        state.blocks = self.blocks.clone();
        state.phase = self.phase;
        state.permissions = self.permissions.clone();
        state.todos = self.todos.clone();
        state.usage = self.usage;
        state.selected_model = self.selected_model.clone();
        state.selected_agent = self.selected_agent.clone();
        state.scroll = self.transcript_list.logical_scroll_top();
        state.stick = self.stick;
        Some(state)
    }

    fn ensure_panel_input(&mut self, session: &str, cx: &mut Context<Self>) {
        if self.panel_inputs.contains_key(session) {
            return;
        }
        let Some(panel) = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .cloned()
        else {
            return;
        };
        let placeholder = format!("Message {}", panel.title);
        let input = cx.new(|cx| {
            let mut input = TextInput::new(false, &placeholder, cx);
            if !panel.composer.is_empty() {
                input.set_text(panel.composer.clone(), cx);
            }
            input
        });
        let id = session.to_owned();
        cx.subscribe(&input, move |app, _, change: &Changed, cx| {
            let text = change.0.clone();
            app.panels
                .update(&id, |panel| panel.composer = text.clone());
            let active = app.active_id.as_deref() == Some(id.as_str());
            if active {
                // Tiled rendering gives every panel its own editor. Mirror the
                // active tile into the shared composer so leaving tiled mode or
                // sending through the rich composer keeps the same draft.
                app.composer = text.clone();
                if app.composer_input.read(cx).buffer.text() != text {
                    app.composer_input
                        .update(cx, |input, cx| input.set_text(text.clone(), cx));
                }
            }
            app.engine.set_draft(id.clone(), text);
            app.update_tile_completions(&id, cx);
            cx.notify();
        })
        .detach();
        self.panel_composers
            .entry(session.to_owned())
            .or_default();
        let forms = self.engine.forms(session);
        self.panels.update(session, |panel| panel.forms = forms);
        self.panel_inputs.insert(session.to_owned(), input.clone());
        self.panel_focuses
            .insert(session.to_owned(), input.focus_handle(cx));
    }

    fn save_active_panel_state(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.capture_panel_state() {
            let session = state.session.clone();
            let composer = state.composer.clone();
            self.panels.save(state);
            if let Some(input) = self.panel_inputs.get(&session).cloned() {
                input.update(cx, |input, cx| {
                    if input.buffer.text() != composer {
                        input.set_text(composer, cx);
                    }
                });
            }
        }
    }

    /// Keep tiled panels live without making them active. The active panel is
    /// already captured from the primary fields; every other open panel reads
    /// its engine view so simultaneous tiles stay current.
    fn refresh_panel_snapshots(&mut self) {
        let active = self.active_id.clone();
        let ids = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<Vec<_>>();
        for id in ids {
            if active.as_deref() == Some(id.as_str()) {
                continue;
            }
            let Some(view) = self.engine.view(&id) else {
                continue;
            };
            let blocks = Arc::new(view.blocks());
            let phase = view.phase();
            let (_, stick) = view.scroll();
            let permissions = self.engine.permissions(&id);
            let todos = self.engine.todos(&id);
            let forms = self.engine.forms(&id);
            let usage = self.engine.usage(&id);
            let title = self.panel_title(&id);
            self.panels.update(&id, |panel| {
                panel.title = title;
                panel.blocks = blocks;
                panel.phase = phase;
                panel.stick = stick;
                panel.permissions = permissions;
                panel.todos = todos;
                panel.forms = forms;
                panel.usage = usage;
            });
        }
    }

    fn activate_panel(&mut self, session: String, cx: &mut Context<Self>) {
        if self.active_id.as_deref() == Some(session.as_str()) {
            self.panels.activate(&session);
            return;
        }
        self.open_session(session, cx);
    }

    fn focus_panel(&mut self, session: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(focus) = self.panel_focuses.get(session).cloned() {
            window.focus(&focus, cx);
        }
    }

    fn tile_text_and_caret(&self, session: &str, cx: &App) -> Option<(String, usize)> {
        let input = self.panel_inputs.get(session)?;
        let input = input.read(cx);
        Some((input.buffer.text().to_string(), input.buffer.caret()))
    }

    fn tile_slash_items(&self, session: &str) -> Vec<CommandOption> {
        if !self.runtime_capabilities.commands {
            return Vec::new();
        }
        let Some(state) = self.panel_composers.get(session) else {
            return Vec::new();
        };
        if state.slash_dismissed {
            return Vec::new();
        }
        let text = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .map(|panel| panel.composer.as_str())
            .unwrap_or_default();
        let Some(query) = text.strip_prefix('/') else {
            return Vec::new();
        };
        if query.chars().any(char::is_whitespace) {
            return Vec::new();
        }
        let query = query.to_lowercase();
        self.command_catalog
            .iter()
            .filter(|item| {
                item.name.to_lowercase().contains(&query)
                    || item
                        .description
                        .as_deref()
                        .is_some_and(|description| description.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    fn update_tile_completions(&mut self, session: &str, cx: &mut Context<Self>) {
        // The focused panel already drives the shared composer's completion
        // state; only inactive tiles need their independent menus.
        if self.active_id.as_deref() == Some(session) {
            return;
        }
        let Some((text, caret)) = self.tile_text_and_caret(session, cx) else {
            return;
        };
        let spans = if self.runtime_capabilities.attachments {
            self.panel_composers
                .get(session)
                .map(|state| attachments::attachment_mention_spans(&state.attachments, &text))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mentions = self.runtime_capabilities.attachments;
        let mention = mentions
            .then(|| attachments::mention_trigger_at(&text, caret, &spans))
            .flatten();
        let argument = self
            .runtime_capabilities
            .commands
            .then(|| crate::slash_args::argument_at(&text, caret))
            .flatten()
            .filter(|trigger| {
                vela_opencode::commands::contains_command(&self.command_catalog, &trigger.command)
            });
        let mention_request = mention.as_ref().map(|(at, query)| {
            self.file_request += 1;
            (self.file_request, *at, query.clone())
        });
        let mut argument_request = None;
        {
            let state = self
                .panel_composers
                .entry(session.to_owned())
                .or_default();
            state.slash_dismissed = false;
            state.slash_selected = 0;
            if let Some((request, at, _)) = &mention_request {
                state.reference_pending = Some((*request, *at));
                state.reference_files.clear();
                state.reference_selected = 0;
            } else {
                state.reference_pending = None;
                state.reference_files.clear();
                state.reference_selected = 0;
            }
            match argument.as_ref() {
                Some(trigger) => {
                    let reuse = state.slash_arg_query == trigger.query
                        && (state.slash_arg_pending.is_some()
                            || !state.slash_arg_files.is_empty());
                    if !reuse {
                        self.file_request += 1;
                        argument_request =
                            Some((self.file_request, trigger.query.clone()));
                        state.slash_arg_pending = Some(self.file_request);
                        state.slash_arg_query = trigger.query.clone();
                        state.slash_arg_files.clear();
                        state.slash_arg_selected = 0;
                    }
                }
                None => {
                    state.slash_arg_files.clear();
                    state.slash_arg_pending = None;
                    state.slash_arg_query.clear();
                    state.slash_arg_selected = 0;
                }
            }
        }
        if let Some((request, _, query)) = mention_request {
            self.engine
                .find_references(request, session.to_owned(), query);
        }
        if let Some((request, query)) = argument_request {
            self.engine
                .find_references(request, session.to_owned(), query);
        }
    }

    fn attach_tile_files(
        &mut self,
        session: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.runtime_capabilities.attachments {
            return;
        }
        let receiver = crate::platform_prompt::files(window, cx);
        cx.spawn(async move |this, cx| {
            let Some(paths) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |app, cx| {
                if app.panels.contains(&session) {
                    let state = app
                        .panel_composers
                        .entry(session)
                        .or_default();
                    attachments::stage(&mut state.attachments, paths);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn apply_tile_reference_selection(
        &mut self,
        session: &str,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((at, rel)) = self.panel_composers.get(session).and_then(|state| {
            let (_, at) = state.reference_pending?;
            let rel = state.reference_files.get(index)?.clone();
            Some((at, rel))
        }) else {
            return;
        };
        let Some((text, caret)) = self.tile_text_and_caret(session, cx) else {
            return;
        };
        if at > caret || caret > text.len() {
            return;
        }
        let token = format!("@{rel}");
        let mut next = String::with_capacity(text.len() + token.len());
        next.push_str(&text[..at]);
        next.push_str(&token);
        next.push_str(&text[caret..]);
        let cursor = at + token.len();
        let Some(input) = self.panel_inputs.get(session).cloned() else {
            return;
        };
        input.update(cx, |input, cx| {
            input.set_text(next.clone(), cx);
            input.buffer.select(cursor, cursor);
        });
        let directory = self.workspace_directory();
        let state = self
            .panel_composers
            .entry(session.to_owned())
            .or_default();
        attachments::attach_staged_reference(
            &mut state.attachments,
            std::path::PathBuf::from(directory).join(&rel),
            &rel,
            vela_opencode::PromptMention {
                start: at,
                end: at + token.len(),
                text: token,
            },
        );
        state.reference_files.clear();
        state.reference_pending = None;
        state.reference_selected = 0;
        self.engine
            .set_draft(session.to_owned(), next);
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }

    fn apply_tile_slash_argument(
        &mut self,
        session: &str,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .panel_composers
            .get(session)
            .and_then(|state| state.slash_arg_files.get(index).cloned())
        else {
            return;
        };
        let Some((text, caret)) = self.tile_text_and_caret(session, cx) else {
            return;
        };
        let Some(trigger) = crate::slash_args::argument_at(&text, caret) else {
            return;
        };
        let (next, cursor) =
            crate::slash_args::insert_argument(&text, trigger.start, caret, &path);
        let Some(input) = self.panel_inputs.get(session).cloned() else {
            return;
        };
        input.update(cx, |input, cx| {
            input.set_text(next.clone(), cx);
            input.buffer.select(cursor, cursor);
        });
        if let Some(state) = self.panel_composers.get_mut(session) {
            state.slash_arg_files.clear();
            state.slash_arg_pending = None;
            state.slash_arg_query.clear();
            state.slash_arg_selected = 0;
        }
        self.engine
            .set_draft(session.to_owned(), next);
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }

    fn apply_tile_slash_selection(
        &mut self,
        session: &str,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = self.tile_slash_items(session);
        let Some(item) = items.get(index.min(items.len().saturating_sub(1))).cloned() else {
            return;
        };
        let Some(input) = self.panel_inputs.get(session).cloned() else {
            return;
        };
        let text = format!("/{}", item.name);
        input.update(cx, |input, cx| input.set_text(text, cx));
        self.send_tile_message(session.to_owned(), window, cx);
    }

    fn on_tile_composer_key(
        &mut self,
        session: &str,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(input) = self.panel_inputs.get(session).cloned() else {
            return false;
        };
        if input.read(cx).buffer.marked.is_some() {
            return false;
        }
        let keystroke = &event.keystroke;
        let plain = !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt;
        let references = self
            .panel_composers
            .get(session)
            .map(|state| (state.reference_files.clone(), state.reference_selected))
            .unwrap_or_default();
        if !references.0.is_empty() && plain {
            match keystroke.key.as_str() {
                "up" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.reference_selected = state
                            .reference_selected
                            .checked_sub(1)
                            .unwrap_or(references.0.len() - 1);
                    }
                    cx.notify();
                    return true;
                }
                "down" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.reference_selected =
                            (state.reference_selected + 1) % references.0.len();
                    }
                    cx.notify();
                    return true;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_tile_reference_selection(
                        session,
                        references.1,
                        window,
                        cx,
                    );
                    return true;
                }
                _ => {}
            }
        }
        let slash_items = self.tile_slash_items(session);
        let has_arguments = self
            .panel_composers
            .get(session)
            .is_some_and(|state| {
                !state.slash_arg_files.is_empty() || state.slash_arg_pending.is_some()
            });
        if keystroke.key == "escape" && (!slash_items.is_empty() || has_arguments) {
            let state = self
                .panel_composers
                .entry(session.to_owned())
                .or_default();
            state.slash_dismissed = true;
            state.slash_arg_files.clear();
            state.slash_arg_pending = None;
            state.slash_arg_query.clear();
            state.slash_arg_selected = 0;
            cx.notify();
            return true;
        }
        let arguments = self
            .panel_composers
            .get(session)
            .map(|state| (state.slash_arg_files.clone(), state.slash_arg_selected))
            .unwrap_or_default();
        if !arguments.0.is_empty() && plain {
            match keystroke.key.as_str() {
                "up" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.slash_arg_selected = state
                            .slash_arg_selected
                            .checked_sub(1)
                            .unwrap_or(arguments.0.len() - 1);
                    }
                    cx.notify();
                    return true;
                }
                "down" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.slash_arg_selected =
                            (state.slash_arg_selected + 1) % arguments.0.len();
                    }
                    cx.notify();
                    return true;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_tile_slash_argument(
                        session,
                        arguments.1,
                        window,
                        cx,
                    );
                    return true;
                }
                "escape" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.slash_arg_files.clear();
                        state.slash_arg_pending = None;
                        state.slash_arg_query.clear();
                        state.slash_arg_selected = 0;
                    }
                    cx.notify();
                    return true;
                }
                _ => {}
            }
        }
        if !slash_items.is_empty() && plain {
            let selected = self
                .panel_composers
                .get(session)
                .map(|state| state.slash_selected)
                .unwrap_or_default();
            match keystroke.key.as_str() {
                "up" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.slash_selected =
                            selected.checked_sub(1).unwrap_or(slash_items.len() - 1);
                    }
                    cx.notify();
                    return true;
                }
                "down" => {
                    if let Some(state) = self.panel_composers.get_mut(session) {
                        state.slash_selected = (selected + 1) % slash_items.len();
                    }
                    cx.notify();
                    return true;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_tile_slash_selection(session, selected, window, cx);
                    return true;
                }
                _ => {}
            }
        }
        if keystroke.key == "enter" && !keystroke.modifiers.shift {
            self.send_tile_message(session.to_owned(), window, cx);
            return true;
        }
        false
    }

    fn render_tile_composer_extras(
        &mut self,
        session: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let mut column = div().w_full().flex().flex_col().gap(px(3.));
        let attachments = self
            .panel_composers
            .get(session)
            .map(|state| state.attachments.clone())
            .unwrap_or_default();
        if !attachments.is_empty() {
            let mut chips = div()
                .id(SharedString::from(format!("panel-attachments-{session}")))
                .debug_selector({
                    let selector = format!("panel-attachments-{session}");
                    move || selector.clone()
                })
                .w_full()
                .flex()
                .flex_wrap()
                .gap(px(3.));
            for (index, attachment) in attachments.iter().enumerate() {
                let remove_session = session.to_owned();
                let remove_index = index;
                chips = chips.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-attachment-{session}-{index}"
                        )))
                        .debug_selector({
                            let selector = format!("panel-attachment-{session}-{index}");
                            move || selector.clone()
                        })
                        .max_w_full()
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .px(px(4.))
                        .py(px(2.))
                        .rounded(px(5.))
                        .border_1()
                        .border_color(theme.border_strong)
                        .bg(theme.accent.opacity(0.07))
                        .text_size(px(9.5))
                        .text_color(theme.dim)
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .child(SharedString::from(attachment.name.clone())),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "panel-remove-attachment-{session}-{index}"
                                )))
                                .debug_selector({
                                    let selector =
                                        format!("panel-remove-attachment-{session}-{index}");
                                    move || selector.clone()
                                })
                                .role(Role::Button)
                                .aria_label(SharedString::from(format!(
                                    "Remove {}",
                                    attachment.name
                                )))
                                .px(px(2.))
                                .text_size(px(12.))
                                .text_color(theme.faint)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .on_click(cx.listener(move |app, _, _, cx| {
                                    if let Some(state) =
                                        app.panel_composers.get_mut(&remove_session)
                                    {
                                        state.attachments.remove(remove_index);
                                    }
                                    cx.notify();
                                }))
                                .child("×"),
                        ),
                );
            }
            column = column.child(chips);
        }
        let references = self
            .panel_composers
            .get(session)
            .map(|state| (state.reference_files.clone(), state.reference_selected))
            .unwrap_or_default();
        if !references.0.is_empty() {
            let selected = references.1.min(references.0.len().saturating_sub(1));
            let mut menu = div()
                .id(SharedString::from(format!("panel-reference-menu-{session}")))
                .debug_selector({
                    let selector = format!("panel-reference-menu-{session}");
                    move || selector.clone()
                })
                .role(Role::Menu)
                .aria_label("File references")
                .w_full()
                .max_h(px(180.))
                .overflow_y_scroll()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.panel)
                .p(px(3.))
                .flex()
                .flex_col()
                .gap(px(1.));
            for (index, path) in references.0.iter().enumerate() {
                let path = path.clone();
                let select_session = session.to_owned();
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-reference-{session}-{index}"
                        )))
                        .debug_selector({
                            let selector = format!("panel-reference-{session}-{index}");
                            move || selector.clone()
                        })
                        .role(Role::MenuItem)
                        .aria_label(SharedString::from(format!("@{path}")))
                        .h(px(26.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .bg(if index == selected {
                            theme.selected
                        } else {
                            theme::rgba(0, 0, 0, 0.)
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.inset))
                        .on_click(cx.listener(move |app, _, window, cx| {
                            app.apply_tile_reference_selection(
                                &select_session,
                                index,
                                window,
                                cx,
                            );
                        }))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family("codicon")
                                .text_size(px(12.))
                                .text_color(theme.faint)
                                .child(icon("ea7b")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(10.5))
                                .text_color(theme.text)
                                .child(path),
                        ),
                );
            }
            column = column.child(menu);
        }
        let slash_items = self.tile_slash_items(session);
        if !slash_items.is_empty() {
            let selected = self
                .panel_composers
                .get(session)
                .map(|state| state.slash_selected)
                .unwrap_or_default()
                .min(slash_items.len().saturating_sub(1));
            let mut menu = div()
                .id(SharedString::from(format!("panel-slash-menu-{session}")))
                .debug_selector({
                    let selector = format!("panel-slash-menu-{session}");
                    move || selector.clone()
                })
                .role(Role::Menu)
                .aria_label("Slash commands")
                .w_full()
                .max_h(px(180.))
                .overflow_y_scroll()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.panel)
                .p(px(3.))
                .flex()
                .flex_col()
                .gap(px(1.));
            for (index, item) in slash_items.iter().enumerate() {
                let name = item.name.clone();
                let description = item.description.clone().unwrap_or_default();
                let select_session = session.to_owned();
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-slash-{session}-{name}"
                        )))
                        .debug_selector({
                            let selector = format!("panel-slash-{session}-{name}");
                            move || selector.clone()
                        })
                        .role(Role::MenuItem)
                        .aria_label(SharedString::from(format!("/{name}")))
                        .h(px(27.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .bg(if index == selected {
                            theme.selected
                        } else {
                            theme::rgba(0, 0, 0, 0.)
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.inset))
                        .on_click(cx.listener(move |app, _, window, cx| {
                            app.apply_tile_slash_selection(
                                &select_session,
                                index,
                                window,
                                cx,
                            );
                        }))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text)
                                .child(format!("/{name}")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(9.5))
                                .text_color(theme.faint)
                                .child(description),
                        ),
                );
            }
            column = column.child(menu);
        }
        let arguments = self
            .panel_composers
            .get(session)
            .map(|state| (state.slash_arg_files.clone(), state.slash_arg_selected))
            .unwrap_or_default();
        if !arguments.0.is_empty() {
            let selected = arguments.1.min(arguments.0.len().saturating_sub(1));
            let mut menu = div()
                .id(SharedString::from(format!(
                    "panel-slash-arguments-{session}"
                )))
                .debug_selector({
                    let selector = format!("panel-slash-arguments-{session}");
                    move || selector.clone()
                })
                .role(Role::Menu)
                .aria_label("Slash command arguments")
                .w_full()
                .max_h(px(180.))
                .overflow_y_scroll()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.panel)
                .p(px(3.))
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .px(px(6.))
                        .py(px(2.))
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.faint)
                        .child("FILE ARGUMENTS"),
                );
            for (index, path) in arguments.0.iter().enumerate() {
                let path = path.clone();
                let select_session = session.to_owned();
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-slash-argument-{session}-{index}"
                        )))
                        .debug_selector({
                            let selector = format!("panel-slash-argument-{session}-{index}");
                            move || selector.clone()
                        })
                        .role(Role::MenuItem)
                        .aria_label(path.clone())
                        .aria_selected(index == selected)
                        .h(px(26.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .bg(if index == selected {
                            theme.selected
                        } else {
                            theme::rgba(0, 0, 0, 0.)
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.inset))
                        .on_click(cx.listener(move |app, _, window, cx| {
                            app.apply_tile_slash_argument(
                                &select_session,
                                index,
                                window,
                                cx,
                            );
                        }))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(10.5))
                                .text_color(theme.text)
                                .child(path),
                        ),
                );
            }
            column = column.child(menu);
        }
        column.into_any_element()
    }

    fn send_tile_message(&mut self, session: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.panel_inputs.get(&session).cloned() else {
            return;
        };
        let text = input.read(cx).buffer.text().trim().to_owned();
        let mut state = self
            .panel_composers
            .remove(&session)
            .unwrap_or_default();
        let slash = prompt_command(&text, self.runtime_capabilities.commands);
        let files = if slash.is_some() {
            Vec::new()
        } else {
            attachments::prompt_files_from_staged(
                &text,
                std::mem::take(&mut state.attachments),
            )
        };
        if text.is_empty() && files.is_empty() {
            self.panel_composers.insert(session, state);
            return;
        }
        let text = if text.is_empty() {
            "Review the attached files.".to_owned()
        } else {
            text
        };
        if self.active_id.as_deref() != Some(session.as_str()) {
            self.open_session(session.clone(), cx);
        }
        if let Some((name, args)) = slash {
            self.file_request += 1;
            let request = self.file_request;
            self.slash_requests.insert(request, session.clone());
            self.engine.run_slash_command(
                request,
                session.clone(),
                name,
                crate::slash_args::unescape_arguments(&args),
            );
        } else {
            self.engine.prompt_with_behavior(
                session.clone(),
                text,
                files,
                self.phase
                    .is_active()
                    .then_some(self.settings.follow_up_behavior),
            );
        }
        self.engine.set_draft(session.clone(), String::new());
        input.update(cx, |input, cx| input.set_text(String::new(), cx));
        self.panels.update(&session, |panel| panel.composer.clear());
        if self.active_id.as_deref() == Some(session.as_str()) {
            self.composer.clear();
            self.composer_input
                .update(cx, |input, cx| input.set_text(String::new(), cx));
            self.stick = true;
            self.transcript_list.scroll_to_end();
        }
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }

    fn close_panel(&mut self, session: String, cx: &mut Context<Self>) {
        let was_active = self.active_id.as_deref() == Some(session.as_str());
        self.panels.close(&session);
        self.panel_composers.remove(&session);
        self.panel_form_docks.remove(&session);
        if !was_active {
            cx.notify();
            return;
        }
        if self.panels.is_empty() {
            self.active_id = None;
            self.workspace_closed = true;
            self.clear_active_session_ui(&session, cx);
        } else if let Some(next) = self.panels.active().map(|state| state.session.clone()) {
            self.active_id = None;
            self.open_session(next, cx);
        }
        cx.notify();
    }

    fn open_session(&mut self, id: String, cx: &mut Context<Self>) {
        self.show_session(id, false, cx);
    }

    /// Match the reference's `reopenSession`: an already-visible session is
    /// focused, otherwise the focused panel is replaced rather than growing the
    /// grid. Child-session cards and the agent-header back button use this so
    /// drill-down leaves no stale parent panel behind.
    fn reopen_session(&mut self, id: String, cx: &mut Context<Self>) {
        self.show_session(id, true, cx);
    }

    fn show_session(&mut self, id: String, replace_active: bool, cx: &mut Context<Self>) {
        self.document_scopes
            .entry(id.clone())
            .or_insert_with(|| uuid::Uuid::new_v4().to_string());
        self.workspace_closed = false;
        if self.active_id.as_deref() == Some(id.as_str()) {
            if !self.panels.contains(&id)
                && let Err(error) = self.panels.open(crate::panels::PanelState::new(
                    id.clone(),
                    self.panel_title(&id),
                ))
            {
                self.toasts
                    .update(cx, |toasts, cx| toasts.push(error, true, cx));
            }
            self.panels.activate(&id);
            return;
        }
        self.save_active_panel_state(cx);
        if !self.panels.contains(&id) {
            let state = crate::panels::PanelState::new(id.clone(), self.panel_title(&id));
            let opened = if replace_active {
                self.panels.replace_active(state)
            } else {
                self.panels.open(state)
            };
            if let Err(error) = opened {
                self.toasts
                    .update(cx, |toasts, cx| toasts.push(error, true, cx));
                cx.notify();
                return;
            }
        } else {
            self.panels.activate(&id);
        }
        let panel_sessions = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<Vec<_>>();
        for session in panel_sessions {
            self.ensure_panel_input(&session, cx);
        }
        // Reopening a context supersedes its older asynchronous close. Saves
        // already dispatched can finish, but must not retire the new activation.
        self.workspace_closures.remove(&id);
        self.status_menu_open = false;
        self.usage_open = false;
        self.pending_create = None;
        self.created_files_to_open.clear();
        self.save_view_state();
        self.attachments.clear();
        self.attachment_drag_over = false;
        self.dismiss_tab_overflow();
        self.directory_requests.clear();
        self.status_request = None;
        self.diff_request = None;
        self.files.clear();
        self.children.clear();
        self.expanded.clear();
        self.file_error = None;
        self.changes.clear();
        self.diff = None;
        self.problems.clear();
        self.problem_path = None;
        self.validation_requests.clear();
        self.validation_summary = None;
        self.validation_failed = false;
        self.validation_key = None;
        self.models.clear();
        self.agents.clear();
        self.selected_model = None;
        self.selected_agent = None;
        self.catalog_session = None;
        self.provider_catalog.clear();
        self.clear_provider_forms(cx);
        self.picker = None;
        self.inbox.clear();
        self.inbox_session = None;
        self.sass = self.engine.sass_watch_state(&id);
        self.preview_notice = None;
        self.preview_menu = None;
        self.active_id = Some(id.clone());
        self.revision = u64::MAX;
        self.pending_reset = true;
        self.permissions = self.engine.permissions(&id);
        self.error = None;
        let cached = self.engine.view(&id);
        if let Some(view) = &cached {
            let (_, stick) = view.scroll();
            self.blocks = Arc::new(view.blocks());
            self.phase = view.phase();
            self.composer = view.draft();
            self.stick = stick;
            self.loading = false;
        } else {
            self.blocks = Arc::new(Vec::new());
            self.phase = SessionPhase::Idle;
            self.composer.clear();
            self.stick = true;
            self.loading = true;
        }
        if let Some(panel) = self.panels.active().cloned() {
            if cached.is_none() && !panel.blocks.is_empty() {
                self.blocks = panel.blocks;
                self.phase = panel.phase;
                self.loading = false;
            }
            if !panel.composer.is_empty() || cached.is_some() {
                self.composer = panel.composer;
            }
            self.permissions = if panel.permissions.is_empty() {
                self.permissions.clone()
            } else {
                panel.permissions
            };
            self.todos = panel.todos;
            self.usage = panel.usage;
            self.selected_model = panel.selected_model;
            self.selected_agent = panel.selected_agent;
            self.scroll_offsets.insert(id.clone(), panel.scroll);
            self.stick = panel.stick;
        }
        self.transcript_rows = Arc::new(transcript_view::project(
            &self.blocks,
            self.phase.is_active(),
            &id,
            &self.transcript_ui,
        ));
        self.composer_input
            .update(cx, |input, cx| input.set_text(self.composer.clone(), cx));
        self.engine.open_session(id.clone());
        self.recovery_records.clear();
        self.recovery_scroll
            .native_handle()
            .set_offset(point(px(0.), px(0.)));
        self.engine.load_recovery(id.clone());
        self.forms
            .update(cx, |dock, cx| dock.set_forms(Vec::new(), cx));
        self.forms_session = Some(id.clone());
        self.form_requests.clear();
        self.slash_requests.clear();
        self.command_catalog.clear();
        self.command_session = Some(id.clone());
        self.slash_selected = 0;
        self.slash_menu.edited();
        self.clear_slash_arguments();
        self.reference_files.clear();
        self.reference_pending = None;
        self.reference_selected = 0;
        self.engine.refresh_forms(id.clone());
        self.engine.refresh_commands(id.clone());
        self.settings.record_session(&id);
        if let Some(dir) = self.engine.data_dir() {
            let _ = self.settings.save(Some(&dir));
        }
        cx.notify();
    }

    fn save_view_state(&mut self) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        self.engine.set_draft(id.clone(), self.composer.clone());
        self.engine.set_scroll(id.clone(), 0.0, self.stick);
        self.scroll_offsets
            .insert(id, self.transcript_list.logical_scroll_top());
    }

    fn refresh_active(&mut self) {
        self.connection = self.engine.connection();
        self.error = self.engine.last_error();
        if self.active_id.is_none() && !self.workspace_closed {
            self.active_id = self.engine.active_session();
        }
        let Some(id) = self.active_id.clone() else {
            return;
        };
        self.document_scopes
            .entry(id.clone())
            .or_insert_with(|| uuid::Uuid::new_v4().to_string());
        let Some(view) = self.engine.view(&id) else {
            return;
        };
        let (revision, blocks) = view.blocks_with_revision();
        self.phase = view.phase();
        if revision != self.revision {
            tracing::debug!(session = %id, revision, blocks = blocks.len(), "ui transcript refresh");
            let old_count = self.transcript_rows.len();
            self.revision = revision;
            self.blocks = Arc::new(blocks);
            self.loading = false;
            self.transcript_rows = Arc::new(transcript_view::project(
                &self.blocks,
                self.phase.is_active(),
                &id,
                &self.transcript_ui,
            ));
            let new_count = self.transcript_rows.len();
            if self.pending_reset {
                self.transcript_list.reset(new_count);
                self.pending_reset = false;
                if let Some(offset) = self.scroll_offsets.get(&id).copied() {
                    self.transcript_list.scroll_to(offset);
                }
            } else if new_count > old_count {
                self.transcript_list
                    .splice(old_count..old_count, new_count - old_count);
            } else if new_count < old_count {
                self.transcript_list.splice(new_count..old_count, 0);
            } else {
                self.transcript_list.remeasure();
            }
            if self.stick {
                self.transcript_list.scroll_to_end();
            }
        }
        self.phase = view.phase();
        self.permissions = self.engine.permissions(&id);
        self.todos = self.engine.todos(&id);
        self.usage = self.engine.usage(&id);
    }

    fn synchronize_terminal_workspace(&mut self, cx: &mut Context<Self>) {
        let workspace = self.workspace_directory();
        if !self.terminals.owns(&workspace, self.active_id.as_deref()) {
            for view in self.terminals.reset(workspace, self.active_id.clone()) {
                self.engine.stop_terminal(view.id);
            }
            self.spawn_terminal(None, false, cx);
        }
    }
    fn fit_terminal(
        &mut self,
        id: &str,
        workspace: &str,
        size: vela_terminal::size::Size,
        cx: &mut Context<Self>,
    ) {
        if !self.tray.open
            || self.terminal_panel != TerminalPanel::Terminal
            || self.terminals.workspace != workspace
        {
            return;
        }
        let Some(view) = self.terminals.active_mut().filter(|view| view.id == id) else {
            return;
        };
        if view.cols == size.cols && view.rows == size.rows {
            return;
        }
        view.cols = size.cols;
        view.rows = size.rows;
        view.emulator.resize(size.cols, size.rows);
        self.engine.terminal_resize(id, size.cols, size.rows);
        cx.notify();
    }
    fn create_terminal(&mut self, cwd: Option<String>, cx: &mut Context<Self>) {
        self.spawn_terminal(cwd, true, cx);
    }
    fn spawn_terminal(&mut self, cwd: Option<String>, reveal: bool, cx: &mut Context<Self>) {
        let root = self.workspace_directory();
        if root.is_empty() {
            return;
        }
        if !self.terminals.owns(&root, self.active_id.as_deref()) {
            for view in self.terminals.reset(root.clone(), self.active_id.clone()) {
                self.engine.stop_terminal(view.id);
            }
        }
        let cwd = cwd.unwrap_or(root.clone());
        self.file_request += 1;
        let id = format!("term-{}", uuid::Uuid::new_v4());
        let mut terminal = TerminalView::new(id.clone(), 100, 24, cx);
        let input_id = id.clone();
        terminal.input_subscriptions.push(cx.subscribe(
            &terminal.input,
            move |app, _, event: &terminal_composition::Committed, cx| {
                let Some(view) = app.terminals.get_mut(&input_id).filter(|view| view.running)
                else {
                    return;
                };
                view.emulator.clear_selection();
                view.emulator.scroll_to_bottom();
                view.selecting = false;
                app.engine
                    .terminal_input(input_id.clone(), event.0.as_bytes().to_vec());
                cx.notify();
            },
        ));
        let composition_id = id.clone();
        terminal
            .input_subscriptions
            .push(cx.observe(&terminal.input, move |app, input, cx| {
                if input.read(cx).buffer.marked.is_some()
                    && let Some(view) = app.terminals.get_mut(&composition_id)
                {
                    view.emulator.scroll_to_bottom();
                }
                cx.notify();
            }));
        terminal
            .input_subscriptions
            .push(cx.observe(&terminal.blink, |_, _, cx| {
                cx.notify();
            }));
        if cwd != root {
            terminal.name = Path::new(&cwd)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
        }
        self.terminals.insert(terminal);
        self.engine.start_terminal(id, cwd, 100, 24);
        if reveal {
            self.tray.open = true;
            self.terminal_panel = TerminalPanel::Terminal;
            self.terminal_focus_pending = true;
        } else if self.tray.open && self.terminal_panel == TerminalPanel::Terminal {
            self.terminal_focus_pending = true;
        }
        cx.notify();
    }
    fn spawn_tui_terminal(&mut self, cx: &mut Context<Self>) {
        let root = self.workspace_directory();
        if root.is_empty() {
            return;
        }
        if !self.terminals.owns(&root, self.active_id.as_deref()) {
            for view in self.terminals.reset(root.clone(), self.active_id.clone()) {
                self.engine.stop_terminal(view.id);
            }
        }
        let configured = std::env::var_os("VELA_OPENCODE_BINARY");
        let path_var = std::env::var_os("PATH");
        let Some(program) = find_tui_program(configured.as_deref(), path_var.as_deref()) else {
            self.toasts.update(cx, |toasts, cx| {
                toasts.push(
                    "OpenCode CLI was not found. Install OpenCode or set VELA_OPENCODE_BINARY.",
                    true,
                    cx,
                )
            });
            cx.notify();
            return;
        };
        let program = program.to_string_lossy().into_owned();
        self.file_request += 1;
        let id = format!("term-{}", uuid::Uuid::new_v4());
        let mut terminal = TerminalView::new(id.clone(), 100, 24, cx);
        let input_id = id.clone();
        terminal.input_subscriptions.push(cx.subscribe(
            &terminal.input,
            move |app, _, event: &terminal_composition::Committed, cx| {
                let Some(view) = app.terminals.get_mut(&input_id).filter(|view| view.running)
                else {
                    return;
                };
                view.emulator.clear_selection();
                view.emulator.scroll_to_bottom();
                view.selecting = false;
                app.engine
                    .terminal_input(input_id.clone(), event.0.as_bytes().to_vec());
                cx.notify();
            },
        ));
        let composition_id = id.clone();
        terminal
            .input_subscriptions
            .push(cx.observe(&terminal.input, move |app, input, cx| {
                if input.read(cx).buffer.marked.is_some()
                    && let Some(view) = app.terminals.get_mut(&composition_id)
                {
                    view.emulator.scroll_to_bottom();
                }
                cx.notify();
            }));
        terminal
            .input_subscriptions
            .push(cx.observe(&terminal.blink, |_, _, cx| {
                cx.notify();
            }));
        terminal.name = Some("Agent TUI".into());
        self.terminals.insert(terminal);
        self.engine
            .start_terminal_command(vela_engine::TerminalCommand {
                id,
                cwd: root,
                program,
                args: Vec::new(),
                envs: Vec::new(),
                cols: 100,
                rows: 24,
            });
        self.tray.open = true;
        self.terminal_panel = TerminalPanel::Terminal;
        self.terminal_focus_pending = true;
        cx.notify();
    }

    fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.browser_open {
            self.browser_open = false;
            if let Some(browser) = &self.browser {
                browser.set_visible(false);
            }
        }
        self.synchronize_terminal_workspace(cx);
        self.cancel_terminal_composition(cx);
        if !self.terminals.initialized {
            self.tray.show();
            self.create_terminal(None, cx);
        } else {
            self.tray.toggle();
        }
        if self.tray.open
            && let Some(terminal) = self.terminals.active()
        {
            window.focus(&terminal.focus, cx);
        }
        cx.notify();
    }
    fn close_terminal(&mut self, cx: &mut Context<Self>) {
        self.cancel_terminal_composition(cx);
        self.tray.close();
        cx.notify();
    }
    fn cancel_terminal_composition(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = self.terminals.active() {
            let input = view.input.clone();
            let blink = view.blink.clone();
            let blinking = view.emulator.cursor_blinking();
            input.update(cx, |input, cx| input.cancel(cx));
            blink.update(cx, |blink, cx| blink.update_state(blinking, false, cx));
        }
    }

    fn select_terminal(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_terminal_composition(cx);
        if self.terminals.select(id) {
            self.terminal_panel = TerminalPanel::Terminal;
            if let Some(terminal) = self.terminals.active() {
                window.focus(&terminal.focus, cx);
            }
            cx.notify();
        }
    }
    fn close_terminal_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        let active = self.terminals.active().is_some_and(|view| view.id == id);
        if active {
            self.cancel_terminal_composition(cx);
        }
        if let Some(view) = self.terminals.remove(id) {
            self.engine.stop_terminal(view.id);
        }
        if self.terminals.views.is_empty() {
            self.tray.open = false;
        }
        if active {
            self.terminal_focus_pending = true;
        }
        cx.notify();
    }

    fn handle_terminal_exit(&mut self, id: &str, code: Option<i32>, cx: &mut Context<Self>) {
        let tui = self
            .terminals
            .views
            .iter()
            .any(|view| view.id == id && view.name.as_deref() == Some("Agent TUI"));
        let active = self.terminals.active().is_some_and(|view| view.id == id);
        if self.terminals.remove(id).is_some() {
            if tui {
                self.toasts
                    .update(cx, |toasts, cx| toasts.push(tui_exit_notice(code), false, cx));
            }
            if self.terminals.views.is_empty() {
                self.tray.open = false;
            }
            if active {
                self.terminal_focus_pending = true;
            }
            cx.notify();
        }
    }

    fn paste_terminal(&mut self, origin: &'static str, cx: &mut Context<Self>) {
        if let Some(terminal) = self.terminals.active_mut()
            && terminal.running
            && let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
        {
            tracing::debug!(terminal_id=%terminal.id,origin,bytes=text.len(),"terminal paste dispatch");
            let bytes =
                vela_terminal::input::paste_bytes(&text, terminal.emulator.bracketed_paste_mode());
            terminal.emulator.scroll_to_bottom();
            terminal.emulator.clear_selection();
            terminal.selecting = false;
            terminal.input.update(cx, |input, cx| input.cancel(cx));
            self.engine.terminal_input(terminal.id.clone(), bytes);
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn on_terminal_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(terminal) = self.terminals.active_mut() else {
            return;
        };
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.platform && key == "c" {
            self.copy_terminal(cx);
            return;
        }
        if matches!(key, "up" | "down" | "enter") {
            tracing::debug!(terminal_id=%terminal.id,key,held=event.is_held,alt=event.keystroke.modifiers.alt,control=event.keystroke.modifiers.control,platform=event.keystroke.modifiers.platform,"terminal navigation dispatch");
        }

        let paste = (event.keystroke.modifiers.platform && key == "v")
            || (event.keystroke.modifiers.shift && key == "insert");
        if paste {
            self.paste_terminal("key", cx);
            return;
        }
        match key {
            "pageup" if event.keystroke.modifiers.shift => {
                terminal.emulator.scroll(8);
                cx.stop_propagation();
                cx.notify();
                return;
            }
            "pagedown" if event.keystroke.modifiers.shift => {
                terminal.emulator.scroll(-8);
                cx.stop_propagation();
                cx.notify();
                return;
            }
            _ => {}
        }
        if !terminal.running {
            return;
        }
        if terminal_composition::platform_text(
            event,
            terminal.input.read(cx).buffer.marked.is_some(),
        ) {
            return;
        }
        let bytes = encode_terminal_key(event, terminal.emulator.app_cursor_mode());
        if !bytes.is_empty() {
            terminal.input.update(cx, |input, cx| input.cancel(cx));
            terminal.emulator.clear_selection();
            terminal.selecting = false;
            terminal.emulator.scroll_to_bottom();
            self.engine.terminal_input(terminal.id.clone(), bytes);
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn interrupt(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = &self.active_id {
            self.engine.interrupt(id.clone());
            cx.notify();
        }
    }

    fn revert_in_flight(&self, session: &str) -> bool {
        self.revert_requests
            .values()
            .any(|(owner, _)| owner == session)
            || self
                .commit_revert_requests
                .values()
                .any(|owner| owner == session)
            || self
                .clear_revert_requests
                .values()
                .any(|owner| owner == session)
    }

    fn revert_message(
        &mut self,
        message_id: String,
        edit: bool,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.active_id.clone() else {
            return;
        };
        // Source disables the revert control while a run is active and ignores
        // repeat activation while any revert request is already in flight.
        if self.phase.is_active() || self.revert_in_flight(&session) {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.revert_requests
            .insert(request, (session.clone(), edit.then_some(text)));
        self.toasts.update(cx, |toasts, cx| {
            toasts.push(
                if edit {
                    "Restoring prompt for editing…"
                } else {
                    "Undoing prompt…"
                },
                false,
                cx,
            )
        });
        self.engine.revert_session(request, session, message_id);
        cx.notify();
    }

    fn commit_staged_revert(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else {
            return;
        };
        if !self.staged_reverts.contains(&session) || self.revert_in_flight(&session) {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.commit_revert_requests.insert(request, session.clone());
        self.engine.commit_revert(request, session);
        cx.notify();
    }

    fn discard_staged_revert(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else {
            return;
        };
        if !self.staged_reverts.contains(&session) || self.revert_in_flight(&session) {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.clear_revert_requests.insert(request, session.clone());
        self.engine.clear_revert(request, session);
        cx.notify();
    }

    fn switch_sidebar(&mut self, tab: SidebarTab, cx: &mut Context<Self>) {
        self.sidebar_tab = tab;
        if tab == SidebarTab::Files {
            self.refresh_files(cx);
            self.refresh_changes(cx);
        } else if tab == SidebarTab::Changes {
            self.refresh_changes(cx);
        }
        cx.notify();
    }

    fn activate_sidebar_tab(&mut self, tab: SidebarTab, cx: &mut Context<Self>) {
        if !self.settings_open && self.sidebar_open && self.sidebar_tab == tab {
            self.sidebar_open = false;
            cx.notify();
            return;
        }
        self.settings_open = false;
        self.sidebar_open = true;
        self.switch_sidebar(tab, cx);
    }

    fn refresh_files(&mut self, cx: &mut Context<Self>) {
        if self.active_id.is_none() {
            self.file_error = Some("Open a session to browse its workspace".into());
            cx.notify();
            return;
        }
        self.request_directory(String::new());
        cx.notify();
    }

    fn request_directory(&mut self, rel: String) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        self.file_request += 1;
        let request = self.file_request;
        let directory = self.workspace_directory();
        self.directory_requests
            .retain(|_, key| key.workspace != directory || key.rel != rel);
        self.directory_requests
            .insert(request, DocumentKey::new(directory, rel.clone()));
        self.engine.list_dir(request, id, rel);
    }

    fn toggle_dir(&mut self, rel: String, cx: &mut Context<Self>) {
        if self.expanded.remove(&rel) {
            cx.notify();
            return;
        }
        self.expanded.insert(rel.clone());
        if !self.children.contains_key(&rel)
            && let Some(id) = self.active_id.clone()
        {
            self.file_request += 1;
            let request = self.file_request;
            self.directory_requests.insert(
                request,
                DocumentKey::new(self.workspace_directory(), rel.clone()),
            );
            self.engine.list_dir(request, id, rel);
        }
        cx.notify();
    }

    fn toggle_sass_watch(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else {
            return;
        };
        let scope = self.document_workspace();
        if self
            .sass_pending
            .as_ref()
            .is_some_and(|(_, owner)| *owner == scope)
        {
            return;
        }
        let token = uuid::Uuid::new_v4();
        self.sass_pending = Some((token, scope.clone()));
        let enabled = !self.sass.enabled;
        let engine = self.engine.clone();
        cx.spawn(async move |owner, cx| {
            // A scheduled task may first run after navigation or root closure.
            // Validate its captured activation before sending any engine command.
            let current = owner
                .update(cx, |app, cx| {
                    let owns_request = app
                        .sass_pending
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == token);
                    let current = owns_request
                        && app.active_id.as_deref() == Some(session.as_str())
                        && app.document_workspace() == scope;
                    if owns_request && !current {
                        app.sass_pending = None;
                        cx.notify();
                    }
                    current
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            let result = engine.update_sass_watch(session.clone(), enabled).await;
            let _ = owner.update(cx, |app, cx| {
                if app
                    .sass_pending
                    .as_ref()
                    .is_none_or(|(pending, _)| *pending != token)
                {
                    return;
                }
                app.sass_pending = None;
                if app.active_id.as_deref() == Some(session.as_str())
                    && app.document_workspace() == scope
                {
                    match result {
                        // Query the latest immutable state: a completed compile
                        // may already have superseded the request's Starting ACK.
                        Ok(_) => app.sass = app.engine.sass_watch_state(&session),
                        Err(error) => {
                            app.sass.status = vela_engine::SassStatus::Error;
                            app.sass.error = Some(error);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn ensure_changes_view(&mut self, cx: &mut Context<Self>) {
        let scope = self.document_workspace();
        let directory = self.workspace_directory();
        if scope.is_empty() || directory.is_empty() || self.change_reviews.contains_key(&scope) {
            return;
        }
        let seed = self.engine.observed_files(&directory);
        let review = cx.new(|cx| crate::changes_view::ChangesView::new(directory, &seed, cx));
        let captured = scope.clone();
        cx.subscribe(
            &review,
            move |app, view, event: &crate::changes_view::Event, cx| {
                if app.document_workspace() != captured {
                    return;
                }
                match event {
                    crate::changes_view::Event::Changed => app.changes = view.read(cx).summary(),
                    crate::changes_view::Event::Close => app.changes_open = false,
                    crate::changes_view::Event::Open(path) => {
                        app.changes_open = false;
                        // Editor focus/opening needs a Window; consume on next render.
                        app.review_file_to_open = Some(DocumentKey::in_workspace(
                            captured.clone(),
                            app.workspace_directory(),
                            path.clone(),
                        ));
                    }
                }
                cx.notify();
            },
        )
        .detach();
        self.change_reviews.insert(scope, review);
    }

    fn workspace_directory(&self) -> String {
        #[cfg(test)]
        if let Some(directory) = &self.test_workspace_directory {
            return directory.clone();
        }
        self.active_id
            .as_ref()
            .and_then(|id| self.engine.view(id))
            .map(|view| view.summary().directory.clone())
            .unwrap_or_default()
    }

    fn document_workspace(&self) -> String {
        self.active_id
            .as_ref()
            .and_then(|id| self.document_scopes.get(id))
            .cloned()
            .unwrap_or_default()
    }

    fn active_document_key(&self) -> Option<DocumentKey> {
        self.documents
            .borrow()
            .active_key(&self.document_workspace())
            .cloned()
    }

    fn open_file(&mut self, rel: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_file_tab(rel, false, window, cx);
    }

    fn open_file_tab(
        &mut self,
        rel: String,
        pinned: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key =
            DocumentKey::in_workspace(self.document_workspace(), self.workspace_directory(), rel);
        self.open_document_tab(key, pinned, window, cx);
    }

    fn open_document_tab(
        &mut self,
        key: DocumentKey,
        pinned: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        if !self.document_sessions.contains_key(&key.directory) {
            self.engine
                .retain_editor_workspace(id.clone(), key.directory.clone());
            self.document_sessions
                .insert(key.directory.clone(), id.clone());
        }
        self.browser_open = false;
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        self.diff = None;
        self.diff_request = None;
        self.changes_open = false;
        self.file_error = None;
        self.file_request += 1;
        let request = self.file_request;
        if self
            .documents
            .borrow_mut()
            .open(request, key.clone(), pinned)
        {
            self.engine.read_document(request, key.clone());
        }
        self.editor_views.retain(|key, _| {
            let documents = self.documents.borrow();
            documents.get(key).is_some()
                || documents
                    .tabs(&key.workspace)
                    .iter()
                    .any(|tab| tab.key == *key)
        });
        let editor = self.editor_view(key, cx);
        window.focus(&editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn dispatch_save(&self, request: FileRequest) {
        self.engine.write_document(request);
    }

    fn release_unused_document_workspaces(&mut self) {
        let unused = self
            .document_sessions
            .keys()
            .filter(|directory| !self.documents.borrow().retains_directory(directory))
            .cloned()
            .collect::<Vec<_>>();
        for directory in unused {
            self.document_sessions.remove(&directory);
            self.engine.release_editor_workspace(directory);
        }
    }

    fn close_document(&mut self, key: DocumentKey, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .documents
            .borrow()
            .get(&key)
            .is_some_and(|d| d.conflict.is_some())
        {
            return;
        }
        if self.documents.borrow().get(&key).is_some_and(|d| d.dirty()) {
            // Match the reference: close only after the exact revision saves.
            // An intervening edit or failed/conflicting write keeps the tab open.
            let Some(session) = self.active_id.clone() else {
                return;
            };
            if key.workspace != self.document_workspace() {
                return;
            }
            self.close_after_save.insert(key.clone());
            self.save_document_key(key, session, cx);
        } else {
            self.documents.borrow_mut().close(&key);
            self.editor_views.remove(&key);
            for group in self.editor_group_layout_mut(&key.workspace) {
                if group.as_ref() == Some(&key) {
                    *group = None;
                }
            }
            self.release_unused_document_workspaces();
            if let Some(next) = self.active_document_key() {
                let editor = self.editor_view(next, cx);
                window.focus(&editor.focus_handle(cx), cx);
            } else {
                window.focus(&self.composer_focus, cx);
            }
        }
        cx.notify();
    }

    fn pending_validation_request(&self) -> Option<u64> {
        let key = self.active_document_key()?;
        pending_validation(&self.validation_requests, &key)
    }

    fn cancel_validation(&mut self, cx: &mut Context<Self>) {
        let Some(request) = self.pending_validation_request() else {
            return;
        };
        self.validation_requests.remove(&request);
        self.validation_summary = None;
        self.validation_failed = false;
        self.validation_key = None;
        self.engine.cancel_validation(request);
        self.status_menu_open = false;
        self.toasts.update(cx, |toasts, cx| {
            toasts.push("Validation cancelled", false, cx)
        });
        cx.notify();
    }

    fn validate_active_document(&mut self, cx: &mut Context<Self>) {
        if self.pending_validation_request().is_some() {
            return;
        }
        let Some(key) = self.active_document_key() else {
            return;
        };
        let target = validation_target(&key.rel);
        if target == ValidationTarget::Unsupported {
            self.toasts.update(cx, |toasts, cx| {
                toasts.push(
                    "Open an HTML, CSS, or SCSS file to run validation",
                    true,
                    cx,
                )
            });
            self.status_menu_open = false;
            cx.notify();
            return;
        }
        let Some(content) = self
            .documents
            .borrow()
            .get(&key)
            .map(|document| document.buffer.text().to_owned())
        else {
            return;
        };
        let Some(session) = self.active_id.clone() else {
            return;
        };
        self.file_request += 1;
        let request = self.file_request;
        track_validation_request(&mut self.validation_requests, request, key.clone());
        self.validation_summary = None;
        self.validation_failed = false;
        self.validation_key = Some(key.clone());
        if let Some(editor) = self.editor_views.get(&key) {
            editor.update(cx, |editor, cx| {
                editor.set_validation_diagnostics(Vec::new(), cx)
            });
        }
        match target {
            ValidationTarget::W3c => {
                self.engine
                    .validate_document(request, session, key.rel.clone(), content);
            }
            ValidationTarget::Scss => {
                self.engine
                    .validate_scss(request, session, key.rel.clone(), content);
            }
            ValidationTarget::Unsupported => {}
        }
        self.status_menu_open = false;
        self.toasts.update(cx, |toasts, cx| {
            toasts.push(
                if target == ValidationTarget::Scss {
                    "Validating with Sass…"
                } else {
                    "Validating with W3C…"
                },
                false,
                cx,
            )
        });
        cx.notify();
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        let Some(key) = self.active_document_key() else {
            return;
        };
        self.save_document_key(key, id, cx);
    }

    fn new_editor_view(
        &mut self,
        key: DocumentKey,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<CodeEditor> {
        let documents = self.documents.clone();
        let view_key = key.clone();
        let editor = cx.new(|cx| CodeEditor::new(view_key, documents, cx));
        cx.subscribe(&editor, |_, _, _: &Edited, cx| cx.notify())
            .detach();
        cx.subscribe(&editor, |this, editor, _: &SaveRequested, cx| {
            let key = editor.read(cx).key.clone();
            if key.workspace != this.document_workspace() {
                return;
            }
            let session = this.active_id.clone();
            if let Some(session) = session {
                this.save_document_key(key, session, cx);
            }
        })
        .detach();
        cx.subscribe(&editor, |this, _, event: &OpenSymbol, cx| {
            this.pending_editor_jump = Some((event.key.clone(), event.offset));
            cx.notify();
        })
        .detach();
        editor
    }

    fn apply_pending_editor_jump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((key, offset)) = self.pending_editor_jump.take() else {
            return;
        };
        if self.active_id.is_none() || key.workspace != self.document_workspace() {
            return;
        }
        let known = self
            .documents
            .borrow()
            .tabs(&key.workspace)
            .iter()
            .any(|tab| tab.key == key);
        if !known {
            return;
        }
        self.open_document_tab(key.clone(), false, window, cx);
        if let Some(editor) = self.editor_views.get(&key) {
            editor.update(cx, |editor, cx| editor.select_offset(offset, cx));
            window.focus(&editor.focus_handle(cx), cx);
        }
        cx.notify();
    }

    fn editor_view(
        &mut self,
        key: DocumentKey,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<CodeEditor> {
        if let Some(view) = self.editor_views.get(&key) {
            return view.clone();
        }
        let editor = self.new_editor_view(key.clone(), cx);
        self.editor_views.insert(key, editor.clone());
        editor
    }

    fn editor_group_layout(&self, workspace: &str) -> &[Option<DocumentKey>] {
        self.editor_groups
            .get(workspace)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn editor_group_layout_mut(&mut self, workspace: &str) -> &mut Vec<Option<DocumentKey>> {
        self.editor_groups.entry(workspace.to_owned()).or_default()
    }

    fn assigned_editor_group_tabs(&self, workspace: &str) -> Vec<DocumentKey> {
        self.editor_group_layout(workspace)
            .iter()
            .flatten()
            .cloned()
            .collect()
    }

    /// Add a group after the active pane, targeting the next tab that is not
    /// already visible. The group may start empty when every tab is displayed.
    fn add_editor_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.document_workspace();
        if 1 + self.editor_group_layout(&workspace).len()
            >= crate::editor_groups::MAX_EDITOR_GROUPS
        {
            return;
        }
        let tabs = self
            .documents
            .borrow()
            .tabs(&workspace)
            .iter()
            .map(|tab| tab.key.clone())
            .collect::<Vec<_>>();
        let active = self.active_document_key();
        let assigned = self.assigned_editor_group_tabs(&workspace);
        let next = crate::editor_groups::next_group_document(active.as_ref(), &tabs, &assigned);
        if let Some(key) = next.clone() {
            let editor = self.editor_view(key, cx);
            window.focus(&editor.focus_handle(cx), cx);
        }
        self.editor_group_layout_mut(&workspace).push(next);
        cx.notify();
    }

    fn move_document_to_editor_group(
        &mut self,
        key: DocumentKey,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if key.workspace != self.document_workspace() {
            return;
        }
        let workspace = self.document_workspace();
        let tabs = self
            .documents
            .borrow()
            .tabs(&workspace)
            .iter()
            .map(|tab| tab.key.clone())
            .collect::<Vec<_>>();
        if !tabs.contains(&key) {
            return;
        }
        if index > 0 {
            let Some(target) = index.checked_sub(1) else {
                return;
            };
            let layout = self.editor_group_layout(&workspace);
            if target >= layout.len() || layout[target].as_ref() == Some(&key) {
                return;
            }
            if self.active_document_key().as_ref() == Some(&key) {
                let assigned = self.assigned_editor_group_tabs(&workspace);
                let Some(next) = tabs
                    .iter()
                    .find(|tab| **tab != key && !assigned.contains(*tab))
                    .or_else(|| tabs.iter().find(|tab| **tab != key))
                    .cloned()
                else {
                    return;
                };
                self.documents.borrow_mut().activate(&next);
            }
        }
        if !assign_editor_group_tab(self.editor_group_layout_mut(&workspace), &key, index) {
            return;
        }
        if index == 0 {
            self.documents.borrow_mut().activate(&key);
        }
        let editor = self.editor_view(key, cx);
        window.focus(&editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn close_editor_group(&mut self, index: usize, cx: &mut Context<Self>) {
        let workspace = self.document_workspace();
        let groups = self.editor_group_layout_mut(&workspace);
        if index < groups.len() {
            groups.remove(index);
            self.editor_group_drag = None;
            cx.notify();
        }
    }

    fn close_last_editor_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.document_workspace();
        if self
            .editor_group_layout_mut(&workspace)
            .pop()
            .is_some()
        {
            self.editor_group_drag = None;
            if let Some(key) = self.active_document_key() {
                let editor = self.editor_view(key, cx);
                window.focus(&editor.focus_handle(cx), cx);
            }
            cx.notify();
        }
    }

    /// Drop group documents that are no longer open and enforce the four-pane
    /// limit. Empty groups remain so group identity and layout stay stable.
    fn reconcile_editor_groups(&mut self) {
        let workspace = self.document_workspace();
        if self.editor_group_layout(&workspace).is_empty() {
            return;
        }
        let open = self
            .documents
            .borrow()
            .tabs(&workspace)
            .iter()
            .map(|tab| tab.key.clone())
            .collect::<Vec<_>>();
        let groups = self.editor_group_layout_mut(&workspace);
        for group in groups.iter_mut() {
            if let Some(key) = group
                && !open.iter().any(|open| open == key)
            {
                *group = None;
            }
        }
        groups.truncate(crate::editor_groups::MAX_EDITOR_GROUPS - 1);
        let empty = groups.is_empty();
        if self.editor_group_drag.is_some() && empty {
            self.editor_group_drag = None;
        }
    }

    fn sync_browser_input(&mut self, cx: &mut Context<Self>) {
        self.browser_editing = false;
        self.browser_input.update(cx, |input, cx| {
            input.set_text(self.browser_address.clone(), cx)
        });
        for field in [&self.browser_input, &self.composer_input] {
            let guard_host = self.browser.as_ref().map(Rc::downgrade);
            let focus_host = guard_host.clone();
            let undo_host = guard_host.clone();
            field.update(cx, |input, _| {
                input.key_guard = guard_host.map(|host| {
                    Rc::new(move || host.upgrade().is_none_or(|browser| !browser.has_focus()))
                        as Rc<dyn Fn() -> bool>
                });
                input.native_undo = undo_host.map(|host| {
                    Rc::new(move |redo| host.upgrade().is_some_and(|browser| browser.undo(redo)))
                        as Rc<dyn Fn(bool) -> bool>
                });
                input.on_focus = focus_host.map(|host| {
                    Rc::new(move || {
                        if let Some(browser) = host.upgrade() {
                            browser.focus_parent();
                        }
                    }) as Rc<dyn Fn()>
                });
            });
        }
    }

    fn browser_download_feedback(
        url: &str,
        path: Option<&Path>,
        success: bool,
    ) -> (String, bool) {
        let name = crate::browser_downloads::download_name(url, path);
        if name.is_empty() || name == "download" {
            return if success {
                ("Download complete".into(), false)
            } else {
                ("Download failed".into(), true)
            };
        }
        if success {
            (format!("Downloaded {name}"), false)
        } else {
            (format!("Download failed: {name}"), true)
        }
    }

    fn save_browser_downloads(&self) {
        let Some(directory) = self.engine.data_dir() else {
            return;
        };
        if let Err(error) = crate::browser_downloads::save(&directory, &self.browser_downloads) {
            tracing::warn!(%error, "could not persist browser download history");
        }
    }

    fn record_browser_download(
        &mut self,
        url: String,
        path: Option<PathBuf>,
        success: bool,
    ) {
        let name = crate::browser_downloads::download_name(&url, path.as_deref());
        crate::browser_downloads::push(
            &mut self.browser_downloads,
            name,
            url,
            path.as_deref(),
            success,
            crate::browser_downloads::now_ms(),
        );
        self.save_browser_downloads();
    }

    fn clear_browser_downloads(&mut self, cx: &mut Context<Self>) {
        self.browser_downloads.clear();
        self.browser_downloads_open = false;
        self.save_browser_downloads();
        cx.notify();
    }

    fn remove_browser_download(&mut self, id: u64, cx: &mut Context<Self>) {
        if !crate::browser_downloads::remove(&mut self.browser_downloads, id) {
            return;
        }
        if self.browser_downloads.is_empty() {
            self.browser_downloads_open = false;
        }
        self.save_browser_downloads();
        cx.notify();
    }

    fn reveal_browser_download(&mut self, path: String, cx: &mut Context<Self>) {
        if !crate::browser_downloads::reveal(&path) {
            self.toasts.update(cx, |toasts, cx| {
                toasts.push("Could not reveal the downloaded file.", true, cx)
            });
        }
    }

    fn poll_browser(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        let mut completed_downloads = Vec::new();
        for (index, tab) in self.browser_tabs.iter_mut().enumerate() {
            let Some(host) = tab.host.clone() else {
                continue;
            };
            let (back, forward) = (host.can_go_back(), host.can_go_forward());
            if (tab.can_go_back, tab.can_go_forward) != (back, forward) {
                tab.can_go_back = back;
                tab.can_go_forward = forward;
                changed = true;
            }
            for event in host.take_events() {
                match event {
                    BrowserEvent::Title(title) => tab.title = title,
                    BrowserEvent::Loading { url, loading } => {
                        if loading {
                            tab.begin_load(url);
                        } else {
                            tab.finish_load(url);
                        }
                    }
                    BrowserEvent::Failed(message) => tab.fail(message),
                    BrowserEvent::External(url) => cx.open_url(&url),
                    BrowserEvent::Downloaded {
                        url,
                        path,
                        success,
                    } => {
                        completed_downloads.push((url, path, success));
                    }
                }
                if index == self.browser_active {
                    self.browser_url = tab.url.clone();
                    self.browser_error = tab.error.clone();
                    if !self.browser_editing {
                        self.browser_address = tab.url.clone();
                        tab.address = tab.url.clone();
                    }
                }
                changed = true;
            }
            if tab.check_timeout(
                std::time::Instant::now(),
                std::time::Duration::from_secs(30),
            ) {
                if index == self.browser_active {
                    self.browser_error = tab.error.clone();
                }
                changed = true;
            }
            // Includes history and same-document navigations that don't load a document.
            if let Some(url) = host.url()
                && url != tab.url
                && vela_browser::allowed_url(&url)
            {
                tab.url = url.clone();
                if index == self.browser_active {
                    self.browser_url = url.clone();
                    if !self.browser_editing {
                        self.browser_address = url;
                    }
                }
                changed = true;
            }
        }
        for (url, path, success) in completed_downloads {
            let (message, error) =
                Self::browser_download_feedback(&url, path.as_deref(), success);
            self.record_browser_download(url, path, success);
            self.toasts
                .update(cx, |toasts, cx| toasts.push(message, error, cx));
        }
        if changed {
            if !self.browser_editing {
                self.browser_input.update(cx, |input, cx| {
                    input.set_text(self.browser_address.clone(), cx)
                });
            }
            cx.notify();
        }
    }

    fn sync_browser_tab(&mut self) {
        if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
            tab.url = self.browser_url.clone();
            tab.address = self.browser_address.clone();
        }
    }

    fn ensure_browser_host(
        &mut self,
        window: &mut Window,
        url: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.browser.is_some() {
            return true;
        }
        match BrowserHost::new(window, url, &self.browser_profile) {
            Ok(browser) => {
                browser.set_visible(false);
                let browser = Rc::new(browser);
                if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
                    tab.host = Some(browser.clone());
                }
                self.browser = Some(browser);
                self.sync_browser_input(cx);
                self.browser_error = None;
                true
            }
            Err(error) => {
                self.browser_error = Some(format!("Could not start the embedded browser: {error}"));
                false
            }
        }
    }

    fn activate_browser_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.browser_tabs.len() {
            return;
        }
        self.sync_browser_tab();
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        self.browser_active = index;
        let tab = self.browser_tabs[index].clone();
        self.browser_url = tab.url;
        self.browser_address = tab.address;
        self.browser = tab.host;
        self.sync_browser_input(cx);
        self.browser_open = true;
        self.diff_request = None;
        self.browser_error = tab.error.clone();
        if self.browser_url.is_empty() {
            if let Some(browser) = &self.browser {
                browser.set_visible(false);
            }
            window.focus(&self.browser_focus, cx);
        } else {
            let url = self.browser_url.clone();
            if self.ensure_browser_host(window, &url, cx)
                && let Some(browser) = &self.browser
            {
                if self.browser_error.is_none() {
                    browser.set_visible(true);
                }
                self.browser_focus_pending = true;
            }
        }
        cx.notify();
    }

    fn toggle_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.browser_open {
            self.browser_open = false;
            if let Some(browser) = &self.browser {
                browser.set_visible(false);
            }
            window.focus(&self.composer_focus, cx);
            cx.notify();
            return;
        }
        if self.browser_tabs.is_empty() {
            let url = std::env::var("VELA_BROWSER_URL").unwrap_or_default();
            let address = url.clone();
            self.browser_tabs.push(BrowserTab {
                url,
                address,
                ..Default::default()
            });
            self.browser_active = 0;
            self.browser_url = self.browser_tabs[0].url.clone();
            self.browser_address = self.browser_tabs[0].address.clone();
        }
        let index = self.browser_active;
        self.activate_browser_tab(index, window, cx);
    }

    fn new_browser_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        self.changes_open = false;
        self.sync_browser_tab();
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        self.browser = None;
        self.browser_tabs.push(BrowserTab::default());
        self.browser_active = self.browser_tabs.len() - 1;
        self.browser_url.clear();
        self.browser_address.clear();
        self.sync_browser_input(cx);
        self.browser_open = true;
        self.diff_request = None;
        self.browser_error = None;
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        window.focus(&self.browser_focus, cx);
        cx.notify();
    }

    fn close_browser_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.browser_tabs.len() {
            return;
        }
        self.sync_browser_tab();
        let previous_len = self.browser_tabs.len();
        let was_active = index == self.browser_active;
        if was_active && let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        self.browser_tabs.remove(index);
        if self.browser_tabs.is_empty() {
            self.browser_active = 0;
            self.browser_url.clear();
            self.browser_address.clear();
            self.close_browser(window, cx);
            self.browser = None;
            self.sync_browser_input(cx);
            return;
        }
        if let Some(next) =
            tabs::browser_active_after_close(previous_len, self.browser_active, index)
        {
            if was_active {
                // Skip sync_browser_tab: the old active index now names a
                // different tab after removal.
                self.browser_active = usize::MAX;
                self.activate_browser_tab(next, window, cx);
            } else {
                self.browser_active = next;
            }
        }
        cx.notify();
    }

    fn set_active_browser_error(&mut self, error: Option<String>) {
        if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
            tab.error = error.clone();
        }
        self.browser_error = error;
    }

    fn fail_active_browser(&mut self, message: String) {
        if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
            tab.fail(message.clone());
        }
        self.browser_error = Some(message);
    }

    fn navigate_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = vela_browser::normalize_address(&self.browser_address) else {
            if !self.browser_address.trim().is_empty() {
                self.fail_active_browser("Use a credential-free HTTP or HTTPS address.".into());
                cx.notify();
            }
            return;
        };
        if !self.ensure_browser_host(window, &url, cx) {
            let error = self
                .browser_error
                .clone()
                .unwrap_or_else(|| "Could not start the embedded browser.".into());
            self.fail_active_browser(error);
            cx.notify();
            return;
        }
        if let Some(browser) = &self.browser {
            if let Err(error) = browser.navigate(&url) {
                self.fail_active_browser(error.to_string());
                cx.notify();
                return;
            }
            browser.set_visible(true);
            browser.set_focus();
        }
        self.browser_url = url.clone();
        self.browser_address = url.clone();
        if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
            tab.begin_load(url);
            tab.address = self.browser_address.clone();
        }
        self.sync_browser_input(cx);
        self.browser_open = true;
        self.diff_request = None;
        self.set_active_browser_error(None);
        self.browser_focus_pending = false;
        self.sync_browser_tab();
        cx.notify();
    }

    fn on_browser_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform && keystroke.key == "l" {
            if let Some(browser) = &self.browser {
                browser.focus_parent();
            }
            self.browser_input.update(cx, |input, cx| {
                input.buffer.select_all();
                cx.notify();
            });
            window.focus(&self.browser_focus, cx);
            cx.stop_propagation();
            return;
        }
        if self
            .browser
            .as_ref()
            .is_some_and(|browser| browser.has_focus())
        {
            return;
        }
        match keystroke.key.as_str() {
            "enter" => self.navigate_browser(window, cx),
            "escape" => {
                self.browser_address = self.browser_url.clone();
                self.sync_browser_input(cx);
                if let Some(browser) = &self.browser {
                    browser.set_focus();
                } else {
                    window.focus(&self.composer_focus, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn close_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.browser_open = false;
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    fn render_browser_downloads(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut list = div()
            .id("browser-downloads-list")
            .role(Role::List)
            .aria_label("Recent downloads")
            .flex()
            .flex_col()
            .gap(px(2.));
        for (row, entry) in self.browser_downloads.iter().take(3).enumerate() {
            let (status, color, icon_name) = if entry.success {
                ("Downloaded", theme.green, "check")
            } else {
                ("Download failed", theme.red, "close")
            };
            let row_id = entry.id;
            let remove_id = entry.id;
            let remove_name = entry.name.clone();
            let remove_debug = format!("browser-download-remove-{row_id}");
            let reveal_path = entry
                .path
                .clone()
                .filter(|path| crate::browser_downloads::reveal_command(path).is_some());
            let reveal_name = entry.name.clone();
            let reveal_debug = format!("browser-download-reveal-{row_id}");
            let reveal_focus = self.browser_download_actions_focus[row * 2].clone();
            let remove_focus = self.browser_download_actions_focus[row * 2 + 1].clone();
            list = list.child(
                div()
                    .id(("browser-download", entry.id))
                    .debug_selector(move || format!("browser-download-{row_id}"))
                    .role(Role::ListItem)
                    .aria_label(format!("{status}: {}", entry.name))
                    .h(px(21.))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(glyph(icon_name, 11., color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(10.5))
                            .text_color(theme.text)
                            .child(SharedString::from(entry.name.clone())),
                    )
                    .when_some(reveal_path, |row, path| {
                        let show_key_path = path.clone();
                        let show_click_path = path.clone();
                        row.child(
                            div()
                                .id(SharedString::from(reveal_debug.clone()))
                                .debug_selector(move || reveal_debug.clone())
                                .role(Role::Button)
                                .aria_label(format!("Reveal {reveal_name} in file manager"))
                                .focusable()
                                .track_focus(&reveal_focus)
                                .tab_stop(true)
                                .px(px(4.))
                                .py(px(1.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .text_size(px(9.))
                                .text_color(theme.dim)
                                .hover(|style| style.bg(theme.elevated).text_color(theme.text))
                                .on_key_down(cx.listener(move |app, event: &KeyDownEvent, _, cx| {
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        app.reveal_browser_download(show_key_path.clone(), cx);
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |app, _, _, cx| {
                                    app.reveal_browser_download(show_click_path.clone(), cx);
                                }))
                                .child("Show"),
                        )
                    })
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(9.))
                            .text_color(theme.dim)
                            .child(status),
                    )
                    .child(
                        div()
                            .id(SharedString::from(remove_debug.clone()))
                            .debug_selector(move || remove_debug.clone())
                            .role(Role::Button)
                            .aria_label(format!("Remove download {remove_name}"))
                            .focusable()
                            .track_focus(&remove_focus)
                            .tab_stop(true)
                            .size(px(15.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_size(px(11.))
                            .text_color(theme.faint)
                            .hover(|style| style.bg(theme.elevated).text_color(theme.red))
                            .on_key_down(cx.listener(move |app, event: &KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    app.remove_browser_download(remove_id, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_click(cx.listener(move |app, _, _, cx| {
                                app.remove_browser_download(remove_id, cx);
                            }))
                            .child("×"),
                    ),
            );
        }
        if self.browser_downloads.len() > 3 {
            list = list.child(
                div()
                    .text_size(px(9.))
                    .text_color(theme.faint)
                    .child(SharedString::from(format!(
                        "+{} older downloads",
                        self.browser_downloads.len() - 3
                    ))),
            );
        }
        div()
            .id("browser-downloads")
            .debug_selector(|| "browser-downloads".into())
            .role(Role::Region)
            .aria_label("Browser downloads")
            .h(px(BROWSER_DOWNLOADS_HEIGHT))
            .flex_shrink_0()
            .w_full()
            .px(px(10.))
            .py(px(6.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.panel)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.dim)
                    .child("Downloads")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                div()
                                    .id("browser-downloads-clear")
                                    .debug_selector(|| "browser-downloads-clear".into())
                                    .role(Role::Button)
                                    .aria_label("Clear download history")
                                    .focusable()
                                    .track_focus(&self.browser_download_actions_focus[6])
                                    .tab_stop(true)
                                    .px(px(5.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .text_size(px(9.))
                                    .text_color(theme.faint)
                                    .hover(|style| style.bg(theme.elevated).text_color(theme.text))
                                    .on_key_down(cx.listener(|app, event: &KeyDownEvent, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                            app.clear_browser_downloads(cx);
                                            cx.stop_propagation();
                                        }
                                    }))
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.clear_browser_downloads(cx);
                                    }))
                                    .child("Clear"),
                            )
                            .child(
                                div()
                                    .id("browser-downloads-close")
                                    .debug_selector(|| "browser-downloads-close".into())
                                    .role(Role::Button)
                                    .aria_label("Close downloads")
                                    .focusable()
                                    .track_focus(&self.browser_download_actions_focus[7])
                                    .tab_stop(true)
                                    .size(px(16.))
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .text_size(px(12.))
                                    .text_color(theme.faint)
                                    .hover(|style| style.bg(theme.elevated).text_color(theme.text))
                                    .on_key_down(cx.listener(|app, event: &KeyDownEvent, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                            app.browser_downloads_open = false;
                                            cx.stop_propagation();
                                            cx.notify();
                                        }
                                    }))
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.browser_downloads_open = false;
                                        cx.notify();
                                    }))
                                    .child("×"),
                            ),
                    ),
            )
            .child(list)
            .into_any_element()
    }

    fn render_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        if self.browser_tabs.is_empty() {
            return div().into_any_element();
        }
        let active = self.browser_active;
        let (loading, error, label) = self
            .browser_tabs
            .get(active)
            .map(|tab| {
                (
                    tab.loading,
                    tab.error.clone(),
                    tabs::browser_tab_label(&tab.url, &tab.title),
                )
            })
            .unwrap_or_default();
        let body_state = tabs::browser_body_state(&self.browser_url, loading, error.as_deref());
        let has_page = !self.browser_url.is_empty();
        let downloads_visible =
            self.browser_downloads_open && !self.browser_downloads.is_empty();
        let downloads_height = if downloads_visible {
            BROWSER_DOWNLOADS_HEIGHT
        } else {
            0.0
        };
        if has_page && self.browser.is_none() {
            let url = self.browser_url.clone();
            self.ensure_browser_host(window, &url, cx);
        }
        if let Some(browser) = &self.browser {
            if has_page && body_state == tabs::BrowserBodyState::Ready {
                let window_size = window.bounds().size;
                let codex = self.is_codex();
                let left = if codex {
                    RAIL_WIDTH
                        + if self.sidebar_open {
                            SIDEBAR_WIDTH
                        } else {
                            0.0
                        }
                } else {
                    10.0 + RAIL_WIDTH
                        + 10.0
                        + if self.sidebar_open {
                            SIDEBAR_WIDTH + 10.0
                        } else {
                            0.0
                        }
                };
                let top =
                    TITLEBAR_HEIGHT + if codex { 36.0 } else { 46.0 } + downloads_height;
                let right = if self.agent_panel_open && self.active_id.is_some() {
                    if codex {
                        AGENT_WIDTH
                    } else {
                        AGENT_WIDTH + 20.0
                    }
                } else if codex {
                    0.0
                } else {
                    10.0
                };
                let bottom = if self.tray.open {
                    self.tray.height + 10.
                } else {
                    0.
                } + if codex { 0. } else { 10. };
                let width = (f32::from(window_size.width) - left - right).max(1.0);
                let height = (f32::from(window_size.height) - top - bottom).max(1.0);
                browser.set_bounds(left as f64, top as f64, width as f64, height as f64);
                browser.set_visible(true);
                if self.browser_focus_pending {
                    self.browser_focus_pending = false;
                    window.focus(&self.browser_focus, cx);
                }
            } else {
                browser.set_visible(false);
            }
        }

        let (back, forward, _) = self
            .browser_tabs
            .get(active)
            .map(|t| (t.can_go_back, t.can_go_forward, t.loading))
            .unwrap_or_default();
        let download_count = self.browser_downloads.len();
        let downloads_open = self.browser_downloads_open;
        let close_focus = self.browser_close_focus.clone();
        let downloads_focus = self.browser_downloads_focus.clone();
        let downloads_panel = downloads_visible.then(|| self.render_browser_downloads(cx));
        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(36.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(browser_button(
                        "‹",
                        "Back",
                        back,
                        self.browser_nav_focus[0].clone(),
                        {
                        let weak = cx.weak_entity();
                        move |cx: &mut App| {
                            let _ = weak.update(cx, |app: &mut VelaApp, _| {
                                if let Some(browser) = &app.browser {
                                    browser.go_back();
                                }
                            });
                        }
                        },
                    ))
                    .child(browser_button(
                        "›",
                        "Forward",
                        forward,
                        self.browser_nav_focus[1].clone(),
                        {
                        let weak = cx.weak_entity();
                        move |cx: &mut App| {
                            let _ = weak.update(cx, |app: &mut VelaApp, _| {
                                if let Some(browser) = &app.browser {
                                    browser.go_forward();
                                }
                            });
                        }
                    }))
                    .child(browser_button(
                        if loading { "×" } else { "↻" },
                        if loading { "Stop loading" } else { "Reload" },
                        has_page,
                        self.browser_nav_focus[2].clone(),
                        {
                            let weak = cx.weak_entity();
                            move |cx: &mut App| {
                                let _ = weak.update(cx, |app: &mut VelaApp, _| {
                                    if let Some(browser) = &app.browser {
                                        if app
                                            .browser_tabs
                                            .get(app.browser_active)
                                            .is_some_and(|tab| tab.loading)
                                        {
                                            browser.stop();
                                        } else {
                                            browser.reload();
                                        }
                                    }
                                });
                            }
                        },
                    ))
                    .child(
                        div()
                            .id("browser-address")
                            .flex_1()
                            .h(px(25.0))
                            .px(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.inset)
                            .border_1()
                            .border_color(theme.border)
                            .on_key_down(cx.listener(Self::on_browser_key))
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .text_color(theme.dim)
                            .child(self.browser_input.clone()),
                    )
                    .when(download_count > 0, |row| {
                        row.child(
                            div()
                                .id("browser-downloads-toggle")
                                .debug_selector(|| "browser-downloads-toggle".into())
                                .role(Role::Button)
                                .aria_label(format!("Downloads ({download_count})"))
                                .focusable()
                                .track_focus(&downloads_focus)
                                .tab_stop(true)
                                .h(px(25.0))
                                .px(px(6.0))
                                .flex()
                                .items_center()
                                .gap(px(3.0))
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(theme.border)
                                .bg(if downloads_open {
                                    theme.selected
                                } else {
                                    theme.inset
                                })
                                .text_color(if downloads_open {
                                    theme.text
                                } else {
                                    theme.dim
                                })
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.elevated))
                                .on_key_down(cx.listener(|app, event: &KeyDownEvent, _, cx| {
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        app.browser_downloads_open = !app.browser_downloads_open;
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(|app, _, _, cx| {
                                    app.browser_downloads_open = !app.browser_downloads_open;
                                    cx.notify();
                                }))
                                .child(glyph(
                                    "arrow-down",
                                    12.,
                                    if downloads_open {
                                        theme.text
                                    } else {
                                        theme.dim
                                    },
                                ))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .child(SharedString::from(download_count.to_string())),
                                ),
                        )
                    })
                    .child(
                        div()
                            .id("browser-close")
                            .role(Role::Button)
                            .aria_label("Close browser")
                            .focusable()
                            .track_focus(&close_focus)
                            .tab_stop(true)
                            .cursor_pointer()
                            .text_size(px(13.0))
                            .text_color(theme.faint)
                            .hover(|style| style.text_color(theme.red))
                            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.close_browser_tab(active, window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    window.focus(&this.browser_close_focus, cx);
                                    this.close_browser_tab(active, window, cx);
                                }),
                            )
                            .child("×"),
                    ),
            )
            .when_some(downloads_panel, |root, panel| root.child(panel))
            .child(match body_state {
                tabs::BrowserBodyState::Ready => div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .bg(theme.inset)
                    .into_any_element(),
                tabs::BrowserBodyState::Loading => div()
                    .id("browser-loading")
                    .role(Role::Status)
                    .aria_label(format!("Loading {label}…"))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .child({
                        use gpui::AnimationExt;
                        glyph("working-spinner", 18., theme.accent).with_animation(
                            "browser-loading-spinner",
                            gpui::Animation::new(Duration::from_millis(700)).repeat(),
                            |svg, progress| {
                                svg.with_transformation(gpui::Transformation::rotate(
                                    gpui::percentage(progress),
                                ))
                            },
                        )
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.dim)
                            .child(SharedString::from(format!("Loading {label}…"))),
                    )
                    .into_any_element(),
                tabs::BrowserBodyState::Error => div()
                    .id("browser-error")
                    .role(Role::Status)
                    .aria_label(
                        error
                            .clone()
                            .unwrap_or_else(|| "The page could not be loaded.".into()),
                    )
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .px(px(20.0))
                    .child(
                        div()
                            .text_size(px(38.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .child(icon("eb01")),
                    )
                    .child(
                        div()
                            .max_w(px(360.0))
                            .text_center()
                            .text_size(px(12.0))
                            .line_height(px(18.0))
                            .text_color(theme.red)
                            .child(SharedString::from(
                                error
                                    .clone()
                                    .unwrap_or_else(|| "The page could not be loaded.".into()),
                            )),
                    )
                    .child(
                        div()
                            .id("browser-retry")
                            .role(Role::Button)
                            .aria_label("Try again")
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.selected))
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.retry_browser(window, cx);
                            }))
                            .child("Try again"),
                    )
                    .into_any_element(),
                tabs::BrowserBodyState::Empty => div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_size(px(38.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .child(icon("eb01")),
                    )
                    .child(
                        div()
                            .max_w(px(260.0))
                            .text_center()
                            .text_size(px(12.0))
                            .line_height(px(18.0))
                            .text_color(theme.faint)
                            .child("Search or enter a URL to browse the web beside your files."),
                    )
                    .into_any_element(),
            })
            .into_any_element()
    }

    fn retry_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self
            .browser_tabs
            .get(self.browser_active)
            .map(|tab| tab.url.clone())
            .unwrap_or_default();
        self.set_active_browser_error(None);
        if let Some(browser) = &self.browser {
            if let Some(tab) = self.browser_tabs.get_mut(self.browser_active) {
                if url.is_empty() {
                    tab.error = None;
                    tab.loading = false;
                    tab.loading_started = None;
                } else {
                    tab.begin_load(url);
                    tab.address = tab.url.clone();
                }
            }
            browser.reload();
            self.browser_open = true;
            cx.notify();
            return;
        }
        self.navigate_browser(window, cx);
    }

    fn record_nav_sample(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((target, started)) = self.nav_pending.take() else {
            return;
        };
        if self.active_id.as_deref() == Some(target.as_str()) && !self.blocks.is_empty() {
            self.nav_samples
                .push(started.elapsed().as_secs_f64() * 1000.0);
            let weak = cx.weak_entity();
            window.on_next_frame(move |_, cx| {
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                let _ = weak.update(cx, |app, _| {
                    app.nav_frame_samples.push(elapsed_ms);
                });
            });
        } else {
            self.nav_pending = Some((target, started));
        }
    }

    fn write_nav_results(&mut self, path: &str) {
        let mut samples = self.nav_samples.clone();
        samples.sort_by(|left, right| left.total_cmp(right));
        let mut frames = self.nav_frame_samples.clone();
        frames.sort_by(|left, right| left.total_cmp(right));
        let percentile = |samples: &[f64], p: f64| -> f64 {
            if samples.is_empty() {
                return 0.0;
            }
            let index = ((samples.len() as f64 - 1.0) * p).round() as usize;
            samples[index]
        };
        let body = format!(
            "{{\"samples\":{},\"p50_ms\":{:.3},\"p95_ms\":{:.3},\"max_ms\":{:.3},\"frame_samples\":{},\"frame_p50_ms\":{:.3},\"frame_p95_ms\":{:.3},\"frame_max_ms\":{:.3},\"open_views\":{},\"warm\":{}}}",
            samples.len(),
            percentile(&samples, 0.50),
            percentile(&samples, 0.95),
            samples.last().copied().unwrap_or(0.0),
            frames.len(),
            percentile(&frames, 0.50),
            percentile(&frames, 0.95),
            frames.last().copied().unwrap_or(0.0),
            self.engine.open_views(),
            self.engine.warm_len(),
        );
        if let Err(error) = std::fs::write(path, &body) {
            tracing::warn!(%error, %path, "could not write navigation benchmark");
        } else {
            tracing::info!(%path, %body, "wrote navigation benchmark");
        }
    }

    fn refresh_changes(&mut self, cx: &mut Context<Self>) {
        self.ensure_changes_view(cx);
        self.changes = self
            .change_reviews
            .get(&self.document_workspace())
            .map(|review| review.read(cx).summary())
            .unwrap_or_default();
        cx.notify();
    }

    fn open_diff(&mut self, rel: String, cx: &mut Context<Self>) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        self.browser_open = false;
        if let Some(browser) = &self.browser {
            browser.set_visible(false);
        }
        self.git_request += 1;
        let request = self.git_request;
        self.diff_request = Some((
            request,
            DocumentKey::new(self.workspace_directory(), rel.clone()),
        ));
        self.engine.git_diff(request, id, rel);
        cx.notify();
    }

    fn close_diff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.diff = None;
        self.diff_request = None;
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    fn persist_preferences(&mut self, _cx: &mut Context<Self>) {
        self.engine.apply_settings(self.settings.clone());
    }

    fn set_shell(&mut self, shell: UiShell, cx: &mut Context<Self>) {
        tracing::info!(?shell, "ui shell selected");
        self.ui_shell = shell;
        self.settings.shell = match shell {
            UiShell::Codex => "codex",
            UiShell::Classic => "classic",
        }
        .into();
        self.persist_preferences(cx);
        cx.notify();
    }

    fn toggle_changes(&mut self, cx: &mut Context<Self>) {
        self.changes_open = !self.changes_open;
        if self.changes_open {
            self.settings_open = false;
            if self.browser_open {
                self.browser_open = false;
                if let Some(browser) = &self.browser {
                    browser.set_visible(false);
                }
            }
            self.refresh_changes(cx);
        }
        cx.notify();
    }

    fn reveal_change(&mut self, path: String, cx: &mut Context<Self>) {
        self.ensure_changes_view(cx);
        self.changes_open = true;
        if let Some(review) = self.change_reviews.get(&self.document_workspace()) {
            review.update(cx, |view, cx| view.reveal(path, cx));
        }
        cx.notify();
    }

    fn render_changes_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.ensure_changes_view(cx);
        if let Some(review) = self.change_reviews.get(&self.document_workspace()) {
            return div()
                .size_full()
                .min_h_0()
                .min_w_0()
                .child(review.clone())
                .into_any_element();
        }
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(Theme::default().faint)
            .child("Open a workspace to review changes.")
            .into_any_element()
    }

    fn choose_model(&mut self, mut model: ModelInfo, cx: &mut Context<Self>) {
        if let Some(selected) = &self.selected_model
            && selected.id == model.id
            && selected.provider_id == model.provider_id
        {
            model.variant = selected.variant.clone();
        }
        if let Some(id) = self.active_id.clone() {
            self.engine.switch_model(id, model);
            self.picker = None;
            self.picker_anchors.pending = Some(PickerKind::Model);
            cx.notify();
        }
    }

    fn settings_model_options(&self) -> (Vec<String>, usize) {
        let mut labels = vec![
            if self.models.is_empty() {
                "No models available"
            } else {
                "Select a model"
            }
            .to_owned(),
        ];
        labels.extend(
            self.models
                .iter()
                .map(|model| format!("{} · {}", model.name, model.provider_id)),
        );
        let selected = self
            .selected_model
            .as_ref()
            .and_then(|selected| {
                self.models.iter().position(|model| {
                    model.id == selected.id && model.provider_id == selected.provider_id
                })
            })
            .map_or(0, |index| index + 1);
        (labels, selected)
    }

    fn open_settings_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone().filter(|_| !self.models.is_empty()) else {
            return;
        };
        window.focus(&self.settings_model_focus, cx);
        #[cfg(target_os = "macos")]
        if let Some(bounds) = self.settings_model_bounds.get() {
            let (labels, selected) = self.settings_model_options();
            let models = self.models.clone();
            if let Some(menu) =
                crate::platform_menu::Popup::new(window, bounds, &labels, selected, 11.)
            {
                cx.spawn(async move |this, cx| {
                    if let Some(index) = menu.show()
                        && let Some(model) = index
                            .checked_sub(1)
                            .and_then(|index| models.get(index))
                            .cloned()
                    {
                        let _ = this.update(cx, |this, cx| {
                            if this.active_id.as_ref() == Some(&session)
                                && this.settings_open
                                && this.settings_section == SettingsSection::Model
                                && this.models.iter().any(|current| {
                                    current.id == model.id
                                        && current.provider_id == model.provider_id
                                })
                            {
                                this.choose_model(model, cx);
                            }
                        });
                    }
                })
                .detach();
                return;
            }
        }
        self.picker = Some(PickerKind::Model);
        cx.notify();
    }

    fn choose_agent(&mut self, agent: AgentInfo, cx: &mut Context<Self>) {
        if let Some(id) = self.active_id.clone() {
            self.engine.switch_agent(id, agent.id.clone());
            self.picker = None;
            self.picker_anchors.pending = Some(PickerKind::Agent);
            cx.notify();
        }
    }

    fn render_picker(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.picker {
            Some(PickerKind::Strength) => self.render_strength_picker(cx),
            Some(PickerKind::Agent) => self.render_agent_picker(cx),
            Some(PickerKind::Model) => self.render_model_picker(cx),
            None => div().into_any_element(),
        }
    }

    fn choose_workspace(&mut self, cx: &mut Context<Self>) {
        self.choose_workspace_dialog(false, cx);
    }

    fn mention_trigger(&self, cx: &App) -> Option<(usize, String)> {
        let caret = self.composer_input.read(cx).buffer.caret();
        let spans = self.attachment_mention_spans(&self.composer);
        attachments::mention_trigger_at(&self.composer, caret, &spans)
    }

    fn update_reference_search(&mut self, cx: &mut Context<Self>) {
        let Some((at, query)) = self.mention_trigger(cx) else {
            self.reference_files.clear();
            self.reference_pending = None;
            return;
        };
        let Some(session) = self.active_id.clone() else {
            return;
        };
        self.file_request += 1;
        let request = self.file_request;
        self.reference_pending = Some((request, session.clone(), at));
        self.reference_files.clear();
        self.reference_selected = 0;
        self.engine.find_references(request, session, query);
    }

    fn clear_slash_arguments(&mut self) {
        self.slash_arg_files.clear();
        self.slash_arg_pending = None;
        self.slash_arg_query.clear();
        self.slash_arg_selected = 0;
    }

    fn update_slash_arguments(&mut self, cx: &mut Context<Self>) {
        let caret = self.composer_input.read(cx).buffer.caret();
        let Some(trigger) = crate::slash_args::argument_at(&self.composer, caret) else {
            self.clear_slash_arguments();
            return;
        };
        if !vela_opencode::commands::contains_command(&self.command_catalog, &trigger.command) {
            self.clear_slash_arguments();
            return;
        }
        if self.slash_arg_query == trigger.query
            && (self.slash_arg_pending.is_some() || !self.slash_arg_files.is_empty())
        {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            self.clear_slash_arguments();
            return;
        };
        self.file_request += 1;
        let request = self.file_request;
        self.slash_arg_pending = Some(request);
        self.slash_arg_query = trigger.query.clone();
        self.slash_arg_files.clear();
        self.slash_arg_selected = 0;
        self.engine.find_references(request, session, trigger.query);
    }

    fn apply_slash_argument(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.slash_arg_files.get(index).cloned() else {
            return;
        };
        let caret = self.composer_input.read(cx).buffer.caret();
        let Some(trigger) = crate::slash_args::argument_at(&self.composer, caret) else {
            return;
        };
        let (next, cursor) =
            crate::slash_args::insert_argument(&self.composer, trigger.start, caret, &path);
        self.composer = next.clone();
        self.composer_input.update(cx, |input, cx| {
            input.set_text(next, cx);
            input.buffer.select(cursor, cursor);
        });
        self.clear_slash_arguments();
        if let Some(id) = self.active_id.clone() {
            self.engine.set_draft(id, self.composer.clone());
        }
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    fn apply_reference_selection(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, session, at)) = self.reference_pending.clone() else {
            return;
        };
        if self.active_id.as_deref() != Some(session.as_str()) {
            return;
        }
        let Some(rel) = self.reference_files.get(index).cloned() else {
            return;
        };
        let caret = self.composer_input.read(cx).buffer.caret();
        if at > caret || caret > self.composer.len() {
            return;
        }
        let token = format!("@{rel}");
        let mut next = String::with_capacity(self.composer.len() + token.len());
        next.push_str(&self.composer[..at]);
        next.push_str(&token);
        next.push_str(&self.composer[caret..]);
        let cursor = at + token.len();
        self.composer = next.clone();
        self.composer_input.update(cx, |input, cx| {
            input.set_text(next, cx);
            input.buffer.select(cursor, cursor);
        });
        let directory = self.workspace_directory();
        self.attach_reference(
            std::path::PathBuf::from(directory).join(&rel),
            &rel,
            vela_opencode::PromptMention {
                start: at,
                end: at + token.len(),
                text: token,
            },
        );
        self.reference_files.clear();
        self.reference_pending = None;
        self.reference_selected = 0;
        if let Some(id) = self.active_id.clone() {
            self.engine.set_draft(id, self.composer.clone());
        }
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    fn slash_items(&self) -> Vec<CommandOption> {
        if !self.runtime_capabilities.commands || !self.slash_menu.visible() {
            return Vec::new();
        }
        let Some(query) = self.composer.strip_prefix('/') else {
            return Vec::new();
        };
        if query.chars().any(char::is_whitespace) {
            return Vec::new();
        }
        let query = query.to_lowercase();
        self.command_catalog
            .iter()
            .filter(|item| {
                item.name.to_lowercase().contains(&query)
                    || item
                        .description
                        .as_deref()
                        .is_some_and(|description| description.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    fn apply_slash_selection(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.slash_items();
        let Some(item) = items.get(index.min(items.len().saturating_sub(1))) else {
            return;
        };
        self.composer = format!("/{}", item.name);
        let text = self.composer.clone();
        self.composer_input
            .update(cx, |input, cx| input.set_text(text, cx));
        self.send(window, cx);
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.active_id.clone() else {
            return;
        };
        let mut text = self.composer.trim().to_string();
        if text.is_empty() && self.attachments.is_empty() {
            return;
        }
        if text.is_empty() {
            text = "Review the attached files.".into();
        }
        if let Some((name, args)) = prompt_command(&text, self.runtime_capabilities.commands) {
            self.file_request += 1;
            let request = self.file_request;
            self.slash_requests.insert(request, id.clone());
            let args = crate::slash_args::unescape_arguments(&args);
            self.engine
                .run_slash_command(request, id.clone(), name, args);
            self.engine.set_draft(id.clone(), String::new());
            self.composer.clear();
            self.slash_menu.edited();
            self.composer_input
                .update(cx, |input, cx| input.set_text(String::new(), cx));
            self.clear_slash_arguments();
            self.attachments.clear();
            self.attachment_drag_over = false;
            self.reference_files.clear();
            self.reference_pending = None;
            self.reference_selected = 0;
            self.picker = None;
            self.stick = true;
            self.transcript_list.scroll_to_end();
            window.focus(&self.composer_focus, cx);
            cx.notify();
            return;
        }
        let files = self.take_prompt_files(&text);
        self.engine.prompt_with_behavior(
            id.clone(),
            text,
            files,
            self.phase
                .is_active()
                .then_some(self.settings.follow_up_behavior),
        );
        self.engine.set_draft(id.clone(), String::new());
        self.composer.clear();
        self.composer_input
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        self.reference_files.clear();
        self.reference_pending = None;
        self.reference_selected = 0;
        self.picker = None;
        self.stick = true;
        self.transcript_list.scroll_to_end();
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    fn on_composer_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.browser_open
            && self
                .browser
                .as_ref()
                .is_some_and(|browser| browser.has_focus())
        {
            return;
        }
        let keystroke = &event.keystroke;
        if self.composer_input.read(cx).buffer.marked.is_some() {
            return;
        }
        if !self.reference_files.is_empty()
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
        {
            match keystroke.key.as_str() {
                "up" => {
                    self.reference_selected = self
                        .reference_selected
                        .checked_sub(1)
                        .unwrap_or(self.reference_files.len() - 1);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "down" => {
                    self.reference_selected =
                        (self.reference_selected + 1) % self.reference_files.len();
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_reference_selection(self.reference_selected, window, cx);
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if keystroke.key == "escape"
            && (!self.slash_items().is_empty()
                || !self.slash_arg_files.is_empty()
                || self.slash_arg_pending.is_some())
        {
            self.slash_menu.dismiss();
            self.clear_slash_arguments();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if !self.slash_arg_files.is_empty()
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
        {
            match keystroke.key.as_str() {
                "up" => {
                    self.slash_arg_selected = self
                        .slash_arg_selected
                        .checked_sub(1)
                        .unwrap_or(self.slash_arg_files.len() - 1);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "down" => {
                    self.slash_arg_selected =
                        (self.slash_arg_selected + 1) % self.slash_arg_files.len();
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_slash_argument(self.slash_arg_selected, window, cx);
                    cx.stop_propagation();
                    return;
                }
                "escape" => {
                    self.clear_slash_arguments();
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }
        let slash_items = self.slash_items();
        if !slash_items.is_empty()
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
        {
            match keystroke.key.as_str() {
                "up" => {
                    self.slash_selected = self
                        .slash_selected
                        .checked_sub(1)
                        .unwrap_or(slash_items.len() - 1);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "down" => {
                    self.slash_selected = (self.slash_selected + 1) % slash_items.len();
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "enter" | "tab" if !keystroke.modifiers.shift => {
                    self.apply_slash_selection(self.slash_selected, window, cx);
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if keystroke.key == "tab"
            && keystroke.modifiers.control
            && !keystroke.modifiers.shift
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.alt
        {
            self.cycle_response_strength();
            cx.stop_propagation();
            return;
        }
        if keystroke.key.eq_ignore_ascii_case("p")
            && keystroke.modifiers.shift
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
        {
            self.cycle_favorite_model(cx);
            cx.stop_propagation();
            return;
        }
        if keystroke.key == "tab"
            && keystroke.modifiers.shift
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
            && !self.agents.is_empty()
        {
            let next = self
                .agents
                .iter()
                .position(|agent| Some(agent.id.as_str()) == self.selected_agent.as_deref())
                .map(|index| (index + 1) % self.agents.len())
                .unwrap_or(0);
            self.choose_agent(self.agents[next].clone(), cx);
            self.picker_anchors.pending = None;
            cx.stop_propagation();
            return;
        }
        let submit = keystroke.key == "enter" && !keystroke.modifiers.shift;
        if submit {
            self.send(window, cx);
            cx.stop_propagation();
        }
    }

    fn can_send(&self) -> bool {
        self.active_id.is_some()
            && (!self.composer.trim().is_empty() || !self.attachments.is_empty())
    }

    fn is_codex(&self) -> bool {
        self.ui_shell == UiShell::Codex
    }

    fn render_rail(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let weak = cx.weak_entity();
        let sessions_active =
            !self.settings_open && self.sidebar_open && self.sidebar_tab == SidebarTab::Sessions;
        let files_active =
            !self.settings_open && self.sidebar_open && self.sidebar_tab == SidebarTab::Files;
        let changes_active = self.changes_open;
        let settings_active = self.settings_open;
        let terminal_active = self.tray.open;
        let mut rail = div()
            .w(px(if self.is_codex() { RAIL_WIDTH } else { 44.2 }))
            .h_full()
            .flex_shrink_0()
            .bg(theme.rail)
            .flex()
            .flex_col()
            .items_center()
            .gap(if self.is_codex() { px(6.0) } else { px(10.0) });
        if self.is_codex() {
            rail = rail.py(px(14.0));
        } else {
            rail = rail
                .rounded(px(PANEL_RADIUS))
                .border_1()
                .border_color(theme.border)
                .px(px(5.0))
                .py(px(10.0));
        }
        rail.child(rail_tool(
            "eac7",
            sessions_active,
            theme,
            self.is_codex(),
            weak.clone(),
            |app, _, cx| app.activate_sidebar_tab(SidebarTab::Sessions, cx),
        ))
        .child(rail_tool(
            "eaf0",
            files_active,
            theme,
            self.is_codex(),
            weak.clone(),
            |app, _, cx| app.activate_sidebar_tab(SidebarTab::Files, cx),
        ))
        .child(rail_tool(
            "ea85",
            terminal_active,
            theme,
            self.is_codex(),
            weak.clone(),
            |app, window, cx| app.toggle_terminal(window, cx),
        ))
        .child(rail_tool(
            "eae1",
            changes_active,
            theme,
            self.is_codex(),
            weak.clone(),
            |app, _, cx| app.toggle_changes(cx),
        ))
        .child(rail_tool(
            "eb01",
            false,
            theme,
            self.is_codex(),
            weak.clone(),
            |app, window, cx| app.new_browser_tab(window, cx),
        ))
        .child(div().flex_1())
        .child(settings_rail_tool(
            settings_active,
            theme,
            self.is_codex(),
            weak,
            |app, _, cx| {
                if !app.settings_open {
                    app.settings_section = SettingsSection::Appearance;
                    app.settings_scroll = crate::scroll::AutoScrollHandle::default();
                }
                app.settings_open = !app.settings_open;
                if app.settings_open
                    && let Some(browser) = &app.browser
                {
                    browser.set_visible(false);
                }
                cx.notify();
            },
        ))
        .into_any_element()
    }

    fn render_codex_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let rail = self.render_rail(cx);
        let sidebar = self.render_sidebar(cx);
        let workspace = self.render_workspace(window, cx);
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .w_full()
            .h_full()
            .flex()
            .bg(theme.top)
            .child(rail)
            .child(codex_card(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .child(sidebar)
                    .child(workspace),
            ))
            .into_any_element()
    }

    fn render_classic_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let rail = self.render_rail(cx);
        let sidebar = self.render_sidebar(cx);
        let workspace = self.render_workspace(window, cx);
        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .h_full()
            .flex()
            .gap(px(10.0))
            .child(rail)
            .child(sidebar)
            .child(workspace)
            .into_any_element()
    }

    fn render_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let codex = self.is_codex();
        let surface = self.render_editor_surface(window, cx);
        let terminal = if self.tray.open {
            self.render_terminal(window, cx)
        } else {
            div().into_any_element()
        };
        let surface = if codex {
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .bg(theme.editor)
                .flex()
                .flex_col()
                .on_drop(cx.listener(|app, paths: &gpui::ExternalPaths, _, cx| {
                    app.open_launch_paths(paths.paths().to_vec(), cx);
                }))
                .child(surface)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .rounded(px(PANEL_RADIUS))
                .border_1()
                .border_color(theme.border)
                .bg(theme.editor)
                .overflow_hidden()
                .flex()
                .flex_col()
                .on_drop(cx.listener(|app, paths: &gpui::ExternalPaths, _, cx| {
                    app.open_launch_paths(paths.paths().to_vec(), cx);
                }))
                .child(RoundedSurface::new(surface, px(PANEL_RADIUS - 1.)))
                .into_any_element()
        };
        let editor_column = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .when(self.tray.open, |column| column.gap(px(10.0)))
            .child(surface)
            .child(terminal);
        let agent = if self.agent_panel_open {
            self.render_agent_panel(window, cx)
        } else {
            div().into_any_element()
        };
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .when(!codex, |row| row.gap(px(10.0)))
            .child(editor_column)
            .child(agent)
            .into_any_element()
    }

    fn render_editor_surface(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        if self.browser_open {
            return self.render_browser(window, cx);
        }
        if self.diff.is_some() {
            return self.render_diff(window, cx);
        }
        if self.changes_open {
            return self.render_changes_page(cx);
        }
        if self.active_document_key().is_some() {
            return self.render_editor(window, cx);
        }
        let message = if self.active_id.is_some() {
            "Select a file from the explorer to view or edit it."
        } else {
            "Open a workspace from the explorer to get started."
        };
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(16.))
            .bg(theme.editor)
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_center()
                    .text_size(px(14.0))
                    .line_height(px(17.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.dim)
                    .child(message),
            )
            .into_any_element()
    }

    fn toggle_usage_popup(&mut self, cx: &mut Context<Self>) {
        self.usage_open = !self.usage_open;
        if self.usage_open {
            self.load_provider_usage(cx);
        }
        cx.notify();
    }

    fn load_provider_usage(&mut self, cx: &mut Context<Self>) {
        if self.provider_usage_loading {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.provider_usage_request = Some(request);
        self.provider_usage_loading = true;
        self.engine.load_provider_usage(request);
        cx.notify();
    }

    fn render_usage_button(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let limit = self
            .selected_model
            .as_ref()
            .and_then(|model| model.context_limit)
            .filter(|limit| *limit > 0);
        let used = self
            .usage
            .as_ref()
            .map(|usage| usage.input_tokens)
            .unwrap_or(0);
        let percent = limit
            .map(|limit| ((used as f64 / limit as f64) * 100.).clamp(0., 100.))
            .unwrap_or(0.);
        let label = if limit.is_some() {
            format!("{}% context used", percent.round() as u64)
        } else {
            "Context and usage".into()
        };
        let anchor = cx.weak_entity();
        div()
            .id("agent-usage-toggle")
            .role(Role::Button)
            .aria_label(label)
            .aria_expanded(self.usage_open)
            .track_focus(&self.usage_focus)
            .absolute()
            .top(px(8.))
            .right(px(10.))
            .h(px(24.))
            .px(px(8.))
            .rounded(px(999.))
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(11.))
            .text_color(if self.usage_open {
                theme.text
            } else {
                theme.dim
            })
            .bg(if self.usage_open {
                theme.selected
            } else {
                theme::rgba(0, 0, 0, 0.)
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.usage_focus, cx);
                    this.toggle_usage_popup(cx);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                    this.toggle_usage_popup(cx);
                    window.focus(&this.usage_focus, cx);
                    cx.stop_propagation();
                } else if event.keystroke.key == "escape" {
                    this.usage_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                div()
                    .font_family("codicon")
                    .text_size(px(12.))
                    .child(icon("ea7c")),
            )
            .child(if limit.is_some() {
                format!("{}%", percent.round() as u64)
            } else {
                "Usage".into()
            })
            .child(
                gpui::canvas(
                    move |bounds, _, cx| {
                        let _ = anchor.update(cx, |app, _| app.usage_anchor = Some(bounds));
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
    }

    fn render_usage_popup(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let limit = self
            .selected_model
            .as_ref()
            .and_then(|model| model.context_limit)
            .filter(|limit| *limit > 0);
        let usage = self.usage;
        let used = usage.as_ref().map(|usage| usage.input_tokens).unwrap_or(0);
        let percent = limit
            .map(|limit| ((used as f64 / limit as f64) * 100.).clamp(0., 100.))
            .unwrap_or(0.);
        let mut content = div()
            .id("agent-usage-content")
            .max_h(px(400.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("Session usage"),
                    )
                    .child(
                        div()
                            .id("usage-close")
                            .role(Role::Button)
                            .aria_label("Close usage")
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(999.))
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .text_color(theme.dim)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.usage_open = false;
                                cx.notify();
                            }))
                            .child("Close"),
                    ),
            );
        if let Some(usage) = usage {
            content = content
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.dim)
                                .child("Total cost"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text)
                                .child(usage_cost(usage.cost)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(11.))
                                .text_color(theme.dim)
                                .child("Context window")
                                .child(if limit.is_some() {
                                    format!("{}%", percent.round() as u64)
                                } else {
                                    "Limit unavailable".into()
                                }),
                        )
                        .child(
                            div()
                                .h(px(4.))
                                .w_full()
                                .rounded(px(999.))
                                .bg(theme.inset)
                                .child(
                                    div()
                                        .h_full()
                                        .rounded(px(999.))
                                        .bg(if percent >= 85. {
                                            theme.red
                                        } else if percent >= 65. {
                                            theme::rgba(0xf0, 0xa6, 0x4b, 1.)
                                        } else {
                                            theme.accent
                                        })
                                        .w(gpui::relative((percent / 100.) as f32)),
                                ),
                        )
                        .when_some(limit, |section, limit| {
                            section.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(10.))
                                    .text_color(theme.faint)
                                    .child(usage_count(used))
                                    .child(format!("of {} tokens", usage_count(limit))),
                            )
                        }),
                );
            let rows = [
                ("Input", usage.input_tokens),
                ("Output", usage.output_tokens),
                ("Reasoning", usage.reasoning_tokens),
                ("Cache read", usage.cache_read_tokens),
                ("Cache write", usage.cache_write_tokens),
            ];
            for (label, count) in rows {
                content = content.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.))
                        .text_color(theme.dim)
                        .child(label)
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.text)
                                .child(usage_count(count)),
                        ),
                );
            }
            content = content.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .text_color(theme.dim)
                    .child("Cache")
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text)
                            .child(usage_cache_summary(&usage)),
                    ),
            );
            content = content.child(
                div()
                    .mt(px(2.))
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .text_color(theme.dim)
                    .child("Total tokens")
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(usage_count(usage.total_tokens())),
                    ),
            );
        } else {
            content = content.child(
                div()
                    .py(px(10.))
                    .text_size(px(11.))
                    .text_color(theme.faint)
                    .child("No usage recorded for this session yet."),
            );
        }
        let provider_loading = self.provider_usage_loading;
        let provider_refresh = div()
            .id("usage-provider-refresh")
            .debug_selector(|| "usage-provider-refresh".into())
            .role(Role::Button)
            .aria_label("Refresh provider usage")
            .focusable()
            .size(px(18.))
            .rounded(px(4.))
            .flex()
            .items_center()
            .justify_center()
            .when(!provider_loading, |button| {
                button.cursor_pointer().hover(|style| style.bg(theme.inset))
            })
            .opacity(if provider_loading { 0.45 } else { 1. })
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.provider_usage_loading {
                    this.load_provider_usage(cx);
                }
            }))
            .child(glyph(
                "refresh",
                12.,
                if provider_loading { theme.faint } else { theme.dim },
            ));
        content = content.child(crate::provider_usage::render_provider_usage_with_refresh(
            &self.provider_usage,
            self.provider_usage_loading,
            Some(provider_refresh.into_any_element()),
        ));
        div()
            .id("agent-usage-popup")
            .role(Role::Region)
            .aria_label("Session usage")
            .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                if !this
                    .usage_anchor
                    .is_some_and(|bounds| bounds.contains(&event.position))
                {
                    this.usage_open = false;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.usage_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(38.))
            .right(px(10.))
            .w(px(320.))
            .max_h(px(430.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(10.))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(6.)),
                blur_radius: px(18.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.24).into(),
            }])
            .child(content)
            .into_any_element()
    }

    fn render_panel_tabs(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let entries = self.panels.entries().to_vec();
        if self.panels.len() < 2 {
            return div().into_any_element();
        }
        let theme = Theme::default();
        let active = self.active_id.clone();
        let mut strip = div()
            .id("agent-panel-tabs")
            .role(Role::TabList)
            .aria_label("Open agent panels")
            .h(px(30.))
            .flex_shrink_0()
            .w_full()
            .px(px(6.))
            .flex()
            .items_center()
            .gap(px(3.))
            .overflow_x_scroll()
            .border_b_1()
            .border_color(theme.border);
        for panel in entries {
            let selected = active.as_deref() == Some(panel.session.as_str());
            let session = panel.session.clone();
            let close_session = panel.session.clone();
            let title = panel.title.clone();
            strip = strip.child(
                div()
                    .id(SharedString::from(format!(
                        "agent-panel-tab-{}",
                        panel.session
                    )))
                    .role(Role::Tab)
                    .aria_label(title.clone())
                    .aria_selected(selected)
                    .h(px(23.))
                    .max_w(px(180.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .pl(px(7.))
                    .pr(px(4.))
                    .rounded(px(6.))
                    .bg(if selected {
                        theme.selected
                    } else {
                        theme::rgba(0, 0, 0, 0.)
                    })
                    .text_size(px(10.5))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.active_id.as_deref() != Some(session.as_str()) {
                            app.open_session(session.clone(), cx);
                        }
                    }))
                    .child(div().min_w_0().truncate().child(SharedString::from(title)))
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "agent-panel-close-{}",
                                close_session
                            )))
                            .role(Role::Button)
                            .aria_label("Close agent panel")
                            .size(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset).text_color(theme.text))
                            .on_click(cx.listener(move |app, _, _, cx| {
                                cx.stop_propagation();
                                app.close_panel(close_session.clone(), cx);
                            }))
                            .child("×"),
                    ),
            );
        }
        strip.into_any_element()
    }

    /// Pointer capture for the tiled-panel splitters. Registered globally so a
    /// drag keeps resizing when the pointer leaves the narrow handle.
    fn panel_drag_observer(&self, cx: &mut Context<Self>) -> AnyElement {
        let movement = cx.weak_entity();
        let release = movement.clone();
        gpui::canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let movement = movement.clone();
                window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
                    if !phase.capture() {
                        return;
                    }
                    let viewport = f32::from(window.viewport_size().width);
                    let active = movement
                        .update(cx, |app, cx| {
                            let Some((boundary, last)) = app.panel_resize else {
                                return false;
                            };
                            if event.pressed_button != Some(MouseButton::Left) {
                                app.panel_resize = None;
                                cx.notify();
                                return false;
                            }
                            let total = (viewport * TILED_PANEL_FRACTION).max(AGENT_WIDTH);
                            let dx = f32::from(event.position.x) - last;
                            app.panels.resize_boundary(boundary, dx / total.max(1.));
                            app.panel_resize = Some((boundary, f32::from(event.position.x)));
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);
                    if active {
                        cx.stop_propagation();
                    }
                });
                let release = release.clone();
                window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, _, cx| {
                    if !phase.capture() || event.button != MouseButton::Left {
                        return;
                    }
                    let active = release
                        .update(cx, |app, cx| {
                            if app.panel_resize.take().is_some() {
                                cx.notify();
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if active {
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }

    fn render_panel_tile(
        &mut self,
        panel: &crate::panels::PanelState,
        active: bool,
        width: f32,
        resizable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let session = panel.session.clone();
        let title = panel.title.clone();
        let preview = crate::panels::preview_rows(&panel.blocks, crate::panels::PREVIEW_BLOCKS);
        let phase = panel.phase;
        let input = self.panel_inputs.get(&session).cloned();
        // Source renders each panel's own pending permissions, so an inactive
        // tile never hides a prompt behind the focused conversation.
        let panel_permissions = (!active && !panel.permissions.is_empty()).then(|| {
            self.render_permission_dock(
                &panel.permissions,
                &panel.session,
                &format!("panel-permission-{}", panel.session),
                cx,
            )
        });
        // The active tile keeps the complete rich transcript, forms dock and
        // composer. Other tiles render their retained snapshot with an
        // independent compact composer until they are activated.
        let rich = active.then(|| {
            (
                self.render_transcript(window, cx),
                self.render_composer(window, cx),
                self.forms.clone().into_any_element(),
            )
        });
        let model = panel
            .selected_model
            .as_ref()
            .map(|model| {
                if model.name.is_empty() {
                    model.id.clone()
                } else {
                    model.name.clone()
                }
            })
            .unwrap_or_else(|| "Model".into());

        let tile_back = self.session_parent(&session).map(|(parent, parent_title)| {
            self.render_session_back(
                parent,
                parent_title,
                format!("panel-session-back-{session}"),
                true,
                Some(session.clone()),
                cx,
            )
        });
        let activate = session.clone();
        let focus = session.clone();
        let mut header = div()
            .id(SharedString::from(format!("panel-tile-header-{session}")))
            .role(Role::Tab)
            .aria_label(title.clone())
            .aria_selected(active)
            .h(px(30.))
            .flex_shrink_0()
            .w_full()
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .when(active, |header| header.bg(theme.inset))
            .cursor_pointer()
            .on_click(cx.listener(move |app, _, window, cx| {
                app.activate_panel(activate.clone(), cx);
                app.focus_panel(&focus, window, cx);
                cx.notify();
            }))
            .when_some(tile_back, |header, back| header.child(back))
            .child(
                div()
                    .size(px(6.))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(if phase.is_active() {
                        theme.green
                    } else if !panel.permissions.is_empty() {
                        theme.yellow
                    } else {
                        theme.faint
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if active { theme.text } else { theme.dim })
                    .child(SharedString::from(title)),
            );
        let close_session = session.clone();
        header = header.child(
            div()
                .id(SharedString::from(format!("panel-tile-close-{session}")))
                .role(Role::Button)
                .aria_label("Close agent panel")
                .size(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_size(px(12.))
                .text_color(theme.faint)
                .cursor_pointer()
                .hover(|style| style.bg(theme.elevated).text_color(theme.text))
                .on_click(cx.listener(move |app, _, _, cx| {
                    cx.stop_propagation();
                    app.close_panel(close_session.clone(), cx);
                }))
                .child("×"),
        );

        let activate_preview = session.clone();
        let focus_preview = session.clone();
        let mut transcript = div()
            .id(SharedString::from(format!(
                "panel-tile-transcript-{session}"
            )))
            .role(Role::List)
            .aria_label(format!("{} transcript", panel.title))
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .bg(theme.editor)
            .px(px(10.))
            .py(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .cursor_pointer()
            .on_click(cx.listener(move |app, _, window, cx| {
                app.activate_panel(activate_preview.clone(), cx);
                app.focus_panel(&focus_preview, window, cx);
                cx.notify();
            }));
        if preview.is_empty() {
            transcript = transcript.child(
                div()
                    .text_size(px(11.))
                    .line_height(px(17.))
                    .text_color(theme.faint)
                    .child(if phase.is_active() {
                        "Working…"
                    } else {
                        "No messages yet."
                    }),
            );
        }
        let copied_message = panel.copied_message.clone();
        let mut index = 0;
        while index < preview.len() {
            let row = &preview[index];
            let source = panel.blocks.get(row.source_index);
            if matches!(
                row.kind,
                BlockKind::Reasoning | BlockKind::Activity | BlockKind::System
            ) {
                let start = index;
                let mut end = index + 1;
                while end < preview.len()
                    && matches!(
                        preview[end].kind,
                        BlockKind::Reasoning | BlockKind::Activity | BlockKind::System
                    )
                {
                    end += 1;
                }
                let key = source
                    .map(|block| format!("work:{}", block.id))
                    .unwrap_or_else(|| format!("work:{start}"));
                let running = phase.is_active() && end == preview.len();
                let work = self.render_tile_work_row(
                    &session,
                    &key,
                    start,
                    &preview[start..end],
                    running,
                    cx,
                );
                transcript = transcript.child(work);
                index = end;
                continue;
            }
            if let Some(block) = source
                .filter(|block| matches!(block.kind, BlockKind::Tool | BlockKind::Shell))
                .cloned()
            {
                let summary = div()
                    .id(SharedString::from(format!(
                        "panel-tile-row-{session}-{index}"
                    )))
                    .role(Role::ListItem)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .text_size(px(8.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.faint)
                            .child(row.label),
                    )
                    .child(self.render_tile_tool_row(&session, &block, cx));
                transcript = transcript.child(summary);
            } else {
                let actions = if row.kind == BlockKind::User {
                    source.cloned().map(|block| {
                        self.render_tile_user_actions(
                            &session,
                            &block,
                            copied_message.as_deref() == Some(block.id.as_ref()),
                            cx,
                        )
                    })
                } else if row.kind == BlockKind::Assistant {
                    source.cloned().map(|block| {
                        self.render_tile_answer_copy(
                            &session,
                            &block,
                            copied_message.as_deref() == Some(block.id.as_ref()),
                            cx,
                        )
                    })
                } else {
                    None
                };
                transcript = transcript.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-tile-row-{session}-{index}"
                        )))
                        .role(Role::ListItem)
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(px(8.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if row.user { theme.accent } else { theme.faint })
                                .child(row.label),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .line_height(px(17.))
                                .text_color(if row.user { theme.text } else { theme.dim })
                                .child(SharedString::from(row.text.clone())),
                        )
                        .when_some(actions, |row, actions| row.child(actions)),
                );
            }
            index += 1;
        }

        let send_session = session.clone();
        let mut composer = div()
            .w_full()
            .flex_shrink_0()
            .p(px(8.))
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap(px(3.))
            .on_key_down(cx.listener(move |app, event: &KeyDownEvent, window, cx| {
                if app.on_tile_composer_key(&send_session, event, window, cx) {
                    cx.stop_propagation();
                }
            }));
        if let Some(input) = input {
            let send = session.clone();
            let attach = session.clone();
            let title = panel.title.clone();
            let mut row = div().flex().items_end().gap(px(4.)).child(
                div()
                    .flex_1()
                    .min_w_0()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if active {
                        theme.border_strong
                    } else {
                        theme.border
                    })
                    .bg(theme.elevated)
                    .px(px(6.))
                    .py(px(4.))
                    .text_size(px(11.5))
                    .line_height(px(17.))
                    .text_color(theme.text)
                    .child(input),
            );
            if self.runtime_capabilities.attachments {
                row = row.child(
                    div()
                        .id(SharedString::from(format!("panel-tile-attach-{session}")))
                        .debug_selector({
                            let selector = format!("panel-tile-attach-{session}");
                            move || selector.clone()
                        })
                        .role(Role::Button)
                        .aria_label(SharedString::from(format!("Attach files to {title}")))
                        .size(px(26.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(7.))
                        .border_1()
                        .border_color(theme.border)
                        .text_size(px(14.))
                        .text_color(theme.dim)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.inset))
                        .font_family("codicon")
                        .on_click(cx.listener(move |app, _, window, cx| {
                            app.attach_tile_files(attach.clone(), window, cx);
                        }))
                        .child(icon("ea60")),
                );
            }
            row = row.child(
                div()
                    .id(SharedString::from(format!("panel-tile-send-{session}")))
                    .role(Role::Button)
                    .aria_label(format!("Send message to {}", panel.title))
                    .size(px(26.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(7.))
                    .bg(if active { theme.accent } else { theme.inset })
                    .text_size(px(12.))
                    .text_color(if active { theme.editor } else { theme.dim })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.accent))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        app.send_tile_message(send.clone(), window, cx);
                    }))
                    .child("↑"),
            );
            composer = composer.child(self.render_tile_composer_extras(&session, cx));
            composer = composer.child(row);
            composer = composer.child(
                div()
                    .text_size(px(9.))
                    .text_color(theme.faint)
                    .child(SharedString::from(format!("{model} · Enter sends"))),
            );
        }

        let tile_forms = (!panel.forms.is_empty()).then(|| {
            let dock = self.tile_form_dock(&session, cx);
            div()
                .id(SharedString::from(format!("panel-form-dock-{session}")))
                .debug_selector({
                    let selector = format!("panel-form-dock-{session}");
                    move || selector.clone()
                })
                .px(px(8.))
                .pb(px(4.))
                .child(dock)
                .into_any_element()
        });
        let tile_revert = self
            .staged_reverts
            .contains(&session)
            .then(|| self.render_tile_revert_dock(&session, cx));
        let activate_permission = session.clone();
        let body = match rich {
            Some((rich_transcript, rich_composer, forms)) => div()
                .flex_1()
                .min_h_0()
                .w_full()
                .flex()
                .flex_col()
                .child(rich_transcript)
                .child(forms)
                .child(rich_composer)
                .into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .w_full()
                .flex()
                .flex_col()
                .child(transcript)
                .when_some(panel_permissions, |tile, permissions| {
                    tile.child(
                        div()
                            .px(px(8.))
                            .pb(px(6.))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |app, _, _, cx| {
                                    app.activate_panel(activate_permission.clone(), cx);
                                    cx.notify();
                                }),
                            )
                            .child(permissions),
                    )
                })
                .when_some(tile_forms, |tile, forms| tile.child(forms))
                .when_some(tile_revert, |tile, revert| {
                    tile.child(div().px(px(8.)).pb(px(6.)).child(revert))
                })
                .child(composer)
                .into_any_element(),
        };
        let mut tile = div()
            .id(SharedString::from(format!("panel-tile-{session}")))
            .role(Role::TabPanel)
            .aria_label(panel.title.clone())
            .relative()
            .h_full()
            .min_w_0()
            .min_h_0()
            .flex_basis(gpui::relative(width.max(0.01)))
            .flex_shrink_1()
            .flex_grow_0()
            .overflow_hidden()
            .border_1()
            .border_color(if active {
                theme.border_strong
            } else {
                theme.border
            })
            .bg(theme.agent)
            .flex()
            .flex_col()
            .child(header)
            .child(body);
        if resizable {
            let boundary_session = session.clone();
            tile = tile.child(
                div()
                    .id(SharedString::from(format!("panel-tile-split-{session}")))
                    .role(Role::Group)
                    .aria_label("Resize agent panel")
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(px(6.))
                    .cursor(gpui::CursorStyle::ResizeColumn)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                            let Some(index) = app
                                .panels
                                .entries()
                                .iter()
                                .position(|panel| panel.session == boundary_session)
                            else {
                                return;
                            };
                            app.panel_resize = Some((index, f32::from(event.position.x)));
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            );
        }
        tile.into_any_element()
    }

    fn revert_tile_message(
        &mut self,
        session: String,
        message_id: String,
        edit: bool,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self.active_id.as_deref() != Some(session.as_str()) {
            self.open_session(session.clone(), cx);
        }
        if self.active_id.as_deref() == Some(session.as_str()) {
            self.revert_message(message_id, edit, text, cx);
        }
    }

    fn tile_form_dock(
        &mut self,
        session: &str,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<forms::FormDock> {
        if let Some(dock) = self.panel_form_docks.get(session) {
            return dock.clone();
        }
        let dock = cx.new(forms::FormDock::new);
        let owner = session.to_owned();
        cx.subscribe(&dock, move |app, _, event: &forms::Event, cx| match event {
            forms::Event::Submit {
                form_id,
                session_id,
                answers,
            } => {
                if app.active_id.as_deref() != Some(owner.as_str()) {
                    app.open_session(owner.clone(), cx);
                }
                app.file_request += 1;
                let request = app.file_request;
                app.form_requests
                    .insert(request, (owner.clone(), form_id.clone()));
                app.engine.reply_form(
                    request,
                    owner.clone(),
                    session_id.clone(),
                    form_id.clone(),
                    answers.clone(),
                );
                cx.notify();
            }
            forms::Event::Dismiss {
                form_id,
                session_id,
            } => {
                if app.active_id.as_deref() != Some(owner.as_str()) {
                    app.open_session(owner.clone(), cx);
                }
                app.file_request += 1;
                let request = app.file_request;
                app.form_requests
                    .insert(request, (owner.clone(), form_id.clone()));
                app.engine.cancel_form(
                    request,
                    owner.clone(),
                    session_id.clone(),
                    form_id.clone(),
                );
                cx.notify();
            }
            forms::Event::OpenExternal(url) => cx.open_url(url),
        })
        .detach();
        let forms = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .map(|panel| panel.forms.clone())
            .unwrap_or_else(|| self.engine.forms(session));
        if !forms.is_empty() {
            dock.update(cx, |dock, cx| dock.set_forms(forms, cx));
        }
        self.panel_form_docks
            .insert(session.to_owned(), dock.clone());
        dock
    }

    fn commit_tile_revert(&mut self, session: String, cx: &mut Context<Self>) {
        if !self.staged_reverts.contains(&session) || self.revert_in_flight(&session) {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.commit_revert_requests
            .insert(request, session.clone());
        self.engine.commit_revert(request, session);
        cx.notify();
    }

    fn discard_tile_revert(&mut self, session: String, cx: &mut Context<Self>) {
        if !self.staged_reverts.contains(&session) || self.revert_in_flight(&session) {
            return;
        }
        self.file_request += 1;
        let request = self.file_request;
        self.clear_revert_requests
            .insert(request, session.clone());
        self.engine.clear_revert(request, session);
        cx.notify();
    }

    fn render_tile_answer_copy(
        &mut self,
        session: &str,
        block: &Block,
        copied: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let copy_text = block.text.to_string();
        let copy_session = session.to_owned();
        let copy_key = block.id.to_string();
        let copy_id = format!("panel-answer-copy-{session}-{}", block.id);
        let copy_selector = copy_id.clone();
        div()
            .id(SharedString::from(copy_id))
            .debug_selector(move || copy_selector.clone())
            .role(Role::Button)
            .aria_label("Copy answer")
            .px(px(4.))
            .py(px(1.))
            .rounded(px(4.))
            .text_size(px(9.5))
            .text_color(theme.faint)
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                if copy_text.is_empty() {
                    return;
                }
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_text.clone()));
                app.panels.update(&copy_session, |panel| {
                    panel.copied_message = Some(copy_key.clone());
                });
                let reset_session = copy_session.clone();
                let reset_key = copy_key.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(2))
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        app.panels.update(&reset_session, |panel| {
                            if panel.copied_message.as_deref() == Some(reset_key.as_str()) {
                                panel.copied_message = None;
                            }
                        });
                        cx.notify();
                    });
                })
                .detach();
                cx.notify();
            }))
            .child(if copied { "Copied" } else { "Copy" })
            .into_any_element()
    }

    fn render_tile_revert_dock(
        &mut self,
        session: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let label = |id: String, text: &'static str, danger: bool| {
            let selector = id.clone();
            div()
                .id(SharedString::from(id))
                .debug_selector(move || selector.clone())
                .role(Role::Button)
                .aria_label(text)
                .focusable()
                .tab_stop(true)
                .h(px(24.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(if danger {
                    theme.red.opacity(0.16)
                } else {
                    theme.elevated
                })
                .text_color(if danger { theme.red } else { theme.text })
                .text_size(px(10.))
                .cursor_pointer()
                .flex()
                .items_center()
                .child(text)
        };
        let busy = self.revert_in_flight(session);
        let commit_session = session.to_owned();
        let mut commit = label(
            format!("panel-revert-commit-{session}"),
            "Commit undo",
            true,
        );
        let discard_session = session.to_owned();
        let mut discard = label(
            format!("panel-revert-discard-{session}"),
            "Discard",
            false,
        );
        if busy {
            commit = commit.opacity(0.45);
            discard = discard.opacity(0.45);
        } else {
            commit = commit.on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.commit_tile_revert(commit_session.clone(), cx);
            }));
            discard = discard.on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.discard_tile_revert(discard_session.clone(), cx);
            }));
        }
        div()
            .id(SharedString::from(format!("panel-revert-dock-{session}")))
            .debug_selector({
                let selector = format!("panel-revert-dock-{session}");
                move || selector.clone()
            })
            .role(Role::Region)
            .aria_label("Undo staged")
            .w_full()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.red.opacity(0.55))
            .bg(theme.inset)
            .p(px(7.))
            .flex()
            .flex_col()
            .gap(px(5.))
            .child(
                div()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.red)
                    .child("Undo staged"),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .line_height(px(14.))
                    .text_color(theme.dim)
                    .whitespace_normal()
                    .child("Messages and file changes after this point will be removed."),
            )
            .child(div().flex().gap(px(5.)).child(commit).child(discard))
            .into_any_element()
    }

    fn render_tile_user_actions(
        &mut self,
        session: &str,
        block: &Block,
        copied: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let copy_text = block.text.to_string();
        let copy_session = session.to_owned();
        let copy_key = block.id.to_string();
        let copy_id = format!("panel-message-copy-{session}-{}", block.id);
        let copy_selector = copy_id.clone();
        let copy = div()
            .id(SharedString::from(copy_id))
            .debug_selector(move || copy_selector.clone())
            .role(Role::Button)
            .aria_label("Copy message")
            .px(px(4.))
            .py(px(1.))
            .rounded(px(4.))
            .text_size(px(9.5))
            .text_color(theme.faint)
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                if copy_text.is_empty() {
                    return;
                }
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_text.clone()));
                app.panels.update(&copy_session, |panel| {
                    panel.copied_message = Some(copy_key.clone());
                });
                let reset_session = copy_session.clone();
                let reset_key = copy_key.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(2))
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        app.panels.update(&reset_session, |panel| {
                            if panel.copied_message.as_deref() == Some(reset_key.as_str()) {
                                panel.copied_message = None;
                            }
                        });
                        cx.notify();
                    });
                })
                .detach();
                cx.notify();
            }))
            .child(if copied { "Copied" } else { "Copy" });
        let undo_session = session.to_owned();
        let undo_message = block.message_id.to_string();
        let undo_id = format!("panel-message-undo-{session}-{}", block.id);
        let undo_selector = undo_id.clone();
        let undo = div()
            .id(SharedString::from(undo_id))
            .debug_selector(move || undo_selector.clone())
            .role(Role::Button)
            .aria_label("Undo message")
            .px(px(4.))
            .py(px(1.))
            .rounded(px(4.))
            .text_size(px(9.5))
            .text_color(theme.faint)
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.revert_tile_message(
                    undo_session.clone(),
                    undo_message.clone(),
                    false,
                    String::new(),
                    cx,
                );
            }))
            .child("Undo");
        let edit_session = session.to_owned();
        let edit_message = block.message_id.to_string();
        let edit_text = block.text.to_string();
        let edit_id = format!("panel-message-edit-{session}-{}", block.id);
        let edit_selector = edit_id.clone();
        let edit = div()
            .id(SharedString::from(edit_id))
            .debug_selector(move || edit_selector.clone())
            .role(Role::Button)
            .aria_label("Edit message")
            .px(px(4.))
            .py(px(1.))
            .rounded(px(4.))
            .text_size(px(9.5))
            .text_color(theme.faint)
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.revert_tile_message(
                    edit_session.clone(),
                    edit_message.clone(),
                    true,
                    edit_text.clone(),
                    cx,
                );
            }))
            .child("Edit");
        div()
            .flex()
            .items_center()
            .gap(px(2.))
            .child(copy)
            .child(undo)
            .child(edit)
            .into_any_element()
    }

    fn render_tile_work_row(
        &mut self,
        session: &str,
        key: &str,
        first_index: usize,
        rows: &[crate::panels::PanelPreviewRow],
        running: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let open = running
            || self
                .panels
                .entries()
                .iter()
                .find(|panel| panel.session == session)
                .is_some_and(|panel| panel.work_open.contains(key));
        let toggle_session = session.to_owned();
        let toggle_key = key.to_owned();
        let head_id = format!("panel-work-{session}-{first_index}");
        let head_selector = head_id.clone();
        let head = div()
            .id(SharedString::from(head_id))
            .debug_selector(move || head_selector.clone())
            .role(Role::Button)
            .aria_label(if running {
                "Working activity"
            } else {
                "Work activity"
            })
            .aria_expanded(open)
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(5.))
            .cursor_pointer()
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.panels.update(&toggle_session, |panel| {
                    if !panel.work_open.remove(&toggle_key) {
                        panel.work_open.insert(toggle_key.clone());
                    }
                });
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.faint)
                    .child(SharedString::from(format!(
                        "{} · {} steps",
                        if running { "Working" } else { "Work" },
                        rows.len()
                    ))),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(9.5))
                    .text_color(theme.faint)
                    .child(if open { "▾" } else { "▸" }),
            );
        let mut container = div()
            .id(SharedString::from(format!(
                "panel-work-group-{session}-{first_index}"
            )))
            .role(Role::ListItem)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(head);
        if open {
            let body_id = format!("panel-work-body-{session}-{first_index}");
            let body_selector = body_id.clone();
            let mut body = div()
                .id(SharedString::from(body_id))
                .debug_selector(move || body_selector.clone())
                .role(Role::Group)
                .aria_label("Expanded work activity")
                .w_full()
                .min_w_0()
                .pl(px(8.))
                .flex()
                .flex_col()
                .gap(px(3.))
                .border_l_1()
                .border_color(theme.border_soft);
            for (offset, row) in rows.iter().enumerate() {
                let row_selector =
                    format!("panel-work-row-{session}-{first_index}-{offset}");
                body = body.child(
                    div()
                        .id(SharedString::from(format!(
                            "panel-work-row-{session}-{first_index}-{offset}"
                        )))
                        .debug_selector(move || row_selector.clone())
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(px(8.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.faint)
                                .child(row.label),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .line_height(px(15.))
                                .text_color(theme.dim)
                                .child(SharedString::from(row.text.clone())),
                        ),
                );
            }
            container = container.child(body);
        }
        container.into_any_element()
    }

    fn render_tile_tool_files(
        &self,
        session: &str,
        namespace: &str,
        files: &[ToolFile],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let mut rows = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .pl(px(10.))
            .pt(px(2.));
        for (index, file) in files.iter().enumerate() {
            let label = file.name.clone().unwrap_or_else(|| file.uri.clone());
            let url = vela_browser::safe_external_url(&file.uri);
            let debug = format!("panel-tool-file-{session}-{namespace}-{index}");
            let selector = debug.clone();
            let mut row = div()
                .id(SharedString::from(debug))
                .debug_selector(move || selector.clone())
                .role(Role::Link)
                .aria_label(SharedString::from(format!("Open tool file {label}")))
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(10.))
                .text_color(if url.is_some() {
                    theme.accent
                } else {
                    theme.dim
                })
                .child(crate::tree_icons::glyph(&label))
                .child(div().min_w_0().truncate().child(SharedString::from(label)));
            if let Some(url) = url {
                row = row.cursor_pointer().on_click(cx.listener(move |_, _, _, cx| {
                    cx.open_url(&url);
                }));
            }
            rows = rows.child(row);
        }
        rows.into_any_element()
    }

    fn render_tile_tool_subcall(
        &mut self,
        session: &str,
        block_id: &str,
        path: &str,
        call: &ToolSubCall,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let key = format!("{block_id}::{path}");
        let open = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .is_some_and(|panel| panel.tool_open.contains(&key));
        let expandable = !call.input.trim().is_empty()
            || !call.output.trim().is_empty()
            || !call.files.is_empty()
            || !call.sub_calls.is_empty();
        let toggle_session = session.to_owned();
        let toggle_key = key.clone();
        let (status, color) = match call.status {
            ToolStatus::Success => ("Done", theme.green),
            ToolStatus::Failed => ("Failed", theme.red),
            ToolStatus::Running | ToolStatus::Streaming => ("Running", theme.yellow),
            ToolStatus::None => ("Queued", theme.faint),
        };
        let head_id = format!("panel-subcall-head-{session}-{block_id}-{path}");
        let head_selector = head_id.clone();
        let mut head = div()
            .id(SharedString::from(head_id))
            .debug_selector(move || head_selector.clone())
            .role(Role::Button)
            .aria_label(SharedString::from(format!("{status}: {}", call.title)))
            .aria_expanded(open)
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(4.))
            .pl(px(depth as f32 * 6.))
            .when(expandable, |head| head.cursor_pointer())
            .on_click(cx.listener(move |app, _, _, cx| {
                if !expandable {
                    return;
                }
                cx.stop_propagation();
                app.panels.update(&toggle_session, |panel| {
                    if !panel.tool_open.remove(&toggle_key) {
                        panel.tool_open.insert(toggle_key.clone());
                    }
                });
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(10.5))
                    .text_color(theme.dim)
                    .child(SharedString::from(call.title.clone())),
            );
        if !call.detail.trim().is_empty() {
            head = head.child(
                div()
                    .min_w_0()
                    .max_w(px(120.))
                    .truncate()
                    .text_size(px(9.5))
                    .text_color(theme.faint)
                    .child(SharedString::from(call.detail.clone())),
            );
        }
        head = head.child(div().flex_shrink_0().text_size(px(9.)).text_color(color).child(status));
        if expandable {
            head = head.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(9.))
                    .text_color(theme.faint)
                    .child(if open { "▾" } else { "▸" }),
            );
        }
        let mut container = div()
            .id(SharedString::from(format!(
                "panel-subcall-{session}-{block_id}-{path}"
            )))
            .role(Role::ListItem)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(1.))
            .py(px(1.))
            .child(head);
        if open && expandable {
            let body_id = format!("panel-subcall-body-{session}-{block_id}-{path}");
            let body_selector = body_id.clone();
            let mut body = div()
                .id(SharedString::from(body_id))
                .debug_selector(move || body_selector.clone())
                .role(Role::Region)
                .aria_label("Nested tool output")
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .pl(px(10. + depth as f32 * 6.))
                .pr(px(2.))
                .py(px(2.))
                .border_t_1()
                .border_color(theme.border_soft);
            for (label, text) in [("IN", &call.input), ("OUT", &call.output)] {
                if text.trim().is_empty() {
                    continue;
                }
                body = body.child(
                    div()
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.faint)
                        .child(label),
                );
                body = body.child(
                    div()
                        .text_size(px(10.))
                        .line_height(px(15.))
                        .text_color(theme.dim)
                        .whitespace_normal()
                        .child(SharedString::from(tool_view::display_output(text))),
                );
            }
            if !call.files.is_empty() {
                body = body.child(self.render_tile_tool_files(
                    session, &key, &call.files, cx,
                ));
            }
            if !call.sub_calls.is_empty() {
                body = body.child(
                    div()
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.faint)
                        .child(SharedString::from(format!(
                            "TOOL CALLS · {}",
                            call.sub_calls.len()
                        ))),
                );
                for (index, nested) in call.sub_calls.iter().enumerate() {
                    let nested_path = format!("{path}.{index}");
                    body = body.child(self.render_tile_tool_subcall(
                        session,
                        block_id,
                        &nested_path,
                        nested,
                        depth + 1,
                        cx,
                    ));
                }
            }
            container = container.child(body);
        }
        container.into_any_element()
    }

    /// Inactive tiles keep a compact transcript, but their tool rows still
    /// expose Source's expand and copy actions. State is per panel so opening
    /// one tile's payload never changes another tile.
    fn render_tile_tool_row(
        &mut self,
        session: &str,
        block: &Block,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let model = tool_view::presentation(block);
        let key = block.id.to_string();
        let open = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .is_some_and(|panel| panel.tool_open.contains(&key));
        let copied = self
            .panels
            .entries()
            .iter()
            .find(|panel| panel.session == session)
            .is_some_and(|panel| panel.copied_tool.as_deref() == Some(key.as_str()));
        let summary = model.summary.clone();
        let expandable = model.expandable();
        let toggle_session = session.to_owned();
        let toggle_key = key.clone();
        let head_id = format!("panel-tool-head-{session}-{key}");
        let head_selector = head_id.clone();
        let mut head = div()
            .id(SharedString::from(head_id))
            .debug_selector(move || head_selector.clone())
            .role(Role::Button)
            .aria_label(summary.clone())
            .aria_expanded(open)
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(4.))
            .when(expandable, |head| head.cursor_pointer())
            .on_click(cx.listener(move |app, _, _, cx| {
                if !expandable {
                    return;
                }
                cx.stop_propagation();
                app.panels.update(&toggle_session, |panel| {
                    if !panel.tool_open.remove(&toggle_key) {
                        panel.tool_open.insert(toggle_key.clone());
                    }
                });
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(theme.dim)
                    .child(SharedString::from(summary)),
            );
        if expandable {
            head = head.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(10.))
                    .text_color(theme.faint)
                    .child(if open { "▾" } else { "▸" }),
            );
        }
        let copy_value = tool_view::copy_text(block);
        let copy_id = format!("panel-tool-copy-{session}-{key}");
        let copy_selector = copy_id.clone();
        let copy_session = session.to_owned();
        let copy_key = key.clone();
        let copy_label = if copied { "Copied" } else { "Copy" };
        let mut row = div()
            .id(SharedString::from(format!("panel-tool-{session}-{key}")))
            .role(Role::ListItem)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(1.));
        let mut head_row = div()
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(head);
        if expandable && !copy_value.is_empty() {
            head_row = head_row.child(
                div()
                    .id(SharedString::from(copy_id))
                    .debug_selector(move || copy_selector.clone())
                    .role(Role::Button)
                    .aria_label("Copy tool output")
                    .flex_shrink_0()
                    .px(px(4.))
                    .py(px(1.))
                    .rounded(px(4.))
                    .text_size(px(9.5))
                    .text_color(theme.faint)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if copy_value.is_empty() {
                            return;
                        }
                        cx.stop_propagation();
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                            copy_value.clone(),
                        ));
                        app.panels.update(&copy_session, |panel| {
                            panel.copied_tool = Some(copy_key.clone());
                        });
                        let reset_session = copy_session.clone();
                        let reset_key = copy_key.clone();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor()
                                .timer(Duration::from_secs(2))
                                .await;
                            let _ = this.update(cx, |app, cx| {
                                app.panels.update(&reset_session, |panel| {
                                    if panel.copied_tool.as_deref()
                                        == Some(reset_key.as_str())
                                    {
                                        panel.copied_tool = None;
                                    }
                                });
                                cx.notify();
                            });
                        })
                        .detach();
                        cx.notify();
                    }))
                    .child(copy_label),
            );
        }
        row = row.child(head_row);
        if open && expandable {
            let output = block.output.to_string();
            let input = block.text.to_string();
            let body_selector = format!("panel-tool-output-{session}-{key}");
            let mut body = div()
                .id(SharedString::from(format!(
                    "panel-tool-output-{session}-{key}"
                )))
                .debug_selector(move || body_selector.clone())
                .role(Role::Region)
                .aria_label("Tool output")
                .w_full()
                .min_w_0()
                .pl(px(10.))
                .pr(px(4.))
                .py(px(3.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .border_t_1()
                .border_color(theme.border_soft);
            let show_input = matches!(model.kind, tool_view::Kind::Generic)
                && !input.trim().is_empty();
            if show_input {
                body = body
                    .child(
                        div()
                            .text_size(px(9.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.faint)
                            .child("IN"),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .line_height(px(16.))
                            .text_color(theme.dim)
                            .whitespace_normal()
                            .child(SharedString::from(tool_view::display_output(
                                &input,
                            ))),
                    );
            }
            if !output.trim().is_empty() {
                body = body
                    .child(
                        div()
                            .text_size(px(9.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.faint)
                            .child("OUT"),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .line_height(px(16.))
                            .text_color(theme.dim)
                            .whitespace_normal()
                            .child(SharedString::from(tool_view::display_output(
                                &output,
                            ))),
                    );
            }
            if !block.files.is_empty() {
                body = body.child(self.render_tile_tool_files(
                    session, &key, &block.files, cx,
                ));
            }
            if !block.sub_calls.is_empty() {
                body = body.child(
                    div()
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.faint)
                        .child(SharedString::from(format!(
                            "TOOL CALLS · {}",
                            block.sub_calls.len()
                        ))),
                );
                for (index, call) in block.sub_calls.iter().enumerate() {
                    body = body.child(self.render_tile_tool_subcall(
                        session,
                        &key,
                        &format!("sub{index}"),
                        call,
                        0,
                        cx,
                    ));
                }
            }
            let (status, color) = match block.status {
                ToolStatus::Success => ("Success", theme.green),
                ToolStatus::Failed => ("Failed", theme.red),
                ToolStatus::Running | ToolStatus::Streaming => ("Running", theme.yellow),
                ToolStatus::None => ("Queued", theme.faint),
            };
            body = body.child(div().text_size(px(9.)).text_color(color).child(status));
            row = row.child(body);
        }
        row.into_any_element()
    }

    fn render_tiled_panels(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let rows = self.panels.rows();
        let entries = self.panels.entries().to_vec();
        let active = self.active_id.clone();
        let total_width =
            (f32::from(window.viewport_size().width) * TILED_PANEL_FRACTION).max(AGENT_WIDTH);
        let theme_border = theme.border;
        let mut column = div()
            .id("tiled-agent-panels")
            .role(Role::Group)
            .aria_label("Simultaneous agent panels")
            .relative()
            .w(px(total_width))
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .bg(theme.agent)
            .flex()
            .flex_col()
            .child(self.render_panel_tabs(cx));
        for row in rows {
            let last_index = row.last().map(|(index, _)| *index);
            let height = row
                .first()
                .and_then(|(index, _)| crate::panel_layout::panel_slot(entries.len(), *index))
                .map(|slot| slot.height / 100.)
                .unwrap_or(1.);
            let mut row_div = div()
                .h(gpui::relative(height))
                .flex_shrink_0()
                .min_h_0()
                .w_full()
                .flex()
                .flex_row()
                .overflow_hidden()
                .border_b_1()
                .border_color(theme_border);
            for (index, width) in row {
                let Some(panel) = entries.get(index) else {
                    continue;
                };
                let is_active = active.as_deref() == Some(panel.session.as_str());
                let resizable = last_index != Some(index);
                row_div = row_div
                    .child(self.render_panel_tile(panel, is_active, width, resizable, window, cx));
            }
            column = column.child(row_div);
        }
        column
            .child(self.panel_drag_observer(cx))
            .into_any_element()
    }

    /// Reference-shaped child-session back control. Root sessions render none.
    /// `compact` is the tiled header variant; the focused panel gets Source's
    /// full-width header row above the transcript.
    fn render_session_back(
        &self,
        parent: String,
        title: String,
        namespace: String,
        compact: bool,
        activate: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let label = format!("Back to {title}");
        let target = parent;
        let debug = namespace.clone();
        let mut button = div()
            .id(SharedString::from(namespace))
            .debug_selector(move || debug.clone())
            .role(Role::Button)
            .aria_label(label)
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(theme.inset).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                if let Some(session) = activate.as_ref() {
                    app.activate_panel(session.clone(), cx);
                }
                app.reopen_session(target.clone(), cx);
            }));
        button = if compact {
            button.size(px(18.)).rounded(px(4.)).text_color(theme.faint)
        } else {
            button
                .w_full()
                .h(px(26.))
                .px(px(8.))
                .gap(px(5.))
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(11.))
                .text_color(theme.dim)
        };
        button = button.child(glyph(
            "arrow-left",
            if compact { 12. } else { 13. },
            theme.dim,
        ));
        if !compact {
            button = button.child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(title)),
            );
        }
        button.into_any_element()
    }

    fn render_agent_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.panels.len() > 1 {
            return self.render_tiled_panels(window, cx);
        }
        let theme = Theme::default();
        let transcript = self.render_transcript(window, cx);
        let composer = self.render_composer(window, cx);
        let picker = self.render_picker(cx);
        let usage_popup = self.usage_open.then(|| self.render_usage_popup(cx));
        let back = self
            .active_id
            .as_deref()
            .and_then(|id| self.session_parent(id))
            .map(|(parent, title)| {
                self.render_session_back(
                    parent,
                    title,
                    "agent-session-back".into(),
                    false,
                    None,
                    cx,
                )
            });
        let mut panel = div()
            .relative()
            .on_drop(cx.listener(Self::stage_dropped_files))
            .on_drag_move(cx.listener(
                |app, event: &gpui::DragMoveEvent<gpui::ExternalPaths>, _, cx| {
                    let inside = event.bounds.contains(&event.event.position);
                    if inside {
                        app.attachment_drag_lifetime.observe(cx, |app, cx| {
                            if app.attachment_drag_over {
                                app.attachment_drag_over = false;
                                cx.notify();
                            }
                        });
                    }
                    if app.attachment_drag_over != inside {
                        app.attachment_drag_over = inside;
                        cx.notify();
                    }
                },
            ))
            .w(px(AGENT_WIDTH))
            .h_full()
            .flex_shrink_0()
            .bg(theme.agent)
            .overflow_hidden()
            .flex()
            .flex_col();
        panel = if self.is_codex() {
            panel.border_l_1().border_color(theme.border)
        } else {
            panel
                .rounded(px(PANEL_RADIUS))
                .border_1()
                .border_color(theme.border)
        };
        let content = div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .child(self.render_panel_tabs(cx))
            .when_some(back, |content, back| content.child(back))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(transcript),
            )
            .child(self.forms.clone())
            .child(composer)
            .child(picker)
            .child(self.render_usage_button(cx))
            .when_some(usage_popup, |content, popup| content.child(popup))
            .into_any_element();
        panel
            .child(if self.is_codex() {
                content
            } else {
                RoundedSurface::new(content, px(PANEL_RADIUS - 1.)).into_any_element()
            })
            .into_any_element()
    }

    fn render_settings_nav(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut nav = div()
            .w(px(if self.is_codex() {
                SIDEBAR_WIDTH - 1.
            } else {
                SIDEBAR_WIDTH
            }))
            .h_full()
            .flex_shrink_0()
            .when(self.is_codex(), |nav| nav.border_r_1())
            .when(!self.is_codex(), |nav| {
                nav.border_1().rounded(px(PANEL_RADIUS))
            })
            .border_color(theme.border)
            .bg(theme.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(18.0))
                    .pt(px(20.0))
                    .pb(px(16.0))
                    .mb(px(9.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(tracked_label("PREFERENCES", 9., 0.15, theme.accent))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.text)
                            .child("Settings"),
                    ),
            );
        for (section, label, mark) in [
            (SettingsSection::Appearance, "Appearance", "eye"),
            (SettingsSection::Providers, "Providers", "cloud-download"),
            (SettingsSection::Voice, "Voice", "mic"),
            (SettingsSection::Model, "Model", "robot"),
            (SettingsSection::Servers, "Servers", "server"),
            (SettingsSection::Mobile, "Mobile Setup", "dashboard"),
            (SettingsSection::About, "About", "dashboard"),
        ] {
            let active = self.settings_section == section;
            nav = nav.child(
                div()
                    .id(SharedString::from(format!("settings-nav-{label}")))
                    .role(gpui::Role::Button)
                    .aria_label(label)
                    .aria_selected(active)
                    .focusable()
                    .tab_stop(true)
                    .mx(px(10.0))
                    .mt(px(3.0))
                    .h(px(38.0))
                    .px(px(9.0))
                    .rounded(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(11.0))
                    .cursor_pointer()
                    .bg(if active { theme.selected } else { theme.panel })
                    .hover(|style| style.bg(theme.selected))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if active { theme.text } else { theme.dim })
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                            this.settings_section = section;
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.settings_section = section;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .size(px(26.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(theme.border_strong)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(glyph(mark, 16., theme.accent)),
                    )
                    .child(label),
            );
        }
        nav.into_any_element()
    }

    fn render_titlebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let hover = gpui::Rgba {
            a: 0.07,
            ..if self.is_codex() {
                theme.text
            } else {
                theme.accent
            }
        };
        let gap = if self.is_codex() { 10. } else { 12. };
        let visible_sidebar = self.settings_open || self.sidebar_open;
        let workspace_left = if self.is_codex() {
            RAIL_WIDTH + if visible_sidebar { SIDEBAR_WIDTH } else { 0. }
        } else {
            10. + 44.2
                + 10.
                + if visible_sidebar {
                    SIDEBAR_WIDTH + 10.
                } else {
                    0.
                }
        };
        // Source useTitlebarTabsOffset aligns the slot to the workspace host;
        // the leading block never shrinks below its own36px control.
        let leading_width = (workspace_left - 80. - gap).round().max(36.);
        let control = |id: &'static str| {
            div()
                .id(id)
                .size(px(36.))
                .flex_shrink_0()
                .rounded(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.dim)
                .hover(move |style| style.bg(hover).text_color(theme.text))
        };
        let sidebar = control("sidebar-toggle")
            .role(gpui::Role::Button)
            .focusable()
            .tab_stop(true)
            .aria_label(if self.sidebar_open {
                "Hide left panel"
            } else {
                "Show left panel"
            })
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            }))
            .child(crate::icons::inherited_glyph(
                if self.sidebar_open {
                    "panel-left"
                } else {
                    "panel-left-dashed"
                },
                20.,
            ));
        // Multi-panel ownership and Agent Mode are still tracked as migration
        // work; this replaces the previous hand-built decorative bot geometry.
        let agent_mode =
            control("titlebar-theme").child(crate::icons::inherited_glyph("agent-mode", 20.));
        let add = control("titlebar-add")
            .role(gpui::Role::Button)
            .focusable()
            .tab_stop(true)
            .aria_label("Add model panel")
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| this.choose_workspace(cx)))
            .child(crate::icons::inherited_glyph("square-plus", 20.));
        let agent = control("titlebar-agent")
            .role(gpui::Role::Button)
            .focusable()
            .tab_stop(true)
            .aria_label(if self.agent_panel_open {
                "Hide agent panel"
            } else {
                "Show agent panel"
            })
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.agent_panel_open = !this.agent_panel_open;
                cx.notify();
            }))
            .child(crate::icons::inherited_glyph(
                if self.agent_panel_open {
                    "panel-right"
                } else {
                    "panel-right-dashed"
                },
                20.,
            ));
        div()
            .id("titlebar")
            .relative()
            .h(px(TITLEBAR_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .pl(px(80.))
            .pr(px(12.))
            .gap(px(gap))
            .window_control_area(WindowControlArea::Drag)
            .bg(theme.top)
            .when(!self.is_codex(), |bar| {
                bar.border_b_1().border_color(theme.border)
            })
            .child(div().w(px(leading_width)).flex_shrink_0().child(sidebar))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.render_titlebar_tabs(cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(agent_mode)
                    .child(add)
                    .child(agent),
            )
            .when(self.is_codex(), |bar| {
                bar.child(
                    div()
                        .absolute()
                        .left(px(RAIL_WIDTH))
                        .right(px(0.))
                        .bottom(px(0.))
                        .h(px(6.))
                        .bg(gpui::linear_gradient(
                            180.,
                            gpui::linear_color_stop(theme::rgba(0, 0, 0, 0.), 0.),
                            gpui::linear_color_stop(theme::rgba(0, 0, 0, 0.02), 1.),
                        )),
                )
            })
            .into_any_element()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let session_list = self.render_session_rows(
            self.filtered_sessions.clone(),
            self.session_list.clone(),
            cx,
        );

        let body = match self.sidebar_tab {
            SidebarTab::Sessions => self.render_sessions_body(cx, session_list),
            SidebarTab::Files => self.render_files_body(cx),
            SidebarTab::Changes => self.render_changes_body(cx),
        };
        let mut sidebar = div()
            .w(px(if self.is_codex() {
                SIDEBAR_WIDTH - 1.
            } else {
                SIDEBAR_WIDTH
            }))
            .h_full()
            .flex_shrink_0()
            .bg(theme.panel)
            .overflow_hidden()
            .flex()
            .flex_col();
        sidebar = if self.is_codex() {
            sidebar.border_r_1().border_color(theme.border)
        } else {
            sidebar
                .rounded(px(PANEL_RADIUS))
                .border_1()
                .border_color(theme.border)
        };
        let content = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(if self.sidebar_tab == SidebarTab::Sessions {
                        46.
                    } else {
                        56.
                    }))
                    .flex_shrink_0()
                    .pl(px(if self.sidebar_tab == SidebarTab::Sessions {
                        29.
                    } else {
                        16.
                    }))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("Vela"),
                    )
                    .when(self.sidebar_tab == SidebarTab::Sessions, |header| {
                        header.child(
                            div()
                                .id("session-filter-toggle")
                                .role(Role::Button)
                                .aria_label(if self.session_search_open {
                                    "Close session filter"
                                } else {
                                    "Filter sessions"
                                })
                                .track_focus(&self.session_filter_focus)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.session_search_open = !this.session_search_open;
                                    if this.session_search_open {
                                        window
                                            .focus(&this.session_search_input.focus_handle(cx), cx);
                                    } else {
                                        window.focus(&this.session_filter_focus, cx);
                                        this.session_filter.clear();
                                        this.session_search_input.update(cx, |input, cx| {
                                            input.set_text(String::new(), cx)
                                        });
                                        this.refresh_session_filter();
                                    }
                                    cx.notify();
                                }))
                                .text_size(px(14.0))
                                .font_family("codicon")
                                .text_color(theme.faint)
                                .child(icon("ea6d")),
                        )
                    }),
            )
            .child(body)
            .into_any_element();
        sidebar
            .child(if self.is_codex() {
                content
            } else {
                RoundedSurface::new(content, px(PANEL_RADIUS - 1.)).into_any_element()
            })
            .into_any_element()
    }

    fn render_sessions_body(&self, cx: &mut Context<Self>, session_list: gpui::List) -> AnyElement {
        let theme = Theme::default();
        let mut body = div().flex_1().min_h_0().w_full().flex().flex_col();
        if self.session_search_open {
            body = body.child(
                div()
                    .mx(px(16.))
                    .mb(px(6.))
                    .p(px(6.))
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(5.))
                    .child(self.session_search_input.clone()),
            );
        }
        body = body.child(
            div().pl(px(21.0)).pr(px(8.0)).pt(px(10.0)).child(
                div()
                    .id("new-session")
                    .role(Role::Button)
                    .aria_label("New Session")
                    .tab_index(0)
                    .h(px(30.0))
                    .px(px(8.0))
                    .rounded(px(7.0))
                    .flex()
                    .items_center()
                    .gap(px(13.0))
                    .cursor_pointer()
                    .text_size(px(13.0))
                    .text_color(theme.text)
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(|this, _, _, cx| this.choose_workspace(cx)))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(theme.border_strong)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(14.0))
                            .font_family("codicon")
                            .text_color(theme.dim)
                            .child(icon("ea73")),
                    )
                    .child("New Session"),
            ),
        );
        if !self.filtered_pinned_sessions.is_empty() {
            body = body
                .child(
                    div()
                        .pl(px(26.))
                        .pt(px(12.))
                        .pb(px(4.))
                        .text_size(px(12.5))
                        .text_color(theme.faint)
                        .child("Pinned"),
                )
                .child(
                    div()
                        .pl(px(21.))
                        .pr(px(6.))
                        .h(px(
                            (self.filtered_pinned_sessions.len() as f32 * 28.).min(168.)
                        ))
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .child(self.render_session_rows(
                            self.filtered_pinned_sessions.clone(),
                            self.pinned_session_list.clone(),
                            cx,
                        )),
                );
        }
        body = body.child(
            div()
                .pl(px(26.0))
                .pr(px(10.0))
                .pt(px(16.0))
                .pb(px(4.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.faint)
                .id("session-projects-toggle")
                .role(Role::Button)
                .aria_label(if self.session_projects_open {
                    "Collapse Projects"
                } else {
                    "Expand Projects"
                })
                .tab_index(0)
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session_projects_open = !this.session_projects_open;
                    cx.notify();
                }))
                .child("Projects")
                .child(
                    div()
                        .text_size(px(9.0))
                        .child(if self.session_projects_open {
                            "⌄"
                        } else {
                            "›"
                        }),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .id("save-workspace")
                        .role(Role::Button)
                        .tab_index(0)
                        .aria_label("Save a workspace in Vela")
                        .px(px(5.))
                        .child("+")
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.choose_saved_workspace(cx);
                        })),
                ),
        );
        if self.session_projects_open && !self.settings.workspaces.is_empty() {
            body = body.child(self.render_workspaces(cx));
        }
        body = body.child(
            div()
                .pl(px(26.0))
                .pr(px(10.0))
                .pt(px(15.0))
                .pb(px(4.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.faint)
                .id("session-recents-toggle")
                .role(Role::Button)
                .aria_label(if self.session_recents_open {
                    "Collapse Recents"
                } else {
                    "Expand Recents"
                })
                .tab_index(0)
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session_recents_open = !this.session_recents_open;
                    cx.notify();
                }))
                .child("Recents")
                .child(
                    div()
                        .text_size(px(9.0))
                        .child(if self.session_recents_open {
                            "⌄"
                        } else {
                            "›"
                        }),
                ),
        );
        body.child(
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .pl(px(21.0))
                .pr(px(6.0))
                .pt(px(12.0))
                .pb(px(8.0))
                .flex()
                .flex_col()
                .when(self.session_recents_open, |body| {
                    if self.filtered_sessions.is_empty() {
                        body.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.faint)
                                .child("No sessions yet."),
                        )
                    } else {
                        body.child(session_list)
                    }
                }),
        )
        .into_any_element()
    }

    fn render_changes_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let weak = cx.weak_entity();
        let mut list = div()
            .id("changes-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(1.0));
        if self.changes.is_empty() {
            list = list.child(
                div()
                    .p(px(10.0))
                    .text_size(px(10.5))
                    .text_color(theme.faint)
                    .child("No changes yet. Files the agent edits appear here."),
            );
        }
        for (ix, entry) in self.changes.iter().enumerate() {
            let rel = entry.path.clone();
            let code = entry.code.clone();
            let label = entry.label.clone();
            let weak = weak.clone();
            let color = if code == "??" {
                theme.green
            } else if code.contains('D') {
                theme.red
            } else {
                theme.yellow
            };
            list = list.child(
                div()
                    .id(SharedString::from(format!("change-{ix}")))
                    .h(px(30.0))
                    .px(px(7.0))
                    .rounded(px(5.0))
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.elevated))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        let rel = rel.clone();
                        let _ = weak.update(cx, |app, cx| app.reveal_change(rel, cx));
                    })
                    .child(
                        div()
                            .w(px(20.0))
                            .text_size(px(9.0))
                            .font_family(theme::FONT_MONO)
                            .text_color(color)
                            .child(SharedString::from(code)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.0))
                            .text_color(theme.dim)
                            .child(SharedString::from(entry.path.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(8.5))
                            .text_color(theme.faint)
                            .child(SharedString::from(label)),
                    ),
            );
        }
        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(38.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.panel)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("Changes")
                            .child(
                                div()
                                    .px(px(6.0))
                                    .rounded(px(999.0))
                                    .bg(theme.selected)
                                    .text_size(px(9.0))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(theme.faint)
                                    .child(SharedString::from(format!("{}", self.changes.len()))),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("changes-close")
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .hover(|style| style.text_color(theme.text))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.switch_sidebar(SidebarTab::Files, cx);
                                }),
                            )
                            .child(icon("ea76")),
                    ),
            )
            .child(if let Some(error) = &self.file_error {
                div()
                    .mx(px(8.0))
                    .mb(px(4.0))
                    .text_size(px(9.5))
                    .text_color(theme.red)
                    .child(SharedString::from(error.clone()))
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .child(div().px(px(4.0)).flex_1().min_h_0().child(list))
            .into_any_element()
    }

    fn render_diff(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let Some(diff) = &self.diff else {
            return div().into_any_element();
        };
        let rel = diff.rel.clone();
        let content = diff.content.clone();
        let full = diff.full.clone();
        let (rows, added, removed) = diff_rows(&content, full.as_deref());
        let changes = self.changes.clone();
        let weak = cx.weak_entity();
        let mut file_list = div().flex().flex_col().py(px(4.0));
        for (index, entry) in changes.iter().enumerate() {
            let path = entry.path.clone();
            let code = entry.code.clone();
            let selected = path == rel;
            let weak = weak.clone();
            file_list = file_list.child(
                div()
                    .id(SharedString::from(format!("diff-file-{index}")))
                    .h(px(30.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .cursor_pointer()
                    .bg(if selected {
                        theme.selected
                    } else {
                        theme.editor
                    })
                    .hover(|style| style.bg(theme.selected))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        let path = path.clone();
                        let _ = weak.update(cx, |app, cx| app.open_diff(path, cx));
                    })
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .child(icon("ec5e")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.5))
                            .text_color(if selected { theme.text } else { theme.dim })
                            .child(SharedString::from(entry.path.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(9.0))
                            .font_family(theme::FONT_MONO)
                            .text_color(if code.contains('D') {
                                theme.red
                            } else {
                                theme.yellow
                            })
                            .child(SharedString::from(code)),
                    ),
            );
        }
        let mut diff_body = div()
            .id("diff-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .bg(theme.editor)
            .py(px(4.0))
            .font_family(theme::FONT_MONO)
            .text_size(px(11.0))
            .line_height(px(20.0));
        for (line, text, is_added, is_removed) in rows {
            let background = if is_added {
                Some(theme::rgba(0xe8, 0xf5, 0xe9, 1.0))
            } else if is_removed {
                Some(theme::rgba(0xfd, 0xec, 0xea, 1.0))
            } else {
                None
            };
            let color = if text.trim_start().starts_with("##") {
                theme::rgba(0x1a, 0x5f, 0xb4, 1.0)
            } else if text.trim_start().starts_with('#') {
                theme::rgba(0x0b, 0x63, 0xce, 1.0)
            } else {
                theme.text
            };
            let mut row = div()
                .flex()
                .h(px(20.0))
                .w_full()
                .child(
                    div()
                        .w(px(42.0))
                        .flex_shrink_0()
                        .pr(px(12.0))
                        .text_right()
                        .text_size(px(9.5))
                        .text_color(theme.faint)
                        .child(SharedString::from(format!("{line}"))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .whitespace_nowrap()
                        .text_color(color)
                        .child(SharedString::from(text)),
                );
            if let Some(background) = background {
                row = row.bg(background);
            }
            diff_body = diff_body.child(row);
        }
        div()
            .h_full()
            .w_full()
            .flex()
            .flex_col()
            .bg(theme.editor)
            .child(
                div()
                    .h(px(38.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("Changes"),
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .rounded(px(999.0))
                            .bg(theme.selected)
                            .text_size(px(9.0))
                            .text_color(theme.faint)
                            .child(SharedString::from(format!("{}", changes.len()))),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .text_size(px(10.5))
                            .text_color(theme.faint)
                            .child("Collapse all")
                            .child("Wrap")
                            .child("Clear all"),
                    )
                    .child(
                        div()
                            .id("changes-close")
                            .ml(px(6.0))
                            .cursor_pointer()
                            .text_size(px(13.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .hover(|style| style.text_color(theme.text))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.diff = None;
                                    this.changes_open = false;
                                    cx.notify();
                                }),
                            )
                            .child(icon("ea76")),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .child(
                        div()
                            .id("diff-file-list")
                            .w(px(223.0))
                            .h_full()
                            .flex_shrink_0()
                            .border_r_1()
                            .border_color(theme.border)
                            .overflow_y_scroll()
                            .child(file_list),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(38.0))
                                    .px(px(12.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(7.0))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_family("codicon")
                                            .text_color(theme.faint)
                                            .child(icon("ec5e")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .truncate()
                                            .text_size(px(12.0))
                                            .text_color(theme.text)
                                            .child(SharedString::from(rel.clone())),
                                    )
                                    .child(
                                        div()
                                            .px(px(7.0))
                                            .py(px(2.0))
                                            .rounded(px(999.0))
                                            .text_size(px(8.5))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.faint)
                                            .child(SharedString::from(format!(
                                                "MODIFIED +{added} -{removed}"
                                            ))),
                                    )
                                    .child(
                                        div()
                                            .id("diff-close")
                                            .cursor_pointer()
                                            .text_size(px(12.0))
                                            .font_family("codicon")
                                            .text_color(theme.faint)
                                            .hover(|style| style.text_color(theme.text))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _, window, cx| {
                                                    this.close_diff(window, cx);
                                                }),
                                            )
                                            .child(icon("ea76")),
                                    ),
                            )
                            .child(diff_body),
                    ),
            )
            .into_any_element()
    }

    fn flattened_files(&self) -> Vec<(FileEntry, usize)> {
        fn walk(
            entries: &[FileEntry],
            depth: usize,
            expanded: &HashSet<String>,
            children: &HashMap<String, Vec<FileEntry>>,
            out: &mut Vec<(FileEntry, usize)>,
        ) {
            for entry in entries {
                out.push((entry.clone(), depth));
                if entry.is_dir
                    && expanded.contains(&entry.rel)
                    && let Some(nested) = children.get(&entry.rel)
                {
                    walk(nested, depth + 1, expanded, children, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.files, 0, &self.expanded, &self.children, &mut out);
        out
    }

    fn render_transcript(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.active_id.is_none() {
            return self.render_empty_transcript();
        }
        if self.blocks.is_empty() {
            return self.render_empty_transcript();
        }
        self.render_native_transcript(window, cx)
    }

    fn render_empty_transcript(&self) -> AnyElement {
        let theme = Theme::default();
        if self.phase.is_active() && !self.loading {
            // VelaTranscript's empty running state belongs at the beginning
            // of the 48rem transcript column, rather than in the empty art.
            let ink = gpui::Rgba {
                a: 0.62,
                ..theme.text
            };
            return div()
                .flex_1()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(768.))
                        .pt(px(16.))
                        .px(px(20.))
                        .pb(px(32.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(ink)
                                .child({
                                    use gpui::AnimationExt;
                                    glyph("working-spinner", 11., ink).with_animation(
                                        "empty-transcript-working",
                                        gpui::Animation::new(Duration::from_millis(700)).repeat(),
                                        |svg, progress| {
                                            svg.with_transformation(gpui::Transformation::rotate(
                                                gpui::percentage(progress),
                                            ))
                                        },
                                    )
                                })
                                .child(if self.permissions.is_empty() {
                                    "Working"
                                } else {
                                    "Waiting for your action"
                                }),
                        ),
                )
                .into_any_element();
        }
        let label = if self.loading {
            "Loading conversation…"
        } else {
            "Tell the agent what to work on."
        };
        div()
            .flex_1()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .when(
                self.settings.experimental_art && !self.loading && !self.phase.is_active(),
                |column| {
                    // With no rendered timeline the reference's :has(light
                    // timeline) selector leaves the original station visible.
                    let width = ((AGENT_WIDTH - if self.is_codex() { 1. } else { 2. }) * 0.6)
                        .clamp(150., 240.);
                    column.child(
                        div()
                            .flex_shrink_0()
                            .mb(px(14.))
                            .child(crate::pixel_art::station(width)),
                    )
                },
            )
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(19.5))
                    .text_color(theme.dim)
                    .child(label),
            )
            .into_any_element()
    }

    fn render_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        if !self.tray.open {
            return div().into_any_element();
        }
        self.install_terminal_focus_reporting(window, cx);
        let panel_focus = self
            .terminals
            .panel_focus
            .get_or_insert_with(|| {
                [
                    cx.focus_handle().tab_stop(true),
                    cx.focus_handle().tab_stop(true),
                    cx.focus_handle().tab_stop(true),
                ]
            })
            .clone();
        let panel_entered = self.terminals.panel_entered.clone();
        let terminal = self.terminals.active();
        let cursor_blink_visible = if let Some(terminal) = terminal {
            let focused = terminal.focus.is_focused(window) && window.is_window_active();
            terminal.blink.update(cx, |blink, cx| {
                blink.update_state(terminal.emulator.cursor_blinking(), focused, cx)
            });
            terminal.blink.read(cx).cursor_visible()
        } else {
            true
        };
        let cell_width = f32::from(
            window
                .text_system()
                .shape_line(
                    "W".into(),
                    px(12.),
                    &[TextRun {
                        len: 1,
                        font: theme::source_editor_font(),
                        color: theme.text.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width(),
        )
        .max(1.);
        let measured_terminal =
            terminal.map(|terminal| (terminal.id.clone(), terminal.cols, terminal.rows));
        let selection_geometry = terminal.map(|terminal| terminal.geometry.clone());
        if let Some(terminal) = terminal {
            terminal
                .input
                .read(cx)
                .anchor
                .set(terminal.emulator.cursor());
        }
        let input_entity = terminal.map(|terminal| terminal.input.clone());
        let input_focus = terminal.map(|terminal| terminal.focus.clone());
        let workspace_owner = self.terminals.workspace.clone();
        let weak = cx.weak_entity();
        let panel = self.terminal_panel;
        let directory = self
            .active_id
            .as_ref()
            .and_then(|id| self.engine.view(id))
            .map(|view| view.summary().directory)
            .unwrap_or_default();
        let rows = terminal.map(|terminal| terminal.rows as usize).unwrap_or(0);
        let cols = terminal.map(|terminal| terminal.cols as usize).unwrap_or(0);
        let cursor = terminal
            .and_then(|terminal| {
                let shape = terminal.emulator.cursor_shape();
                (shape != vela_terminal::emulator::CursorShape::Hidden)
                    .then(|| (terminal.emulator.cursor(), shape))
            })
            .filter(|_| cursor_blink_visible);
        let cursor_width = terminal
            .map(|terminal| terminal.emulator.cursor_cell_width())
            .unwrap_or(1);
        let mut grid = div()
            .relative()
            .w_full()
            .h(px(rows.max(1) as f32 * 15.0))
            .overflow_hidden()
            .font_family(theme::source_font())
            .text_size(px(12.))
            .line_height(px(15.))
            .text_color(theme.text);
        for row_ix in 0..rows {
            let cells = terminal.unwrap().emulator.line(row_ix);
            grid = grid.child(render_terminal_row(
                &cells, cols, cell_width, row_ix, &theme,
            ));
        }
        if let Some((cursor, shape)) = cursor
            && cursor.row < rows
        {
            use vela_terminal::emulator::CursorShape;
            let base = div()
                .absolute()
                .left(px(cursor.col as f32 * cell_width))
                .top(px(cursor.row as f32 * 15.))
                .w(px(cursor_width as f32 * cell_width))
                .h(px(15.));
            let cursor = match shape {
                CursorShape::Block => base
                    .bg(theme.accent.opacity(0.32))
                    .border_1()
                    .border_color(theme.accent),
                CursorShape::Underline => base.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.))
                        .bg(theme.accent),
                ),
                CursorShape::Beam => base.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(2.))
                        .bg(theme.accent),
                ),
                CursorShape::HollowBlock => base.border_1().border_color(theme.accent),
                CursorShape::Hidden => base,
            };
            grid = grid.child(cursor);
        }
        if let Some(terminal) = terminal
            && !terminal.input.read(cx).buffer.text().is_empty()
        {
            let composition = &terminal.input.read(cx).buffer;
            let caret = terminal.emulator.cursor();
            if caret.row < rows {
                grid = grid.child(
                    div()
                        .absolute()
                        .left(px(caret.col as f32 * cell_width))
                        .top(px(caret.row as f32 * 15.))
                        .bg(theme.editor)
                        .child(
                            StyledText::new(SharedString::from(composition.text().to_owned()))
                                .with_runs(vec![TextRun {
                                    len: composition.text().len(),
                                    font: theme::source_editor_font(),
                                    color: theme.text.into(),
                                    background_color: None,
                                    underline: Some(gpui::UnderlineStyle {
                                        thickness: px(1.),
                                        color: Some(theme.accent.into()),
                                        wavy: false,
                                    }),
                                    strikethrough: None,
                                }]),
                        ),
                );
            }
        }
        let focus = terminal.map(|terminal| terminal.focus.clone());
        if terminal.is_none() {
            grid = grid.child(
                div()
                    .id("empty-terminal-message")
                    .role(Role::Label)
                    .aria_label("No terminal open. Press + to start one.")
                    .child("No terminal open. Press + to start one."),
            );
        }
        let active = panel == TerminalPanel::Terminal;
        let terminal_mouse = terminal.is_some_and(|terminal| {
            terminal.emulator.mouse_report_mode() != vela_terminal::emulator::MouseReportMode::Off
        });
        let body = match panel {
            TerminalPanel::Terminal => div()
                .id("terminal-grid")
                .relative()
                .role(Role::Group)
                .aria_label("Integrated terminal")
                .key_context("VelaTerminal")
                .when_some(focus, |row, focus| {
                    row.track_focus(&focus)
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            window.focus(&focus, cx)
                        })
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(Self::begin_terminal_selection),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(Self::begin_terminal_selection),
                )
                .on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(Self::begin_terminal_selection),
                )
                .on_scroll_wheel(cx.listener(Self::scroll_terminal))
                .on_key_down(cx.listener(Self::on_terminal_key))
                .on_action(cx.listener(|app, _: &crate::input::Copy, _, cx| app.copy_terminal(cx)))
                .on_action(
                    cx.listener(|app, _: &crate::input::Paste, _, cx| {
                        app.paste_terminal("menu", cx)
                    }),
                )
                .flex_1()
                .min_h_0()
                .w_full()
                .bg(theme.editor)
                .px(px(8.))
                .py(px(6.0))
                .flex()
                .flex_col()
                .cursor(if terminal_mouse {
                    gpui::CursorStyle::Arrow
                } else {
                    gpui::CursorStyle::IBeam
                })
                .child(grid)
                .child(
                    gpui::canvas(
                        move |bounds, window, _| {
                            if let Some(geometry) = &selection_geometry {
                                geometry.set(Some((bounds, cell_width)));
                            }
                            let Some((id, cols, rows)) = &measured_terminal else {
                                return;
                            };
                            let Some(size) = vela_terminal::size::fit(
                                f32::from(bounds.size.width),
                                f32::from(bounds.size.height),
                                cell_width,
                                15.,
                            ) else {
                                return;
                            };
                            if size.cols != *cols || size.rows != *rows {
                                let id = id.clone();
                                let owner = workspace_owner.clone();
                                let weak = weak.clone();
                                window.on_next_frame(move |_, cx| {
                                    let _ = weak.update(cx, |app, cx| {
                                        app.fit_terminal(&id, &owner, size, cx)
                                    });
                                });
                            }
                        },
                        move |bounds, _, window, cx| {
                            if let (Some(focus), Some(input)) = (&input_focus, &input_entity) {
                                window.handle_input(
                                    focus,
                                    gpui::ElementInputHandler::new(bounds, input.clone()),
                                    cx,
                                );
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .into_any_element(),
            TerminalPanel::Problems => div()
                .id("problems-list")
                .role(Role::List)
                .aria_label("Validation problems")
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_y_scroll()
                .bg(theme.editor)
                .p(px(8.))
                .flex()
                .flex_col()
                .gap(px(3.))
                .when(self.problems.is_empty(), |list| {
                    list.child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.faint)
                            .child("No problems have been detected in the workspace."),
                    )
                })
                .when_some(self.problem_path.clone(), |list, path| {
                    list.child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(path),
                    )
                })
                .children(self.problems.iter().map(|problem| {
                    div()
                        .id(SharedString::from(format!(
                            "problem-{}-{}-{}",
                            problem.source, problem.line, problem.column
                        )))
                        .role(Role::ListItem)
                        .aria_label(SharedString::from(format!(
                            "Line {}, column {}: {}",
                            problem.line, problem.column, problem.message
                        )))
                        .text_size(px(11.0))
                        .line_height(px(16.0))
                        .text_color(if problem.severity == "error" {
                            theme.red
                        } else {
                            theme.yellow
                        })
                        .child(format!(
                            "{}:{}  {}",
                            problem.line, problem.column, problem.message
                        ))
                }))
                .into_any_element(),
            TerminalPanel::Sass => {
                let state = self.sass.clone();
                let scroll = self.sass_scroll.clone();
                self.sass_output.update(cx, |output, cx| {
                    output.sync(state, directory, scroll, cx);
                });
                self.sass_output.clone().into_any_element()
            }
        };
        let tab = |label: &'static str, index: usize, selected: bool, cx: &mut Context<Self>| {
            let focus = panel_focus[index].clone();
            terminal_state::reveal_on_focus(
                div()
                    .id(SharedString::from(format!("terminal-tab-{label}")))
                    .role(Role::Button)
                    .aria_label(label)
                    .track_focus(&focus)
                    .h(px(34.0))
                    .flex_shrink_0()
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .border_b_1()
                    .border_color(if selected { theme.accent } else { theme.editor })
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if selected { theme.accent } else { theme.dim })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.terminal_panel = match label {
                            "PROBLEMS" => TerminalPanel::Problems,
                            "SASS OUTPUT" => TerminalPanel::Sass,
                            _ => TerminalPanel::Terminal,
                        };
                        cx.notify();
                    }))
                    .child(label),
                vec![focus],
                self.terminals.scroll.clone(),
                index,
                panel_entered[index].clone(),
            )
        };
        let mut first_tabs = self.render_terminal_tabs(active, cx);
        let remaining_tabs = first_tabs.split_off(first_tabs.len().min(1));
        let panel = div()
            .h(px((self.tray.height - 5.).max(0.)))
            .w_full()
            .flex_shrink_0()
            .rounded(px(PANEL_RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.editor)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("terminal-header")
                    .role(Role::Group)
                    .aria_label("Terminal panel header")
                    .on_click(cx.listener(|app, _, _, cx| {
                        if app.tray.snapped {
                            app.tray.show();
                            app.terminal_focus_pending = true;
                            cx.notify();
                        }
                    }))
                    .h(px(34.0))
                    .flex_shrink_0()
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .when(self.tray.snapped, |bar| {
                        bar.child(
                            div()
                                .id("terminal-expand")
                                .role(Role::Button)
                                .aria_label("Expand terminal panel")
                                .focusable()
                                .tab_stop(true)
                                .font_family("codicon")
                                .text_size(px(12.))
                                .cursor_pointer()
                                .on_click(cx.listener(|app, _, _, cx| {
                                    cx.stop_propagation();
                                    app.tray.show();
                                    app.terminal_focus_pending = true;
                                    cx.notify();
                                }))
                                .child(icon("eab7")),
                        )
                    })
                    .child(
                        div()
                            .id("terminal-tabs-viewport")
                            .flex_1()
                            .min_w_0()
                            .h(px(34.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .overflow_x_scroll()
                            .track_scroll(&self.terminals.scroll)
                            .when(first_tabs.is_empty(), |bar| {
                                bar.child(tab("Terminal", 0, active, cx))
                            })
                            .children(first_tabs)
                            .child(tab("PROBLEMS", 1, panel == TerminalPanel::Problems, cx))
                            .child(tab("SASS OUTPUT", 2, panel == TerminalPanel::Sass, cx))
                            .children(remaining_tabs),
                    )
                    .when(self.runtime_capabilities.tui, |bar| {
                        bar.child(
                            div()
                                .id("new-tui-terminal")
                                .debug_selector(|| "new-tui-terminal".into())
                                .role(Role::Button)
                                .aria_label("Open Agent TUI")
                                .focusable()
                                .tab_stop(true)
                                .h(px(22.0))
                                .px(px(5.0))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(5.0))
                                .text_size(px(9.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.dim)
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.inset))
                                .on_click(cx.listener(|app, _, _, cx| app.spawn_tui_terminal(cx)))
                                .child("TUI"),
                        )
                    })
                    .child(
                        div()
                            .id("new-terminal")
                            .role(Role::Button)
                            .aria_label("New terminal")
                            .focusable()
                            .tab_stop(true)
                            .on_click(cx.listener(|app, _, _, cx| app.create_terminal(None, cx)))
                            .size(px(22.0))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.0))
                            .font_family("codicon")
                            .text_color(theme.dim)
                            .child(icon("ea60")),
                    )
                    .child(self.render_preview_button(cx))
                    .when_some(
                        self.preview_notice
                            .as_ref()
                            .filter(|(owner, notice)| {
                                Some(owner) == self.active_id.as_ref() && !notice.is_empty()
                            })
                            .map(|(_, notice)| notice.clone()),
                        |bar, notice| {
                            bar.child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .text_color(theme.yellow)
                                    .child(notice),
                            )
                        },
                    )
                    .child(
                        div()
                            .id("terminal-close")
                            .flex_shrink_0()
                            .role(Role::Button)
                            .aria_label("Close the terminal panel")
                            .focusable()
                            .tab_stop(true)
                            .on_click(cx.listener(|app, _, _, cx| {
                                cx.stop_propagation();
                                app.close_terminal(cx);
                            }))
                            .cursor_pointer()
                            .px(px(4.0))
                            .text_size(px(12.0))
                            .font_family("codicon")
                            .text_color(theme.faint)
                            .hover(|style| style.text_color(theme.red))
                            .child(icon("eab4")),
                    ),
            )
            .child(body)
            .into_any_element();
        div()
            .h(px(self.tray.height))
            .w_full()
            .flex_shrink_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("terminal-resize-divider")
                    .role(Role::Group)
                    .aria_label("Resize terminal panel")
                    .h(px(5.))
                    .w_full()
                    .flex_shrink_0()
                    .cursor(gpui::CursorStyle::ResizeRow)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|app, event: &gpui::MouseDownEvent, _, cx| {
                            if app.tray.begin(f32::from(event.position.y)) {
                                cx.stop_propagation();
                                cx.notify();
                            }
                        }),
                    ),
            )
            .child(panel)
            .into_any_element()
    }

    fn render_todos(&self) -> AnyElement {
        let theme = Theme::default();
        if self.todos.is_empty() {
            return div().into_any_element();
        }
        let mut column = div()
            .w_full()
            .rounded(px(9.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.inset)
            .px(px(10.0))
            .py(px(7.0))
            .flex()
            .flex_col()
            .gap(px(3.0));
        for todo in self.todos.iter().take(8) {
            let (marker, color) = match todo.status.as_str() {
                "completed" => ("✓", theme.green),
                "in_progress" => ("●", theme.accent),
                "cancelled" => ("–", theme.faint),
                _ => ("○", theme.faint),
            };
            column = column.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .text_size(px(10.5))
                    .child(div().text_color(color).child(marker))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(if todo.status == "completed" {
                                theme.faint
                            } else {
                                theme.dim
                            })
                            .child(SharedString::from(todo.content.clone())),
                    ),
            );
        }
        column.into_any_element()
    }

    fn render_status_popover(
        &mut self,
        line: usize,
        column: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let validation_pending = self.pending_validation_request().is_some();
        let active_key = self.active_document_key();
        let result_active = self.validation_key.as_ref() == active_key.as_ref();
        let validation_failed = result_active && self.validation_failed;
        let validation_summary = result_active.then_some(self.validation_summary).flatten();
        let validation_label = validation_action_label(
            validation_pending,
            validation_failed,
            validation_summary.is_some(),
        );
        let row = |label: &'static str, value: String, active: bool| {
            div()
                .h(px(28.0))
                .px(px(7.0))
                .rounded(px(6.0))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(11.0))
                .text_color(theme.text)
                .child(label)
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(if active { theme.green } else { theme.faint })
                        .child(SharedString::from(value)),
                )
        };
        let sass_pending = self
            .sass_pending
            .as_ref()
            .is_some_and(|(_, scope)| *scope == self.document_workspace());
        let sass_meta = sass_menu_metadata(
            self.sass.status,
            self.sass.enabled,
            self.sass.error.as_deref(),
            sass_pending,
            &self.workspace_directory(),
        );
        let sass_disabled = sass_meta.disabled;
        let mut sass_item = div()
            .id("status-sass")
            .role(Role::MenuItem)
            .aria_description(sass_meta.description.clone())
            .opacity(if sass_disabled { 0.5 } else { 1.0 })
            .aria_label(format!(
                "Watch Sass for {}: {}",
                self.workspace_directory(),
                self.sass.status.as_str()
            ))
            .aria_toggled(if self.sass.enabled {
                gpui::Toggled::True
            } else {
                gpui::Toggled::False
            })
            .a11y_synthetic_children(move |builder| {
                apply_sass_disabled(builder.parent_node(), sass_disabled);
            })
            .child(row(sass_meta.label, String::new(), self.sass.enabled));
        if !sass_disabled {
            sass_item = sass_item
                .track_focus(&self.status_action_focus[1])
                .cursor_pointer()
                .hover(|style| style.bg(theme.inset))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        window.focus(&this.status_action_focus[1], cx);
                        this.toggle_sass_watch(cx);
                        cx.notify();
                    }),
                )
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                        this.toggle_sass_watch(cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                }));
        }
        div()
            .id("editor-status-popover")
            .role(Role::Menu)
            .aria_label("Editor status and actions")
            .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                if !this
                    .status_menu_anchor
                    .is_some_and(|bounds| bounds.contains(&event.position))
                {
                    this.status_menu_open = false;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.status_menu_open = false;
                    window.focus(&this.status_menu_focus, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(52.0))
            .right(px(10.0))
            .w(px(228.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(5.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(
                div()
                    .id("status-save")
                    .role(Role::MenuItem)
                    .aria_label("Save file")
                    .track_focus(&self.status_action_focus[0])
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            window.focus(&this.status_menu_focus, cx);
                            this.save_editor(cx);
                            this.status_menu_open = false;
                            cx.notify();
                        }),
                    )
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                            this.save_editor(cx);
                            this.status_menu_open = false;
                            window.focus(&this.status_menu_focus, cx);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .child(row("Save file", "⌘S".into(), false)),
            )
            .child(sass_item)
            .child(
                div()
                    .id("status-validate")
                    .debug_selector(move || {
                        if validation_pending {
                            "validation-cancel".into()
                        } else if validation_failed {
                            "validation-retry".into()
                        } else {
                            "validation-action".into()
                        }
                    })
                    .role(Role::MenuItem)
                    .aria_label(if validation_pending {
                        "Cancel validation"
                    } else {
                        "Validate HTML, CSS, or SCSS"
                    })
                    .track_focus(&self.status_action_focus[2])
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            window.focus(&this.status_action_focus[2], cx);
                            if validation_pending {
                                this.cancel_validation(cx);
                            } else {
                                this.validate_active_document(cx);
                            }
                        }),
                    )
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                            if validation_pending {
                                this.cancel_validation(cx);
                            } else {
                                this.validate_active_document(cx);
                            }
                            cx.stop_propagation();
                        }
                    }))
                    .child(row(validation_label, "W3C / Sass".into(), false)),
            )
            .when(validation_failed, |menu| {
                menu.child(
                    row("Validation", "Failed".into(), true)
                        .debug_selector(|| "validation-failed-row".into()),
                )
            })
            .when_some(validation_summary, |menu, (errors, warnings)| {
                menu.child(row(
                    "Validation",
                    validation_summary_label(errors, warnings),
                    errors == 0 && warnings == 0,
                ))
            })
            .child(div().h(px(1.0)).bg(theme.border).my(px(3.0)))
            .child(
                div()
                    .px(px(7.0))
                    .h(px(18.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(9.5))
                    .text_color(theme.faint)
                    .child(SharedString::from(format!(
                        "Ln {}, Col {}",
                        line + 1,
                        column + 1
                    )))
                    .child("Spaces: 2"),
            )
            .child(
                div()
                    .px(px(7.0))
                    .h(px(18.0))
                    .text_size(px(9.5))
                    .text_color(theme.faint)
                    .child("UTF-8"),
            )
            .into_any_element()
    }

    fn render_editor_group_pane(
        &mut self,
        key: Option<DocumentKey>,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let body = match key {
            Some(key) => {
                let editor = self.editor_view(key, cx);
                div().size_full().child(editor).into_any_element()
            }
            None => div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .bg(theme.editor)
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.dim)
                        .child("Empty editor group"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.faint)
                        .child("Open a file from the Explorer to show it here."),
                )
                .into_any_element(),
        };
        let group_id = format!("editor-group-{index}");
        let selector = group_id.clone();
        let mut pane = div()
            .id(SharedString::from(group_id))
            .debug_selector(move || selector.clone())
            .role(Role::Group)
            .aria_label(format!("Editor group {}", index + 1))
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .can_drop(|value, _, _| {
                value.downcast_ref::<tabs::DraggedEditorTab>().is_some()
            })
            .drag_over::<tabs::DraggedEditorTab>(|style, _, _, _| {
                style.bg(Theme::default().selected)
            })
            .on_drop(cx.listener(move |app, drag: &tabs::DraggedEditorTab, window, cx| {
                app.move_document_to_editor_group(drag.key.clone(), index, window, cx);
            }))
            .child(body);
        if index > 0 {
            let close_index = index - 1;
            let close_id = format!("editor-group-close-{index}");
            let close_selector = close_id.clone();
            pane = pane.child(
                div()
                    .id(SharedString::from(close_id))
                    .debug_selector(move || close_selector.clone())
                    .role(Role::Button)
                    .aria_label(format!("Close editor group {}", index + 1))
                    .absolute()
                    .top(px(6.))
                    .right(px(8.))
                    .size(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .bg(theme.panel)
                    .text_size(px(12.))
                    .text_color(theme.faint)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected).text_color(theme.text))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.close_editor_group(close_index, cx);
                    }))
                    .child("×"),
            );
        }
        pane.into_any_element()
    }

    fn render_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.reconcile_editor_groups();
        let theme = Theme::default();
        let workspace_name = self
            .active_id
            .as_ref()
            .and_then(|id| self.engine.view(id))
            .and_then(|view| {
                std::path::Path::new(&view.summary().directory)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_default();
        let workspace = self.document_workspace();
        let documents = self.documents.borrow();
        let Some(key) = documents.active_key(&workspace) else {
            return div().into_any_element();
        };
        let rel = key.rel.clone();
        let Some(editor) = documents.get(key) else {
            return div().flex_1().child("Loading file…").into_any_element();
        };
        let dirty = editor.dirty();
        let conflict = editor.conflict.clone();
        let deleted = editor.deleted;
        let mut path = div()
            .id("editor-breadcrumb-path")
            .h(px(30.))
            .min_w_0()
            .px(px(16.))
            .rounded(px(999.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.bg)
            .flex()
            .items_center()
            .gap(px(2.))
            .overflow_hidden()
            .role(gpui::Role::Group)
            .aria_label("File path");
        let segments = std::iter::once(workspace_name)
            .chain(rel.split('/').map(str::to_owned))
            .collect::<Vec<_>>();
        for (index, segment) in segments.iter().enumerate() {
            if index > 0 {
                path = path.child(
                    div()
                        .size(px(16.))
                        .flex_shrink_0()
                        .font_family("codicon")
                        .text_size(px(16.))
                        .text_color(theme.faint)
                        .child(icon("eab6")),
                );
            }
            path = path.child(
                div()
                    .h(px(24.))
                    .px(px(4.))
                    .min_w_0()
                    .truncate()
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(if index + 1 == segments.len() {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if index + 1 == segments.len() {
                        theme.text
                    } else {
                        theme.dim
                    })
                    .child(SharedString::from(segment.clone())),
            );
        }
        let can_save = dirty && conflict.is_none();
        let (cursor_line, cursor_col) = editor.line_col();
        let key = key.clone();
        drop(documents);
        let editor = self.editor_view(key.clone(), cx);
        let conflict_banner =
            conflict.map(|change| self.render_conflict_banner(key.clone(), change, _window, cx));
        let status_anchor = cx.weak_entity();
        let popover = if self.status_menu_open {
            Some(self.render_status_popover(cursor_line, cursor_col, cx))
        } else {
            None
        };
        let group_keys = std::iter::once(Some(key.clone()))
            .chain(self.editor_group_layout(&workspace).iter().cloned())
            .collect::<Vec<_>>();
        let group_count = group_keys.len();
        let fraction_x = self.editor_group_fraction_x;
        let fraction_y = self.editor_group_fraction_y;
        let drag_axis = self.editor_group_drag.map(|drag| drag.axis);
        let group_measure = cx.weak_entity();
        let editor_pane = if group_count == 1 {
            editor.into_any_element()
        } else {
            let cells = crate::editor_groups::group_cells(group_count, fraction_x, fraction_y);
            let mut panes = Vec::new();
            for (index, cell) in cells.iter().enumerate() {
                let pane = self.render_editor_group_pane(
                    group_keys.get(index).cloned().flatten(),
                    index,
                    cx,
                );
                let right_column = index % 2 == 1;
                let bottom_row = cell.top > 0.;
                let id = format!("editor-group-cell-{index}");
                let selector = id.clone();
                panes.push(
                    div()
                        .id(SharedString::from(id))
                        .debug_selector(move || selector.clone())
                        .absolute()
                        .left(relative(cell.left / 100.))
                        .top(relative(cell.top / 100.))
                        .w(relative(cell.width / 100.))
                        .h(relative(cell.height / 100.))
                        .when(right_column, |pane| {
                            pane.border_l_1().border_color(theme.border)
                        })
                        .when(bottom_row, |pane| {
                            pane.border_t_1().border_color(theme.border)
                        })
                        .child(pane),
                );
            }
            let mut groups = div()
                .relative()
                .size_full()
                .min_w_0()
                .min_h_0()
                .on_mouse_move(cx.listener(|app, event: &MouseMoveEvent, _, cx| {
                    let Some(drag) = app.editor_group_drag else {
                        return;
                    };
                    if event.pressed_button != Some(MouseButton::Left) {
                        app.editor_group_drag = None;
                        return;
                    }
                    let (extent, coordinate) = match drag.axis {
                        EditorGroupAxis::Horizontal => (
                            app.editor_group_bounds
                                .map(|bounds| f32::from(bounds.size.width))
                                .unwrap_or(0.),
                            f32::from(event.position.x),
                        ),
                        EditorGroupAxis::Vertical => (
                            app.editor_group_bounds
                                .map(|bounds| f32::from(bounds.size.height))
                                .unwrap_or(0.),
                            f32::from(event.position.y),
                        ),
                    };
                    let delta = coordinate - drag.last;
                    match drag.axis {
                        EditorGroupAxis::Horizontal => {
                            app.editor_group_fraction_x =
                                resized_split_fraction(app.editor_group_fraction_x, delta, extent);
                        }
                        EditorGroupAxis::Vertical => {
                            app.editor_group_fraction_y =
                                resized_split_fraction(app.editor_group_fraction_y, delta, extent);
                        }
                    }
                    app.editor_group_drag = Some(EditorGroupDrag {
                        axis: drag.axis,
                        last: coordinate,
                    });
                    cx.notify();
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|app, _, _, cx| {
                        if app.editor_group_drag.take().is_some() {
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|app, _, _, cx| {
                        if app.editor_group_drag.take().is_some() {
                            cx.notify();
                        }
                    }),
                )
                .children(panes);
            let x_height = if group_count == 3 { fraction_y } else { 1.0 };
            groups = groups.child(
                div()
                    .id("editor-group-divider-x")
                    .debug_selector(|| "editor-group-divider-x".into())
                    .absolute()
                    .left(relative(fraction_x))
                    .top_0()
                    .ml(px(-2.5))
                    .w(px(5.))
                    .h(relative(x_height))
                    .bg(if drag_axis == Some(EditorGroupAxis::Horizontal) {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .cursor(CursorStyle::ResizeColumn)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|app, event: &MouseDownEvent, _, cx| {
                            app.editor_group_drag = Some(EditorGroupDrag {
                                axis: EditorGroupAxis::Horizontal,
                                last: f32::from(event.position.x),
                            });
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            );
            if group_count >= 3 {
                groups = groups.child(
                    div()
                        .id("editor-group-divider-y")
                        .debug_selector(|| "editor-group-divider-y".into())
                        .absolute()
                        .top(relative(fraction_y))
                        .left_0()
                        .mt(px(-2.5))
                        .w_full()
                        .h(px(5.))
                        .bg(if drag_axis == Some(EditorGroupAxis::Vertical) {
                            theme.accent
                        } else {
                            theme.border
                        })
                        .cursor(CursorStyle::ResizeRow)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|app, event: &MouseDownEvent, _, cx| {
                                app.editor_group_drag = Some(EditorGroupDrag {
                                    axis: EditorGroupAxis::Vertical,
                                    last: f32::from(event.position.y),
                                });
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ),
                );
            }
            groups
                .child(
                    gpui::canvas(
                        move |bounds, _, cx| {
                            let _ = group_measure.update(cx, |app, _| {
                                app.editor_group_bounds = Some(bounds);
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .into_any_element()
        };
        div()
            .h_full()
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(52.))
                    .flex_shrink_0()
                    .px(px(10.))
                    .pt(px(8.))
                    .pb(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .relative()
                    .child(path)
                    .child(div().flex_1())
                    .child({
                        let at_limit = group_count >= crate::editor_groups::MAX_EDITOR_GROUPS;
                        let mut controls = div().flex().items_center().gap(px(6.));
                        let mut split = div()
                            .id("editor-group-split")
                            .debug_selector(|| "editor-group-split".into())
                            .role(Role::Button)
                            .aria_label(if at_limit {
                                "Vela supports up to four editor groups"
                            } else {
                                "Split editor right"
                            })
                            .h(px(24.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .text_color(if at_limit { theme.faint } else { theme.dim })
                            .opacity(if at_limit { 0.5 } else { 1.0 });
                        if !at_limit {
                            split = split
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.selected).text_color(theme.text))
                                .on_click(cx.listener(|app, _, window, cx| {
                                    app.add_editor_group(window, cx)
                                }));
                        }
                        controls = controls.child(split.child(if at_limit {
                            "Split (4 max)"
                        } else {
                            "Split"
                        }));
                        if group_count > 1 {
                            controls = controls.child(
                                div()
                                    .id("editor-group-close-last")
                                    .debug_selector(|| "editor-group-close-last".into())
                                    .role(Role::Button)
                                    .aria_label("Unsplit editor group")
                                    .h(px(24.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.5))
                                    .text_color(theme.dim)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.selected).text_color(theme.text))
                                    .on_click(cx.listener(|app, _, window, cx| {
                                        app.close_last_editor_group(window, cx)
                                    }))
                                    .child("Unsplit"),
                            );
                        }
                        controls
                    })
                    .when(can_save, |bar| {
                        bar.child(
                            div()
                                .id("editor-save-button")
                                .role(gpui::Role::Button)
                                .aria_label(SharedString::from(format!(
                                    "Save {}",
                                    rel.rsplit('/').next().unwrap_or(&rel)
                                )))
                                .h(px(24.))
                                .px(px(12.))
                                .rounded(px(999.))
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.dim)
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.selected).text_color(theme.text))
                                .on_click(cx.listener(|app, _, _, cx| app.save_editor(cx)))
                                .child("Save"),
                        )
                    })
                    .child(
                        div()
                            .id("editor-status-menu")
                            .relative()
                            .role(Role::Button)
                            .aria_label("Editor status and actions")
                            .aria_expanded(self.status_menu_open)
                            .track_focus(&self.status_menu_focus)
                            .h(px(24.0))
                            .px(px(5.0))
                            .rounded(px(6.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .text_size(px(14.0))
                            .font_family("codicon")
                            .text_color(if self.status_menu_open {
                                theme.text
                            } else {
                                theme.faint
                            })
                            .hover(|style| style.bg(theme.inset))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.status_menu_focus, cx);
                                    this.status_menu_open = !this.status_menu_open;
                                    cx.notify();
                                }),
                            )
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                                    this.status_menu_open = !this.status_menu_open;
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            }))
                            .child(icon("ea7c"))
                            .child(
                                gpui::canvas(
                                    move |bounds, _, cx| {
                                        let _ = status_anchor.update(cx, |app, _| {
                                            app.status_menu_anchor = Some(bounds)
                                        });
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .size_full(),
                            ),
                    ),
            )
            .when(deleted, |root| {
                root.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.red)
                        .bg(theme.red.opacity(0.1))
                        .child("This file was deleted from disk while you were viewing it."),
                )
            })
            .when_some(conflict_banner, |root, banner| root.child(banner))
            .child(div().flex_1().min_h_0().w_full().child(editor_pane))
            .when_some(popover, |root, popover| root.child(popover))
            .into_any_element()
    }

    fn resolve_conflict(
        &mut self,
        key: DocumentKey,
        action: ConflictAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.documents.borrow().is_saving(&key)
            || self.documents.borrow().relocation_contains(&key)
        {
            return;
        }
        self.close_after_save.remove(&key);
        match action {
            ConflictAction::Reload => {
                self.documents.borrow_mut().reload_disk(&key);
            }
            ConflictAction::Merge => self.documents.borrow_mut().keep_editing_to_merge(&key),
            ConflictAction::Overwrite => {
                let Some(session) = self.active_id.clone() else {
                    return;
                };
                if key.workspace != self.document_workspace() {
                    return;
                }
                self.file_request += 1;
                let request =
                    self.documents
                        .borrow_mut()
                        .overwrite(self.file_request, session, &key);
                if let Some(request) = request {
                    self.dispatch_save(request);
                }
            }
        }
        self.file_error = None;
        let editor = self.editor_view(key, cx);
        editor.update(cx, |_, cx| cx.notify());
        window.focus(&editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn render_recovery_notice(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let records = self
            .recovery_records
            .iter()
            .filter(|record| !record.acknowledged)
            .cloned()
            .collect::<Vec<_>>();
        if records.is_empty() {
            return div().into_any_element();
        }
        let session = self.active_id.clone().unwrap_or_default();
        let mut rows = Vec::new();
        for record in records {
            let path = record.recovery_path.clone();
            let open_url = url::Url::from_file_path(&path)
                .ok()
                .map(|url| url.to_string());
            let ack_session = session.clone();
            let ack_id = record.id.clone();
            rows.push(
                div()
                    .border_t_1()
                    .border_color(theme::rgba(255, 255, 255, 0.08))
                    .py(px(9.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(theme.text)
                            .child(record.original_path.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.))
                            .text_color(theme.faint)
                            .child(format!(
                                "{}: {}",
                                record.artifact,
                                record.recovery_path.display()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id(SharedString::from(format!("recovery-open-{}", record.id)))
                                    .role(Role::Button)
                                    .aria_label("Open recovery file")
                                    .h(px(24.))
                                    .px(px(10.))
                                    .rounded(px(999.))
                                    .border_1()
                                    .border_color(theme.border_strong)
                                    .bg(theme.elevated)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .text_color(theme.text)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.inset))
                                    .on_click(cx.listener(move |_, _: &gpui::ClickEvent, _, cx| {
                                        if let Some(url) = open_url.as_deref() {
                                            cx.open_url(url);
                                        }
                                    }))
                                    .child("Open"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("recovery-ack-{}", record.id)))
                                    .role(Role::Button)
                                    .aria_label("Acknowledge recovery file")
                                    .h(px(24.))
                                    .px(px(10.))
                                    .rounded(px(999.))
                                    .border_1()
                                    .border_color(theme.border_strong)
                                    .bg(theme.elevated)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .text_color(theme.text)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.inset))
                                    .on_click(cx.listener(
                                        move |app, _: &gpui::ClickEvent, _, cx| {
                                            app.engine.acknowledge_recovery(
                                                ack_session.clone(),
                                                ack_id.clone(),
                                            );
                                            cx.notify();
                                        },
                                    ))
                                    .child("Acknowledge"),
                            ),
                    ),
            );
        }
        let state = self.recovery_scroll.clone();
        div()
            .id("recovery-notice")
            .role(Role::Region)
            .aria_label("Recovery artifacts")
            .absolute()
            .right(px(16.))
            .bottom(px(16.))
            .w(px(440.))
            .max_h(px(420.))
            .flex()
            .flex_col()
            .rounded(px(12.))
            .border_1()
            .border_color(theme::rgba(224, 175, 104, 0.45))
            .bg(theme::rgba(28, 24, 18, 1.0))
            .text_color(theme::rgba(239, 231, 216, 1.0))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.28).into(),
            }])
            .p(px(14.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Recovery files preserved"),
            )
            .child(
                div()
                    .mt(px(5.))
                    .mb(px(8.))
                    .text_size(px(12.))
                    .text_color(theme::rgba(150, 145, 135, 1.0))
                    .child(
                        "Vela kept displaced or proposed bytes. Review them before acknowledging.",
                    ),
            )
            .child(crate::scroll::AutoScroll::new(
                state,
                move |handle, gutter| {
                    div()
                        .id("recovery-scroll")
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&handle)
                        .pb(gutter)
                        .children(rows)
                        .into_any_element()
                },
            ))
            .into_any_element()
    }

    fn render_conflict_banner(
        &mut self,
        key: DocumentKey,
        change: ExternalChange,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let busy = self.documents.borrow().is_saving(&key)
            || self.documents.borrow().relocation_contains(&key);
        let mut controls = div().flex().items_center().gap(px(6.)).flex_shrink_0();
        let options = [
            (
                "Reload disk version",
                ConflictAction::Reload,
                change.read_error.is_none(),
            ),
            (
                if change.merging {
                    "Save merged content"
                } else {
                    "Keep editing to merge"
                },
                if change.merging {
                    ConflictAction::Overwrite
                } else {
                    ConflictAction::Merge
                },
                true,
            ),
            ("Overwrite disk", ConflictAction::Overwrite, true),
        ];
        for (label, action, available) in options {
            let weak = cx.weak_entity();
            let key = key.clone();
            let mouse_weak = weak.clone();
            let mouse_key = key.clone();
            controls = controls.child(
                div()
                    .id(SharedString::from(format!("conflict-{label}")))
                    .role(gpui::Role::Button)
                    .aria_label(label)
                    .focusable()
                    .tab_stop(true)
                    .px(px(10.))
                    .py(px(4.))
                    .rounded(px(999.))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(theme.elevated)
                    .text_color(if label == "Overwrite disk" {
                        theme.red
                    } else {
                        theme.text
                    })
                    .opacity(if available && !busy { 1. } else { 0.5 })
                    .when(available && !busy, |button| {
                        button
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .on_click(move |_, window, cx| {
                                let _ = mouse_weak.update(cx, |this, cx| {
                                    this.resolve_conflict(mouse_key.clone(), action, window, cx)
                                });
                            })
                            .on_key_down(move |event, window, cx| {
                                if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.resolve_conflict(key.clone(), action, window, cx)
                                    });
                                    cx.stop_propagation();
                                }
                            })
                    })
                    .child(label),
            );
        }
        conflict_banner::render(change, controls, window)
    }

    fn render_settings_header(
        &mut self,
        title: &'static str,
        description: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        div()
            .flex_col()
            .w_full()
            .min_w_0()
            .flex()
            .pb(px(36.))
            .border_b_1()
            .border_color(theme.border_strong)
            .child(
                div()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .gap(px(18.0))
                    .child(
                        div()
                            .size(px(54.0))
                            .flex_shrink_0()
                            .rounded(px(14.0))
                            .border_1()
                            .border_color(theme.border_strong)
                            .bg(theme.panel)
                            .shadow(theme.small_shadow())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(vela_mark(34., theme.accent, theme.text)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().mt(px(3.)).child(tracked_label(
                                "VELA PREFERENCES",
                                9.,
                                0.16,
                                theme.accent,
                            )))
                            .child(
                                div()
                                    .id("settings-heading")
                                    .role(gpui::Role::Heading)
                                    .aria_label(title)
                                    .aria_level(1)
                                    .text_size(px(28.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text)
                                    .child(crate::typography::TrackedText::new(
                                        title,
                                        px(-0.035 * 28.),
                                    )),
                            )
                            .child(
                                div()
                                    .whitespace_normal()
                                    .mt(px(3.))
                                    .text_size(px(12.0))
                                    .text_color(theme.faint)
                                    .child(description),
                            ),
                    )
                    .child(
                        div()
                            .id("settings-back")
                            .role(gpui::Role::Button)
                            .aria_label("Back to workspace")
                            .focusable()
                            .tab_stop(true)
                            .flex_shrink_0()
                            .py(px(8.))
                            .px(px(13.0))
                            .rounded(px(999.0))
                            .border_1()
                            .border_color(theme.border_strong)
                            .bg(theme.panel)
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .text_size(px(13.))
                            .text_color(theme.dim)
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                if event.keystroke.key == "enter" || event.keystroke.key == "space"
                                {
                                    this.settings_open = false;
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            }))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.settings_open = false;
                                    cx.notify();
                                }),
                            )
                            .child("Back to workspace"),
                    ),
            )
            .into_any_element()
    }

    fn render_settings_body(
        &mut self,
        title: &'static str,
        description: &'static str,
        body: AnyElement,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let header = self.render_settings_header(title, description, cx);
        let viewport = window.viewport_size();
        let padding = if f32::from(viewport.width) <= 760. {
            20.
        } else {
            (f32::from(viewport.width) * 0.06).clamp(34., 76.)
        };
        crate::settings::page(
            header,
            body,
            padding,
            self.settings_scroll.clone(),
            self.is_codex(),
        )
    }

    fn provider_scope(&self) -> Option<ProviderScope> {
        let session = self.active_id.clone()?;
        let directory = self.workspace_directory();
        (!directory.is_empty()).then_some(ProviderScope { session, directory })
    }

    fn clear_provider_forms(&mut self, cx: &mut Context<Self>) {
        for (_, handle) in self.provider_oauth_forms.drain() {
            if let Some(flow_id) = handle.form.read(cx).flow_id {
                self.engine.cancel_provider_oauth(flow_id);
            }
            handle.form.update(cx, |form, cx| form.reset(cx));
        }
        for (_, handle) in self.provider_forms.drain() {
            handle
                .form
                .update(cx, |form, cx| form.clear_credentials(cx));
        }
        self.provider_forms_scope = None;
        self.provider_pending.clear();
        self.provider_removals.clear();
        self.provider_remove_errors.clear();
        self.provider_remove_confirmation = None;
    }

    fn remove_provider_credential(
        &mut self,
        target: crate::providers::RemoveCredential,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(scope) = self.provider_scope() else {
            return;
        };
        if self.provider_remove_confirmation.is_some()
            || self
                .provider_removals
                .values()
                .any(|pending| pending.target.provider == target.provider)
            || self
                .provider_forms
                .get(&target.provider)
                .is_some_and(|handle| handle.form.read(cx).busy())
            || !self.provider_catalog.providers.iter().any(|provider| {
                provider.id == target.provider
                    && provider
                        .credentials
                        .iter()
                        .any(|credential| credential.id == target.credential)
            })
        {
            return;
        }
        self.file_request += 1;
        let request_id = self.file_request;
        self.provider_remove_confirmation = Some(request_id);
        let confirmation = window.prompt(
            gpui::PromptLevel::Info,
            &format!("Remove this {} credential?", target.name),
            None,
            &[
                gpui::PromptButton::ok("OK"),
                gpui::PromptButton::cancel("Cancel"),
            ],
            cx,
        );
        cx.spawn(async move |this, cx| {
            let accepted = confirmation.await == Ok(0);
            let _ = this.update(cx, |this, cx| {
                if this.provider_remove_confirmation != Some(request_id) {
                    return;
                }
                this.provider_remove_confirmation = None;
                if !accepted
                    || this.provider_scope().as_ref() != Some(&scope)
                    || !this.settings_open
                    || this.settings_section != SettingsSection::Providers
                {
                    return;
                }
                this.provider_remove_errors.remove(&target.provider);
                if let Some(form) = this.provider_forms.get(&target.provider) {
                    form.form.update(cx, |form, cx| form.set_busy(cx));
                }
                this.engine
                    .remove_provider_credential(vela_engine::ProviderCredentialRemoval {
                        request_id,
                        session_id: scope.session.clone(),
                        directory: scope.directory.clone(),
                        integration_id: target.provider.clone(),
                        credential_id: target.credential.clone(),
                    });
                this.provider_removals.insert(
                    request_id,
                    crate::providers::PendingRemoval { scope, target },
                );
                cx.notify();
            });
        })
        .detach();
    }

    fn open_provider_form(
        &mut self,
        provider: vela_opencode::providers::ProviderIntegration,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(scope) = self.provider_scope() else {
            return;
        };
        if !self.settings_open || self.settings_section != SettingsSection::Providers {
            return;
        }
        if provider.key_method.is_none() {
            if provider.oauth.is_empty() || self.provider_oauth_forms.contains_key(&provider.id) {
                return;
            }
            use crate::provider_oauth::{OAuthEvent, OAuthForm, OAuthHandle};
            let id = provider.id.clone();
            let form_id = id.clone();
            let captured_scope = scope.clone();
            let form = cx.new(|cx| OAuthForm::new(provider, self.settings_scroll.clone(), cx));
            let subscription = cx.subscribe(&form, move |this, form, event: &OAuthEvent, cx| {
                if this.provider_scope().as_ref() != Some(&captured_scope)
                    || !this.settings_open
                    || this.settings_section != SettingsSection::Providers
                {
                    return;
                }
                match event {
                    OAuthEvent::Start(method) => {
                        this.file_request += 1;
                        let flow_id = this.file_request;
                        form.update(cx, |form, cx| form.begin(flow_id, cx));
                        this.engine
                            .start_provider_oauth(vela_engine::ProviderOAuthRequest {
                                flow_id,
                                session_id: captured_scope.session.clone(),
                                directory: captured_scope.directory.clone(),
                                integration_id: id.clone(),
                                method_id: method.clone(),
                            });
                    }
                    OAuthEvent::Code(code) => {
                        if let Some(flow_id) = form.read(cx).flow_id {
                            this.engine.complete_provider_oauth(flow_id, code.clone());
                        }
                    }
                    OAuthEvent::Cancel => {
                        if let Some(flow_id) = form.read(cx).flow_id {
                            this.engine.cancel_provider_oauth(flow_id);
                        }
                        form.update(cx, |form, cx| form.reset(cx));
                    }
                }
            });
            self.provider_oauth_forms.insert(
                form_id,
                OAuthHandle {
                    form,
                    scope,
                    _subscription: subscription,
                },
            );
            cx.notify();
            return;
        }
        if let Some(handle) = self.provider_forms.get(&provider.id) {
            if let Some(bounds) = handle.form.update(cx, |form, cx| form.focus(window, cx)) {
                self.settings_scroll.reveal(bounds);
                window.refresh();
            }
            return;
        }
        let id = provider.id.clone();
        let form_id = id.clone();
        let form = cx.new(|cx| ProviderForm::new(provider, self.settings_scroll.clone(), cx));
        if self
            .provider_removals
            .values()
            .any(|pending| pending.target.provider == form_id)
        {
            form.update(cx, |form, cx| form.set_busy(cx));
        } else if let Some(error) = self.provider_remove_errors.get(&form_id) {
            form.update(cx, |form, cx| form.set_result(Some(error.clone()), cx));
        }
        let subscription = cx.subscribe(&form, move |this, form, event: &ProviderFormEvent, cx| {
            if this.provider_scope().as_ref() != Some(&scope)
                || !this.settings_open
                || this.settings_section != SettingsSection::Providers
            {
                return;
            }
            match event {
                ProviderFormEvent::Cancel => {
                    form.update(cx, |form, cx| form.clear_credentials(cx));
                    this.provider_forms.remove(&id);
                    cx.notify();
                }
                ProviderFormEvent::Submit {
                    key,
                    label,
                    answers,
                } => {
                    this.provider_remove_errors.remove(&id);
                    this.file_request += 1;
                    let request_id = this.file_request;
                    this.provider_pending.insert(
                        request_id,
                        ProviderPending {
                            scope: scope.clone(),
                            provider_id: id.clone(),
                            form: form.downgrade(),
                        },
                    );
                    form.update(cx, |form, cx| form.set_busy(cx));
                    this.engine
                        .connect_provider_key(vela_engine::ProviderKeyConnection {
                            request_id,
                            session_id: scope.session.clone(),
                            directory: scope.directory.clone(),
                            integration_id: id.clone(),
                            key: key.clone(),
                            label: label.clone(),
                            answers: answers.clone(),
                        });
                }
            }
        });
        // The new field must be in the dispatch tree before it receives focus.
        // Focusing it during the button's mouse-down can be replaced by that
        // button's own focus processing in the current frame.
        let weak_form = form.downgrade();
        let scroll = self.settings_scroll.clone();
        window.on_next_frame(move |window, cx| {
            if let Ok(Some(bounds)) = weak_form.update(cx, |form, cx| form.focus(window, cx)) {
                scroll.reveal(bounds);
                window.refresh();
            }
        });
        self.provider_forms.insert(
            form_id,
            ProviderFormHandle {
                form,
                _subscription: subscription,
            },
        );
        cx.notify();
    }

    fn render_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.settings_section == SettingsSection::Providers {
            let scope = self.provider_scope();
            if self.provider_forms_scope != scope {
                self.clear_provider_forms(cx);
                self.provider_forms_scope = scope.clone();
            }
            if let Some(scope) = scope
                && self.provider_catalog.needs_load(&scope)
            {
                self.file_request += 1;
                let request = self.file_request;
                let session = scope.session.clone();
                self.provider_catalog.begin(request, scope);
                self.engine.load_providers(request, session);
            }
        }
        match self.settings_section {
            SettingsSection::Appearance => {
                let theme = Theme::default();
                let page = self.render_appearance_settings(window, cx);
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .bg(theme.editor)
                    .overflow_hidden()
                    .child(page)
                    .into_any_element()
            }
            SettingsSection::Providers => self.render_settings_providers(window, cx),
            SettingsSection::Voice => self.render_settings_voice(window, cx),
            SettingsSection::Model => self.render_settings_model(window, cx),
            SettingsSection::Servers => self.render_settings_servers(window, cx),
            SettingsSection::Mobile => self.render_settings_mobile(window, cx),
            SettingsSection::About => self.render_settings_about(window, cx),
        }
    }

    fn render_settings_voice(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let badge = crate::settings::badge("Unavailable");
        let body = settings_list(setting_row(
            "Voice input",
            "Voice transcription is not included in this build. Controls will appear here when a voice service is available.",
            badge,
        ));
        self.render_settings_body(
            "Voice",
            "Configure voice input preferences and review availability.",
            body,
            window,
            cx,
        )
    }

    fn render_settings_about(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let body = div().w_full().min_w_0().flex().flex_col()
            .child(div().w_full().min_w_0().p(px(26.)).rounded(px(12.)).border_1()
                .border_color(theme.border).bg(theme.panel).flex().items_center().gap(px(24.))
                .child(vela_mark(72., theme.accent, theme.text))
                .child(div().flex_1().min_w_0().flex().flex_col()
                    .child(div().text_size(px(24.)).font_weight(FontWeight::BOLD).child("Vela"))
                    .child(div().my(px(4.)).text_size(px(12.)).text_color(theme.dim)
                        .child(concat!("Version ", env!("CARGO_PKG_VERSION"), " · Development/test build")))
                    .child(div().text_size(px(13.)).text_color(theme.faint)
                        .child("A native desktop cockpit for coding agents."))))
            .child(settings_group_title("APP UPDATES"))
            .child(settings_list(setting_row("Development build",
                "GitHub app updates are disabled here. Develop and test this local checkout; source edits do not update the installed app through the release updater.",
                div().w(px(80.)).flex_shrink_0().whitespace_normal().text_size(px(10.5)).text_color(theme.faint).child("Updater disabled"))));
        self.render_settings_body(
            "About",
            "Version and product information for this installation.",
            body.into_any_element(),
            window,
            cx,
        )
    }

    fn render_settings_providers(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.runtime_capabilities.provider_credentials {
            let theme = Theme::default();
            let body = div()
                .id("settings-provider-managed")
                .debug_selector(|| "settings-provider-managed".into())
                .role(gpui::Role::Group)
                .aria_label("Provider credentials managed by OpenCode")
                .w_full()
                .min_w_0()
                .p(px(26.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.panel)
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::BOLD)
                        .child("Managed by OpenCode"),
                )
                .child(
                    div()
                        .whitespace_normal()
                        .text_size(px(12.))
                        .text_color(theme.dim)
                        .child("This runtime does not expose provider credential editing through Vela. Configure credentials in the runtime, then refresh its model list here."),
                )
                .into_any_element();
            return self.render_settings_body(
                "Providers",
                "Connect model services supported by OpenCode.",
                body,
                window,
                cx,
            );
        }
        let workspace_open = self.active_id.is_some() && !self.workspace_directory().is_empty();
        let weak = cx.weak_entity();
        let remove_weak = weak.clone();
        let mut busy = self
            .provider_forms
            .iter()
            .filter(|(_, handle)| handle.form.read(cx).busy())
            .map(|(id, _)| id.clone())
            .collect::<HashSet<_>>();
        busy.extend(
            self.provider_removals
                .values()
                .map(|pending| pending.target.provider.clone()),
        );
        let body = crate::providers::catalog_view(
            &self.provider_catalog,
            workspace_open,
            f32::from(window.viewport_size().width),
            &self.provider_forms,
            &self.provider_oauth_forms,
            crate::providers::ProviderHandlers {
                open: Rc::new(move |provider, window, cx| {
                    let _ =
                        weak.update(cx, |this, cx| this.open_provider_form(provider, window, cx));
                }),
                remove: Rc::new(move |target, window, cx| {
                    let _ = remove_weak.update(cx, |this, cx| {
                        this.remove_provider_credential(target, window, cx)
                    });
                }),
                busy,
                errors: self.provider_remove_errors.clone(),
            },
            self.settings_scroll.clone(),
        );
        self.render_settings_body(
            "Providers",
            "Connect model services supported by OpenCode.",
            body,
            window,
            cx,
        )
    }

    fn render_settings_mobile(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut steps = div().flex().flex_col();
        for (number, title, text) in [
            (
                "01",
                "Prepare this Mac",
                "Leave Vela open. This desktop starts the mobile server automatically and stops it when you quit.",
            ),
            (
                "02",
                "Open the mobile companion app",
                "On your phone, open the mobile companion app and choose Scan QR code on its connection screen.",
            ),
            (
                "03",
                "Scan the code from this page",
                "The code works once and expires after 10 minutes. It contains no reusable password or device token.",
            ),
        ] {
            steps = steps.child(
                div()
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .gap(px(16.0))
                    .child(
                        div()
                            .w(px(28.0))
                            .text_size(px(11.0))
                            .font_family(theme::FONT_MONO)
                            .text_color(theme.accent)
                            .child(number),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .whitespace_normal()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .whitespace_normal()
                                    .text_size(px(10.5))
                                    .line_height(px(16.0))
                                    .text_color(theme.dim)
                                    .child(text),
                            ),
                    ),
            );
        }
        let left = div()
            .w(px(450.0))
            .flex_shrink_0()
            .min_w_0()
            .overflow_hidden()
            .whitespace_normal()
            .rounded(px(14.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.panel)
            .px(px(24.0))
            .py(px(22.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(8.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.accent)
                    .child("VELA · PHONE COMPANION"),
            )
            .child(
                div()
                    .whitespace_normal()
                    .text_size(px(26.0))
                    .font_weight(FontWeight::BOLD)
                    .line_height(px(30.0))
                    .text_color(theme.text)
                    .child("Continue your workspace on your phone."),
            )
            .child(
                div()
                    .whitespace_normal()
                    .text_size(px(11.0))
                    .line_height(px(17.0))
                    .text_color(theme.dim)
                    .child("Vela Desktop starts an authenticated companion server while this app is open. Pair once; your phone then remembers this desktop. When the desktop OpenCode service is available, conversations are shared across both apps."),
            )
            .child(steps)
            .child(
                div()
                    .pt(px(4.0))
                    .whitespace_normal()
                    .text_size(px(9.5))
                    .line_height(px(15.0))
                    .text_color(theme.faint)
                    .child("The code includes this Mac's reachable network address and a relay route when available. For manual setup, use an address the phone can reach—never localhost."),
            );
        let right = div()
            .w(px(390.0))
            .flex_shrink_0()
            .whitespace_normal()
            .rounded(px(14.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.panel)
            .px(px(24.0))
            .py(px(22.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(8.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.faint)
                    .child("PAIR A DEVICE"),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Connect the mobile companion"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .text_size(px(10.0))
                    .text_color(theme.dim)
                    .child(div().size(px(5.0)).rounded(px(2.5)).bg(theme.green))
                    .child("Ready to pair"),
            )
            .child(div().h(px(1.0)).w_full().bg(theme.border).my(px(4.0)))
            .child(
                div()
                    .flex_1()
                    .min_h(px(150.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(54.0))
                    .font_family(theme::FONT_MONO)
                    .text_color(theme.accent_soft)
                    .child("⌜     ⌝\n       \n⌞     ⌟"),
            )
            .child(
                div()
                    .text_center()
                    .text_size(px(10.0))
                    .text_color(theme.faint)
                    .child("Your pairing code will appear here."),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .line_height(px(15.0))
                    .text_color(theme.faint)
                    .child("The desktop companion server is running. Generate a one-time code for your phone."),
            )
            .child(
                div()
                    .id("generate-pairing")
                    .h(px(34.0))
                    .w_full()
                    .rounded(px(8.0))
                    .bg(theme.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.bg)
                    .child("Generate pairing QR"),
            );
        let body = div()
            .w_full()
            .flex()
            .gap(px(16.0))
            .items_start()
            .child(left)
            .child(right);
        self.render_settings_body(
            "Mobile Setup",
            "Prepare secure access to Vela from another device.",
            body.into_any_element(),
            window,
            cx,
        )
    }

    fn render_appearance_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let padding = (f32::from(window.viewport_size().width) * 0.04).clamp(20., 48.);
        let hover = if self.ui_shell == UiShell::Codex {
            theme.text
        } else {
            theme.accent
        }
        .opacity(0.07);
        let themes = vela_core::appearance::profiles();
        let mut cards = Vec::new();
        for (ix, profile) in themes.iter().enumerate() {
            let selected =
                vela_core::appearance::profile(&self.settings.appearance).id == profile.id;
            let name = profile.name.as_str();
            let base_color = theme::color(&profile.colors["base"]);
            let preview_muted = theme::color(&profile.colors["muted"]);
            let line = preview_muted.opacity(0.24);
            let soft_line = preview_muted.opacity(0.18);
            let accent_color = theme::color(&profile.colors["accent"]);
            let pane_color = theme::color(&profile.colors["pane"]);
            let appearance_id = profile.id.clone();
            let focus = self.appearance_grid.focus(&profile.id, cx);
            let click_focus = focus.clone();
            cards.push(
                crate::scroll::FocusReveal::element(
                    SharedString::from(format!("appearance-theme-reveal-{ix}")),
                    self.settings_scroll.clone(),
                    focus.clone(),
                    div()
                        .id(SharedString::from(format!("theme-{ix}")))
                        .role(gpui::Role::RadioButton)
                        .aria_label(SharedString::from(name.to_owned()))
                        .aria_toggled(if selected {
                            gpui::Toggled::True
                        } else {
                            gpui::Toggled::False
                        })
                        .tab_index(0)
                        .track_focus(&focus)
                        .map(|card| {
                            crate::focus::outline_style(
                                card,
                                &focus,
                                crate::focus::OutlineStyle {
                                    radius: 10.,
                                    width: 2.,
                                    border: 1.,
                                    offset: 3.,
                                },
                                theme.accent.into(),
                                &self.picker_anchors.keyboard,
                            )
                        })
                        .w_full()
                        .min_w_0()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            window.focus(&click_focus, cx);
                            this.settings.appearance = appearance_id.clone();
                            theme::set_appearance(&appearance_id);
                            crate::platform_menu::sync_appearance(Theme::default().dark);
                            this.persist_preferences(cx);
                            cx.notify();
                        }))
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(if selected { theme.dim } else { theme.border })
                        .bg(if selected {
                            hover
                        } else {
                            theme::rgba(0, 0, 0, 0.)
                        })
                        .hover(|style| {
                            style.bg(hover).border_color(if selected {
                                theme.dim
                            } else {
                                theme.border_strong
                            })
                        })
                        .p(px(8.0))
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(
                            div()
                                .h(px(68.0))
                                .w_full()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(line)
                                .bg(base_color)
                                .p(px(4.0))
                                .flex()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .w(px(8.0))
                                        .h_full()
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(preview_muted.opacity(0.22))
                                        .bg(base_color),
                                )
                                .child(
                                    div()
                                        .w(gpui::relative(0.22))
                                        .h_full()
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(soft_line)
                                        .bg(pane_color),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .h_full()
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .child(
                                            div()
                                                .flex_1()
                                                .rounded(px(4.0))
                                                .border_1()
                                                .border_color(soft_line)
                                                .bg(theme::color(&profile.colors["editor"]))
                                                .p(px(6.0))
                                                .flex()
                                                .flex_col()
                                                .gap(px(4.0))
                                                .child(
                                                    div()
                                                        .h(px(3.0))
                                                        .w(gpui::relative(0.78))
                                                        .rounded(px(2.0))
                                                        .bg(accent_color.opacity(0.72)),
                                                )
                                                .child(
                                                    div()
                                                        .h(px(3.0))
                                                        .w(gpui::relative(0.48))
                                                        .rounded(px(2.0))
                                                        .bg(accent_color.opacity(0.35)),
                                                )
                                                .child(
                                                    div()
                                                        .h(px(3.0))
                                                        .w(gpui::relative(0.64))
                                                        .rounded(px(2.0))
                                                        .bg(accent_color.opacity(0.5)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .h(gpui::relative(0.32))
                                                .rounded(px(4.0))
                                                .border_1()
                                                .border_color(soft_line)
                                                .bg(base_color),
                                        ),
                                )
                                .child(
                                    div()
                                        .w(gpui::relative(0.27))
                                        .h_full()
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(soft_line)
                                        .bg(theme::color(&profile.colors["agent"])),
                                ),
                        )
                        .child(
                            div()
                                .h(px(16.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child(SharedString::from(name.to_owned())),
                                )
                                .child(if selected {
                                    div()
                                        .text_size(px(13.0))
                                        .font_family("codicon")
                                        .text_color(theme.text)
                                        .child(icon("eab2"))
                                        .into_any_element()
                                } else {
                                    div().into_any_element()
                                }),
                        )
                        .into_any_element(),
                )
                .into_any_element(),
            );
        }
        let cards = self.appearance_grid.render(cards, cx.entity_id());
        let shell_choice = |shell: UiShell, label: &'static str, id: &'static str| {
            let selected = self.ui_shell == shell;
            div()
                .id(id)
                .role(gpui::Role::RadioButton)
                .aria_label(label)
                .aria_toggled(if selected {
                    gpui::Toggled::True
                } else {
                    gpui::Toggled::False
                })
                .tab_index(0)
                .px(px(12.))
                .py(px(6.))
                .rounded(px(7.))
                .cursor_pointer()
                .bg(if selected {
                    theme.panel
                } else {
                    theme::rgba(0, 0, 0, 0.)
                })
                .border_1()
                .border_color(if selected {
                    theme.border
                } else {
                    theme::rgba(0, 0, 0, 0.)
                })
                .text_size(px(12.))
                .text_color(theme.text)
                .on_click(cx.listener(move |this, _, _, cx| this.set_shell(shell, cx)))
                .child(label)
        };
        let art = self.settings.experimental_art;
        let header = div()
            .h(px(30.))
            .flex_shrink_0()
            .w_full()
            .max_w(px(760.))
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .text_size(px(22.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .id("appearance-heading")
                    .role(gpui::Role::Heading)
                    .aria_level(1)
                    .aria_label("Appearance")
                    .child(crate::typography::TrackedText::new("Appearance", px(-0.55))),
            )
            .child(
                div()
                    .id("settings-close")
                    .role(gpui::Role::Button)
                    .aria_label("Back to workspace")
                    .tab_index(0)
                    .size(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .font_family("codicon")
                    .text_size(px(14.))
                    .text_color(theme.dim)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.settings_open = false;
                        window.focus(&this.composer_focus, cx);
                        cx.notify();
                    }))
                    .child(icon("ea76")),
            )
            .into_any_element();
        let body = div()
            .mt(px(24.))
            .pb(px(24.))
            .flex_shrink_0()
            .w_full()
            .max_w(px(760.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap(px(16.))
                    .pb(px(24.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Interface"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.dim)
                                    .line_height(px(18.))
                                    .child("Choose your workspace layout."),
                            ),
                    )
                    .child(
                        div()
                            .h(px(38.))
                            .flex()
                            .items_center()
                            .rounded(px(9.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .p(px(2.))
                            .child(shell_choice(
                                UiShell::Codex,
                                "Codex interface",
                                "shell-codex",
                            ))
                            .child(shell_choice(
                                UiShell::Classic,
                                "Classic interface",
                                "shell-classic",
                            )),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Experimental art"),
                                    )
                                    .child(
                                        div().text_size(px(11.)).text_color(theme.dim).child(
                                            "Show optional artwork in the empty agent panel.",
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("experimental-art")
                                    .role(gpui::Role::Switch)
                                    .aria_label("Experimental art")
                                    .aria_toggled(if art {
                                        gpui::Toggled::True
                                    } else {
                                        gpui::Toggled::False
                                    })
                                    .tab_index(0)
                                    .w(px(34.))
                                    .h(px(19.))
                                    .flex_shrink_0()
                                    .rounded(px(99.))
                                    .p(px(2.))
                                    .bg(if art {
                                        theme.accent.opacity(0.18)
                                    } else {
                                        theme.inset
                                    })
                                    .flex()
                                    .items_center()
                                    .when(art, |s| s.justify_end())
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.settings.experimental_art =
                                            !this.settings.experimental_art;
                                        this.persist_preferences(cx);
                                        cx.notify();
                                    }))
                                    .child(div().size(px(15.)).rounded(px(99.)).bg(if art {
                                        theme.accent
                                    } else {
                                        theme.faint
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .pt(px(24.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Color theme"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.dim)
                            .line_height(px(18.))
                            .child("Applies to your workspace, editor and terminal."),
                    ),
            )
            .child(cards)
            .into_any_element();
        crate::settings::appearance_page(header, body, padding, self.settings_scroll.clone())
    }

    fn staged_revert_active(&self) -> bool {
        self.active_id
            .as_ref()
            .is_some_and(|session| self.staged_reverts.contains(session))
    }

    fn render_staged_revert(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        if !self.staged_revert_active() {
            return div().into_any_element();
        }
        let button = |id: &'static str, label: &'static str, danger: bool| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .role(gpui::Role::Button)
                .aria_label(label)
                .focusable()
                .tab_stop(true)
                .h(px(26.0))
                .px(px(10.0))
                .rounded(px(6.0))
                .bg(if danger {
                    theme.red.opacity(0.16)
                } else {
                    theme.elevated
                })
                .text_color(if danger { theme.red } else { theme.text })
                .text_size(px(10.5))
                .cursor_pointer()
                .flex()
                .items_center()
                .child(label)
        };
        let busy = self
            .active_id
            .as_deref()
            .is_some_and(|session| self.revert_in_flight(session));
        let mut commit = button("staged-revert-commit", "Commit undo", true);
        let mut discard = button("staged-revert-discard", "Discard", false);
        if busy {
            commit = commit.opacity(0.45);
            discard = discard.opacity(0.45);
        } else {
            commit = commit.on_click(cx.listener(|app, _, _, cx| app.commit_staged_revert(cx)));
            discard = discard.on_click(cx.listener(|app, _, _, cx| app.discard_staged_revert(cx)));
        }
        div()
            .id("staged-revert-dock")
            .debug_selector(|| "staged-revert-dock".into())
            .role(gpui::Role::Region)
            .aria_label("Undo staged")
            .w_full()
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.red.opacity(0.55))
            .bg(theme.inset)
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(7.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.red)
                    .child("Undo staged"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(theme.dim)
                    .whitespace_normal()
                    .child("Messages and file changes after this point will be removed."),
            )
            .child(div().flex().gap(px(6.0)).child(commit).child(discard))
            .into_any_element()
    }

    fn render_permissions(&self, cx: &mut Context<Self>) -> AnyElement {
        let session_id = self.active_id.clone().unwrap_or_default();
        self.render_permission_dock(&self.permissions, &session_id, "permission", cx)
    }

    /// Permission cards are scoped to one session so simultaneous panels can
    /// each answer their own prompts. `namespace` keeps element ids unique
    /// across tiled copies.
    fn render_permission_dock(
        &self,
        requests: &[PermissionRequest],
        session_id: &str,
        namespace: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        if requests.is_empty() {
            return div().into_any_element();
        }
        let weak = cx.weak_entity();
        let requests = requests.to_vec();
        let session_id = session_id.to_owned();
        let mut column = div().flex().flex_col().gap(px(6.0)).w_full();
        for (ix, request) in requests.into_iter().enumerate() {
            let request_id = request.id.clone();
            let session = session_id.clone();
            let weak_allow = weak.clone();
            let weak_always = weak.clone();
            let weak_deny = weak.clone();
            let allow_session = session.clone();
            let allow_request = request_id.clone();
            let always_session = session.clone();
            let always_request = request_id.clone();
            let deny_session = session.clone();
            let deny_request = request_id.clone();
            let resources = if request.resources.is_empty() {
                request.message.clone().unwrap_or_default()
            } else {
                request.resources.join(" · ")
            };
            let card_id = format!("{namespace}-{ix}");
            let debug_card = card_id.clone();
            column = column.child(
                div()
                    .id(SharedString::from(card_id))
                    .debug_selector(move || debug_card.clone())
                    .w_full()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(theme.yellow)
                    .bg(theme.inset)
                    .p(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(7.0))
                    .child(
                        div().flex().items_center().gap(px(7.0)).child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.yellow)
                                .child(SharedString::from(format!(
                                    "Permission · {}",
                                    request.action
                                ))),
                        ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.dim)
                            .child(SharedString::from(resources)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(permission_button(
                                SharedString::from(format!("{namespace}-button-Allow")),
                                "Allow",
                                theme.accent,
                                theme.bg,
                                move |cx| {
                                    let _ = weak_allow.update(cx, |app, cx| {
                                        app.engine.reply_permission(
                                            allow_session.clone(),
                                            allow_request.clone(),
                                            PermissionReply::Allow,
                                        );
                                        cx.notify();
                                    });
                                },
                            ))
                            .child(permission_button(
                                SharedString::from(format!("{namespace}-button-Always")),
                                "Always",
                                theme.elevated,
                                theme.text,
                                move |cx| {
                                    let _ = weak_always.update(cx, |app, cx| {
                                        app.engine.reply_permission(
                                            always_session.clone(),
                                            always_request.clone(),
                                            PermissionReply::AllowAlways,
                                        );
                                        cx.notify();
                                    });
                                },
                            ))
                            .child(permission_button(
                                SharedString::from(format!("{namespace}-button-Deny")),
                                "Deny",
                                theme.elevated,
                                theme.red,
                                move |cx| {
                                    let _ = weak_deny.update(cx, |app, cx| {
                                        app.engine.reply_permission(
                                            deny_session.clone(),
                                            deny_request.clone(),
                                            PermissionReply::Deny,
                                        );
                                        cx.notify();
                                    });
                                },
                            )),
                    ),
            );
        }
        column.into_any_element()
    }
}

impl Render for VelaApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_pending_editor_jump(window, cx);
        self.advance_workspace_closes(cx);
        self.synchronize_terminal_workspace(cx);
        if self.terminal_focus_pending && self.tray.open {
            self.terminal_focus_pending = false;
            if let Some(terminal) = self.terminals.active() {
                window.focus(&terminal.focus, cx);
            }
        }
        if self.editor_focus_pending {
            self.editor_focus_pending = false;
            if let Some(key) = self.active_document_key() {
                let editor = self.editor_view(key, cx);
                window.focus(&editor.focus_handle(cx), cx);
            } else {
                window.focus(&self.composer_focus, cx);
            }
        }
        if self.pump_entry_renames(cx) && self.closing.is_some() {
            let owner = cx.weak_entity();
            window.on_next_frame(move |window, cx| {
                let _ = owner.update(cx, |app, cx| app.advance_close(window, cx));
            });
        }
        if let Some((session, directory, rel)) = self.file_workspace_to_open.clone()
            && self.active_id.as_deref() == Some(session.as_str())
            && self.workspace_directory() == directory
        {
            self.file_workspace_to_open = None;
            self.sidebar_tab = SidebarTab::Files;
            self.open_file_tab(rel, true, window, cx);
        }
        for path in std::mem::take(&mut self.external_files_to_open) {
            let key = DocumentKey::in_workspace(
                self.document_workspace(),
                self.workspace_directory(),
                path,
            );
            self.sidebar_tab = SidebarTab::Files;
            self.open_document_tab(key, true, window, cx);
        }
        if let Some(key) = self.review_file_to_open.take()
            && key.workspace == self.document_workspace()
            && !self.settings_open
        {
            self.open_file_tab(key.rel, false, window, cx);
        }
        let mut created = std::mem::take(&mut self.created_files_to_open);
        let mut retained = Vec::new();
        for key in created.drain(..) {
            if key.workspace == self.document_workspace() && !self.settings_open {
                self.open_file_tab(key.rel, true, window, cx);
            } else {
                retained.push(key);
            }
        }
        self.created_files_to_open = retained;
        if !self.focused_once {
            self.focused_once = true;
            window.focus(&self.composer_focus, cx);
        }
        if std::mem::take(&mut self.composer_focus_pending) {
            window.focus(&self.composer_focus, cx);
        }
        if let Some(anchor) = self.picker_anchors.take_pending() {
            if self.settings_open {
                window.focus(&self.settings_model_focus, cx);
            } else if self.agent_panel_open {
                window.focus(&anchor, cx);
            }
        }
        let theme = Theme::default();
        if !self.bootstrapped {
            self.connection = self.engine.connection();
            let sessions = Arc::new(self.engine.sessions());
            if sessions != self.sessions {
                self.sessions = sessions;
                self.refresh_session_filter();
            }
            if self.connection == ConnectionState::Ready {
                self.bootstrap(cx);
            }
        }
        self.refresh_active();
        self.ensure_changes_view(cx);
        if !self.automation_done
            && let Ok(action) = std::env::var("VELA_UI_ACTION")
            && self.active_id.is_some()
        {
            self.automation_done = true;
            if action == "terminal" {
                self.toggle_terminal(window, cx);
            } else if action == "sass" {
                if !self.tray.open || self.tray.snapped {
                    self.toggle_terminal(window, cx);
                }
                self.terminal_panel = TerminalPanel::Sass;
            } else if action == "sasswatch" {
                if !self.tray.open || self.tray.snapped {
                    self.toggle_terminal(window, cx);
                }
                self.terminal_panel = TerminalPanel::Sass;
                if let Some(id) = self.active_id.clone() {
                    self.sass.enabled = true;
                    self.engine.set_sass_watch(id, true);
                }
            } else if action == "problems" {
                if !self.tray.open || self.tray.snapped {
                    self.toggle_terminal(window, cx);
                }
                self.terminal_panel = TerminalPanel::Problems;
            } else if action == "browser" {
                self.toggle_browser(window, cx);
            } else if action == "files" {
                self.switch_sidebar(SidebarTab::Files, cx);
            } else if action == "sessions" {
                self.switch_sidebar(SidebarTab::Sessions, cx);
            } else if action == "settings" {
                self.settings_open = true;
            } else if let Some(section) = action.strip_prefix("settings:") {
                self.settings_open = true;
                self.settings_section = match section {
                    "providers" => SettingsSection::Providers,
                    "voice" => SettingsSection::Voice,
                    "model" => SettingsSection::Model,
                    "servers" => SettingsSection::Servers,
                    "mobile" => SettingsSection::Mobile,
                    "about" => SettingsSection::About,
                    _ => SettingsSection::Appearance,
                };
            } else if action == "classic" {
                self.set_shell(UiShell::Classic, cx);
            } else if action == "codex" {
                self.set_shell(UiShell::Codex, cx);
            } else if action == "picker" {
                self.picker = Some(PickerKind::Model);
            } else if action == "changes" {
                self.toggle_changes(cx);
            } else if let Some(rel) = action.strip_prefix("diff:") {
                self.refresh_changes(cx);
                self.open_diff(rel.to_string(), cx);
            } else if let Some(rel) = action.strip_prefix("file:") {
                self.sidebar_tab = SidebarTab::Files;
                self.refresh_files(cx);
                self.open_file(rel.to_string(), window, cx);
            } else if let Some(id) = action.strip_prefix("session:") {
                self.open_session(id.to_string(), cx);
            }
        }
        self.record_nav_sample(window, cx);

        let show_runtime = self.settings_open;
        if show_runtime && !self.runtime_probe_visible && self.runtime_action.is_none() {
            self.file_request += 1;
            self.runtime_probe = Some(self.file_request);
            self.engine.load_runtime_info(self.file_request);
        }
        self.runtime_probe_visible = show_runtime;

        if !self.settings_open || self.settings_section != SettingsSection::Providers {
            self.provider_catalog.clear();
            self.clear_provider_forms(cx);
        }

        let rail = self.render_rail(cx);
        let titlebar = self.render_titlebar(cx);
        let body = if self.settings_open {
            let settings = self.render_settings(window, cx);
            let nav = self.render_settings_nav(cx);
            crate::settings::shell(rail, nav, settings, self.is_codex())
        } else if self.is_codex() {
            if self.sidebar_open {
                self.render_codex_shell(window, cx)
            } else {
                let rail = self.render_rail(cx);
                let workspace = self.render_workspace(window, cx);
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .bg(theme.top)
                    .child(rail)
                    .child(codex_card(
                        div().flex_1().min_w_0().h_full().flex().child(workspace),
                    ))
                    .into_any_element()
            }
        } else if self.sidebar_open {
            self.render_classic_shell(window, cx)
        } else {
            let workspace = self.render_workspace(window, cx);
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .gap(px(10.0))
                .child(rail)
                .child(workspace)
                .into_any_element()
        };

        let connection_recovery = self.render_connection_recovery(cx);
        div()
            .size_full()
            .relative()
            .bg(theme.bg)
            .text_color(theme.text)
            .font(theme::ui_font())
            .on_action(
                cx.listener(|this, _: &crate::input::RequestQuit, window, cx| {
                    this.quit_if_allowed(window, cx);
                }),
            )
            .on_action(
                cx.listener(|_, _: &crate::input::MinimizeWindow, window, _| {
                    window.minimize_window();
                }),
            )
            .on_action(cx.listener(|_, _: &crate::input::ZoomWindow, window, _| {
                window.zoom_window();
            }))
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.picker_anchors.keyboard.key_down() {
                    cx.notify();
                }
                if this.navigate_session_context(event, window, cx) {
                    return;
                }
                if this.navigate_explorer_menu(event, window, cx) {
                    return;
                }
                if attachments::cancel_drag(event, window, cx) {
                    this.attachment_drag_over = false;
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this
                    .transcript_ui
                    .navigate(event, &this.transcript_list, window)
                {
                    this.stick =
                        crate::scroll::list_distance_from_bottom(&this.transcript_list) <= px(24.);
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "escape" && this.dismiss_tab_overflow() {
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "escape"
                    && let Some(kind) = this.picker.take()
                {
                    this.picker_anchors.pending = Some(kind);
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "escape" && this.saved_workspace_menu.take().is_some() {
                    window.focus(&this.session_filter_focus, cx);
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "escape" && this.preview_menu.is_some() {
                    this.preview_menu = None;
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if !this
                    .browser
                    .as_ref()
                    .is_some_and(|browser| browser.has_focus())
                {
                    crate::input::navigate_focus(event, window, cx);
                }
            }))
            .flex()
            .flex_col()
            .child(titlebar)
            .child(connection_recovery)
            .child(if self.is_codex() {
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .child(body)
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .p(px(10.0))
                    .flex()
                    .child(body)
                    .into_any_element()
            })
            .child(self.render_preview_menu(window, cx))
            .child(self.render_tab_overflow(window, cx))
            .child(self.render_saved_workspace_menu(window, cx))
            .child(self.render_session_context_menu(window, cx))
            .child(self.render_explorer_menu(window, cx))
            .child(self.toasts.clone())
            .child(self.render_recovery_notice(cx))
            .child(self.picker_anchors.keyboard.pointer_observer())
            .child(self.tray_drag_observer(cx))
    }
}

fn connection_recovery_element(
    connection: ConnectionState,
    error: Option<&str>,
    reload: AnyElement,
) -> AnyElement {
    if connection == ConnectionState::Ready {
        return div().into_any_element();
    }
    let theme = Theme::default();
    let (title, detail, color) = match connection {
        ConnectionState::Connecting => (
            "Reconnecting to OpenCode…",
            "Your workspace stays open. Vela will reload it when the service returns.",
            theme.yellow,
        ),
        ConnectionState::Error | ConnectionState::Disconnected => (
            "OpenCode connection lost",
            error.unwrap_or("Vela is waiting for the OpenCode service to return."),
            theme.red,
        ),
        ConnectionState::Ready => unreachable!(),
    };
    div()
        .id("connection-recovery")
        .debug_selector(|| "connection-recovery".into())
        .role(Role::Region)
        .aria_label(format!("{title} {detail}"))
        .h(px(34.))
        .flex_shrink_0()
        .w_full()
        .px(px(12.))
        .flex()
        .items_center()
        .gap(px(8.))
        .border_b_1()
        .border_color(color.opacity(0.45))
        .bg(color.opacity(0.08))
        .child(div().size(px(6.)).flex_shrink_0().rounded_full().bg(color))
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(title),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(10.5))
                .text_color(theme.dim)
                .child(SharedString::from(detail.to_owned())),
        )
        .child(reload)
        .into_any_element()
}

fn permission_button(
    id: SharedString,
    label: &'static str,
    background: gpui::Rgba,
    foreground: gpui::Rgba,
    action: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let debug_id = id.to_string();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .h(px(26.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .bg(background)
        .cursor_pointer()
        .flex()
        .items_center()
        .text_size(px(10.5))
        .text_color(foreground)
        .on_mouse_down(MouseButton::Left, move |_, _, cx| action(cx))
        .child(label)
        .into_any_element()
}

fn diff_rows(content: &str, full: Option<&str>) -> (Vec<DiffRow>, usize, usize) {
    let mut added_lines = HashSet::new();
    let mut added = 0;
    let mut removed = 0;
    let mut new_line = 1usize;
    for line in content.lines() {
        if line.starts_with("@@") {
            if let Some(start) = line
                .split('+')
                .nth(1)
                .and_then(|part| part.split([',', ' ']).next())
                .and_then(|part| part.parse::<usize>().ok())
            {
                new_line = start;
            }
            continue;
        }
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("\\")
        {
            continue;
        }
        if let Some(_text) = line.strip_prefix('+') {
            added += 1;
            added_lines.insert(new_line);
            new_line += 1;
        } else if line.starts_with('-') {
            removed += 1;
        } else {
            new_line += 1;
        }
    }
    if let Some(full) = full {
        let mut rows: Vec<DiffRow> = full
            .split('\n')
            .enumerate()
            .map(|(index, text)| {
                let line = index + 1;
                (line, text.to_string(), added_lines.contains(&line), false)
            })
            .collect();
        if rows.last().is_some_and(|(_, text, _, _)| text.is_empty()) && rows.len() > 1 {
            rows.pop();
        }
        return (rows, added, removed);
    }
    let mut rows = Vec::new();
    let mut new_line = 1usize;
    for line in content.lines() {
        if line.starts_with("@@") {
            if let Some(start) = line
                .split('+')
                .nth(1)
                .and_then(|part| part.split([',', ' ']).next())
                .and_then(|part| part.parse::<usize>().ok())
            {
                new_line = start;
            }
            continue;
        }
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("\\")
        {
            continue;
        }
        if let Some(text) = line.strip_prefix('+') {
            rows.push((new_line, text.to_string(), true, false));
            new_line += 1;
        } else if let Some(text) = line.strip_prefix('-') {
            rows.push((new_line, text.to_string(), false, true));
        } else {
            let text = line.strip_prefix(' ').unwrap_or(line);
            rows.push((new_line, text.to_string(), false, false));
            new_line += 1;
        }
    }
    (rows, added, removed)
}

#[allow(dead_code)]
fn _unused(_: Duration) {}

#[derive(Clone, Copy, PartialEq)]
struct RunStyle {
    fg: Hsla,
    bg: Option<Hsla>,
    bold: bool,
    italic: bool,
    underline: UnderlineKind,
    underline_color: Option<Hsla>,
    strikethrough: bool,
}

fn cell_display_text(cell: &Cell) -> String {
    let mut text = String::new();
    if cell.ch == '\0' || cell.ch.is_control() {
        text.push(' ');
    } else {
        text.push(cell.ch);
    }
    text.extend(cell.combining.iter());
    text
}

fn terminal_cell_width(cell: &Cell) -> usize {
    if cell.wide { 2 } else { 1 }
}

/// Absolute segments for variants gpui's single `TextRun.underline` cannot
/// express. Each tuple is `(left, bottom, width)` inside the cell.
fn terminal_underline_segments(kind: UnderlineKind, width: f32) -> Vec<(f32, f32, f32)> {
    match kind {
        UnderlineKind::None | UnderlineKind::Single | UnderlineKind::Curly => Vec::new(),
        UnderlineKind::Double => vec![(0., 1., width), (0., 3., width)],
        UnderlineKind::Dotted => {
            let mut segments = Vec::new();
            let mut left = 0.;
            while left < width {
                segments.push((left, 1., 1.));
                left += 3.;
            }
            segments
        }
        UnderlineKind::Dashed => {
            let mut segments = Vec::new();
            let mut left = 0.;
            while left < width {
                segments.push((left, 1., (width - left).min(3.)));
                left += 5.;
            }
            segments
        }
    }
}

/// Fixed-cell rows place every glyph at an exact column-derived x offset.
fn render_terminal_row(
    cells: &[Cell],
    cols: usize,
    cell_width: f32,
    row_index: usize,
    theme: &Theme,
) -> AnyElement {
    let mut row = div().relative().h(px(15.)).w_full();
    let mut column = 0;
    for cell in cells.iter().take(cols) {
        if cell.wide_spacer {
            continue;
        }
        let width = terminal_cell_width(cell);
        let text = cell_display_text(cell);
        let (fg, bg) = cell.display_colors();
        let background = if cell.selected {
            Some(theme.accent_soft.into())
        } else {
            match bg {
                CellColor::Background => None,
                other => Some(cell_color(other, theme)),
            }
        };
        let link = cell
            .hyperlink
            .as_deref()
            .and_then(vela_browser::safe_external_url);
        let style = RunStyle {
            fg: terminal_foreground(fg, cell.dim, theme),
            bg: None,
            bold: cell.bold,
            italic: cell.italic,
            underline: if cell.underline == UnderlineKind::None && cell.hyperlink.is_some() {
                UnderlineKind::Single
            } else {
                cell.underline
            },
            underline_color: cell.underline_color.map(|color| cell_color(color, theme)),
            strikethrough: cell.strikethrough,
        };
        let selector = format!("terminal-cell-{row_index}-{column}");
        let mut cell_element = div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .absolute()
            .left(px(column as f32 * cell_width))
            .top_0()
            .h(px(15.))
            .w(px(width as f32 * cell_width))
            .overflow_hidden()
            .when_some(background, |cell, color| cell.bg(color))
            .when_some(link.clone(), |cell, _| cell.cursor_pointer())
            .when_some(link, |cell, url| {
                cell.on_click(move |_, _, cx| cx.open_url(&url))
            })
            .child(
                StyledText::new(SharedString::from(text.clone()))
                    .with_runs(vec![text_run(style, text.len())]),
            );
        for (index, (left, bottom, segment_width)) in terminal_underline_segments(
            cell.underline,
            width as f32 * cell_width,
        )
        .into_iter()
        .enumerate()
        {
            let debug = format!("terminal-underline-{row_index}-{column}-{index}");
            cell_element = cell_element.child(
                div()
                    .id(SharedString::from(debug.clone()))
                    .debug_selector(move || debug.clone())
                    .absolute()
                    .left(px(left))
                    .bottom(px(bottom))
                    .h(px(1.))
                    .w(px(segment_width))
                    .bg(style.underline_color.unwrap_or(style.fg)),
            );
        }
        row = row.child(cell_element);
        column += width;
    }
    row.into_any_element()
}

#[cfg(test)]
fn terminal_row(cells: &[Cell], cols: usize, theme: &Theme) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut current: Option<(RunStyle, usize)> = None;
    for cell in cells.iter().take(cols) {
        if cell.wide_spacer {
            continue;
        }
        let ch = if cell.ch == '\0' || cell.ch.is_control() {
            ' '
        } else {
            cell.ch
        };
        let (fg, bg) = cell.display_colors();
        let style = RunStyle {
            fg: terminal_foreground(fg, cell.dim, theme),
            bg: if cell.selected {
                Some(theme.accent_soft.into())
            } else {
                match bg {
                    CellColor::Background => None,
                    other => Some(cell_color(other, theme)),
                }
            },
            bold: cell.bold,
            italic: cell.italic,
            underline: if cell.underline == UnderlineKind::None && cell.hyperlink.is_some() {
                UnderlineKind::Single
            } else {
                cell.underline
            },
            underline_color: cell.underline_color.map(|color| cell_color(color, theme)),
            strikethrough: cell.strikethrough,
        };
        if current.map(|(existing, _)| existing) != Some(style) {
            if let Some((old, len)) = current.take()
                && len > 0
            {
                runs.push(text_run(old, len));
            }
            current = Some((style, 0));
        }
        text.push(ch);
        let mut cell_bytes = ch.len_utf8();
        for mark in &cell.combining {
            text.push(*mark);
            cell_bytes += mark.len_utf8();
        }
        if let Some((_, len)) = current.as_mut() {
            *len += cell_bytes;
        }
    }
    if let Some((style, len)) = current
        && len > 0
    {
        runs.push(text_run(style, len));
    }
    (text, runs)
}

fn text_run(style: RunStyle, len: usize) -> TextRun {
    TextRun {
        len,
        font: Font {
            weight: if style.bold {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            },
            style: if style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
            ..theme::source_editor_font()
        },
        color: style.fg,
        background_color: style.bg,
        underline: match style.underline {
            UnderlineKind::None => None,
            UnderlineKind::Single => Some(gpui::UnderlineStyle {
                thickness: px(1.0),
                color: Some(style.underline_color.unwrap_or(style.fg)),
                wavy: false,
            }),
            UnderlineKind::Double | UnderlineKind::Dotted | UnderlineKind::Dashed => None,
            UnderlineKind::Curly => Some(gpui::UnderlineStyle {
                thickness: px(1.0),
                color: Some(style.underline_color.unwrap_or(style.fg)),
                wavy: true,
            }),
        },
        strikethrough: style.strikethrough.then_some(gpui::StrikethroughStyle {
            thickness: px(1.0),
            color: Some(style.fg),
        }),
    }
}

fn terminal_foreground(color: CellColor, dim: bool, theme: &Theme) -> Hsla {
    let mut color = cell_color(color, theme);
    if dim {
        color.a *= 0.55;
    }
    color
}

fn cell_color(color: CellColor, theme: &Theme) -> Hsla {
    let rgba = match color {
        CellColor::Foreground => theme.text,
        CellColor::Background => theme.code_bg,
        CellColor::Rgb(r, g, b) => theme::rgba(r, g, b, 1.0),
        CellColor::Indexed(index) => indexed_color(index),
    };
    Hsla::from(rgba)
}

fn indexed_color(index: u8) -> gpui::Rgba {
    const ANSI: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00),
        (0xcd, 0x31, 0x31),
        (0x00, 0xbc, 0x00),
        (0x94, 0x98, 0x00),
        (0x04, 0x51, 0xa5),
        (0xbc, 0x05, 0xbc),
        (0x05, 0x98, 0xbc),
        (0xe5, 0xe5, 0xe5),
        (0x66, 0x66, 0x66),
        (0xcd, 0x31, 0x31),
        (0x14, 0xce, 0x14),
        (0xb5, 0xb5, 0x00),
        (0x24, 0x72, 0xc8),
        (0xd6, 0x70, 0xd6),
        (0x29, 0xb8, 0xdb),
        (0x00, 0x00, 0x00),
    ];
    if (index as usize) < ANSI.len() {
        let (r, g, b) = ANSI[index as usize];
        return theme::rgba(r, g, b, 1.0);
    }
    if index < 232 {
        let value = index - 16;
        let convert = |part: u8| -> u8 { if part == 0 { 0 } else { 55 + part * 40 } };
        return theme::rgba(
            convert(value / 36),
            convert((value % 36) / 6),
            convert(value % 6),
            1.0,
        );
    }
    let level = 8 + (index - 232) * 10;
    theme::rgba(level, level, level, 1.0)
}

fn browser_button(
    label: &'static str,
    aria_label: &'static str,
    enabled: bool,
    focus: gpui::FocusHandle,
    action: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let theme = Theme::default();
    let action = Rc::new(action);
    let click_action = action.clone();
    let key_action = action;
    let mouse_focus = focus.clone();
    let key_focus = focus.clone();
    let debug_id = format!("browser-nav-{label}");
    div()
        .id(SharedString::from(debug_id.clone()))
        .debug_selector(move || debug_id.clone())
        .role(Role::Button)
        .aria_label(aria_label)
        .focusable()
        .track_focus(&focus)
        .tab_stop(enabled)
        .size(px(24.0))
        .rounded(px(5.0))
        .flex()
        .items_center()
        .justify_center()
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(theme.inset))
        })
        .text_size(px(13.0))
        .text_color(theme.dim)
        .opacity(if enabled { 1. } else { 0.4 })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.focus(&mouse_focus, cx);
        })
        .on_click(move |_, _, cx| {
            if enabled {
                click_action(cx);
            }
        })
        .on_key_down(move |event, window, cx| {
            if enabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                window.focus(&key_focus, cx);
                key_action(cx);
                cx.stop_propagation();
            }
        })
        .child(label)
        .into_any_element()
}

fn settings_rail_tool(
    active: bool,
    theme: Theme,
    codex: bool,
    weak: gpui::WeakEntity<VelaApp>,
    action: impl Fn(&mut VelaApp, &mut Window, &mut Context<VelaApp>) + 'static,
) -> AnyElement {
    div()
        .id("rail-settings")
        .role(gpui::Role::Button)
        .aria_label("Settings")
        .focusable()
        .tab_stop(true)
        .size(px(36.0))
        .rounded(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(if active {
            theme.selected
        } else {
            theme::rgba(0, 0, 0, 0.)
        })
        .text_size(px(20.0))
        .line_height(gpui::relative(1.))
        .font_family("codicon")
        .text_color(if active {
            if codex { theme.text } else { theme.accent }
        } else {
            theme.dim
        })
        .hover(move |style| {
            style
                .bg(gpui::Rgba {
                    a: 0.07,
                    ..if codex { theme.text } else { theme.accent }
                })
                .text_color(if codex { theme.text } else { theme.accent })
        })
        .on_click(move |_, window, cx| {
            let _ = weak.update(cx, |app, cx| action(app, window, cx));
        })
        .child(icon("eb51"))
        .into_any_element()
}

fn rail_tool(
    icon_code: &'static str,
    active: bool,
    theme: Theme,
    codex: bool,
    weak: gpui::WeakEntity<VelaApp>,
    action: impl Fn(&mut VelaApp, &mut Window, &mut Context<VelaApp>) + 'static,
) -> AnyElement {
    let label = match icon_code {
        "eac7" => "Sessions",
        "eaf0" => "Files",
        "ea85" => "Terminal",
        "eae1" => "Changes",
        "eb01" => "Open a browser tab",
        _ => "Workspace view",
    };
    div()
        .id(SharedString::from(format!("rail-{icon_code}")))
        .role(gpui::Role::Button)
        .aria_label(label)
        .focusable()
        .tab_stop(true)
        .size(px(36.0))
        .rounded(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(if active {
            theme.selected
        } else {
            theme::rgba(0, 0, 0, 0.)
        })
        .text_size(px(20.0))
        .line_height(gpui::relative(1.))
        .font_family("codicon")
        .text_color(if active {
            if codex { theme.text } else { theme.accent }
        } else {
            theme.dim
        })
        .hover(move |style| {
            style
                .bg(gpui::Rgba {
                    a: 0.07,
                    ..if codex { theme.text } else { theme.accent }
                })
                .text_color(if codex { theme.text } else { theme.accent })
        })
        .on_click(move |_, window, cx| {
            let _ = weak.update(cx, |app, cx| action(app, window, cx));
        })
        .child(icon(icon_code))
        .into_any_element()
}

fn tree_status_dot(label: &'static str, color: gpui::Rgba, blur: f32) -> gpui::Stateful<gpui::Div> {
    div()
        .id(label)
        .role(gpui::Role::Label)
        .aria_label(label)
        .flex_shrink_0()
        .size(px(7.))
        .rounded_full()
        .bg(color)
        .shadow(vec![
            gpui::BoxShadow::new(px(0.), px(0.), color.opacity(0.5).into()).blur_radius(px(blur)),
        ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_rows_have_byte_exact_runs() {
        let mut emulator = Emulator::new(40, 4);
        emulator.feed(b"\x1b[31mred\x1b[0m plain");
        let cells = emulator.line(0);
        let (text, runs) = terminal_row(&cells, 40, &Theme::default());
        assert!(text.starts_with("red plain"));
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
        assert!(runs.len() >= 2, "ANSI color should create a separate run");
    }

    #[test]
    fn terminal_rows_carry_dim_and_strikethrough_runs() {
        let mut emulator = Emulator::new(40, 4);
        emulator.feed(b"\x1b[2;9mghost\x1b[0m plain");
        let (text, runs) = terminal_row(&emulator.line(0), 40, &Theme::default());
        assert!(text.starts_with("ghost plain"));
        assert_eq!(
            runs.iter().map(|run| run.len).sum::<usize>(),
            text.len()
        );
        assert!(
            runs[0].strikethrough.is_some(),
            "SGR 9 produces a strikethrough run"
        );
        assert!(
            runs[0].color.a < 1.0,
            "SGR 2 lowers foreground alpha"
        );
        assert!(runs[1].strikethrough.is_none());
    }

    #[test]
    fn terminal_rows_preserve_underline_variants_in_runs() {
        let mut emulator = Emulator::new(40, 4);
        emulator.feed(b"\x1b[4mS\x1b[4:2mD\x1b[4:3mC\x1b[4:4mO\x1b[4:5mH\x1b[0m");
        let cells = emulator.line(0);
        assert_eq!(cells[0].underline, UnderlineKind::Single);
        assert_eq!(cells[1].underline, UnderlineKind::Double);
        assert_eq!(cells[2].underline, UnderlineKind::Curly);
        assert_eq!(cells[3].underline, UnderlineKind::Dotted);
        assert_eq!(cells[4].underline, UnderlineKind::Dashed);
        let (text, runs) = terminal_row(&cells, 40, &Theme::default());
        assert!(text.starts_with("SDCOH"));
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
        assert!(
            runs[0].underline.is_some_and(|style| style.thickness == px(1.0) && !style.wavy),
            "single underline keeps a solid run"
        );
        assert!(
            runs[2].underline.is_some_and(|style| style.wavy),
            "curly underline keeps wavy styling"
        );
        for index in [1usize, 3, 4] {
            assert!(
                runs[index].underline.is_none(),
                "double/dotted/dashed are painted as exact cell segments"
            );
        }
        assert_eq!(terminal_underline_segments(UnderlineKind::Double, 7.5).len(), 2);
        assert_eq!(terminal_underline_segments(UnderlineKind::Dotted, 7.5).len(), 3);
        assert_eq!(terminal_underline_segments(UnderlineKind::Dashed, 7.5).len(), 2);
    }

    #[gpui::test]
    fn terminal_rows_paint_exact_double_dotted_and_dashed_segments(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut emulator = Emulator::new(8, 2);
        emulator.feed(b"\x1b[4:2;58;2;1;2;3mD\x1b[4:4mO\x1b[4:5mH\x1b[0m");
        let cells = emulator.line(0);
        let (view, cx) = cx.add_window_view(|_, _| TerminalRowProbe { cells });
        cx.simulate_resize(size(px(200.), px(40.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        let double_top = cx
            .debug_bounds("terminal-underline-0-0-0")
            .expect("double underline upper segment");
        let double_bottom = cx
            .debug_bounds("terminal-underline-0-0-1")
            .expect("double underline lower segment");
        assert_eq!(double_top.size.height, px(1.));
        assert!(
            double_bottom.top() < double_top.top(),
            "the two double-underline rules occupy different heights: {double_top:?} {double_bottom:?}"
        );
        for debug in [
            "terminal-underline-0-1-0",
            "terminal-underline-0-1-1",
            "terminal-underline-0-1-2",
        ] {
            assert!(
                cx.debug_bounds(debug).is_some(),
                "dotted underline segment {debug} is painted"
            );
        }
        assert!(cx.debug_bounds("terminal-underline-0-2-0").is_some());
        assert!(cx.debug_bounds("terminal-underline-0-2-1").is_some());
    }

    #[test]
    fn terminal_rows_carry_explicit_underline_color() {
        let mut emulator = Emulator::new(40, 4);
        emulator.feed(b"\x1b[4;58;2;10;20;30mX\x1b[59mY\x1b[0m");
        let cells = emulator.line(0);
        assert_eq!(cells[0].underline_color, Some(CellColor::Rgb(10, 20, 30)));
        assert_eq!(cells[1].underline_color, None);
        let (text, runs) = terminal_row(&cells, 40, &Theme::default());
        assert!(text.starts_with("XY"));
        let expected = Hsla::from(theme::rgba(10, 20, 30, 1.0));
        assert_eq!(
            runs[0].underline.and_then(|style| style.color),
            Some(expected),
            "SGR 58 sets the shaped underline color"
        );
        assert_ne!(
            runs[1].underline.and_then(|style| style.color),
            Some(expected),
            "SGR 59 restores the default underline color"
        );
    }

    #[test]
    fn terminal_rows_underline_osc8_links() {
        let mut emulator = Emulator::new(40, 4);
        emulator.feed(b"\x1b]8;;https://example.com\x07link\x1b]8;;\x07 plain");
        let cells = emulator.line(0);
        assert_eq!(cells[0].hyperlink.as_deref(), Some("https://example.com"));
        let (text, runs) = terminal_row(&cells, 40, &Theme::default());
        assert!(text.starts_with("link plain"));
        assert!(
            runs[0].underline.is_some(),
            "OSC 8 text is underlined like a terminal hyperlink"
        );
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
    }

    #[test]
    fn terminal_rows_preserve_combining_marks_and_ansi_run_boundaries() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed("\x1b[31me\u{301}\u{327}\x1b[32m界\u{301}X".as_bytes());
        let (text, runs) = terminal_row(&emulator.line(0), 20, &Theme::default());
        assert!(text.starts_with("e\u{301}\u{327}界\u{301}X"));
        assert_eq!(runs[0].len, "e\u{301}\u{327}".len());
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
        let mut offset = 0;
        for run in runs {
            offset += run.len;
            assert!(text.is_char_boundary(offset));
        }
    }

    #[test]
    fn cell_metadata_marks_emoji_wide_and_spacer() {
        let mut emoji = Emulator::new(20, 4);
        emoji.feed("🌍".as_bytes());
        let cells = emoji.line(0);
        assert!(cells[0].wide);
        assert!(cells[1].wide_spacer);
        let mut combining = Emulator::new(20, 4);
        combining.feed("e\u{301}".as_bytes());
        assert_eq!(combining.line(0)[0].combining, vec!['\u{301}']);
    }

    struct TerminalRowProbe {
        cells: Vec<Cell>,
    }

    impl Render for TerminalRowProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(render_terminal_row(
                &self.cells,
                8,
                7.5,
                0,
                &Theme::default(),
            ))
        }
    }

    #[gpui::test]
    fn fixed_cell_row_places_wide_combining_and_following_glyphs_at_exact_columns(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut emulator = Emulator::new(8, 2);
        emulator.feed("a界e\u{301}b".as_bytes());
        let cells = emulator.line(0);
        let (view, cx) = cx.add_window_view(|_, _| TerminalRowProbe { cells });
        cx.simulate_resize(size(px(200.), px(40.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        let a = cx.debug_bounds("terminal-cell-0-0").expect("cell a");
        let wide = cx.debug_bounds("terminal-cell-0-1").expect("cell 界");
        let combined = cx.debug_bounds("terminal-cell-0-3").expect("cell e+mark");
        let following = cx.debug_bounds("terminal-cell-0-4").expect("cell b");
        assert!((f32::from(a.left()) - 0.).abs() < 0.01);
        assert!((f32::from(wide.left()) - 7.5).abs() < 0.01);
        assert!((f32::from(wide.size.width) - 15.).abs() < 0.01);
        assert!((f32::from(combined.left()) - 22.5).abs() < 0.01);
        assert!((f32::from(following.left()) - 30.).abs() < 0.01);
    }

    #[gpui::test]
    fn fixed_cell_row_places_ascii_and_fallback_glyphs_at_exact_columns(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut emulator = Emulator::new(8, 2);
        emulator.feed("ab→de".as_bytes());
        let cells = emulator.line(0);
        assert!(!cells[2].wide, "the arrow is a single fallback cell");
        let (view, cx) = cx.add_window_view(|_, _| TerminalRowProbe { cells });
        cx.simulate_resize(size(px(200.), px(40.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        for (column, expected) in [(0, 0.), (1, 7.5), (2, 15.), (3, 22.5), (4, 30.)] {
            let id: &'static str = Box::leak(format!("terminal-cell-0-{column}").into_boxed_str());
            let bounds = cx.debug_bounds(id).expect("cell bounds");
            assert!(
                (f32::from(bounds.left()) - expected).abs() < 0.01,
                "column {column} left {} != {expected}",
                f32::from(bounds.left())
            );
            assert!((f32::from(bounds.size.width) - 7.5).abs() < 0.01);
        }
    }

    #[test]
    fn selected_terminal_cells_have_a_distinct_background_and_valid_runs() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed("e\u{301}界X".as_bytes());
        emulator.begin_selection(
            0,
            0,
            false,
            vela_terminal::emulator::SelectionKind::Simple,
            false,
        );
        emulator.update_selection(0, 2, true);
        let (text, runs) = terminal_row(&emulator.line(0), 20, &Theme::default());
        assert_eq!(runs[0].len, "e\u{301}界".len());
        assert_eq!(
            runs[0].background_color,
            Some(Theme::default().accent_soft.into())
        );
        assert!(runs[1].background_color.is_none());
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
    }

    #[test]
    fn editor_group_tab_assignment_moves_between_extras_and_main() {
        let key = |rel: &str| DocumentKey::in_workspace("workspace".into(), "root".into(), rel.into());
        let a = key("a.txt");
        let b = key("b.txt");
        let c = key("c.txt");
        let mut groups = vec![Some(b.clone()), Some(c.clone())];
        assert!(assign_editor_group_tab(&mut groups, &a, 1));
        assert_eq!(groups, vec![Some(a.clone()), Some(c.clone())]);
        assert!(assign_editor_group_tab(&mut groups, &c, 2));
        assert_eq!(groups, vec![Some(a.clone()), Some(c.clone())]);
        assert!(assign_editor_group_tab(&mut groups, &a, 0));
        assert_eq!(groups, vec![None, Some(c.clone())]);
        assert!(!assign_editor_group_tab(&mut groups, &a, 3));
        assert_eq!(groups, vec![None, Some(c)]);
    }

    #[test]
    fn tui_exit_notice_matches_source_success_and_failure_copy() {
        assert_eq!(tui_exit_notice(Some(0)), "Agent TUI exited.");
        assert_eq!(tui_exit_notice(None), "Agent TUI exited.");
        assert_eq!(
            tui_exit_notice(Some(7)),
            "Agent TUI exited with code 7."
        );
    }

    #[gpui::test]
    fn dragging_a_tab_to_an_editor_group_moves_the_active_document(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let first = DocumentKey::in_workspace(
            "workspace".into(),
            "workspace".into(),
            "a.txt".into(),
        );
        let second = DocumentKey::in_workspace(
            "workspace".into(),
            "workspace".into(),
            "b.txt".into(),
        );
        let third = DocumentKey::in_workspace(
            "workspace".into(),
            "workspace".into(),
            "c.txt".into(),
        );
        app.update(cx, |app, cx| {
            app.active_id = Some("session".into());
            app.document_scopes
                .insert("session".into(), "workspace".into());
            app.documents.borrow_mut().open(1, first.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(1, &first.rel, Some("a".into()));
            app.documents.borrow_mut().open(2, second.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(2, &second.rel, Some("b".into()));
            app.documents.borrow_mut().open(3, third.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(3, &third.rel, Some("c".into()));
            app.documents.borrow_mut().activate(&first);
            app.editor_groups
                .entry("workspace".into())
                .or_default()
                .push(Some(second.clone()));
            app.tab_strip.set_width_for_test(1000.);
            cx.notify();
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        draw(cx);
        draw(cx);
        let tab = cx
            .debug_bounds("titlebar-tab-0")
            .expect("first tab is rendered");
        let target = cx
            .debug_bounds("editor-group-1")
            .expect("second editor group is rendered");
        let start = tab.center();
        let end = target.center();
        cx.simulate_event(gpui::MouseDownEvent {
            position: start,
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseMoveEvent {
            position: start + point(px(6.), px(6.)),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseMoveEvent {
            position: end,
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseUpEvent {
            position: end,
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        app.read_with(cx, |app, _| {
            assert_eq!(
                app.editor_groups
                    .get("workspace")
                    .and_then(|groups| groups.first())
                    .cloned()
                    .flatten(),
                Some(first.clone())
            );
            assert_eq!(app.active_document_key(), Some(third.clone()));
        });
    }

    #[gpui::test]
    fn inactive_tile_answer_copy_forms_and_revert_docks_are_interactive(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.active_id = Some("ses_active".into());
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_active",
                    "Active thread",
                ))
                .unwrap();
            let mut waiting =
                crate::panels::PanelState::new("ses_waiting", "Waiting thread");
            waiting.blocks = Arc::new(vec![Block {
                id: "answer-1".into(),
                message_id: "answer-message".into(),
                part_id: "answer-1".into(),
                kind: BlockKind::Assistant,
                text: "done".into(),
                detail: String::new().into(),
                output: String::new().into(),
                status: ToolStatus::None,
                streaming: false,
                agent: None,
                model: None,
                created_at: 0,
                completed_at: None,
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            waiting.forms = vec![vela_opencode::FormRequest {
                id: "form-waiting".into(),
                session_id: "ses_waiting".into(),
                title: "Need input".into(),
                fields: vec![vela_opencode::FormField {
                    key: "answer".into(),
                    title: Some("Answer".into()),
                    description: None,
                    required: false,
                    kind: vela_opencode::FormKind::Text,
                    placeholder: Some("Type here".into()),
                    url: None,
                    options: Vec::new(),
                    default: None,
                }],
            }];
            app.panels.open(waiting).unwrap();
            app.panels.activate("ses_active");
            app.staged_reverts.insert("ses_waiting".into());
        });
        draw(cx);
        cx.run_until_parked();
        let copy = cx
            .debug_bounds("panel-answer-copy-ses_waiting-answer-1")
            .expect("answer copy action");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("done")
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_active".into()),
            "copying an answer does not steal the active panel"
        );
        assert!(
            cx.debug_bounds("panel-revert-dock-ses_waiting").is_some(),
            "inactive tile renders its own staged revert dock"
        );
        let commit = cx
            .debug_bounds("panel-revert-commit-ses_waiting")
            .expect("revert commit action");
        cx.simulate_click(commit.center(), gpui::Modifiers::default());
        draw(cx);
        let commit_request = app.read_with(cx, |app, _| {
            app.commit_revert_requests
                .iter()
                .find_map(|(request, session)| {
                    (session == "ses_waiting").then_some(*request)
                })
                .expect("commit request targets the owning inactive panel")
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::RevertFinished {
                    request_id: commit_request,
                    session_id: "ses_waiting".into(),
                    error: None,
                },
                cx,
            );
            app.staged_reverts.insert("ses_waiting".into());
        });
        draw(cx);
        let discard = cx
            .debug_bounds("panel-revert-discard-ses_waiting")
            .expect("revert discard action");
        cx.simulate_click(discard.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            app.read_with(cx, |app, _| {
                app.clear_revert_requests
                    .values()
                    .any(|session| session == "ses_waiting")
            }),
            "discard targets the owning inactive panel"
        );
        assert!(
            cx.debug_bounds("panel-form-dock-ses_waiting").is_some(),
            "inactive tile renders its own pending form dock"
        );
        let dismiss = cx
            .debug_bounds("form-dismiss-form-waiting")
            .expect("form dismiss action");
        cx.simulate_click(dismiss.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            app.read_with(cx, |app, _| {
                app.form_requests
                    .values()
                    .any(|(session, form)| session == "ses_waiting" && form == "form-waiting")
            }),
            "dismissing an inactive form routes to the owning session"
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_waiting".into()),
            "interacting with an inactive form focuses its panel"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn inactive_tile_user_actions_and_work_rows_are_interactive(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        let block = |id: &str, message_id: &str, kind, text: &str| Block {
            id: id.into(),
            message_id: message_id.into(),
            part_id: id.into(),
            kind,
            text: text.into(),
            detail: String::new().into(),
            output: String::new().into(),
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at: 0,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.active_id = Some("ses_active".into());
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_active",
                    "Active thread",
                ))
                .unwrap();
            let mut waiting =
                crate::panels::PanelState::new("ses_waiting", "Waiting thread");
            waiting.blocks = Arc::new(vec![
                block("user-1", "message-1", BlockKind::User, "fix it"),
                block("reason-1", "", BlockKind::Reasoning, "checked the parser"),
                block("activity-1", "", BlockKind::Activity, "indexed files"),
                block("answer-1", "", BlockKind::Assistant, "done"),
            ]);
            app.panels.open(waiting).unwrap();
            app.panels.activate("ses_active");
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("panel-message-copy-ses_waiting-user-1")
                .is_some(),
            "inactive user rows expose Copy"
        );
        assert!(
            cx.debug_bounds("panel-message-undo-ses_waiting-user-1")
                .is_some(),
            "inactive user rows expose Undo"
        );
        assert!(
            cx.debug_bounds("panel-message-edit-ses_waiting-user-1")
                .is_some(),
            "inactive user rows expose Edit"
        );
        assert!(
            cx.debug_bounds("panel-work-body-ses_waiting-1")
                .is_none(),
            "work rows start collapsed"
        );
        let work = cx
            .debug_bounds("panel-work-ses_waiting-1")
            .expect("work row head");
        cx.simulate_click(work.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("panel-work-body-ses_waiting-1").is_some(),
            "clicking the work row expands its reasoning and activity"
        );
        assert!(
            cx.debug_bounds("panel-work-row-ses_waiting-1-1").is_some(),
            "expanded work rows keep every step"
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_active".into()),
            "expanding work does not steal the active panel"
        );

        let copy = cx
            .debug_bounds("panel-message-copy-ses_waiting-user-1")
            .expect("copy action");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("fix it")
        );
        assert_eq!(
            app.read_with(cx, |app, _| {
                app.panels
                    .entries()
                    .iter()
                    .find(|panel| panel.session == "ses_waiting")
                    .and_then(|panel| panel.copied_message.clone())
            }),
            Some("user-1".to_owned())
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_active".into()),
            "copying does not steal the active panel"
        );

        let edit = cx
            .debug_bounds("panel-message-edit-ses_waiting-user-1")
            .expect("edit action");
        cx.simulate_click(edit.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_waiting".into()),
            "Edit focuses the tile that owns the message"
        );
        assert!(
            app.read_with(cx, |app, _| {
                app.revert_requests
                    .values()
                    .any(|(session, prompt)| {
                        session == "ses_waiting"
                            && prompt.as_deref() == Some("fix it")
                    })
            }),
            "Edit stages the prompt on the owning session"
        );

        app.update(cx, |app, cx| {
            app.active_id = Some("ses_active".into());
            app.panels.activate("ses_active");
            app.revert_requests.clear();
            cx.notify();
        });
        draw(cx);
        let undo = cx
            .debug_bounds("panel-message-undo-ses_waiting-user-1")
            .expect("undo action");
        cx.simulate_click(undo.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_waiting".into()),
            "Undo focuses the tile that owns the message"
        );
        assert!(
            app.read_with(cx, |app, _| {
                app.revert_requests
                    .values()
                    .any(|(session, prompt)| {
                        session == "ses_waiting" && prompt.is_none()
                    })
            }),
            "Undo targets the owning session without edit text"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn slash_text_is_a_plain_prompt_without_runtime_command_support(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        app.update(cx, |app, cx| {
            app.active_id = Some("ses_commands".into());
            app.loading = false;
            app.workspace_closed = false;
            app.runtime_capabilities.commands = false;
            app.composer = "/compact".into();
            app.composer_input
                .update(cx, |input, cx| input.set_text("/compact".into(), cx));
        });
        app.update_in(cx, |app, window, cx| app.send(window, cx));
        assert!(
            app.read_with(cx, |app, _| app.slash_requests.is_empty()),
            "a slash-looking prompt must not invoke the command API"
        );
        assert_eq!(app.read_with(cx, |app, _| app.composer.clone()), "");

        app.update(cx, |app, cx| {
            app.runtime_capabilities.commands = true;
            app.composer = "/compact".into();
            app.composer_input
                .update(cx, |input, cx| input.set_text("/compact".into(), cx));
        });
        app.update_in(cx, |app, window, cx| app.send(window, cx));
        assert!(
            app.read_with(cx, |app, _| {
                app.slash_requests
                    .values()
                    .any(|session| session == "ses_commands")
            }),
            "the same text routes through the command API when supported"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn empty_workspace_files_sidebar_offers_open_workspace(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = true;
            app.active_id = None;
            app.sidebar_open = true;
            app.sidebar_tab = SidebarTab::Files;
            app.settings_open = false;
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("explorer-open-workspace").is_some(),
            "an empty files sidebar keeps Source's Open Workspace action"
        );
        assert!(
            app.read_with(cx, |app, _| app.workspace_directory().is_empty()),
            "the empty state is not attached to a phantom workspace"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn same_root_panels_keep_independent_editor_tabs_and_split_groups(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        let first = DocumentKey::in_workspace(
            "scope-a".into(),
            "/workspace".into(),
            "a.txt".into(),
        );
        let second = DocumentKey::in_workspace(
            "scope-b".into(),
            "/workspace".into(),
            "b.txt".into(),
        );
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.test_workspace_directory = Some("/workspace".into());
            app.active_id = Some("ses_a".into());
            app.document_scopes
                .insert("ses_a".into(), "scope-a".into());
            app.document_scopes
                .insert("ses_b".into(), "scope-b".into());
            app.panels
                .open(crate::panels::PanelState::new("ses_a", "Panel A"))
                .unwrap();
            app.panels
                .open(crate::panels::PanelState::new("ses_b", "Panel B"))
                .unwrap();
            app.panels.activate("ses_a");
            app.documents.borrow_mut().open(1, first.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(1, &first.rel, Some("alpha".into()));
            app.documents.borrow_mut().open(2, second.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(2, &second.rel, Some("beta".into()));
            app.documents.borrow_mut().activate(&first);
        });
        draw(cx);
        cx.run_until_parked();
        let split = cx
            .debug_bounds("editor-group-split")
            .expect("split control");
        cx.simulate_click(split.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("editor-group-cell-1").is_some(),
            "panel A owns its split editor group"
        );
        assert_eq!(
            app.read_with(cx, |app, _| {
                app.documents
                    .borrow()
                    .tabs("scope-a")
                    .iter()
                    .map(|tab| tab.key.rel.clone())
                    .collect::<Vec<_>>()
            }),
            vec!["a.txt".to_owned()]
        );

        app.update(cx, |app, _| {
            app.active_id = Some("ses_b".into());
            app.panels.activate("ses_b");
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("editor-group-cell-1").is_none(),
            "panel B starts without panel A's split group"
        );
        assert_eq!(
            app.read_with(cx, |app, _| {
                app.documents
                    .borrow()
                    .tabs("scope-b")
                    .iter()
                    .map(|tab| tab.key.rel.clone())
                    .collect::<Vec<_>>()
            }),
            vec!["b.txt".to_owned()]
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_document_key()),
            Some(second.clone())
        );

        app.update(cx, |app, _| {
            app.active_id = Some("ses_a".into());
            app.panels.activate("ses_a");
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("editor-group-cell-1").is_some(),
            "panel A keeps its split group after visiting panel B"
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_document_key()),
            Some(first)
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn agent_tui_exit_removes_the_tab_and_emits_a_notice(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        app.update(cx, |app, cx| {
            let mut tui = TerminalView::new("term-tui".into(), 80, 24, cx);
            tui.name = Some("Agent TUI".into());
            app.terminals.insert(tui);
            app.handle_terminal_exit("term-tui", Some(7), cx);
        });
        app.read_with(cx, |app, cx| {
            assert!(app.terminals.views.is_empty());
            assert_eq!(
                app.toasts.read(cx).texts(),
                vec!["Agent TUI exited with code 7.".to_owned()]
            );
        });
        app.update(cx, |app, cx| {
            let mut shell = TerminalView::new("term-shell".into(), 80, 24, cx);
            shell.name = Some("zsh".into());
            app.terminals.insert(shell);
            app.handle_terminal_exit("term-shell", Some(1), cx);
        });
        app.read_with(cx, |app, cx| {
            assert!(app.terminals.views.is_empty());
            assert_eq!(
                app.toasts.read(cx).texts(),
                vec!["Agent TUI exited with code 7.".to_owned()],
                "ordinary terminal exits stay silent"
            );
        });
    }

    #[test]
    fn terminal_key_encoding_covers_control_and_arrows() {
        use gpui::Keystroke;
        let mut event = KeyDownEvent {
            keystroke: Keystroke::parse("up").unwrap(),
            is_held: false,
            prefer_character_input: false,
        };
        assert_eq!(encode_terminal_key(&event, false), b"\x1b[A");
        assert_eq!(encode_terminal_key(&event, true), b"\x1bOA");
        event.keystroke = Keystroke::parse("ctrl-c").unwrap();
        assert_eq!(encode_terminal_key(&event, false), vec![0x03]);
        event.keystroke = Keystroke::parse("enter").unwrap();
        assert_eq!(encode_terminal_key(&event, false), b"\r");
    }

    #[test]
    fn slash_commands_split_once_and_ignore_plain_or_empty_names() {
        assert_eq!(
            parse_slash_command("/compact"),
            Some(("compact".into(), String::new()))
        );
        assert_eq!(
            parse_slash_command("/review src/lib.rs now"),
            Some(("review".into(), "src/lib.rs now".into()))
        );
        assert_eq!(parse_slash_command(" /compact"), None);
        assert_eq!(parse_slash_command("/"), None);
        assert_eq!(parse_slash_command("/a/b"), None);
        assert_eq!(parse_slash_command("normal prompt"), None);
    }

    #[test]
    fn slash_commands_require_the_runtime_command_capability() {
        assert_eq!(
            prompt_command("/compact", true),
            Some(("compact".into(), String::new()))
        );
        assert_eq!(prompt_command("/compact", false), None);
        assert_eq!(prompt_command("plain prompt", true), None);
    }

    #[cfg(unix)]
    #[test]
    fn tui_program_resolves_configured_binary_and_path_entries() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("vela-tui-probe-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let binary = root.join("opencode");
        std::fs::write(&binary, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            find_tui_program(Some(binary.as_os_str()), None),
            Some(binary.clone())
        );
        let path = std::ffi::OsString::from(root.as_os_str());
        assert_eq!(
            find_tui_program(None, Some(path.as_os_str())),
            Some(binary.clone()),
            "a bare program name searches PATH"
        );
        let not_executable = root.join("not-executable");
        std::fs::write(&not_executable, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&not_executable, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            find_tui_program(Some(not_executable.as_os_str()), None),
            None
        );
        assert_eq!(
            find_tui_program(
                Some(std::ffi::OsStr::new("opencode-missing")),
                Some(path.as_os_str())
            ),
            None
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sass_menu_metadata_matches_source_labels_descriptions_and_disabled_state() {
        let off = sass_menu_metadata(
            vela_engine::SassStatus::Off,
            false,
            None,
            false,
            "/workspace",
        );
        assert_eq!(off.label, "Watch Sass");
        assert_eq!(
            off.description,
            "Start Sass watching in /workspace; saved Sass files compile to CSS on disk"
        );
        assert!(!off.disabled);

        let on = sass_menu_metadata(vela_engine::SassStatus::On, true, None, false, "/workspace");
        assert_eq!(on.label, "Sass: On");
        assert_eq!(
            on.description,
            "Stop Sass watching in /workspace; saved Sass files compile to CSS on disk"
        );

        let error = sass_menu_metadata(
            vela_engine::SassStatus::Error,
            true,
            Some("compile failed"),
            true,
            "/workspace",
        );
        assert_eq!(error.label, "Sass: Error");
        assert_eq!(
            error.description,
            "compile failed\nSee Sass Output in the terminal panel. Click to stop watching"
        );
        assert!(error.disabled, "pending toggles expose disabled semantics");

        let starting = sass_menu_metadata(
            vela_engine::SassStatus::Starting,
            false,
            None,
            true,
            "/workspace",
        );
        assert_eq!(starting.label, "Starting Sass…");
        assert!(starting.disabled);

        let mut disabled_node = gpui::accesskit::Node::new(Role::MenuItem);
        apply_sass_disabled(&mut disabled_node, true);
        assert!(disabled_node.is_disabled());
        let mut enabled_node = gpui::accesskit::Node::new(Role::MenuItem);
        apply_sass_disabled(&mut enabled_node, false);
        assert!(!enabled_node.is_disabled());
    }

    #[test]
    fn validation_summary_labels_match_source_pluralization() {
        assert_eq!(validation_summary_label(0, 0), "No problems");
        assert_eq!(validation_summary_label(1, 0), "1 error");
        assert_eq!(validation_summary_label(2, 1), "2 errors, 1 warning");
        assert_eq!(validation_summary_label(0, 3), "3 warnings");
        assert_eq!(validation_target("index.html"), ValidationTarget::W3c);
        assert_eq!(validation_target("styles.CSS"), ValidationTarget::W3c);
        assert_eq!(validation_target("src/theme.scss"), ValidationTarget::Scss);
        assert_eq!(validation_target("src/legacy.SASS"), ValidationTarget::Scss);
        assert_eq!(validation_target("notes.md"), ValidationTarget::Unsupported);
    }

    #[test]
    fn newer_validation_request_replaces_stale_request_for_same_document() {
        let a = DocumentKey::new("/workspace".into(), "a.html".into());
        let b = DocumentKey::new("/workspace".into(), "b.css".into());
        let mut requests = HashMap::new();
        track_validation_request(&mut requests, 1, a.clone());
        track_validation_request(&mut requests, 2, b.clone());
        track_validation_request(&mut requests, 3, a.clone());
        assert_eq!(requests.len(), 2);
        assert_eq!(requests.get(&3), Some(&a));
        assert!(!requests.contains_key(&1));
        assert_eq!(requests.get(&2), Some(&b));
        assert_eq!(pending_validation(&requests, &a), Some(3));
        assert_eq!(pending_validation(&requests, &b), Some(2));
    }

    #[test]
    fn validation_action_labels_reflect_pending_failure_and_summary() {
        assert_eq!(
            validation_action_label(true, false, false),
            "Cancel validation"
        );
        assert_eq!(
            validation_action_label(false, true, false),
            "Retry validation"
        );
        assert_eq!(
            validation_action_label(false, false, true),
            "Validate again"
        );
        assert_eq!(validation_action_label(false, false, false), "Validate");
    }

    #[test]
    fn validation_diagnostics_map_to_editor_markers() {
        let diagnostics = vec![vela_engine::validation::Diagnostic {
            line: 2,
            column: 3,
            end_line: 2,
            end_column: 4,
            message: "Bad markup".into(),
            severity: "error".into(),
            source: "w3c-html".into(),
        }];
        let w3c = validation_diagnostics(ValidationTarget::W3c, &diagnostics);
        assert_eq!(w3c.len(), 1);
        assert_eq!(w3c[0].source, "W3C");
        assert_eq!(w3c[0].kind, crate::diagnostics::DiagnosticKind::Validation);
        assert_eq!((w3c[0].line, w3c[0].column), (2, 3));
        assert_eq!(w3c[0].message, "Bad markup");
        let sass = validation_diagnostics(ValidationTarget::Scss, &diagnostics);
        assert_eq!(sass[0].source, "Sass");
    }

    #[test]
    fn usage_formatting_matches_source_compact_counts_and_costs() {
        assert_eq!(usage_count(0), "0");
        assert_eq!(usage_count(999), "999");
        assert_eq!(usage_count(1_250), "1.2k");
        assert_eq!(usage_count(2_500_000), "2.5M");
        assert_eq!(usage_cost(0.), "$0.00");
        assert_eq!(usage_cost(0.01234), "$0.0123");
        assert_eq!(usage_cost(1.5), "$1.50");
    }

    #[gpui::test]
    fn active_user_message_renders_agent_and_catalog_model_metadata(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_meta".into());
            app.loading = false;
            app.workspace_closed = false;
            app.models = vec![ModelInfo {
                id: "claude".into(),
                provider_id: "anthropic".into(),
                name: "Claude Sonnet".into(),
                context_limit: None,
                variant: None,
                variants: Vec::new(),
            }];
            app.blocks = Arc::new(vec![Block {
                id: "user-1".into(),
                message_id: "message-1".into(),
                part_id: "user-1".into(),
                kind: BlockKind::User,
                text: "fix it".into(),
                detail: String::new().into(),
                output: String::new().into(),
                status: ToolStatus::None,
                streaming: false,
                agent: Some("build".into()),
                model: Some("claude".into()),
                created_at: 0,
                completed_at: None,
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            let rows = transcript_view::project(
                &app.blocks,
                false,
                "ses_meta",
                &app.transcript_ui,
            );
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(
            cx.debug_bounds("user-message-meta-user-1").is_some(),
            "user messages expose Source's agent · model metadata"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn mixed_launch_batch_waits_for_the_directory_before_opening_files(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_string_lossy().into_owned();
        let file = directory.path().join("a.txt");
        std::fs::write(&file, "alpha").unwrap();
        app.update(cx, |app, cx| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = true;
            app.test_workspace_directory = Some(root.clone());
            app.open_launch_paths(
                vec![directory.path().to_path_buf(), file.clone()],
                cx,
            );
        });
        app.read_with(cx, |app, _| {
            assert!(app.launch_busy);
            assert_eq!(app.launch_directory.as_deref(), Some(root.as_str()));
            assert_eq!(app.pending_launch_paths.len(), 1);
            assert!(app.created_files_to_open.is_empty());
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::WorkspaceOpened {
                    directory: root.clone(),
                    result: Ok("ses_launch".into()),
                },
                cx,
            );
        });
        draw(cx);
        app.read_with(cx, |app, _| {
            assert!(!app.launch_busy);
            assert!(app.pending_launch_paths.is_empty());
            let scope = app
                .document_scopes
                .get("ses_launch")
                .expect("directory context scope");
            assert!(
                app.documents
                    .borrow()
                    .tabs(scope)
                    .iter()
                    .any(|tab| tab.key.rel == "a.txt"),
                "the queued file opens as a tab in the directory context: {:?}",
                app.documents.borrow().tabs(scope)
            );
            assert!(app.external_files_to_open.is_empty());
        });
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[test]
    fn multi_file_launch_classification_preserves_order_and_membership() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("sub")).unwrap();
        let first = workspace.path().join("a.ts");
        let second = workspace.path().join("sub/b.ts");
        let external = outside.path().join("c.ts");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();
        std::fs::write(&external, "c").unwrap();
        let workspace = workspace.path().canonicalize().unwrap();
        let targets = [&first, &second, &external]
            .into_iter()
            .map(|path| classify_launch_file(path, &workspace.to_string_lossy(), "token"))
            .collect::<Vec<_>>();
        assert_eq!(targets.len(), 3);
        match targets[0].as_ref().unwrap() {
            Ok(key) => assert_eq!(key.rel, "a.ts"),
            Err(error) => panic!("expected workspace file, got {error}"),
        }
        match targets[1].as_ref().unwrap() {
            Ok(key) => assert_eq!(key.rel, "sub/b.ts"),
            Err(error) => panic!("expected nested workspace file, got {error}"),
        }
        match targets[2].as_ref().unwrap() {
            Err(path) => assert_eq!(
                path,
                &external
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            ),
            Ok(key) => panic!("expected external file, got {}", key.rel),
        }
    }

    #[test]
    fn pending_launch_paths_trim_dedupe_cap_and_preserve_order() {
        let mut pending = PendingOpenPaths::default();
        pending.push(vec![
            std::path::PathBuf::from(" /a "),
            std::path::PathBuf::from("/a"),
            std::path::PathBuf::from("   "),
            std::path::PathBuf::from("/b"),
        ]);
        pending.push(vec![
            std::path::PathBuf::from("/b"),
            std::path::PathBuf::from("/c"),
        ]);
        assert_eq!(
            pending.take(),
            vec![
                std::path::PathBuf::from("/a"),
                std::path::PathBuf::from("/b"),
                std::path::PathBuf::from("/c")
            ]
        );
        for index in 0..12 {
            pending.push(vec![std::path::PathBuf::from(format!("/p{index}"))]);
        }
        let paths = pending.take();
        assert_eq!(paths.len(), MAX_PENDING_PATHS);
        assert_eq!(paths[0], std::path::PathBuf::from("/p0"));
        assert_eq!(paths[9], std::path::PathBuf::from("/p9"));
    }

    struct SplitDragProbe {
        fraction: f32,
        bounds: Option<Bounds<Pixels>>,
        drag: Option<f32>,
    }

    impl Render for SplitDragProbe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .relative()
                .flex()
                .size_full()
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    let Some(last) = this.drag else {
                        return;
                    };
                    if event.pressed_button != Some(MouseButton::Left) {
                        this.drag = None;
                        return;
                    }
                    let width = this
                        .bounds
                        .map(|bounds| f32::from(bounds.size.width))
                        .unwrap_or(0.);
                    let x = f32::from(event.position.x);
                    this.fraction = resized_split_fraction(this.fraction, x - last, width);
                    this.drag = Some(x);
                    cx.notify();
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.drag = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .id("probe-split-left")
                        .debug_selector(|| "probe-split-left".into())
                        .flex_basis(relative(self.fraction))
                        .flex_grow_0()
                        .flex_shrink_0()
                        .h_full()
                        .bg(Theme::default().editor),
                )
                .child(
                    div()
                        .id("probe-split-divider")
                        .debug_selector(|| "probe-split-divider".into())
                        .w(px(5.))
                        .h_full()
                        .flex_shrink_0()
                        .bg(Theme::default().border)
                        .cursor(CursorStyle::ResizeColumn)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                this.drag = Some(f32::from(event.position.x));
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .id("probe-split-right")
                        .debug_selector(|| "probe-split-right".into())
                        .flex_1()
                        .h_full()
                        .bg(Theme::default().inset),
                )
                .child(
                    gpui::canvas(
                        |_, _, _| {},
                        |bounds, _, _, _| {
                            let _ = bounds;
                        },
                    )
                    .absolute()
                    .size_full(),
                )
        }
    }

    struct EditorGroupGridProbe {
        count: usize,
    }

    impl Render for EditorGroupGridProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let cells = crate::editor_groups::group_cells(self.count, 0.5, 0.5);
            let mut root = div().relative().size_full();
            for (index, cell) in cells.into_iter().enumerate() {
                let id = format!("group-cell-{index}");
                let selector = id.clone();
                root = root.child(
                    div()
                        .id(SharedString::from(id))
                        .debug_selector(move || selector.clone())
                        .absolute()
                        .left(relative(cell.left / 100.))
                        .top(relative(cell.top / 100.))
                        .w(relative(cell.width / 100.))
                        .h(relative(cell.height / 100.))
                        .bg(theme.inset),
                );
            }
            root
        }
    }

    #[gpui::test]
    fn settings_controls_follow_runtime_capabilities(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.settings_open = true;
            app.settings_section = SettingsSection::Providers;
            app.runtime_probe_visible = true;
            app.runtime_capabilities = RuntimeCapabilities::opencode(true);
        });
        draw(cx);
        assert!(cx.debug_bounds("settings-provider-managed").is_none());
        app.update(cx, |app, _| {
            app.runtime_capabilities.provider_credentials = false;
        });
        draw(cx);
        assert!(cx.debug_bounds("settings-provider-managed").is_some());

        app.update(cx, |app, _| {
            app.settings_section = SettingsSection::Model;
            app.runtime_capabilities = RuntimeCapabilities::opencode(true);
        });
        draw(cx);
        assert!(cx.debug_bounds("settings-permission-row").is_some());
        assert!(
            cx.debug_bounds("settings-steering-row").is_none(),
            "OpenCode reports steering disabled"
        );
        app.update(cx, |app, _| {
            app.runtime_capabilities.steering = true;
        });
        draw(cx);
        assert!(cx.debug_bounds("settings-steering-row").is_some());
        let steer = cx
            .debug_bounds("follow-up-steer")
            .expect("follow-up steer control");
        cx.simulate_click(steer.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.settings.follow_up_behavior),
            vela_engine::message_queue::FollowUpBehavior::Steer
        );
        app.update(cx, |app, _| {
            app.runtime_capabilities.permissions = false;
        });
        draw(cx);
        assert!(cx.debug_bounds("settings-permission-row").is_none());

        app.update(cx, |app, cx| {
            app.settings_open = false;
            app.runtime_capabilities = RuntimeCapabilities::opencode(true);
            app.tray.show();
            app.composer = "/".into();
            app.command_catalog = vela_opencode::commands::builtins();
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds("composer-add").is_some());
        assert!(cx.debug_bounds("agent-picker").is_some());
        assert!(cx.debug_bounds("composer-slash-menu").is_some());
        assert!(cx.debug_bounds("new-tui-terminal").is_some());

        app.update(cx, |app, cx| {
            app.runtime_capabilities.attachments = false;
            app.runtime_capabilities.commands = false;
            app.runtime_capabilities.agents = false;
            app.runtime_capabilities.tui = false;
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds("composer-add").is_none());
        assert!(cx.debug_bounds("agent-picker").is_none());
        assert!(cx.debug_bounds("composer-slash-menu").is_none());
        assert!(cx.debug_bounds("new-tui-terminal").is_none());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn settings_changes_are_handed_off_to_the_engine(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        app.update(cx, |app, cx| {
            app.settings_open = true;
            app.settings_section = SettingsSection::Appearance;
            app.set_shell(UiShell::Classic, cx);
        });
        cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
            app.clone().into_any_element()
        });
        assert_eq!(app.read_with(cx, |app, _| app.ui_shell), UiShell::Classic);
        runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while engine.settings().shell != "classic" {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("engine receives the settings handoff");
        });
        assert_eq!(engine.settings().shell, "classic");
    }

    #[gpui::test]
    fn three_and_four_editor_group_cells_fill_the_grid_without_overlap(
        cx: &mut gpui::TestAppContext,
    ) {
        for count in [3, 4] {
            let (probe, cx) = cx.add_window_view(|_, _| EditorGroupGridProbe { count });
            cx.simulate_resize(size(px(600.), px(400.)));
            cx.draw(point(px(0.), px(0.)), size(px(600.), px(400.)), |_, _| {
                probe.clone().into_any_element()
            });
            let first = cx.debug_bounds("group-cell-0").expect("top-left cell");
            let second = cx.debug_bounds("group-cell-1").expect("top-right cell");
            let third = cx.debug_bounds("group-cell-2").expect("bottom-left cell");
            assert!((f32::from(first.size.width) - 300.).abs() < 0.5);
            assert!((f32::from(first.size.height) - 200.).abs() < 0.5);
            assert!((f32::from(second.left()) - 300.).abs() < 0.5);
            assert!((f32::from(third.top()) - 200.).abs() < 0.5);
            if count == 3 {
                assert!(
                    (f32::from(third.size.width) - 600.).abs() < 0.5,
                    "the third group spans the full row"
                );
            } else {
                let fourth = cx.debug_bounds("group-cell-3").expect("bottom-right cell");
                assert!((f32::from(third.size.width) - 300.).abs() < 0.5);
                assert!((f32::from(fourth.left()) - 300.).abs() < 0.5);
                assert!((f32::from(fourth.top()) - 200.).abs() < 0.5);
            }
        }
    }

    #[gpui::test]
    fn split_divider_drag_resizes_and_clamps(cx: &mut gpui::TestAppContext) {
        let (probe, cx) = cx.add_window_view(|_, _| SplitDragProbe {
            fraction: 0.5,
            bounds: None,
            drag: None,
        });
        cx.simulate_resize(size(px(600.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(600.), px(200.)), |_, _| {
                probe.clone().into_any_element()
            })
        };
        draw(cx);
        probe.update(cx, |probe, _| {
            probe.bounds = Some(Bounds::new(point(px(0.), px(0.)), size(px(600.), px(200.))))
        });
        let divider = cx.debug_bounds("probe-split-divider").unwrap().center();
        let left_before = cx.debug_bounds("probe-split-left").unwrap().size.width;
        cx.simulate_event(gpui::MouseDownEvent {
            position: divider,
            button: MouseButton::Left,
            ..Default::default()
        });
        cx.simulate_event(gpui::MouseMoveEvent {
            position: point(divider.x + px(100.), divider.y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        cx.simulate_event(gpui::MouseUpEvent {
            position: point(divider.x + px(100.), divider.y),
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        let left_after = cx.debug_bounds("probe-split-left").unwrap().size.width;
        assert!(left_after > left_before + px(90.));
        assert!(probe.read_with(cx, |probe, _| probe.fraction) > 0.6);
    }

    #[test]
    fn group_resize_fraction_matches_clamped_drag_math() {
        assert!((resized_split_fraction(0.5, 100., 1000.) - 0.6).abs() < 1e-5);
        assert!((resized_split_fraction(0.5, -100., 1000.) - 0.4).abs() < 1e-5);
        assert_eq!(resized_split_fraction(0.5, 10000., 1000.), 0.8);
        assert_eq!(resized_split_fraction(0.5, -10000., 1000.), 0.2);
        assert_eq!(resized_split_fraction(0.5, 100., 0.), 0.5);
    }

    #[gpui::test]
    fn browser_download_tray_opens_lists_statuses_and_clears(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.browser_open = true;
            app.browser_tabs = vec![BrowserTab::default()];
            app.browser_active = 0;
            app.browser_downloads = vec![
                crate::browser_downloads::BrowserDownload {
                    id: 2,
                    name: "Report.pdf".into(),
                    url: "https://example.com/Report.pdf".into(),
                    path: Some("/Users/qa/Downloads/Report.pdf".into()),
                    success: true,
                    at: 2,
                },
                crate::browser_downloads::BrowserDownload {
                    id: 1,
                    name: "Broken.zip".into(),
                    url: "https://example.com/Broken.zip".into(),
                    path: None,
                    success: false,
                    at: 1,
                },
            ];
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("browser-downloads-toggle").is_some(),
            "a non-empty history exposes the tray control"
        );
        assert!(cx.debug_bounds("browser-downloads").is_none());
        let toggle = cx.debug_bounds("browser-downloads-toggle").unwrap();
        cx.simulate_click(toggle.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("browser-downloads").is_some());
        assert!(cx.debug_bounds("browser-download-2").is_some());
        assert!(cx.debug_bounds("browser-download-1").is_some());
        assert!(
            cx.debug_bounds("browser-download-reveal-2").is_some(),
            "a saved path exposes the reveal action"
        );
        assert!(
            cx.debug_bounds("browser-download-reveal-1").is_none(),
            "an entry without a path has no reveal action"
        );
        let remove = cx
            .debug_bounds("browser-download-remove-2")
            .expect("per-download remove action");
        cx.simulate_click(remove.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("browser-download-2").is_none());
        assert!(cx.debug_bounds("browser-download-1").is_some());
        assert_eq!(
            app.read_with(cx, |app, _| app.browser_downloads.len()),
            1,
            "removing one row leaves the other history entry"
        );
        let clear = cx
            .debug_bounds("browser-downloads-clear")
            .expect("clear action");
        cx.simulate_click(clear.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("browser-downloads").is_none());
        assert!(cx.debug_bounds("browser-downloads-toggle").is_none());
        assert!(
            app.read_with(cx, |app, _| app.browser_downloads.is_empty()),
            "clearing the tray removes the persistent entries"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    struct BrowserButtonProbe {
        focus: gpui::FocusHandle,
        activations: Rc<std::cell::Cell<usize>>,
    }
    impl Render for BrowserButtonProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let activations = self.activations.clone();
            div().size_full().child(browser_button(
                "‹",
                "Back",
                true,
                self.focus.clone(),
                move |_| activations.set(activations.get() + 1),
            ))
        }
    }

    #[gpui::test]
    fn browser_download_tray_actions_support_keyboard_activation(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.browser_open = true;
            app.browser_tabs = vec![BrowserTab::default()];
            app.browser_active = 0;
            app.browser_downloads_open = true;
            app.browser_downloads = vec![
                crate::browser_downloads::BrowserDownload {
                    id: 2,
                    name: "Report.pdf".into(),
                    url: "https://example.com/Report.pdf".into(),
                    path: None,
                    success: true,
                    at: 2,
                },
                crate::browser_downloads::BrowserDownload {
                    id: 1,
                    name: "Broken.zip".into(),
                    url: "https://example.com/Broken.zip".into(),
                    path: None,
                    success: false,
                    at: 1,
                },
            ];
        });
        draw(cx);
        let remove_focus =
            app.read_with(cx, |app, _| app.browser_download_actions_focus[1].clone());
        cx.update(|window, cx| window.focus(&remove_focus, cx));
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(cx.debug_bounds("browser-download-2").is_none());
        assert!(cx.debug_bounds("browser-download-1").is_some());
        assert_eq!(app.read_with(cx, |app, _| app.browser_downloads.len()), 1);

        let clear_focus =
            app.read_with(cx, |app, _| app.browser_download_actions_focus[6].clone());
        cx.update(|window, cx| window.focus(&clear_focus, cx));
        cx.simulate_keystrokes("space");
        draw(cx);
        assert!(cx.debug_bounds("browser-downloads").is_none());
        assert!(app.read_with(cx, |app, _| app.browser_downloads.is_empty()));
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn browser_nav_buttons_support_pointer_and_keyboard_activation(
        cx: &mut gpui::TestAppContext,
    ) {
        let (probe, cx) = cx.add_window_view(|_, cx| BrowserButtonProbe {
            focus: cx.focus_handle().tab_stop(true),
            activations: Rc::new(std::cell::Cell::new(0)),
        });
        cx.simulate_resize(size(px(200.), px(100.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(100.)), |_, _| {
                probe.clone().into_any_element()
            });
        };
        draw(cx);
        let button = cx
            .debug_bounds("browser-nav-‹")
            .expect("browser nav button");
        cx.simulate_click(button.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(probe.read_with(cx, |probe, _| probe.activations.get()), 1);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            probe.read_with(cx, |probe, _| probe.activations.get()),
            2,
            "Enter activates a focused browser nav button"
        );
        cx.simulate_keystrokes("space");
        draw(cx);
        assert_eq!(
            probe.read_with(cx, |probe, _| probe.activations.get()),
            3,
            "Space activates a focused browser nav button"
        );
    }

    #[test]
    fn usage_cache_summary_matches_source_total_and_percent_wording() {
        let usage = SessionUsage {
            input_tokens: 1_000,
            output_tokens: 1_000,
            reasoning_tokens: 0,
            cache_read_tokens: 1_500,
            cache_write_tokens: 500,
            cost: 0.,
        };
        assert_eq!(usage.total_tokens(), 4_000);
        assert_eq!(usage_cache_summary(&usage), "2.0k · 50%");
        assert_eq!(
            usage_cache_summary(&SessionUsage {
                input_tokens: 100,
                ..Default::default()
            }),
            "0"
        );
    }

    #[gpui::test]
    fn connection_recovery_banner_reloads_ui_state_and_clears_after_ready(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.connection = vela_engine::ConnectionState::Connecting;
            app.recovering_from_disconnect = true;
            app.error = Some("OpenCode service stopped.".into());
            app.sessions = Arc::new(vec![SessionSummary {
                id: "ses_stale".into(),
                title: "Stale session".into(),
                directory: "/workspace".into(),
                updated_at: 1,
                parent_id: None,
                agent: None,
            }]);
            app.catalog_session = Some("ses_stale".into());
            app.command_session = Some("ses_stale".into());
            app.forms_session = Some("ses_stale".into());
            app.inbox_session = Some("ses_stale".into());
            app.command_catalog.push(CommandOption {
                name: "stale".into(),
                description: None,
                kind: vela_opencode::CommandKind::Command,
            });
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("connection-recovery").is_some(),
            "a non-ready connection exposes the recovery banner"
        );
        let reload = cx
            .debug_bounds("connection-reload")
            .expect("recovery banner exposes Reload");
        cx.simulate_click(reload.center(), gpui::Modifiers::default());
        draw(cx);
        app.read_with(cx, |app, _| {
            assert!(app.sessions.is_empty(), "reload re-reads the engine inventory");
            assert_eq!(app.catalog_session, None);
            assert_eq!(app.command_session, None);
            assert_eq!(app.forms_session, None);
            assert_eq!(app.inbox_session, None);
            assert!(app.command_catalog.is_empty());
        });
        assert!(
            cx.debug_bounds("connection-recovery").is_some(),
            "the banner stays while the service is still reconnecting"
        );

        app.update(cx, |app, _| {
            app.sessions = Arc::new(vec![SessionSummary {
                id: "ses_again".into(),
                title: "Stale again".into(),
                directory: "/workspace".into(),
                updated_at: 2,
                parent_id: None,
                agent: None,
            }]);
            app.catalog_session = Some("ses_again".into());
            app.command_catalog.push(CommandOption {
                name: "stale-again".into(),
                description: None,
                kind: vela_opencode::CommandKind::Command,
            });
        });
        app.update(cx, |app, cx| {
            app.apply_connection_state(vela_engine::ConnectionState::Ready, None, cx);
        });
        app.read_with(cx, |app, _| {
            assert!(
                app.sessions.is_empty(),
                "the Ready transition automatically reloads the inventory"
            );
            assert_eq!(app.catalog_session, None);
            assert!(app.command_catalog.is_empty());
            assert!(
                !app.recovering_from_disconnect,
                "the reconnect flag is consumed by the Ready transition"
            );
        });
        runtime.block_on(engine.shutdown_and_wait());
    }

    struct ConnectionBannerProbe {
        connection: Rc<std::cell::Cell<vela_engine::ConnectionState>>,
        error: Option<&'static str>,
    }

    impl Render for ConnectionBannerProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let reload = div()
                .id("probe-reload")
                .debug_selector(|| "probe-reload".into())
                .child("Reload");
            div().size_full().child(connection_recovery_element(
                self.connection.get(),
                self.error,
                reload.into_any_element(),
            ))
        }
    }

    #[gpui::test]
    fn connection_recovery_banner_renders_for_non_ready_states_only(
        cx: &mut gpui::TestAppContext,
    ) {
        let connection = Rc::new(std::cell::Cell::new(
            vela_engine::ConnectionState::Connecting,
        ));
        let (probe, cx) = cx.add_window_view(|_, _| ConnectionBannerProbe {
            connection: connection.clone(),
            error: Some("OpenCode service stopped."),
        });
        cx.simulate_resize(size(px(900.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(200.)), |_, _| {
                probe.clone().into_any_element()
            })
        };
        draw(cx);
        assert!(cx.debug_bounds("connection-recovery").is_some());
        assert!(cx.debug_bounds("probe-reload").is_some());

        connection.set(vela_engine::ConnectionState::Ready);
        draw(cx);
        assert!(
            cx.debug_bounds("connection-recovery").is_none(),
            "a Ready engine renders no recovery banner"
        );

        connection.set(vela_engine::ConnectionState::Error);
        draw(cx);
        assert!(
            cx.debug_bounds("connection-recovery").is_some(),
            "the banner returns when the service is lost again"
        );
    }

    #[test]
    fn browser_download_feedback_names_saved_files_successes_and_failures() {
        assert_eq!(
            VelaApp::browser_download_feedback(
                "https://example.com/report.pdf",
                Some(Path::new("report (1).pdf")),
                true
            ),
            ("Downloaded report (1).pdf".into(), false)
        );
        assert_eq!(
            VelaApp::browser_download_feedback(
                "https://example.com/files/%E4%B8%96%E7%95%8C.txt",
                None,
                true
            ),
            ("Downloaded 世界.txt".into(), false)
        );
        assert_eq!(
            VelaApp::browser_download_feedback("https://example.com/", None, true),
            ("Download complete".into(), false)
        );
        assert_eq!(
            VelaApp::browser_download_feedback("https://example.com/report.pdf", None, false),
            ("Download failed: report.pdf".into(), true)
        );
        assert_eq!(
            VelaApp::browser_download_feedback("https://example.com/", None, false),
            ("Download failed".into(), true)
        );
    }

    #[test]
    fn browser_tab_load_failure_and_timeout_clear_loading_and_set_error() {
        use std::time::{Duration, Instant};
        let mut tab = BrowserTab::default();
        tab.begin_load("https://example.com".into());
        assert!(tab.loading);
        assert!(tab.error.is_none());
        assert!(tab.loading_started.is_some());
        assert!(!tab.check_timeout(Instant::now(), Duration::from_secs(30)));

        tab.fail("The browser process stopped unexpectedly.".into());
        assert!(!tab.loading);
        assert_eq!(
            tab.error.as_deref(),
            Some("The browser process stopped unexpectedly.")
        );
        assert!(tab.loading_started.is_none());

        tab.begin_load("https://example.com/retry".into());
        assert!(tab.error.is_none());
        assert!(tab.check_timeout(
            Instant::now() + Duration::from_secs(31),
            Duration::from_secs(30)
        ));
        assert_eq!(
            tab.error.as_deref(),
            Some("The page took too long to load.")
        );
        assert!(!tab.loading);

        tab.begin_load("https://example.com/finally".into());
        tab.finish_load("https://example.com/finally".into());
        assert!(!tab.loading);
        assert!(tab.error.is_none());
        assert!(tab.loading_started.is_none());
    }

    #[gpui::test]
    fn open_session_rows_close_panels_without_deleting_conversations(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.active_id = Some("ses_open".into());
            app.loading = false;
            app.workspace_closed = false;
            app.sessions = Arc::new(vec![
                SessionSummary {
                    id: "ses_open".into(),
                    title: "Open thread".into(),
                    directory: "/workspace".into(),
                    updated_at: 2,
                    parent_id: None,
                    agent: None,
                },
                SessionSummary {
                    id: "ses_closed".into(),
                    title: "Closed thread".into(),
                    directory: "/workspace".into(),
                    updated_at: 1,
                    parent_id: None,
                    agent: None,
                },
            ]);
            let panel = crate::panels::PanelState::new("ses_open", "Open thread");
            app.panels.open(panel).unwrap();
            app.panels.activate("ses_open");
            app.refresh_session_filter();
            assert_eq!(app.filtered_sessions.len(), 2);
        });
        draw(cx);
        cx.run_until_parked();
        assert!(app.read_with(cx, |app, _| app.panels.contains("ses_open")));
        assert_eq!(app.read_with(cx, |app, _| app.filtered_sessions.len()), 2);
        assert!(
            cx.debug_bounds("close-session:ses_closed").is_none(),
            "closed conversations have no close action"
        );
        let close = cx
            .debug_bounds("close-session:ses_open")
            .expect("open conversation exposes Source's close action");
        cx.simulate_click(close.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(!app.read_with(cx, |app, _| app.panels.contains("ses_open")));
        assert_eq!(app.read_with(cx, |app, _| app.active_id.clone()), None);
        assert!(
            app.read_with(cx, |app, _| app
                .sessions
                .iter()
                .any(|session| session.id == "ses_open")),
            "closing a panel must not delete the conversation"
        );
        assert!(cx.debug_bounds("close-session:ses_open").is_none());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn subagent_synthetic_records_render_a_card_that_opens_the_child(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_parent".into());
            app.loading = false;
            app.workspace_closed = false;
            app.sessions = Arc::new(vec![SessionSummary {
                id: "ses_child".into(),
                title: "Run the tests".into(),
                directory: "/workspace".into(),
                updated_at: 200,
                parent_id: Some("ses_parent".into()),
                agent: Some("build".into()),
            }]);
            app.agents = vec![AgentInfo {
                id: "build".into(),
                name: "Build".into(),
            }];
            app.blocks = Arc::new(vec![Block {
                id: "synthetic".into(),
                message_id: "synthetic-message".into(),
                part_id: "synthetic".into(),
                kind: BlockKind::System,
                text: "<subagent id=\"ses_child\" agent=\"build\" description=\"Run the tests\" state=\"completed\"></subagent>".into(),
                detail: String::new().into(),
                output: String::new().into(),
                status: ToolStatus::None,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(2000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            let rows = transcript_view::project(
                &app.blocks,
                false,
                "ses_parent",
                &app.transcript_ui,
            );
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        let card = cx
            .debug_bounds("subagent-link")
            .expect("subagent record renders as a delegated-agent card");
        cx.simulate_click(card.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_child".into())
        );
        assert!(app.read_with(cx, |app, _| app.panels.contains("ses_child")));
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn child_card_replaces_the_parent_panel_and_back_returns(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.active_id = Some("ses_parent".into());
            app.loading = false;
            app.workspace_closed = false;
            app.sessions = Arc::new(vec![
                SessionSummary {
                    id: "ses_parent".into(),
                    title: "Parent thread".into(),
                    directory: "/workspace".into(),
                    updated_at: 100,
                    parent_id: None,
                    agent: None,
                },
                SessionSummary {
                    id: "ses_child".into(),
                    title: "Run the tests".into(),
                    directory: "/workspace".into(),
                    updated_at: 200,
                    parent_id: Some("ses_parent".into()),
                    agent: Some("build".into()),
                },
            ]);
            app.agents = vec![AgentInfo {
                id: "build".into(),
                name: "Build".into(),
            }];
            app.blocks = Arc::new(vec![Block {
                id: "synthetic".into(),
                message_id: "synthetic-message".into(),
                part_id: "synthetic".into(),
                kind: BlockKind::System,
                text: "<subagent id=\"ses_child\" agent=\"build\" description=\"Run the tests\" state=\"completed\"></subagent>".into(),
                detail: String::new().into(),
                output: String::new().into(),
                status: ToolStatus::None,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(2000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_parent",
                    "Parent thread",
                ))
                .unwrap();
            app.panels.activate("ses_parent");
            let rows =
                transcript_view::project(&app.blocks, false, "ses_parent", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        let card = cx
            .debug_bounds("subagent-link")
            .expect("subagent record renders as a delegated-agent card");
        cx.simulate_click(card.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_child".into())
        );
        assert_eq!(app.read_with(cx, |app, _| app.panels.len()), 1);
        assert!(
            !app.read_with(cx, |app, _| app.panels.contains("ses_parent")),
            "drill-down replaces the focused panel like Source's reopenSession"
        );
        let back = cx
            .debug_bounds("agent-session-back")
            .expect("child sessions expose Source's agent-header back button");
        cx.simulate_click(back.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_parent".into())
        );
        assert_eq!(app.read_with(cx, |app, _| app.panels.len()), 1);
        assert!(
            !app.read_with(cx, |app, _| app.panels.contains("ses_child")),
            "back navigation replaces the child slot instead of growing the grid"
        );
        assert!(
            cx.debug_bounds("agent-session-back").is_none(),
            "root sessions have no parent back control"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn task_tool_cards_open_children_and_unresolved_cards_are_inert(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let tool = Block {
            id: "task-tool".into(),
            message_id: "task-message".into(),
            part_id: "task-tool".into(),
            kind: BlockKind::Tool,
            text: "{\"agent\":\"build\",\"description\":\"Run the tests\"}".into(),
            detail: "task".into(),
            output: String::new().into(),
            status: ToolStatus::Running,
            streaming: true,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_parent".into());
            app.loading = false;
            app.workspace_closed = false;
            app.sessions = Arc::new(Vec::new());
            app.agents = vec![AgentInfo {
                id: "build".into(),
                name: "Build".into(),
            }];
            app.blocks = Arc::new(vec![tool.clone()]);
            let rows =
                transcript_view::project(&app.blocks, true, "ses_parent", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(
            cx.debug_bounds("subagent-card").is_some(),
            "an unresolved dispatch still shows its task card"
        );
        assert!(cx.debug_bounds("subagent-link").is_none());
        let card = cx.debug_bounds("subagent-card").unwrap();
        cx.simulate_click(card.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_parent".into())
        );

        app.update(cx, |app, _| {
            app.sessions = Arc::new(vec![SessionSummary {
                id: "ses_child".into(),
                title: "Run the tests".into(),
                directory: "/workspace".into(),
                updated_at: 200,
                parent_id: Some("ses_parent".into()),
                agent: Some("build".into()),
            }]);
            let mut linked = tool.clone();
            linked.child_session_id = Some("ses_child".into());
            linked.status = ToolStatus::Success;
            linked.streaming = false;
            app.blocks = Arc::new(vec![linked]);
            let rows =
                transcript_view::project(&app.blocks, true, "ses_parent", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("subagent-card").is_none());
        let card = cx
            .debug_bounds("subagent-link")
            .expect("a resolved task tool renders a clickable delegated-agent card");
        cx.simulate_click(card.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_child".into())
        );
        assert!(app.read_with(cx, |app, _| app.panels.contains("ses_child")));
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn retry_notice_renders_spinner_message_and_info(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_retry".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![Block {
                id: "retry".into(),
                message_id: "assistant-retry".into(),
                part_id: "retry".into(),
                kind: BlockKind::Retry,
                text: "[VELA_RETRY_SCHEDULED] Rate limited".into(),
                detail: "attempt 2".into(),
                output: String::new().into(),
                status: ToolStatus::None,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: None,
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            let rows =
                transcript_view::project(&app.blocks, false, "ses_retry", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("retry-notice").is_some());
        assert!(cx.debug_bounds("retry-message").is_some());
        assert!(cx.debug_bounds("retry-info").is_some());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn compaction_notice_renders_divider_and_error(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_compact".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![Block {
                id: "compaction".into(),
                message_id: "assistant-compaction".into(),
                part_id: "compaction".into(),
                kind: BlockKind::Compaction,
                text: "Summary of the compacted context.".into(),
                detail: String::new().into(),
                output: "[VELA_COMPACTION_FAILED] Compaction failed".into(),
                status: ToolStatus::Failed,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(2000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            let rows =
                transcript_view::project(&app.blocks, false, "ses_compact", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("compaction-notice").is_some());
        assert!(cx.debug_bounds("compaction-error").is_some());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn skill_notice_renders_label_detail_and_text(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_skill".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![Block {
                id: "skill".into(),
                message_id: "skill-event".into(),
                part_id: "skill".into(),
                kind: BlockKind::Skill,
                text: "Checked the changes.".into(),
                detail: "review".into(),
                output: "Review".into(),
                status: ToolStatus::None,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(1000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            let rows =
                transcript_view::project(&app.blocks, false, "ses_skill", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("skill-notice").is_some());
        assert!(cx.debug_bounds("skill-label").is_some());
        assert!(cx.debug_bounds("skill-detail").is_some());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn running_shimmer_renders_on_tool_and_compaction_labels(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let tool = Block {
            id: "tool".into(),
            message_id: "assistant-tool".into(),
            part_id: "tool".into(),
            kind: BlockKind::Tool,
            text: "{\"command\":\"cargo test\"}".into(),
            detail: "bash".into(),
            output: String::new().into(),
            status: ToolStatus::Running,
            streaming: true,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        let compaction = Block {
            id: "compaction".into(),
            message_id: "assistant-compaction".into(),
            part_id: "compaction".into(),
            kind: BlockKind::Compaction,
            text: String::new().into(),
            detail: String::new().into(),
            output: String::new().into(),
            status: ToolStatus::Running,
            streaming: true,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_shimmer".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![tool.clone(), compaction.clone()]);
            let rows =
                transcript_view::project(&app.blocks, true, "ses_shimmer", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("tool-shimmer").is_some());
        assert!(cx.debug_bounds("compaction-shimmer").is_some());

        app.update(cx, |app, _| {
            let mut done_tool = tool.clone();
            done_tool.status = ToolStatus::Success;
            done_tool.streaming = false;
            done_tool.completed_at = Some(2000);
            let mut done_compaction = compaction.clone();
            done_compaction.status = ToolStatus::Success;
            done_compaction.streaming = false;
            done_compaction.completed_at = Some(2000);
            app.blocks = Arc::new(vec![done_tool, done_compaction]);
            let rows =
                transcript_view::project(&app.blocks, true, "ses_shimmer", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("tool-shimmer").is_none());
        assert!(cx.debug_bounds("compaction-shimmer").is_none());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn answer_footer_metadata_renders_agent_model_duration_and_omits_absent_fields(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let block = |id: &str, kind, text: &str, created_at, completed_at| Block {
            id: id.into(),
            message_id: "answer-message".into(),
            part_id: id.into(),
            kind,
            text: text.into(),
            detail: String::new().into(),
            output: String::new().into(),
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at,
            completed_at,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        let mut answer = block(
            "answer",
            BlockKind::Assistant,
            "Final answer",
            1_000,
            Some(4_000),
        );
        answer.agent = Some("build".into());
        answer.model = Some("claude".into());
        app.update(cx, |app, _| {
            app.active_id = Some("ses_footer".into());
            app.loading = false;
            app.workspace_closed = false;
            app.models = vec![ModelInfo {
                id: "claude".into(),
                provider_id: "anthropic".into(),
                name: "Claude Sonnet".into(),
                context_limit: None,
                variant: None,
                variants: Vec::new(),
            }];
            app.blocks = Arc::new(vec![
                block("prompt", BlockKind::User, "Prompt", 1_000, Some(1_000)),
                answer.clone(),
            ]);
            let rows =
                transcript_view::project(&app.blocks, false, "ses_footer", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(
            cx.debug_bounds("answer-meta").is_some(),
            "agent/model/duration metadata renders in the answer footer"
        );

        answer.agent = None;
        answer.model = None;
        answer.completed_at = None;
        app.update(cx, |app, _| {
            app.blocks = Arc::new(vec![
                block("prompt", BlockKind::User, "Prompt", 1_000, Some(1_000)),
                answer.clone(),
            ]);
            let rows =
                transcript_view::project(&app.blocks, false, "ses_footer", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(
            cx.debug_bounds("answer-meta").is_none(),
            "the meta row disappears when agent, model and duration are all absent"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn large_tool_output_and_patch_render_scrollable_regions(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(1400.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(1400.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let tool = |id: &str, detail: &str, output: String| Block {
            id: id.into(),
            message_id: "assistant-tools".into(),
            part_id: id.into(),
            kind: BlockKind::Tool,
            text: "{}".into(),
            detail: detail.into(),
            output: output.into(),
            status: ToolStatus::Success,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: Some(2000),
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        let mut file_tool = tool("file-tool", "custom", "done".into());
        file_tool.text = "{\"query\":\"fixture\"}".into();
        file_tool.files = vec![
            vela_core::transcript::ToolFile {
                uri: "https://example.com/report.csv".into(),
                name: Some("report.csv".into()),
                mime: Some("text/csv".into()),
            },
            vela_core::transcript::ToolFile {
                uri: "file:///tmp/private.log".into(),
                name: None,
                mime: None,
            },
        ];
        file_tool.sub_calls = vec![
            vela_core::transcript::ToolSubCall {
                id: "sub-1".into(),
                title: "Inspect".into(),
                detail: "src/a.ts".into(),
                status: ToolStatus::Success,
                input: "{}".into(),
                output: "ok".into(),
                files: vec![vela_core::transcript::ToolFile {
                    uri: "https://example.com/nested.txt".into(),
                    name: Some("nested.txt".into()),
                    mime: Some("text/plain".into()),
                }],
                sub_calls: vec![vela_core::transcript::ToolSubCall {
                    id: "sub-1-1".into(),
                    title: "Nested child".into(),
                    detail: "src/a.ts".into(),
                    status: ToolStatus::Success,
                    input: "{\"path\":\"src/a.ts\"}".into(),
                    output: "child ok".into(),
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                }],
            },
            vela_core::transcript::ToolSubCall {
                id: "sub-2".into(),
                title: "Skip".into(),
                detail: String::new(),
                status: ToolStatus::Failed,
                input: String::new(),
                output: String::new(),
                files: Vec::new(),
                sub_calls: Vec::new(),
            },
        ];
        app.update(cx, |app, _| {
            app.active_id = Some("ses_tools".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![
                tool("read-tool", "read", "x".repeat(7000)),
                tool(
                    "patch-tool",
                    "patch",
                    "--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new".into(),
                ),
                file_tool.clone(),
            ]);
            app.transcript_ui
                .work_open
                .insert("ses_tools:assistant-tools".into());
            app.transcript_ui
                .tool_open
                .insert("ses_tools:read-tool".into());
            app.transcript_ui
                .tool_open
                .insert("ses_tools:patch-tool".into());
            app.transcript_ui
                .tool_open
                .insert("ses_tools:file-tool".into());
            let rows =
                transcript_view::project(&app.blocks, false, "ses_tools", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("tool-output-region").is_some());
        assert!(cx.debug_bounds("tool-io-input").is_some());
        assert!(cx.debug_bounds("tool-io-output").is_some());
        assert!(cx.debug_bounds("tool-output-diff").is_some());
        assert!(cx.debug_bounds("tool-file:file-tool:0").is_some());
        assert!(cx.debug_bounds("tool-file:file-tool:1").is_some());
        assert!(
            cx.debug_bounds("tool-status-footer:file-tool").is_some(),
            "completed tools render Source's Success/Failed footer"
        );
        assert!(cx.debug_bounds("tool-subcall:file-tool:0").is_some());
        assert!(cx.debug_bounds("tool-subcall:file-tool:1").is_some());
        assert!(
            cx.debug_bounds("tool-subcall-io:file-tool:0").is_none(),
            "nested tool payloads start collapsed like Source's recursive ToolPart"
        );
        let nested = cx
            .debug_bounds("tool-subcall-head:file-tool:0")
            .expect("nested tool head");
        cx.simulate_click(nested.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("tool-subcall-io:file-tool:0").is_some(),
            "expanding a nested tool call reveals its IN/OUT payload"
        );
        assert!(
            cx.debug_bounds("tool-file:file-tool:0:files:0").is_some(),
            "nested files render inside the expanded nested card"
        );
        assert!(
            cx.debug_bounds("tool-subcall:file-tool:0:0").is_some(),
            "nested sub-calls recurse inside the expanded card"
        );
        app.update(cx, |app, _| {
            app.transcript_ui
                .tool_open
                .insert("file-tool::sub::0::0".into());
        });
        draw(cx);
        assert!(
            cx.debug_bounds("tool-subcall-io:file-tool:0:0").is_some(),
            "a recursively nested tool card expands its own payload"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn expanded_shell_copy_writes_command_and_output(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_copy".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![Block {
                id: "shell-tool".into(),
                message_id: "assistant-copy".into(),
                part_id: "shell-tool".into(),
                kind: BlockKind::Tool,
                text: "{\"command\":\"printf hi\"}".into(),
                detail: "bash".into(),
                output: "hello".into(),
                status: ToolStatus::Success,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(2000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            app.transcript_ui
                .work_open
                .insert("ses_copy:assistant-copy".into());
            app.transcript_ui
                .tool_open
                .insert("ses_copy:shell-tool".into());
            let rows = transcript_view::project(&app.blocks, false, "ses_copy", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        let copy = cx.debug_bounds("tool-copy").expect("copy action");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("$ printf hi\n\nhello")
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn tool_card_accessibility_and_keyboard_expansion(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let press = |cx: &mut gpui::VisualTestContext, key: &str| {
            let keystroke = gpui::Keystroke::parse(key).expect("keystroke");
            cx.simulate_event(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(gpui::KeyUpEvent { keystroke });
        };
        let tool = |id: &str, text: &str, output: &str| Block {
            id: id.into(),
            message_id: "assistant-a11y".into(),
            part_id: id.into(),
            kind: BlockKind::Tool,
            text: text.into(),
            detail: "read".into(),
            output: output.into(),
            status: ToolStatus::Success,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: Some(2000),
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_a11y".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![
                tool("read-tool", r#"{"filePath":"/tmp/a"}"#, "output"),
                tool("static-tool", "", ""),
            ]);
            app.transcript_ui
                .work_open
                .insert("ses_a11y:assistant-a11y".into());
            let rows = transcript_view::project(&app.blocks, false, "ses_a11y", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        assert!(cx.debug_bounds("tool-trigger").is_some());
        assert!(cx.debug_bounds("tool-disabled").is_some());
        assert!(cx.debug_bounds("tool-output-region").is_none());

        let trigger = cx.debug_bounds("tool-trigger").unwrap();
        cx.simulate_click(trigger.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("tool-output-region").is_some());

        press(cx, "enter");
        draw(cx);
        assert!(cx.debug_bounds("tool-output-region").is_none());

        press(cx, "space");
        draw(cx);
        assert!(cx.debug_bounds("tool-output-region").is_some());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn model_picker_favorites_and_visibility_toggle_in_settings(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_models".into());
            app.loading = false;
            app.workspace_closed = false;
            app.models = vec![
                ModelInfo {
                    id: "m".into(),
                    name: "Model".into(),
                    provider_id: "p".into(),
                    context_limit: None,
                    variant: None,
                    variants: Vec::new(),
                },
                ModelInfo {
                    id: "n".into(),
                    name: "Other".into(),
                    provider_id: "q".into(),
                    context_limit: None,
                    variant: None,
                    variants: Vec::new(),
                },
            ];
            app.picker = Some(PickerKind::Model);
            app.model_menu_settings = false;
        });
        draw(cx);
        assert!(cx.debug_bounds("model-picker-menu").is_some());
        let favorite = cx.debug_bounds("favorite-p::m").expect("favorite star");
        cx.simulate_click(favorite.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.settings.favorite_models.contains("p::m")));

        let gear = cx
            .debug_bounds("model-menu-settings")
            .expect("settings gear");
        cx.simulate_click(gear.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.model_menu_settings));
        let row = cx.debug_bounds("model-row-p::m").expect("model row");
        cx.simulate_click(row.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.settings.hidden_models.contains("p::m")));
        let provider = cx
            .debug_bounds("visibility-provider-p")
            .expect("provider visibility control");
        cx.simulate_click(provider.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.settings.hidden_providers.contains("p")));
        let other = cx
            .debug_bounds("visibility-provider-q")
            .expect("second provider visibility control");
        cx.simulate_click(other.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.settings.hidden_providers.contains("q")));
        let gear = cx
            .debug_bounds("model-menu-settings")
            .expect("settings gear");
        cx.simulate_click(gear.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("model-row-q::n").is_none());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn validation_status_menu_renders_cancel_and_retry_states(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let key = DocumentKey::new("status-scope".into(), "page.html".into());
        app.update(cx, |app, _| {
            app.active_id = Some("ses_status".into());
            app.document_scopes
                .insert("ses_status".into(), "status-scope".into());
            app.documents.borrow_mut().open(1, key.clone(), true);
            app.documents
                .borrow_mut()
                .finish_load(1, &key.rel, Some("<p>valid</p>\n".into()));
            app.loading = false;
            app.workspace_closed = false;
            app.status_menu_open = true;
            app.validation_requests.insert(7, key.clone());
            app.validation_key = Some(key.clone());
            app.validation_failed = false;
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        draw(cx);
        let cancel = cx
            .debug_bounds("validation-cancel")
            .expect("pending validation cancel action");
        cx.simulate_click(cancel.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(app.read_with(cx, |app, _| app.validation_requests.is_empty()));
        assert!(!app.read_with(cx, |app, _| app.status_menu_open));

        app.update(cx, |app, _| {
            app.status_menu_open = true;
            app.validation_key = Some(key.clone());
            app.validation_failed = true;
        });
        draw(cx);
        assert!(cx.debug_bounds("validation-retry").is_some());
        assert!(cx.debug_bounds("validation-failed-row").is_some());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn local_queue_chips_reorder_by_pointer_and_keyboard(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let data = tempfile::tempdir().unwrap();
        let directory = data.path().to_string_lossy().into_owned();
        let key = format!("{directory}\nses_queue");
        std::fs::write(
            data.path().join("message-queues.json"),
            serde_json::json!({
                "version": 3,
                "queues": {
                    key: [
                        {"id":"q1","text":"First","files":[],"created_at":1},
                        {"id":"q2","text":"Second","files":[],"created_at":2},
                        {"id":"q3","text":"Third","files":[],"created_at":3}
                    ]
                }
            })
            .to_string(),
        )
        .unwrap();
        let config = vela_engine::EngineConfig {
            data_dir: Some(data.path().to_path_buf()),
            ..Default::default()
        };
        let engine = Engine::spawn(config, runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        app.update(cx, |app, _| {
            app.active_id = Some("ses_queue".into());
            app.test_workspace_directory = Some(directory.clone());
            app.loading = false;
            app.workspace_closed = false;
        });
        let wait_order = |expected: &[&str]| {
            runtime.block_on(async {
                for _ in 0..300 {
                    let order = engine
                        .local_queue(&directory, "ses_queue")
                        .into_iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>();
                    if order == expected {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                panic!(
                    "queue order did not settle: {:?}",
                    engine
                        .local_queue(&directory, "ses_queue")
                        .into_iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>()
                );
            });
        };
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        let press_enter = |cx: &mut gpui::VisualTestContext| {
            let keystroke = gpui::Keystroke::parse("enter").expect("enter");
            cx.simulate_event(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(gpui::KeyUpEvent { keystroke });
        };
        draw(cx);
        assert!(cx.debug_bounds("queued-message-list").is_some());
        let down = cx.debug_bounds("queue-1-q1").expect("move down control");
        cx.simulate_click(down.center(), gpui::Modifiers::default());
        wait_order(&["q2", "q1", "q3"]);
        draw(cx);
        press_enter(cx);
        wait_order(&["q2", "q3", "q1"]);
        draw(cx);
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn expanded_tool_copy_is_focus_visible_and_keyboard_activated(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_copy_keys".into());
            app.loading = false;
            app.workspace_closed = false;
            app.blocks = Arc::new(vec![Block {
                id: "shell-tool".into(),
                message_id: "assistant-copy-keys".into(),
                part_id: "shell-tool".into(),
                kind: BlockKind::Tool,
                text: "{\"command\":\"printf hi\"}".into(),
                detail: "bash".into(),
                output: "hello".into(),
                status: ToolStatus::Success,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1000,
                completed_at: Some(2000),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            }]);
            app.transcript_ui
                .work_open
                .insert("ses_copy_keys:assistant-copy-keys".into());
            let rows =
                transcript_view::project(&app.blocks, false, "ses_copy_keys", &app.transcript_ui);
            app.transcript_list =
                ListState::new(rows.len(), ListAlignment::Top, px(420.)).measure_all();
            app.transcript_rows = Arc::new(rows);
        });
        draw(cx);
        let trigger = cx.debug_bounds("tool-trigger").expect("tool trigger");
        cx.simulate_click(trigger.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("tool-output-region").is_some());
        assert!(cx.debug_bounds("tool-copy").is_some());
        let copy = cx.debug_bounds("tool-copy").expect("copy action");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("tool-copy-visible").is_some());
        app.update(cx, |app, cx| {
            app.transcript_ui.copied_tool = None;
            cx.notify();
        });
        draw(cx);
        assert!(
            cx.debug_bounds("tool-copy-visible").is_some(),
            "keyboard focus should reveal the copy action"
        );
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(String::new()));
        let keystroke = gpui::Keystroke::parse("enter").expect("enter");
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
        draw(cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("$ printf hi\n\nhello")
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn queue_chips_mix_runtime_and_local_recovery_actions(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let data = tempfile::tempdir().unwrap();
        let directory = data.path().to_string_lossy().into_owned();
        let key = format!("{directory}\nses_queue");
        std::fs::write(
            data.path().join("message-queues.json"),
            serde_json::json!({
                "version": 3,
                "queues": {
                    key: [
                        {"id":"local1","text":"Local one","files":[],"created_at":1},
                        {"id":"local2","text":"Local two","files":[],"created_at":2}
                    ]
                }
            })
            .to_string(),
        )
        .unwrap();
        let engine = Engine::spawn(
            vela_engine::EngineConfig {
                data_dir: Some(data.path().to_path_buf()),
                ..Default::default()
            },
            runtime.handle(),
        );
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        app.update(cx, |app, _| {
            app.active_id = Some("ses_queue".into());
            app.test_workspace_directory = Some(directory.clone());
            app.loading = false;
            app.workspace_closed = false;
            app.inbox = vec![
                InboxItem {
                    id: "runtime1".into(),
                    session_id: "ses_queue".into(),
                    text: "Runtime one".into(),
                    delivery: "queue".into(),
                    kind: "queued".into(),
                    attachment_count: 0,
                },
                InboxItem {
                    id: "runtime2".into(),
                    session_id: "ses_queue".into(),
                    text: "Runtime two".into(),
                    delivery: "queue".into(),
                    kind: "queued".into(),
                    attachment_count: 0,
                },
            ];
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("queued-message-list").is_some());
        let runtime_preview = cx
            .debug_bounds("queue-preview-runtime1")
            .expect("runtime preview");
        let local_preview = cx
            .debug_bounds("queue-preview-local1")
            .expect("local preview");
        assert!(runtime_preview.origin.y < local_preview.origin.y);
        assert!(cx.debug_bounds("queue-edit-runtime1").is_some());
        assert!(cx.debug_bounds("queue-steer-runtime1").is_some());
        assert!(cx.debug_bounds("queue-remove-runtime1").is_some());
        assert!(cx.debug_bounds("queue-1-runtime1").is_none());
        assert!(cx.debug_bounds("queue-edit-local1").is_some());
        assert!(cx.debug_bounds("queue-steer-local1").is_some());
        assert!(cx.debug_bounds("queue-remove-local1").is_some());
        assert!(cx.debug_bounds("queue-1-local1").is_some());

        let edit = cx
            .debug_bounds("queue-edit-runtime1")
            .expect("runtime edit");
        cx.simulate_click(edit.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.composer.clone()),
            "Runtime one"
        );
        assert_eq!(app.read_with(cx, |app, _| app.inbox.len()), 1);
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn staged_revert_dock_commits_and_discards_retained_stage(cx: &mut gpui::TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, _| {
            app.active_id = Some("ses_revert".into());
            app.loading = false;
            app.workspace_closed = false;
            app.staged_reverts.insert("ses_revert".into());
        });
        draw(cx);
        assert!(cx.debug_bounds("staged-revert-dock").is_some());
        assert!(cx.debug_bounds("staged-revert-commit").is_some());
        assert!(cx.debug_bounds("staged-revert-discard").is_some());

        let discard = cx
            .debug_bounds("staged-revert-discard")
            .expect("discard action");
        cx.simulate_click(discard.center(), gpui::Modifiers::default());
        draw(cx);
        let discard_request = app.read_with(cx, |app, _| {
            *app.clear_revert_requests
                .keys()
                .next()
                .expect("discard request")
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::RevertFinished {
                    request_id: discard_request,
                    session_id: "ses_revert".into(),
                    error: None,
                },
                cx,
            );
        });
        draw(cx);
        assert!(!app.read_with(cx, |app, _| app.staged_reverts.contains("ses_revert")));
        assert!(cx.debug_bounds("staged-revert-dock").is_none());

        app.update(cx, |app, _| {
            app.staged_reverts.insert("ses_revert".into());
        });
        draw(cx);
        let commit = cx
            .debug_bounds("staged-revert-commit")
            .expect("commit action");
        cx.simulate_click(commit.center(), gpui::Modifiers::default());
        draw(cx);
        let commit_request = app.read_with(cx, |app, _| {
            *app.commit_revert_requests
                .keys()
                .next()
                .expect("commit request")
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::RevertFinished {
                    request_id: commit_request,
                    session_id: "ses_revert".into(),
                    error: None,
                },
                cx,
            );
        });
        draw(cx);
        assert!(!app.read_with(cx, |app, _| app.staged_reverts.contains("ses_revert")));
        assert!(cx.debug_bounds("staged-revert-dock").is_none());
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn revert_busy_actions_and_inflight_requests_are_bounded(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        app.update(cx, |app, cx| {
            app.active_id = Some("ses_revert".into());
            app.loading = false;
            app.workspace_closed = false;
            app.phase = SessionPhase::Busy;
            let before = app.toasts.read(cx).texts().len();
            app.revert_message("message-1".into(), false, String::new(), cx);
            assert!(app.revert_requests.is_empty());
            assert_eq!(
                app.toasts.read(cx).texts().len(),
                before,
                "busy transcripts ignore revert activation like Source's disabled control"
            );
            app.phase = SessionPhase::Idle;
            app.revert_message("message-1".into(), false, String::new(), cx);
            assert_eq!(app.revert_requests.len(), 1);
            assert_eq!(
                app.toasts.read(cx).texts().last().map(String::as_str),
                Some("Undoing prompt…")
            );
            app.revert_requests.clear();
            app.revert_message("message-1".into(), true, "edit me".into(), cx);
            assert_eq!(
                app.toasts.read(cx).texts().last().map(String::as_str),
                Some("Restoring prompt for editing…")
            );
            app.revert_requests.clear();
            app.staged_reverts.insert("ses_revert".into());
            app.commit_staged_revert(cx);
            app.commit_staged_revert(cx);
            assert_eq!(app.commit_revert_requests.len(), 1);
            app.commit_revert_requests.clear();
            app.discard_staged_revert(cx);
            app.discard_staged_revert(cx);
            assert_eq!(app.clear_revert_requests.len(), 1);
            cx.notify();
        });
        draw(cx);
        assert!(
            cx.debug_bounds("staged-revert-dock").is_some(),
            "an empty transcript still renders its staged revert dock"
        );
        let commit = cx
            .debug_bounds("staged-revert-commit")
            .expect("commit action");
        cx.simulate_click(commit.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            app.read_with(cx, |app, _| app.commit_revert_requests.is_empty()),
            "a duplicate commit is ignored while discard is in flight"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn inactive_tile_renders_and_activates_its_own_permission_prompt(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.active_id = Some("ses_active".into());
            app.sessions = Arc::new(vec![
                SessionSummary {
                    id: "ses_active".into(),
                    title: "Active thread".into(),
                    directory: "/workspace".into(),
                    updated_at: 2,
                    parent_id: None,
                    agent: None,
                },
                SessionSummary {
                    id: "ses_waiting".into(),
                    title: "Waiting thread".into(),
                    directory: "/workspace".into(),
                    updated_at: 1,
                    parent_id: None,
                    agent: None,
                },
            ]);
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_active",
                    "Active thread",
                ))
                .unwrap();
            let mut waiting =
                crate::panels::PanelState::new("ses_waiting", "Waiting thread");
            waiting.permissions.push(PermissionRequest {
                id: "perm_waiting".into(),
                session_id: "ses_waiting".into(),
                action: "bash".into(),
                resources: vec!["rm -rf build".into()],
                message: None,
            });
            app.panels.open(waiting).unwrap();
            app.panels.activate("ses_active");
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("panel-permission-ses_waiting-0").is_some(),
            "inactive tiles expose their own pending permission"
        );
        assert!(
            cx.debug_bounds("panel-permission-ses_active-0").is_none(),
            "the active tile does not borrow another panel's prompt"
        );
        let deny = cx
            .debug_bounds("panel-permission-ses_waiting-button-Deny")
            .expect("inactive permission actions are interactive");
        cx.simulate_click(deny.center(), gpui::Modifiers::default());
        draw(cx);
        cx.run_until_parked();
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_waiting".into()),
            "answering a prompt brings its panel forward like Source's mouse-down focus"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn inactive_tile_composer_keeps_its_own_attachments_references_and_slash_menus(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        app.update(cx, |app, cx| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.active_id = Some("ses_active".into());
            app.runtime_capabilities.attachments = true;
            app.runtime_capabilities.commands = true;
            app.command_catalog = vela_opencode::commands::builtins();
            app.command_catalog.push(CommandOption {
                name: "review".into(),
                description: Some("Review a path".into()),
                kind: vela_opencode::CommandKind::Command,
            });
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_active",
                    "Active thread",
                ))
                .unwrap();
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_waiting",
                    "Waiting thread",
                ))
                .unwrap();
            app.panels.activate("ses_active");
            app.ensure_panel_input("ses_waiting", cx);
            app.panel_composers
                .entry("ses_waiting".into())
                .or_default()
                .attachments
                .push(attachments::StagedAttachment {
                    path: std::path::PathBuf::from("/tmp/notes.txt"),
                    name: "notes.txt".into(),
                    mention: None,
                });
        });
        draw(cx);
        assert!(
            cx.debug_bounds("panel-attachment-ses_waiting-0").is_some(),
            "inactive tile renders its staged attachment"
        );
        let remove = cx
            .debug_bounds("panel-remove-attachment-ses_waiting-0")
            .expect("attachment remove action");
        cx.simulate_click(remove.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("panel-attachment-ses_waiting-0").is_none(),
            "removing an attachment is scoped to its tile"
        );

        app.update(cx, |app, cx| {
            let input = app
                .panel_inputs
                .get("ses_waiting")
                .cloned()
                .expect("tile input");
            input.update(cx, |input, cx| {
                input.set_text("@src".into(), cx);
                let caret = input.buffer.text().len();
                input.buffer.select(caret, caret);
            });
            app.update_tile_completions("ses_waiting", cx);
        });
        draw(cx);
        let request = app.read_with(cx, |app, _| {
            app.panel_composers
                .get("ses_waiting")
                .and_then(|state| state.reference_pending)
                .map(|(request, _)| request)
                .expect("tile reference request")
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::References {
                    request_id: request,
                    session_id: "ses_waiting".into(),
                    files: vec!["src/lib.rs".into()],
                    error: None,
                },
                cx,
            );
        });
        draw(cx);
        assert!(
            cx.debug_bounds("panel-reference-menu-ses_waiting").is_some(),
            "inactive tile renders its own reference menu"
        );
        let reference = cx
            .debug_bounds("panel-reference-ses_waiting-0")
            .expect("reference row");
        cx.simulate_click(reference.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, cx| {
                app.panel_inputs
                    .get("ses_waiting")
                    .expect("tile input")
                    .read(cx)
                    .buffer
                    .text()
                    .to_string()
            }),
            "@src/lib.rs"
        );
        assert!(
            app.read_with(cx, |app, _| {
                app.panel_composers
                    .get("ses_waiting")
                    .is_some_and(|state| state.attachments.iter().any(|attachment| {
                        attachment
                            .mention
                            .as_ref()
                            .is_some_and(|mention| mention.text == "@src/lib.rs")
                    }))
            }),
            "selecting a reference stages its mention on the same tile"
        );

        app.update(cx, |app, cx| {
            app.panels
                .update("ses_waiting", |panel| panel.composer = "/review src".into());
            let input = app
                .panel_inputs
                .get("ses_waiting")
                .cloned()
                .expect("tile input");
            input.update(cx, |input, cx| {
                input.set_text("/review src".into(), cx);
                let caret = input.buffer.text().len();
                input.buffer.select(caret, caret);
            });
            app.update_tile_completions("ses_waiting", cx);
        });
        draw(cx);
        let argument = app.read_with(cx, |app, _| {
            app.panel_composers
                .get("ses_waiting")
                .and_then(|state| state.slash_arg_pending)
                .expect("tile slash argument request")
        });
        app.update(cx, |app, cx| {
            app.on_update(
                EngineUpdate::References {
                    request_id: argument,
                    session_id: "ses_waiting".into(),
                    files: vec!["src/lib.rs".into()],
                    error: None,
                },
                cx,
            );
        });
        draw(cx);
        assert!(
            cx.debug_bounds("panel-slash-arguments-ses_waiting").is_some(),
            "inactive tile renders its own slash argument menu"
        );
        let argument = cx
            .debug_bounds("panel-slash-argument-ses_waiting-0")
            .expect("slash argument row");
        cx.simulate_click(argument.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, cx| {
                app.panel_inputs
                    .get("ses_waiting")
                    .expect("tile input")
                    .read(cx)
                    .buffer
                    .text()
                    .to_string()
            }),
            "/review src/lib.rs "
        );

        app.update(cx, |app, cx| {
            app.panels
                .update("ses_waiting", |panel| panel.composer = "/co".into());
            let input = app
                .panel_inputs
                .get("ses_waiting")
                .cloned()
                .expect("tile input");
            input.update(cx, |input, cx| {
                input.set_text("/co".into(), cx);
                let caret = input.buffer.text().len();
                input.buffer.select(caret, caret);
            });
            app.update_tile_completions("ses_waiting", cx);
        });
        draw(cx);
        assert!(
            cx.debug_bounds("panel-slash-menu-ses_waiting").is_some(),
            "inactive tile renders its own slash menu"
        );
        let slash = cx
            .debug_bounds("panel-slash-ses_waiting-compact")
            .expect("slash command row");
        cx.simulate_click(slash.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            app.read_with(cx, |app, cx| {
                app.panel_inputs
                    .get("ses_waiting")
                    .expect("tile input")
                    .read(cx)
                    .buffer
                    .text()
                    .to_string()
            }),
            "",
            "choosing a slash command submits and clears the tile composer"
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[gpui::test]
    fn inactive_tile_tool_rows_expand_and_copy_without_stealing_focus(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig::default(), runtime.handle());
        let settings = engine.settings();
        let pending = Rc::new(std::cell::RefCell::new(PendingOpenPaths::default()));
        let engine_for_app = engine.clone();
        let (app, cx) = cx.add_window_view(move |_, cx| {
            VelaApp::new(engine_for_app, settings, None, pending, cx)
        });
        cx.simulate_resize(size(px(1400.), px(900.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
        };
        runtime.block_on(engine.shutdown_and_wait());
        draw(cx);
        cx.run_until_parked();
        let tool = Block {
            id: "tool-waiting".into(),
            message_id: "message-waiting".into(),
            part_id: "tool-waiting".into(),
            kind: BlockKind::Tool,
            text: "{\"command\":\"printf hi\"}".into(),
            detail: "bash".into(),
            output: "hello".into(),
            status: ToolStatus::Success,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: Some(2000),
            child_session_id: None,
            files: vec![ToolFile {
                uri: "file:///tmp/private.log".into(),
                name: Some("private.log".into()),
                mime: None,
            }],
            sub_calls: vec![ToolSubCall {
                id: "read-call".into(),
                title: "Read".into(),
                detail: "src/lib.rs".into(),
                status: ToolStatus::Success,
                input: "{}".into(),
                output: "file body".into(),
                files: vec![ToolFile {
                    uri: "https://example.com/report.csv".into(),
                    name: Some("report.csv".into()),
                    mime: Some("text/csv".into()),
                }],
                sub_calls: vec![ToolSubCall {
                    id: "nested-grep".into(),
                    title: "Grep".into(),
                    detail: "src".into(),
                    status: ToolStatus::Success,
                    input: "fixture".into(),
                    output: "matched line".into(),
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                }],
            }],
            attachments: Vec::new(),
        };
        app.update(cx, |app, _| {
            app.bootstrapped = true;
            app.loading = false;
            app.workspace_closed = false;
            app.active_id = Some("ses_active".into());
            app.panels
                .open(crate::panels::PanelState::new(
                    "ses_active",
                    "Active thread",
                ))
                .unwrap();
            let mut waiting =
                crate::panels::PanelState::new("ses_waiting", "Waiting thread");
            waiting.blocks = Arc::new(vec![tool.clone()]);
            app.panels.open(waiting).unwrap();
            let mut other = crate::panels::PanelState::new("ses_other", "Other thread");
            other.blocks = Arc::new(vec![tool]);
            app.panels.open(other).unwrap();
            app.panels.activate("ses_active");
        });
        draw(cx);
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("panel-tool-head-ses_waiting-tool-waiting")
                .is_some(),
            "inactive tile renders a tool head"
        );
        assert!(
            cx.debug_bounds("panel-tool-output-ses_waiting-tool-waiting")
                .is_none(),
            "tool payload starts collapsed"
        );
        let head = cx
            .debug_bounds("panel-tool-head-ses_waiting-tool-waiting")
            .expect("tool head");
        cx.simulate_click(head.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("panel-tool-output-ses_waiting-tool-waiting")
                .is_some(),
            "clicking the tool head expands its output"
        );
        assert!(
            cx.debug_bounds("panel-tool-output-ses_other-tool-waiting")
                .is_none(),
            "another panel keeps its own collapsed tool row"
        );
        assert!(
            cx.debug_bounds("panel-tool-file-ses_waiting-tool-waiting-0")
                .is_some(),
            "top-level tool files render inside the inactive tile"
        );
        assert!(
            cx.debug_bounds("panel-subcall-head-ses_waiting-tool-waiting-sub0")
                .is_some(),
            "an expanded tool lists its nested calls"
        );
        assert!(
            cx.debug_bounds("panel-subcall-body-ses_waiting-tool-waiting-sub0")
                .is_none(),
            "nested calls start collapsed"
        );
        let subcall = cx
            .debug_bounds("panel-subcall-head-ses_waiting-tool-waiting-sub0")
            .expect("nested tool head");
        cx.simulate_click(subcall.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("panel-subcall-body-ses_waiting-tool-waiting-sub0")
                .is_some(),
            "clicking a nested call expands its input and output"
        );
        assert!(
            cx.debug_bounds(
                "panel-tool-file-ses_waiting-tool-waiting::sub0-0"
            )
            .is_some(),
            "nested call files render inside the inactive tile"
        );
        let deep = cx
            .debug_bounds("panel-subcall-head-ses_waiting-tool-waiting-sub0.0")
            .expect("deep nested tool head");
        cx.simulate_click(deep.center(), gpui::Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("panel-subcall-body-ses_waiting-tool-waiting-sub0.0")
                .is_some(),
            "recursive nested calls keep independent expansion state"
        );
        assert!(
            cx.debug_bounds("panel-subcall-body-ses_other-tool-waiting-sub0")
                .is_none(),
            "another panel keeps its nested calls collapsed"
        );
        assert_eq!(
            app.read_with(cx, |app, _| app.active_id.clone()),
            Some("ses_active".into()),
            "inspecting an inactive tool row does not steal the active panel"
        );
        let copy = cx
            .debug_bounds("panel-tool-copy-ses_waiting-tool-waiting")
            .expect("copy action");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("$ printf hi\n\nhello")
        );
        assert_eq!(
            app.read_with(cx, |app, _| {
                app.panels
                    .entries()
                    .iter()
                    .find(|panel| panel.session == "ses_waiting")
                    .and_then(|panel| panel.copied_tool.clone())
            }),
            Some("tool-waiting".to_owned())
        );
        runtime.block_on(engine.shutdown_and_wait());
    }

    #[test]
    fn finder_urls_become_absolute_paths_without_losing_unicode_or_spaces() {
        let paths = launch_paths_from_urls(vec![
            "file:///private/tmp/Vela%20Workspace/%E7%95%8C%20%F0%9F%8C%8D.txt".into(),
            "https://example.com/file.txt".into(),
            "relative/path.txt".into(),
            "/private/tmp/plain.txt".into(),
        ]);
        assert_eq!(
            paths,
            vec![
                std::path::PathBuf::from("/private/tmp/Vela Workspace/界 🌍.txt"),
                std::path::PathBuf::from("/private/tmp/plain.txt"),
            ]
        );
    }
}
