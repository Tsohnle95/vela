//! Native structured Markdown. The parser owns syntax; GPUI owns presentation.
use super::*;
use vela_core::markdown::{self, CodeBlockKind, Event, Tag};

#[derive(Default)]
pub(super) struct MarkdownUi {
    wrap: HashMap<String, bool>,
    copied: Option<String>,
    copy_generation: u64,
    focus: HashMap<String, MarkdownFocus>,
    image_sizes: HashMap<String, Option<gpui::Size<gpui::Pixels>>>,
    horizontal: HashMap<String, crate::scroll::AutoScrollHandle>,
    pub(super) keyboard: crate::focus::KeyboardMode,
}
struct MarkdownFocus {
    handle: gpui::FocusHandle,
    answer: Option<Arc<str>>,
    window: gpui::WindowId,
    subscriptions: Vec<gpui::Subscription>,
}
impl MarkdownUi {
    pub(super) fn contains_focused(&self, window: &Window) -> bool {
        self.focus
            .values()
            .any(|focus| focus.handle.is_focused(window))
    }
    pub(super) fn answer_contains_focused(&self, answer: &str, window: &Window) -> bool {
        self.focus
            .values()
            .any(|focus| focus.answer.as_deref() == Some(answer) && focus.handle.is_focused(window))
    }
}
struct RenderContext<'a> {
    ui: &'a mut MarkdownUi,
    window: &'a mut Window,
    cx: &'a mut App,
    owner: Option<gpui::WeakEntity<VelaApp>>,
    user: bool,
    streaming: bool,
    list_depth: usize,
    list: Option<ListState>,
    answer: Option<Arc<str>>,
}
fn control_focus(key: &str, context: &mut RenderContext) -> gpui::FocusHandle {
    let window_id = context.window.window_handle().window_id();
    let owner = context.window.current_view();
    let entry = context
        .ui
        .focus
        .entry(key.to_owned())
        .or_insert_with(|| MarkdownFocus {
            handle: context.cx.focus_handle().tab_stop(true),
            answer: context.answer.clone(),
            window: window_id,
            subscriptions: Vec::new(),
        });
    entry.answer = context.answer.clone();
    if entry.window != window_id || entry.subscriptions.is_empty() {
        entry.window = window_id;
        entry.subscriptions = vec![
            context
                .window
                .on_focus_in(&entry.handle, context.cx, move |_, cx| cx.notify(owner)),
            context
                .window
                .on_focus_out(&entry.handle, context.cx, move |_, _, cx| cx.notify(owner)),
        ];
    }
    entry.handle.clone()
}
#[derive(Clone)]
enum CodeAction {
    Copy(Arc<str>),
    Wrap(String),
}
fn code_action(app: &mut VelaApp, key: &str, action: &CodeAction, cx: &mut Context<VelaApp>) {
    let ui = &mut app.transcript_ui.markdown;
    match action {
        CodeAction::Wrap(target) => {
            let wrap = ui.wrap.entry(target.clone()).or_insert(true);
            *wrap = !*wrap;
            app.transcript_list.remeasure();
        }
        CodeAction::Copy(text) => {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.to_string()));
            ui.copied = Some(key.to_owned());
            ui.copy_generation += 1;
            let generation = ui.copy_generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let _ = this.update(cx, |app, cx| {
                    if app.transcript_ui.markdown.copy_generation == generation {
                        app.transcript_ui.markdown.copied = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }
    cx.notify();
}
fn code_button(
    key: &str,
    label: &str,
    action: CodeAction,
    theme: &Theme,
    context: &mut RenderContext,
) -> AnyElement {
    let focus = control_focus(key, context);
    let action_key = key.to_owned();
    let owner = context.owner.clone();
    focused_button(
        key,
        label,
        theme,
        &focus,
        &context.ui.keyboard,
        move |_, cx| {
            if let Some(owner) = &owner {
                let _ = owner.update(cx, |app, cx| code_action(app, &action_key, &action, cx));
            }
        },
    )
}

fn focused_button(
    key: &str,
    label: &str,
    theme: &Theme,
    focus: &gpui::FocusHandle,
    keyboard: &crate::focus::KeyboardMode,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    let mouse_focus = focus.clone();
    let click_focus = focus.clone();
    div()
        .id(SharedString::from(format!("code-action:{key}")))
        .track_focus(focus)
        .map(|button| {
            crate::focus::outline(button, focus, 6., 1., 0., theme.accent.into(), keyboard)
        })
        .role(gpui::Role::Button)
        .aria_label(SharedString::from(label.to_owned()))
        .px(px(6.))
        .py(px(1.))
        .rounded(px(6.))
        .text_size(px(12.))
        .line_height(px(22.75))
        .cursor_pointer()
        .bg(theme.text.opacity(0.))
        .hover(|button| button.bg(theme.text.opacity(0.05)).text_color(theme.text))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.focus(&mouse_focus, cx)
        })
        // GPUI on_click includes a guarded Enter/Space down-up activation.
        // Another key handler would execute the action twice.
        .on_click(move |_, window, cx| {
            window.focus(&click_focus, cx);
            action(window, cx);
        })
        .child(label.to_owned())
        .into_any_element()
}

#[derive(Clone, Debug)]
enum Node {
    Container(Tag<'static>, Vec<Node>),
    Leaf(Event<'static>),
}

fn tree(events: &mut impl Iterator<Item = Event<'static>>) -> Vec<Node> {
    let mut nodes = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::Start(tag) => nodes.push(Node::Container(tag, tree(events))),
            Event::End(_) => break,
            event => nodes.push(Node::Leaf(event)),
        }
    }
    nodes
}

fn task_state(nodes: &[Node]) -> Option<bool> {
    nodes.iter().find_map(|node| match node {
        Node::Leaf(Event::TaskListMarker(checked)) => Some(*checked),
        Node::Container(Tag::Paragraph, children) => task_state(children),
        _ => None,
    })
}

#[derive(Clone, Default)]
struct InlineStyle {
    weight: FontWeight,
    italic: bool,
    strike: bool,
    code: bool,
    math: bool,
    link: Option<String>,
}
#[derive(Default)]
struct Inline {
    text: String,
    code: Vec<std::ops::Range<usize>>,
    runs: Vec<TextRun>,
    links: Vec<(std::ops::Range<usize>, String)>,
    math_sources: Vec<String>,
}
impl Inline {
    fn append(&mut self, text: &str, style: &InlineStyle, ink: gpui::Rgba, theme: &Theme) {
        if text.is_empty() {
            return;
        }
        let math;
        let text = if style.math {
            self.math_sources.push(text.to_owned());
            math = crate::math::render(text);
            math.as_str()
        } else {
            text
        };
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if style.code {
            self.code.push(start..self.text.len());
        }
        if let Some(url) = &style.link {
            self.links.push((start..self.text.len(), url.clone()));
        }
        let path_code = style.code
            && matches!(
                crate::syntax::inline_code_kind(text),
                Some(crate::syntax::InlineCodeKind::Path)
            );
        self.runs.push(TextRun {
            len: text.len(),
            font: Font {
                family: if style.math {
                    crate::math::MATH_FONT.into()
                } else if style.code {
                    theme::source_font().into()
                } else {
                    theme::FONT_UI.into()
                },
                weight: style.weight,
                style: if style.italic {
                    gpui::FontStyle::Italic
                } else {
                    gpui::FontStyle::Normal
                },
                ..theme::ui_font()
            },
            color: if path_code { theme.accent } else { ink }.into(),
            background_color: style.code.then(|| theme.text.opacity(0.08).into()),
            underline: style.link.as_ref().map(|_| gpui::UnderlineStyle {
                thickness: px(1.),
                color: None,
                wavy: false,
            }),
            strikethrough: style.strike.then_some(gpui::StrikethroughStyle {
                thickness: px(1.),
                color: None,
            }),
        });
    }
}
fn flatten(
    nodes: &[Node],
    style: &InlineStyle,
    result: &mut Inline,
    ink: gpui::Rgba,
    theme: &Theme,
) {
    for node in nodes {
        match node {
            Node::Container(tag, children) => {
                let mut style = style.clone();
                match tag {
                    Tag::Strong => {
                        style.weight = if style.weight.0 >= 550. {
                            FontWeight::BLACK
                        } else if style.weight.0 >= 350. {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        }
                    }
                    Tag::Emphasis => style.italic = true,
                    Tag::Strikethrough => style.strike = true,
                    Tag::Link { dest_url, .. } => style.link = Some(dest_url.to_string()),
                    _ => {}
                }
                flatten(children, &style, result, ink, theme);
            }
            Node::Leaf(Event::Text(text) | Event::Html(text) | Event::InlineHtml(text)) => {
                result.append(text, style, ink, theme)
            }
            Node::Leaf(Event::Code(text)) => result.append(
                text,
                &InlineStyle {
                    code: true,
                    ..style.clone()
                },
                ink,
                theme,
            ),
            Node::Leaf(Event::SoftBreak | Event::HardBreak) => {
                result.append("\n", style, ink, theme)
            }
            Node::Leaf(Event::InlineMath(text) | Event::DisplayMath(text)) => result.append(
                text,
                &InlineStyle {
                    math: true,
                    ..style.clone()
                },
                ink,
                theme,
            ),
            // Task markers are controls owned by the list item, not prose.
            Node::Leaf(Event::TaskListMarker(_)) => {}
            Node::Leaf(Event::FootnoteReference(text)) => result.append(text, style, ink, theme),
            _ => {}
        }
    }
}
enum InlinePart {
    Text(Vec<Node>),
    Image {
        uri: String,
        alt: String,
        link: Option<String>,
    },
}
fn contains_images(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| {
        matches!(node, Node::Container(Tag::Image { .. }, _))
            || matches!(node,Node::Container(_,children) if contains_images(children))
    })
}
fn image_parts(nodes: &[Node], link: Option<&str>) -> Vec<InlinePart> {
    let mut parts = Vec::new();
    for node in nodes {
        let next = match node {
            Node::Container(Tag::Image { dest_url, .. }, children) => {
                let mut alt = Inline::default();
                let theme = Theme::default();
                flatten(
                    children,
                    &InlineStyle::default(),
                    &mut alt,
                    theme.text,
                    &theme,
                );
                vec![InlinePart::Image {
                    uri: dest_url.to_string(),
                    alt: alt.text,
                    link: link.map(str::to_owned),
                }]
            }
            Node::Container(tag, children) if contains_images(children) => {
                let child_link = if let Tag::Link { dest_url, .. } = tag {
                    Some(dest_url.as_ref())
                } else {
                    link
                };
                image_parts(children, child_link)
                    .into_iter()
                    .map(|part| match part {
                        InlinePart::Text(children) => {
                            InlinePart::Text(vec![Node::Container(tag.clone(), children)])
                        }
                        image => image,
                    })
                    .collect()
            }
            _ => vec![InlinePart::Text(vec![node.clone()])],
        };
        for part in next {
            match part {
                InlinePart::Text(mut nodes) => {
                    if let Some(InlinePart::Text(previous)) = parts.last_mut() {
                        previous.append(&mut nodes);
                    } else {
                        parts.push(InlinePart::Text(nodes));
                    }
                }
                image => parts.push(image),
            }
        }
    }
    parts
}
fn cache_image_size(
    cache: &mut HashMap<String, Option<gpui::Size<gpui::Pixels>>>,
    key: &str,
    size: Option<gpui::Size<gpui::Pixels>>,
) -> bool {
    let changed = size.is_some() && cache.get(key) != Some(&size);
    cache.insert(key.to_owned(), size);
    changed
}

fn remember_image_size(
    context: &mut RenderContext,
    key: &str,
    size: Option<gpui::Size<gpui::Pixels>>,
) {
    if cache_image_size(&mut context.ui.image_sizes, key, size)
        && let Some(owner) = context.owner.clone()
    {
        // Resolving a lazy image changes a virtual row's height. Its owner
        // invalidates the list only after the current row borrow is released.
        context.cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                app.transcript_list.remeasure();
                cx.notify();
            });
        });
    }
}

fn render_image(
    uri: &str,
    alt: &str,
    link: &Option<String>,
    key: &str,
    theme: &Theme,
    context: &mut RenderContext,
    spaced: bool,
) -> AnyElement {
    use gpui::StyledImage;
    let label = SharedString::from(alt.to_owned());
    let fallback = alt.to_owned();
    let resolved = markdown_images::source(uri)
        .ok()
        .map(|source| markdown_images::resolve(&source, context.window, context.cx))
        .unwrap_or(Some(Err(())));
    let observed = resolved
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|image| {
            let natural = image.size(0);
            markdown_images::fit(natural.width.0 as f32, natural.height.0 as f32)
        });
    remember_image_size(context, key, observed);
    let background = if context.user {
        theme.user_bubble
    } else {
        theme.agent
    };
    let content = match resolved {
        Some(Ok(image)) => {
            let natural = image.size(0);
            let fitted = markdown_images::fit(natural.width.0 as f32, natural.height.0 as f32);
            // GPUI paints an Img across its entire border box. Keep the CSS
            // content box separate so the raster cannot cover its 1px border.
            div()
                .id(SharedString::from(format!("image:{key}")))
                .w(fitted.width)
                .h(fitted.height)
                .flex_shrink_0()
                .flex()
                .bg(background)
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border_soft)
                .shadow(theme.small_shadow())
                .overflow_hidden()
                .child(
                    gpui::img(image)
                        .w(fitted.width - px(2.))
                        .h(fitted.height - px(2.))
                        .flex_shrink_0()
                        .rounded(px(5.))
                        .object_fit(gpui::ObjectFit::Contain),
                )
                .into_any_element()
        }
        Some(Err(())) => div()
            .bg(background)
            .w(px(200.))
            .h(px(24.75))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border_soft)
            .shadow(theme.small_shadow())
            .overflow_hidden()
            .flex()
            .items_start()
            .child(
                gpui::img(markdown_images::broken(context.window))
                    .w(px(14.))
                    .h(px(16.)),
            )
            .child(
                div()
                    .min_w_0()
                    .mt(px(-1.))
                    .text_size(px(14.))
                    .line_height(px(17.))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(fallback),
            )
            .into_any_element(),
        None => div().h(px(0.)).into_any_element(),
    };
    let container_id = format!("image-container:{key}");
    let debug_id = container_id.clone();
    div()
        .id(SharedString::from(container_id))
        .debug_selector(move || debug_id.clone())
        .when(spaced, |image| image.my(px(12.)))
        .flex()
        .items_start()
        .self_start()
        .max_w(px(200.))
        .max_h(px(200.))
        .role(if link.is_some() {
            gpui::Role::Link
        } else {
            gpui::Role::Image
        })
        .aria_label(label)
        .when_some(link.clone(), |image, url| {
            image.cursor_pointer().on_click(move |_, _, cx| {
                if let Some(url) = vela_browser::safe_external_url(&url) {
                    cx.open_url(&url);
                }
            })
        })
        .child(content)
        .into_any_element()
}
fn inline_element(result: Inline, key: &str, theme: &Theme) -> AnyElement {
    if result.text.is_empty() {
        return div().h(px(0.)).into_any_element();
    }
    // Accumulate CSS fractional line heights before the containing block is
    // snapped to the display grid. StyledText snaps each line independently,
    // which changes multiline block heights and all following virtual rows.
    inline_flow::InlineFlow::new(
        SharedString::from(format!("mixed-inline:{key}")),
        SharedString::from(result.text),
        result.runs,
        result.code,
        result.links,
        theme.text.opacity(0.08),
    )
    .into_any_element()
}
fn inline(
    nodes: &[Node],
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    weight: FontWeight,
) -> AnyElement {
    let mut result = Inline::default();
    flatten(
        nodes,
        &InlineStyle {
            weight,
            ..Default::default()
        },
        &mut result,
        ink,
        theme,
    );
    let label = result.text.clone();
    let math_source = (!result.math_sources.is_empty())
        .then(|| format!("TeX: {}", result.math_sources.join(" | ")));
    div()
        .id(SharedString::from(format!("label:{key}")))
        .debug_selector(|| format!("label:{key}"))
        .min_w_0()
        .whitespace_normal()
        .role(gpui::Role::Label)
        .aria_label(SharedString::from(label))
        .when_some(math_source, |label, source| label.aria_description(source))
        .child(inline_element(result, key, theme))
        .into_any_element()
}

/// Keep the bubble's intrinsic text width available to the surrounding flex
/// row. A stack of auto-width flex containers collapses that measurement.
fn user_inline(text: &str, key: &str, ink: gpui::Rgba, theme: &Theme) -> AnyElement {
    let nodes = tree(&mut markdown::events(text).into_iter());
    let mut result = Inline::default();
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            result.append("\n", &InlineStyle::default(), ink, theme);
        }
        flatten(
            std::slice::from_ref(node),
            &InlineStyle::default(),
            &mut result,
            ink,
            theme,
        );
    }
    inline_element(result, &format!("user:{key}"), theme)
}
/// The source user message is a column aligned to its end. Keeping that
/// cross-axis contract lets a structured document retain its intrinsic width.
fn bubble(
    content: AnyElement,
    key: &str,
    theme: &Theme,
    label: Option<SharedString>,
) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_end()
        .child(
            div()
                .id(SharedString::from(format!("user-label:{key}")))
                .debug_selector(|| format!("user-bubble:{key}"))
                .when_some(label, |bubble, label| {
                    bubble.role(gpui::Role::Label).aria_label(label)
                })
                .max_w(gpui::relative(0.77))
                .min_w_0()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(16.))
                .overflow_hidden()
                .bg(theme.text.opacity(0.05))
                .text_size(px(14.))
                .line_height(px(22.75))
                .text_color(theme.text)
                .whitespace_normal()
                .child(content),
        )
        .into_any_element()
}
pub(super) fn user(
    text: &str,
    key: &str,
    theme: &Theme,
    ui: &mut MarkdownUi,
    window: &mut Window,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    let mut context = RenderContext {
        ui,
        window,
        owner: Some(cx.weak_entity()),
        cx,
        user: true,
        streaming: false,
        list_depth: 0,
        list: None,
        answer: None,
    };
    let (content, label) = user_document(text, key, theme, &mut context);
    bubble(content, key, theme, label)
}
fn user_document(
    text: &str,
    key: &str,
    theme: &Theme,
    context: &mut RenderContext,
) -> (AnyElement, Option<SharedString>) {
    let nodes = tree(&mut markdown::events(text).into_iter());
    if matches!(nodes.as_slice(),[Node::Container(Tag::Paragraph,children)] if !image_parts(children,None).iter().any(|part|matches!(part,InlinePart::Image {..})))
    {
        return (
            user_inline(text, key, theme.text, theme),
            Some(SharedString::from(text.to_owned())),
        );
    }
    let mut previous_bottom = 0f32;
    let last = nodes.len().saturating_sub(1);
    let content=div().min_w_0().children(nodes.iter().enumerate().map(|(index,node)| {
        let (mut top,mut bottom)=user_margins(node,index.checked_sub(1).map(|index|&nodes[index]));
        if index==last && image_paragraph(node) {bottom=12.;}
        if index==0 && !image_paragraph(node) {top=0.;}
        if index==last && !image_paragraph(node) && !matches!(node,Node::Container(Tag::Paragraph,_) if index>0 && matches!(nodes[index-1],Node::Container(Tag::Paragraph,_))) {bottom=0.;}
        let gap=top.max(previous_bottom);
        previous_bottom=bottom;
        div().pt(px(gap)).pb(px(if index==last {bottom}else{0.}))
            .child(render(node,&format!("user:{key}:{index}"),theme.text,theme,context))
    })).into_any_element();
    (content, None)
}
fn image_paragraph(node: &Node) -> bool {
    let Node::Container(Tag::Paragraph, children) = node else {
        return false;
    };
    let parts = image_parts(children, None);
    !parts.is_empty()
        && parts
            .iter()
            .all(|part| matches!(part, InlinePart::Image { .. }))
}
fn user_margins(node: &Node, previous: Option<&Node>) -> (f32, f32) {
    match node {
        Node::Container(Tag::Paragraph, _) => {
            if image_paragraph(node) {
                return (12., 14.);
            }
            if matches!(previous, Some(Node::Container(Tag::Paragraph, _))) {
                (14., 14.)
            } else if matches!(
                previous,
                Some(Node::Container(
                    Tag::Heading {
                        level: markdown::HeadingLevel::H1
                            | markdown::HeadingLevel::H2
                            | markdown::HeadingLevel::H3,
                        ..
                    },
                    _
                ))
            ) {
                (0., 14.)
            } else {
                (7., 14.)
            }
        }
        Node::Container(Tag::Heading { .. }, _) => (14., 7.),
        // Marked renders list-item and quote text as paragraphs. Their UA
        // 1em margins collapse through those containers in the user bubble.
        Node::Container(Tag::List(_), items) if is_task_list(items) => (7., 7.),
        Node::Container(Tag::List(_) | Tag::BlockQuote(_), _) => (14., 14.),
        Node::Container(Tag::CodeBlock(_), _) => (7., 7.),
        Node::Container(Tag::Table(_), _) => (7., 7.),
        Node::Leaf(Event::Rule) => (24.5, 24.5),
        _ => (0., 0.),
    }
}

fn is_task_list(items: &[Node]) -> bool {
    !items.is_empty()
        && items.iter().all(
            |node| matches!(node, Node::Container(Tag::Item, body) if task_state(body).is_some()),
        )
}

#[derive(Clone, Copy, Default)]
pub(super) struct BlockSpacing {
    pub(super) top: f32,
    pub(super) bottom: f32,
    paragraph: bool,
    image: bool,
    heading: bool,
    last_paragraph: bool,
    tail_margin: f32,
    single_node: bool,
}
impl BlockSpacing {
    pub(super) fn bottom_at_document_end(self, previous: Self) -> f32 {
        if self.single_node && self.last_paragraph && previous.last_paragraph && !self.image {
            14.
        } else {
            self.tail_margin
        }
    }
    pub(super) fn top_after(self, previous: Self) -> f32 {
        if self.paragraph && previous.paragraph {
            14.
        } else if self.paragraph && previous.heading {
            if self.image { 12. } else { 0. }
        } else {
            self.top
        }
    }
}
pub(super) fn block_spacing(text: &str) -> BlockSpacing {
    let nodes = tree(&mut markdown::events(text).into_iter());
    let Some(first) = nodes.first() else {
        return BlockSpacing::default();
    };
    BlockSpacing {
        top: user_margins(first, None).0,
        bottom: user_margins(nodes.last().unwrap(), None).1,
        paragraph: matches!(first, Node::Container(Tag::Paragraph, _)),
        image: image_paragraph(first),
        heading: matches!(
            nodes.last(),
            Some(Node::Container(
                Tag::Heading {
                    level: markdown::HeadingLevel::H1
                        | markdown::HeadingLevel::H2
                        | markdown::HeadingLevel::H3,
                    ..
                },
                _
            ))
        ),
        last_paragraph: matches!(nodes.last(), Some(Node::Container(Tag::Paragraph, _))),
        single_node: nodes.len() == 1,
        tail_margin: if image_paragraph(nodes.last().unwrap()) {
            12.
        } else if nodes.len() > 1
            && matches!(nodes.last(), Some(Node::Container(Tag::Paragraph, _)))
            && matches!(&nodes[nodes.len() - 2], Node::Container(Tag::Paragraph, _))
        {
            14.
        } else {
            0.
        },
    }
}

fn stack(
    nodes: &[Node],
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    gap: f32,
    context: &mut RenderContext,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(gap))
        .children(
            nodes
                .iter()
                .enumerate()
                .map(|(index, node)| render(node, &format!("{key}:{index}"), ink, theme, context)),
        )
        .into_any_element()
}
#[derive(Clone, Copy)]
struct TableColumn {
    minimum: f32,
    maximum: f32,
}
fn table_shrink_factors(columns: &[TableColumn]) -> Vec<f32> {
    let factors = columns
        .iter()
        .map(|column| (column.maximum - column.minimum).max(0.) / column.maximum)
        .collect::<Vec<_>>();
    let total = factors.iter().sum::<f32>();
    // Flexbox deliberately leaves some negative free space undistributed
    // when the factors total less than one. Auto tables must distribute all
    // of it, with weights proportional to each column's min/max range.
    factors
        .into_iter()
        .map(|factor| factor / total.max(f32::MIN_POSITIVE))
        .collect()
}
fn table_column(column: TableColumn, shrink: f32) -> gpui::Div {
    div()
        .flex_grow(column.maximum)
        .flex_shrink(shrink)
        .flex_basis(px(column.maximum))
        .min_w(px(column.minimum))
}
fn snap_table_columns(columns: &mut [TableColumn], scale: f32) {
    for column in columns {
        // Taffy snaps cell edges to physical pixels. Round an intrinsic
        // minimum upwards so the snapped content box still fits its word.
        column.minimum = (column.minimum * scale).ceil() / scale;
        column.maximum = (column.maximum * scale).ceil() / scale;
    }
}
/// Widest image in a cell after CSS `max-width/max-height` fitting. Source
/// stacks cell images as block content, so the column only needs the widest
/// one rather than their sum.
fn cell_image_width(nodes: &[Node], key: &str, context: &mut RenderContext) -> f32 {
    let mut width: f32 = 0.;
    for (index, part) in image_parts(nodes, None).into_iter().enumerate() {
        let InlinePart::Image { uri, .. } = part else {
            continue;
        };
        let resolved = markdown_images::source(&uri)
            .ok()
            .map(|source| markdown_images::resolve(&source, context.window, context.cx))
            .unwrap_or(Some(Err(())));
        let (image_width, observed) = match resolved.as_ref() {
            Some(Ok(image)) => {
                let natural = image.size(0);
                let fitted = markdown_images::fit(
                    natural.width.0 as f32,
                    natural.height.0 as f32,
                );
                (f32::from(fitted.width), Some(fitted))
            }
            Some(Err(_)) => (200., None),
            None => (0., None),
        };
        remember_image_size(context, &format!("{key}:{index}"), observed);
        width = width.max(image_width);
    }
    width
}

fn syntax_color(token: crate::syntax::SyntaxToken, theme: &Theme) -> gpui::Rgba {
    match token {
        crate::syntax::SyntaxToken::Keyword => {
            if theme.dark {
                gpui::rgb(0xff7b72)
            } else {
                gpui::rgb(0xcf222e)
            }
        }
        crate::syntax::SyntaxToken::String => {
            if theme.dark {
                gpui::rgb(0xa5d6ff)
            } else {
                gpui::rgb(0x0a3069)
            }
        }
        crate::syntax::SyntaxToken::Number => {
            if theme.dark {
                gpui::rgb(0x79c0ff)
            } else {
                gpui::rgb(0x0550ae)
            }
        }
        crate::syntax::SyntaxToken::Comment => theme.faint,
        crate::syntax::SyntaxToken::Function => {
            if theme.dark {
                gpui::rgb(0xd2a8ff)
            } else {
                gpui::rgb(0x8250df)
            }
        }
        crate::syntax::SyntaxToken::Property | crate::syntax::SyntaxToken::Attribute => {
            if theme.dark {
                gpui::rgb(0x79c0ff)
            } else {
                gpui::rgb(0x0550ae)
            }
        }
        crate::syntax::SyntaxToken::Type => {
            if theme.dark {
                gpui::rgb(0xffa657)
            } else {
                gpui::rgb(0x953800)
            }
        }
        crate::syntax::SyntaxToken::Tag => {
            if theme.dark {
                gpui::rgb(0x7ee787)
            } else {
                gpui::rgb(0x116329)
            }
        }
        crate::syntax::SyntaxToken::Added => theme.green,
        crate::syntax::SyntaxToken::Removed => theme.red,
        crate::syntax::SyntaxToken::Ansi(style) => style
            .foreground
            .map(gpui::rgb)
            .unwrap_or(theme.text),
    }
}

/// Build complete, gap-free runs for a fenced code block. Unknown languages and
/// tabbed content keep the plain code style so layout remains exact.
fn syntax_runs(
    language: &str,
    text: &str,
    streaming: bool,
    theme: &Theme,
    ink: gpui::Rgba,
) -> Option<Vec<gpui::TextRun>> {
    // Source keeps a fence on the plain code surface while it is still
    // streaming and highlights only after the block settles.
    if streaming {
        return None;
    }
    let language = crate::syntax::language(language)?;
    if text.is_empty() || text.contains('\t') || text.contains('\r') {
        return None;
    }
    let spans = crate::syntax::highlight(language, text);
    if spans.is_empty() {
        return None;
    }
    let font = theme::markdown_code_font();
    let run = |len: usize, color: gpui::Rgba| gpui::TextRun {
        len,
        font: font.clone(),
        color: color.into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let mut runs = Vec::new();
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor {
            continue;
        }
        if span.start > cursor {
            runs.push(run(span.start - cursor, ink));
        }
        runs.push(run(span.end - span.start, syntax_color(span.token, theme)));
        cursor = span.end;
    }
    if cursor < text.len() {
        runs.push(run(text.len() - cursor, ink));
    }
    if runs.len() <= 1 {
        return None;
    }
    Some(runs)
}

/// Shiki's ANSI path removes escape bytes from layout. The caller supplies the
/// cleaned text; spans carry foreground/background/decorations for each run.
fn ansi_runs(
    text: &str,
    spans: &[crate::syntax::Span],
    theme: &Theme,
    ink: gpui::Rgba,
) -> Vec<gpui::TextRun> {
    let base = theme::markdown_code_font();
    let run = |len: usize, style: Option<crate::syntax::AnsiStyle>| {
        let mut font = base.clone();
        let mut color = ink;
        let mut background = None;
        let mut underline = None;
        let mut strikethrough = None;
        if let Some(style) = style {
            if style.bold {
                font.weight = FontWeight::BOLD;
            }
            if style.italic {
                font.style = gpui::FontStyle::Italic;
            }
            let foreground = style.foreground.map(gpui::rgb);
            let fill = style.background.map(gpui::rgb);
            if style.reverse {
                color = fill.unwrap_or(theme.bg);
                background = Some(foreground.unwrap_or(ink));
            } else {
                color = foreground.unwrap_or(ink);
                background = fill;
            }
            if style.dim {
                color = color.opacity(0.5);
            }
            if style.underline {
                underline = Some(gpui::UnderlineStyle {
                    thickness: px(1.),
                    color: None,
                    wavy: false,
                });
            }
            if style.strikethrough {
                strikethrough = Some(gpui::StrikethroughStyle {
                    thickness: px(1.),
                    color: None,
                });
            }
        }
        gpui::TextRun {
            len,
            font,
            color: color.into(),
            background_color: background.map(Into::into),
            underline,
            strikethrough,
        }
    };
    let mut runs = Vec::new();
    let mut cursor = 0usize;
    for span in spans {
        if span.start < cursor || span.end > text.len() {
            continue;
        }
        if span.start > cursor {
            runs.push(run(span.start - cursor, None));
        }
        let style = match span.token {
            crate::syntax::SyntaxToken::Ansi(style) => Some(style),
            _ => None,
        };
        runs.push(run(span.end - span.start, style));
        cursor = span.end;
    }
    if cursor < text.len() {
        runs.push(run(text.len() - cursor, None));
    }
    if runs.is_empty() {
        runs.push(run(text.len(), None));
    }
    runs
}

fn table_cell_role(header: bool) -> gpui::Role {
    if header {
        gpui::Role::ColumnHeader
    } else {
        gpui::Role::Cell
    }
}

/// AccessKit's macOS adapter maps both header roles to a plain cell. Keep the
/// header semantics audible with an explicit platform description while the
/// semantic role and row/column indexes remain on the node.
fn table_cell_description(header: bool) -> Option<&'static str> {
    header.then_some("Column header")
}

/// Plain accessible text for a table cell. Source exposes the rendered cell
/// content as the `<th>`/`<td>` name; this mirrors that while collapsing
/// whitespace and falling back to a positional name for empty cells.
fn table_cell_label(body: &[Node], header: bool, row: usize, column: usize) -> SharedString {
    let mut text = String::new();
    table_node_text(body, &mut text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.is_empty() {
        return text.into();
    }
    if header {
        format!("Column {} header", column + 1).into()
    } else {
        format!("Row {}, column {}", row + 1, column + 1).into()
    }
}

fn table_node_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Container(Tag::Image { .. }, children) => {
                let before = out.len();
                table_node_text(children, out);
                if out[before..].trim().is_empty() {
                    out.truncate(before);
                    out.push_str("Image");
                }
            }
            Node::Container(_, children) => table_node_text(children, out),
            Node::Leaf(
                Event::Text(text)
                | Event::Code(text)
                | Event::Html(text)
                | Event::InlineHtml(text)
                | Event::InlineMath(text)
                | Event::DisplayMath(text)
                | Event::FootnoteReference(text),
            ) => {
                out.push_str(text);
                out.push(' ');
            }
            Node::Leaf(Event::SoftBreak | Event::HardBreak) => out.push(' '),
            Node::Leaf(_) => {}
        }
    }
}

fn table_widths(
    rows: &[Node],
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    context: &mut RenderContext,
) -> Vec<TableColumn> {
    let mut widths: Vec<TableColumn> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let Node::Container(tag, cells) = row else {
            continue;
        };
        for (column, cell) in cells.iter().enumerate() {
            let Node::Container(_, body) = cell else {
                continue;
            };
            let image_width =
                cell_image_width(body, &format!("{key}:{row_index}:{column}"), context);
            let mut value = Inline::default();
            flatten(
                body,
                &InlineStyle {
                    weight: if matches!(tag, Tag::TableHead) {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    },
                    ..Default::default()
                },
                &mut value,
                ink,
                theme,
            );
            let line = context.window.text_system().shape_line(
                SharedString::from(value.text.clone()),
                px(12.25),
                &value.runs,
                None,
            );
            if widths.len() <= column {
                widths.resize(
                    column + 1,
                    TableColumn {
                        minimum: 21.,
                        maximum: 21.,
                    },
                );
            }
            // CSS auto tables shrink at normal Unicode line-break
            // opportunities. overflow-wrap:break-word does not shrink the
            // min-content width of an otherwise unbreakable identifier.
            let mut start = 0;
            let mut minimum: f32 = 0.;
            for (end, _) in unicode_linebreak::linebreaks(&value.text) {
                let segment = &value.text[start..end];
                let trimmed_end = start + segment.trim_end_matches(char::is_whitespace).len();
                minimum = minimum.max(f32::from(
                    line.x_for_index(trimmed_end) - line.x_for_index(start),
                ));
                start = end;
            }
            widths[column].minimum = widths[column]
                .minimum
                .max(minimum + 21.)
                .max(image_width + 21.);
            widths[column].maximum = widths[column]
                .maximum
                .max(f32::from(line.width()) + 21.)
                .max(image_width + 21.);
        }
    }
    snap_table_columns(&mut widths, context.window.scale_factor());
    widths
}

/// Source renders table cells through the same inline token renderer as prose,
/// so images, links and styled runs stay mixed instead of collapsing to their
/// alt text. Block images stack vertically inside the cell.
fn table_cell(
    body: &[Node],
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    weight: FontWeight,
    context: &mut RenderContext,
) -> AnyElement {
    if !contains_images(body) {
        return inline(body, key, ink, theme, weight);
    }
    let parts = image_parts(body, None);
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .children(
            parts
                .into_iter()
                .enumerate()
                .map(|(index, part)| match part {
                    InlinePart::Text(nodes) => inline(
                        &nodes,
                        &format!("{key}:{index}"),
                        ink,
                        theme,
                        weight,
                    ),
                    InlinePart::Image { uri, alt, link } => render_image(
                        &uri,
                        &alt,
                        &link,
                        &format!("{key}:{index}"),
                        theme,
                        context,
                        true,
                    ),
                }),
        )
        .into_any_element()
}

fn math_block(tex: &str, key: &str, theme: &Theme) -> AnyElement {
    let text = crate::math::render(tex);
    let id = format!("math-block:{key}");
    let debug = id.clone();
    div()
        .id(SharedString::from(id))
        .debug_selector(move || debug.clone())
        .role(gpui::Role::Label)
        .aria_label(format!("Math: {text}"))
        .aria_description(format!("TeX: {tex}"))
        .w_full()
        .flex()
        .justify_center()
        .py(px(8.))
        .text_size(px(15.))
        .line_height(px(24.))
        .text_color(theme.text)
        .font(Font {
            family: crate::math::MATH_FONT.into(),
            ..theme::ui_font()
        })
        .child(SharedString::from(text))
        .into_any_element()
}

fn render(
    node: &Node,
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    context: &mut RenderContext,
) -> AnyElement {
    let Node::Container(tag, children) = node else {
        return if matches!(node, Node::Leaf(Event::Rule)) {
            div()
                .w_full()
                .h(px(1.))
                .bg(theme.border_soft)
                .into_any_element()
        } else if let Node::Leaf(Event::DisplayMath(tex)) = node {
            math_block(tex, key, theme)
        } else {
            inline(
                std::slice::from_ref(node),
                key,
                ink,
                theme,
                FontWeight::NORMAL,
            )
        };
    };
    match tag {
        Tag::Paragraph if contains_images(children) => {
            let parts = image_parts(children, None);
            let spaced = parts.iter().any(|part| matches!(part, InlinePart::Text(_)));
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .children(
                    parts
                        .into_iter()
                        .enumerate()
                        .map(|(index, part)| match part {
                            InlinePart::Text(nodes) => inline(
                                &nodes,
                                &format!("{key}:{index}"),
                                ink,
                                theme,
                                FontWeight::NORMAL,
                            ),
                            InlinePart::Image { uri, alt, link } => render_image(
                                &uri,
                                &alt,
                                &link,
                                &format!("{key}:{index}"),
                                theme,
                                context,
                                spaced,
                            ),
                        }),
                )
                .into_any_element()
        }
        Tag::Paragraph
            if children
                .iter()
                .any(|child| matches!(child, Node::Leaf(Event::DisplayMath(_)))) =>
        {
            let mut body = div().w_full().min_w_0().flex().flex_col();
            for (index, child) in children.iter().enumerate() {
                if let Node::Leaf(Event::DisplayMath(tex)) = child {
                    body = body.child(math_block(tex, &format!("{key}:{index}"), theme));
                } else {
                    body = body.child(inline(
                        std::slice::from_ref(child),
                        &format!("{key}:{index}"),
                        ink,
                        theme,
                        FontWeight::NORMAL,
                    ));
                }
            }
            body.into_any_element()
        }
        Tag::Paragraph => inline(children, key, ink, theme, FontWeight::NORMAL),
        Tag::Heading { level, .. } => {
            let size = match *level as u8 {
                1 => 21.,
                2 => 18.2,
                3 => 16.1,
                _ => 14.,
            };
            let mut label = Inline::default();
            flatten(children, &InlineStyle::default(), &mut label, ink, theme);
            div()
                .id(SharedString::from(format!("heading:{key}")))
                .text_size(px(size))
                .line_height(px(size * 1.3))
                .role(gpui::Role::Heading)
                .aria_level(*level as usize)
                .aria_label(SharedString::from(label.text))
                .child(inline(children, key, ink, theme, FontWeight::SEMIBOLD))
                .into_any_element()
        }
        Tag::BlockQuote(_) => div()
            .relative()
            .mx(px(40.))
            .pl(px(21.))
            .text_color(ink.opacity(0.62))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(7.))
                    .bottom(px(7.))
                    .w(px(3.5))
                    .rounded(px(1.75))
                    .bg(theme.border_soft),
            )
            .child(stack(children, key, ink.opacity(0.62), theme, 14., context))
            .into_any_element(),
        Tag::List(start) => {
            let depth = context.list_depth;
            context.list_depth += 1;
            let task_list = is_task_list(children);
            let list = div()
                .id(SharedString::from(format!("list:{key}")))
                .role(gpui::Role::List)
                .aria_size_of_set(children.len())
                .flex()
                .flex_col()
                .min_w_0()
                .pl(px(21.))
                // Browser list/body margins collapse; GPUI flex gaps do not.
                .pt(px(if task_list {
                    0.
                } else if depth == 0 {
                    1.5
                } else {
                    3.5
                }))
                .gap(px(if task_list { 3.5 } else { 13. }))
                .children(children.iter().enumerate().map(|(index, child)| {
                    let body = if let Node::Container(Tag::Item, body) = child {
                        body.as_slice()
                    } else {
                        std::slice::from_ref(child)
                    };
                    let task = task_state(body);
                    let body = body
                        .iter()
                        .filter(|node| !matches!(node, Node::Leaf(Event::TaskListMarker(_))))
                        .cloned()
                        .collect::<Vec<_>>();
                    let marker = start
                        .map(|start| format!("{}.", start + index as u64))
                        .unwrap_or_else(|| {
                            match depth % 3 {
                                0 => "•",
                                1 => "◦",
                                _ => "▪",
                            }
                            .into()
                        });
                    div()
                        .id(SharedString::from(format!("list-item:{key}:{index}")))
                        .role(gpui::Role::ListItem)
                        .aria_position_in_set(index + 1)
                        .aria_size_of_set(children.len())
                        .relative()
                        .flex()
                        .items_start()
                        .min_w_0()
                        .when_some(task, |item, checked| {
                            item.gap(px(8.)).child(
                                div()
                                    .size(px(13.))
                                    .mt(px(3.))
                                    .flex_shrink_0()
                                    .rounded(px(2.))
                                    .border_1()
                                    .border_color(if checked {
                                        ink
                                    } else {
                                        gpui::rgb(if theme.dark { 0x858585 } else { 0x767676 })
                                    })
                                    .bg(if checked {
                                        ink
                                    } else {
                                        gpui::rgb(if theme.dark { 0x3b3b3b } else { 0xffffff })
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(checked, |checkbox| {
                                        checkbox.child(glyph(
                                            "check",
                                            11.,
                                            gpui::rgb(if theme.dark { 0x000000 } else { 0xffffff }),
                                        ))
                                    }),
                            )
                        })
                        .when(task.is_none(), |item| {
                            item.child(
                                div()
                                    .id(SharedString::from(format!("list-marker:{key}:{index}")))
                                    .role(gpui::Role::ListMarker)
                                    .aria_label(SharedString::from(marker.clone()))
                                    .absolute()
                                    .left(px(-16.))
                                    .flex_shrink_0()
                                    .child(marker),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .when(task.is_some(), |element| {
                                    element.pt(px(14.)).pb(px(
                                        if matches!(
                                            body.last(),
                                            Some(Node::Container(Tag::List(_), _))
                                        ) {
                                            3.5
                                        } else {
                                            14.
                                        },
                                    ))
                                })
                                .child(stack(
                                    &body,
                                    &format!("{key}:{index}"),
                                    ink,
                                    theme,
                                    14.,
                                    context,
                                )),
                        )
                }))
                .into_any_element();
            context.list_depth = depth;
            list
        }
        Tag::CodeBlock(kind) => {
            let mut code = Inline::default();
            flatten(children, &InlineStyle::default(), &mut code, ink, theme);
            if context.user {
                let raw = code
                    .text
                    .strip_suffix('\n')
                    .unwrap_or(&code.text)
                    .to_owned();
                return div()
                    .id(SharedString::from(format!("user-code:{key}")))
                    .role(gpui::Role::Label)
                    .aria_label(SharedString::from(raw.clone()))
                    .min_w_0()
                    .p(px(12.))
                    .rounded(px(8.))
                    .bg(theme.inset)
                    .font(theme::markdown_code_font())
                    .text_size(px(12.25))
                    .line_height(px(18.375))
                    .whitespace_normal()
                    .child(crate::preformatted::PreformattedText::new(raw, true))
                    .into_any_element();
            }
            let language = match kind {
                CodeBlockKind::Fenced(info) => {
                    if info.is_empty() {
                        "text"
                    } else {
                        info.as_ref()
                    }
                }
                CodeBlockKind::Indented => "text",
            };
            let wrap = context.ui.wrap.get(key).copied().unwrap_or(true);
            let copied = context.ui.copied.as_deref() == Some(key);
            let raw = code
                .text
                .strip_suffix('\n')
                .unwrap_or(&code.text)
                .to_owned();
            let (display, highlighted) = if crate::syntax::language(language) == Some("ansi") {
                let (display, spans) = crate::syntax::ansi(&raw);
                let runs = ansi_runs(&display, &spans, theme, ink);
                (display, Some(runs))
            } else {
                (
                    raw.clone(),
                    syntax_runs(language, &raw, context.streaming, theme, ink),
                )
            };
            let copy = code_button(
                key,
                if copied { "Copied" } else { "Copy" },
                CodeAction::Copy(Arc::from(raw.clone())),
                theme,
                context,
            );
            let toggle = code_button(
                &format!("{key}:wrap"),
                if wrap { "No wrap" } else { "Wrap" },
                CodeAction::Wrap(key.to_owned()),
                theme,
                context,
            );
            let scroll = context
                .ui
                .horizontal
                .entry(key.to_owned())
                .or_default()
                .clone();
            let body_id = SharedString::from(format!("code-body:{key}"));
            let body_debug = body_id.to_string();
            let layout_owner = context.window.current_view();
            let layout_list = context.list.clone();
            // Text can paint past a constrained no-wrap box without expanding
            // GPUI's scroll extent. Give the text its measured intrinsic width.
            let unwrapped_width = if wrap {
                px(0.)
            } else {
                crate::preformatted::unwrapped_width(
                    &display,
                    px(12.5),
                    &TextRun {
                        len: display.len(),
                        font: theme::markdown_code_font(),
                        color: ink.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    },
                    context.window,
                )
            };
            let body = crate::scroll::AutoScroll::horizontal(scroll, move |scroll, gutter| {
                tracing::trace!(?unwrapped_width, ?gutter, wrap, "code scroll layout");
                div()
                    .id(body_id)
                    .debug_selector(move || body_debug.clone())
                    .role(gpui::Role::Label)
                    .aria_label(SharedString::from(display.clone()))
                    .overflow_x_scroll()
                    .track_scroll(&scroll)
                    .p(px(12.))
                    .pb(px(12.) + gutter)
                    .font(theme::markdown_code_font())
                    .text_size(px(12.5))
                    .line_height(px(18.75))
                    .when(wrap, |code| code.whitespace_normal())
                    .when(!wrap, |code| code.whitespace_nowrap())
                    .child(
                        div()
                            .w_full()
                            .when(!wrap, |text| text.min_w(unwrapped_width))
                            .child(match highlighted {
                                Some(runs) => crate::preformatted::PreformattedText::with_runs(
                                    display.clone(),
                                    wrap,
                                    runs,
                                )
                                .into_any_element(),
                                None => crate::preformatted::PreformattedText::new(
                                    display.clone(),
                                    wrap,
                                )
                                .into_any_element(),
                            }),
                    )
                    .into_any_element()
            })
            .on_gutter_change(move |cx| {
                if let Some(list) = &layout_list {
                    list.remeasure();
                }
                cx.notify(layout_owner);
            });
            div()
                .w_full()
                .min_w_0()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border_soft)
                .bg(theme.inset)
                .overflow_hidden()
                .child(
                    div()
                        .px(px(10.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .line_height(px(22.75))
                        .border_b_1()
                        .border_color(theme.border_soft)
                        .text_color(ink.opacity(0.62))
                        .bg(theme.inset)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font(theme::markdown_code_font())
                                .child(language.to_lowercase()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(toggle)
                                .child(copy),
                        ),
                )
                .child(body)
                .into_any_element()
        }
        Tag::Table(_) => {
            let widths = table_widths(children, key, ink, theme, context);
            let shrink = table_shrink_factors(&widths);
            let minimum_width = widths.iter().map(|column| column.minimum).sum::<f32>() + 1.;
            let table_key = format!("table:{key}");
            let focus = control_focus(&table_key, context);
            let scroll = context
                .ui
                .horizontal
                .entry(table_key.clone())
                .or_default()
                .clone();
            let navigation = scroll.native_handle();
            let layout_owner = context.window.current_view();
            let layout_list = context.list.clone();
            let keyboard = context.ui.keyboard.clone();
            let outline_color = theme.accent.into();
            let table_debug = table_key.clone();
            let table = div()
                .id(SharedString::from(table_key.clone()))
                .debug_selector(move || table_debug.clone())
                .role(gpui::Role::Table)
                .aria_row_count(children.len())
                .aria_column_count(widths.len())
                .w_full()
                .min_w(px(minimum_width))
                .border_l_1()
                .border_color(theme.border_soft)
                .text_size(px(12.25))
                .line_height(px(22.75))
                .children(children.iter().enumerate().map(|(index, row)| {
                    let (header, cells) = match row {
                        Node::Container(Tag::TableHead, cells) => (true, cells.as_slice()),
                        Node::Container(Tag::TableRow, cells) => (false, cells.as_slice()),
                        _ => (false, std::slice::from_ref(row)),
                    };
                    let row_id = format!("table-row:{key}:{index}");
                    let row_debug = row_id.clone();
                    div()
                        .id(SharedString::from(row_id))
                        .debug_selector(move || row_debug.clone())
                        .role(gpui::Role::Row)
                        .aria_row_index(index)
                        .flex()
                        .w_full()
                        .children(cells.iter().enumerate().map(|(column, cell)| {
                            let body = if let Node::Container(Tag::TableCell, body) = cell {
                                body.as_slice()
                            } else {
                                std::slice::from_ref(cell)
                            };
                            let cell_id = format!("table-cell:{key}:{index}:{column}");
                            let cell_debug = cell_id.clone();
                            let cell_label = table_cell_label(body, header, index, column);
                            table_column(widths[column], shrink[column])
                                .id(SharedString::from(cell_id))
                                .debug_selector(move || cell_debug.clone())
                                .role(table_cell_role(header))
                                .aria_label(cell_label)
                                .when_some(table_cell_description(header), |cell, description| {
                                    cell.aria_description(description)
                                })
                                .aria_row_index(index)
                                .aria_column_index(column)
                                .flex()
                                .flex_col()
                                .justify_center()
                                // Grow in proportion to max-content widths;
                                // between min/max, shrink by each column's
                                // available range rather than by text length.
                                .border_b_1()
                                .border_r_1()
                                .when(index == 0, |cell| cell.border_t_1())
                                .border_color(theme.border_soft)
                                .px(px(10.))
                                .py(px(6.))
                                .child(table_cell(
                                    body,
                                    &format!("{key}:{index}:{column}"),
                                    ink,
                                    theme,
                                    if header {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::NORMAL
                                    },
                                    context,
                                ))
                        }))
                }));
            crate::scroll::AutoScroll::horizontal(scroll, move |scroll, gutter| {
                div()
                    .id(SharedString::from(format!("table-scroller:{table_key}")))
                    .role(gpui::Role::Region)
                    .aria_label("Scrollable table")
                    .track_focus(&focus)
                    .map(|control| {
                        crate::focus::outline(control, &focus, 0., 1., 0., outline_color, &keyboard)
                    })
                    .w_full()
                    .min_w_0()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.focus(&focus, cx);
                        cx.notify(layout_owner);
                    })
                    .on_key_down(move |event, window, cx| {
                        let direction = match event.keystroke.key.as_str() {
                            "left" => 1.,
                            "right" => -1.,
                            _ => return,
                        };
                        let step = if event.keystroke.modifiers.alt {
                            navigation.bounds().size.width
                        } else {
                            px(40.)
                        };
                        let mut offset = navigation.offset();
                        offset.x =
                            (offset.x + step * direction).clamp(-navigation.max_offset().x, px(0.));
                        navigation.set_offset(offset);
                        window.refresh();
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("table-viewport:{table_key}")))
                            .w_full()
                            .min_w_0()
                            .overflow_x_scroll()
                            .track_scroll(&scroll)
                            .pb(gutter)
                            .child(table),
                    )
                    .into_any_element()
            })
            .on_gutter_change(move |cx| {
                if let Some(list) = &layout_list {
                    list.remeasure();
                }
                cx.notify(layout_owner);
            })
            .into_any_element()
        }
        _ => inline(children, key, ink, theme, FontWeight::NORMAL),
    }
}

pub(super) struct BlockContext<'a> {
    pub ui: &'a mut MarkdownUi,
    pub list: ListState,
    pub answer: Option<Arc<str>>,
}
pub(super) fn block(
    block: &Block,
    ink: gpui::Rgba,
    theme: &Theme,
    context: BlockContext,
    window: &mut Window,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    document(
        &block.text,
        &block.id,
        ink,
        theme,
        &mut RenderContext {
            ui: context.ui,
            window,
            owner: Some(cx.weak_entity()),
            cx,
            user: false,
            streaming: block.streaming,
            list_depth: 0,
            list: Some(context.list),
            answer: context.answer,
        },
    )
}
fn document(
    text: &str,
    key: &str,
    ink: gpui::Rgba,
    theme: &Theme,
    context: &mut RenderContext,
) -> AnyElement {
    let nodes = tree(&mut markdown::events(text).into_iter());
    div()
        .min_w_0()
        .text_size(px(14.))
        .line_height(px(22.75))
        .text_color(ink)
        .child(stack(&nodes, key, ink, theme, 14., context))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_type_tokens_use_the_github_type_palette() {
        let light = Theme::from_profile(vela_core::appearance::profile("codex"));
        let dark = Theme::from_profile(vela_core::appearance::profile("codex-dark"));
        assert_eq!(
            syntax_color(crate::syntax::SyntaxToken::Type, &light),
            gpui::rgb(0x953800)
        );
        assert_eq!(
            syntax_color(crate::syntax::SyntaxToken::Type, &dark),
            gpui::rgb(0xffa657)
        );
    }

    #[test]
    fn image_projection_preserves_surrounding_styles_alt_text_and_link_targets() {
        let nodes=tree(&mut markdown::events("before **bold ![image 🌍](data:image/png;base64,AA==) after** [![linked](file:///fixture.png)](https://example.com)").into_iter());
        let parts = image_parts(&nodes, None);
        let images = parts
            .iter()
            .filter_map(|part| {
                if let InlinePart::Image { uri, alt, link } = part {
                    Some((uri.as_str(), alt.as_str(), link.as_deref()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            images,
            vec![
                ("data:image/png;base64,AA==", "image 🌍", None),
                ("file:///fixture.png", "linked", Some("https://example.com"))
            ]
        );
        let theme = Theme::default();
        let mut text = Inline::default();
        for part in parts {
            if let InlinePart::Text(nodes) = part {
                flatten(
                    &nodes,
                    &InlineStyle::default(),
                    &mut text,
                    theme.text,
                    &theme,
                );
            }
        }
        assert_eq!(text.text, "before bold  after ");
        assert!(
            text.runs
                .iter()
                .any(|run| run.font.weight == FontWeight::BOLD)
        );
    }
    struct CodeControlFixture {
        focus: gpui::FocusHandle,
        wrapped: bool,
        activations: usize,
    }
    impl Render for CodeControlFixture {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let owner = cx.weak_entity();
            focused_button(
                "wrap-test",
                if self.wrapped { "No wrap" } else { "Wrap" },
                &Theme::default(),
                &self.focus,
                &crate::focus::KeyboardMode::default(),
                move |_, cx| {
                    let _ = owner.update(cx, |view, cx| {
                        view.wrapped = !view.wrapped;
                        view.activations += 1;
                        cx.notify();
                    });
                },
            )
        }
    }
    struct SoftBreakProbe(&'static str, &'static str);

    impl Render for SoftBreakProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(220.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "soft-break".into())
                        .whitespace_normal()
                        .child(self.0),
                )
                .child(
                    div()
                        .debug_selector(|| "space-break".into())
                        .whitespace_normal()
                        .child(self.1),
                )
        }
    }

    #[gpui::test]
    fn soft_breaks_render_as_hard_lines_like_marked_breaks(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| SoftBreakProbe("one\ntwo", "one two"));
        cx.simulate_resize(size(px(300.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(300.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        let soft = cx.debug_bounds("soft-break").unwrap();
        let space = cx.debug_bounds("space-break").unwrap();
        assert!(
            soft.size.height > space.size.height,
            "a soft break occupies a hard line like Marked's breaks: true: {soft:?} vs {space:?}"
        );
        assert_eq!(soft.size.height, space.size.height * 2.);
    }

    #[gpui::test]
    fn code_control_enter_and_space_activate_exactly_once(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle().tab_stop(true);
            window.focus(&focus, cx);
            CodeControlFixture {
                focus,
                wrapped: true,
                activations: 0,
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(200.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        for (key, count, wrapped) in [("enter", 1, false), ("enter", 2, true), ("space", 3, false)]
        {
            cx.simulate_keystrokes(key);
            cx.simulate_event(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse(key).unwrap(),
            });
            draw(cx);
            view.read_with(cx, |view, _| {
                assert_eq!(view.activations, count);
                assert_eq!(view.wrapped, wrapped);
            });
        }
    }
    struct BubbleFixture(&'static str);
    impl Render for BubbleFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            div().w(px(284.)).child(bubble(
                user_inline(self.0, "test", theme.text, &theme),
                "test",
                &theme,
                None,
            ))
        }
    }
    #[gpui::test]
    fn user_bubble_retains_intrinsic_width_and_wraps_into_normal_lines(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(move |_, _| {
            BubbleFixture("Check every table column, including the far edge.")
        });
        cx.draw(point(px(0.), px(0.)), size(px(284.), px(300.)), |_, _| {
            view.clone().into_any_element()
        });
        let bounds = cx
            .debug_bounds("user-bubble:test")
            .expect("rendered bubble bounds");
        assert!(
            bounds.size.width > px(150.),
            "bubble must not collapse to one character per line: {bounds:?}"
        );
        assert!(
            bounds.size.width >= px(218.) && bounds.size.width <= px(219.5),
            "a wrapped bubble must retain its CSS77-percent width cap: {bounds:?}"
        );
        assert!(
            bounds.size.height < px(90.),
            "the prompt must wrap as a normal short paragraph: {bounds:?}"
        );
    }
    #[gpui::test]
    fn short_user_bubbles_hug_the_longest_explicit_line(cx: &mut gpui::TestAppContext) {
        let mut longest_line_width = None;
        for text in ["Hi", "Hello", "Hello  \nHi"] {
            let (view, visual) = cx.add_window_view(move |_, _| BubbleFixture(text));
            visual.draw(point(px(0.), px(0.)), size(px(284.), px(300.)), |_, _| {
                view.into_any_element()
            });
            let bounds = visual.debug_bounds("user-bubble:test").unwrap();
            assert!(
                bounds.size.width > px(24.) && bounds.size.width < px(100.),
                "short content must keep its intrinsic width: {bounds:?}"
            );
            if text == "Hello" {
                longest_line_width = Some(bounds.size.width);
            }
            if text.contains('\n') {
                assert_eq!(Some(bounds.size.width), longest_line_width);
            }
        }
    }
    struct RichBubbleFixture {
        width: f32,
        text: String,
        ui: MarkdownUi,
        observed: std::rc::Rc<std::cell::RefCell<Option<Bounds<gpui::Pixels>>>>,
    }
    struct CodeSurfaceFixture {
        ui: MarkdownUi,
        wrap: bool,
        code: Option<String>,
        language: &'static str,
    }
    struct AnswerFocusFixture {
        ui: transcript_view::TranscriptUi,
        footer: [gpui::FocusHandle; 2],
        outside: gpui::FocusHandle,
        visible: [bool; 2],
    }
    impl Render for AnswerFocusFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let mut content = Vec::new();
            for (key, answer, source) in [
                ("answer-a", Some("A"), "```text\nFirst answer\n```"),
                (
                    "answer-b",
                    Some("B"),
                    "| Name | State |\n| --- | --- |\n| Unicode 🌍 | Retained |",
                ),
                (
                    "process",
                    None,
                    "```text\nReasoning outside the answer\n```",
                ),
            ] {
                let mut context = RenderContext {
                    ui: &mut self.ui.markdown,
                    window,
                    cx,
                    owner: None,
                    user: false,
                    streaming: false,
                    list_depth: 0,
                    list: None,
                    answer: answer.map(Arc::from),
                };
                content.push(document(source, key, theme.text, &theme, &mut context));
            }
            self.visible = [
                self.ui.answer_footer_visible("A", &self.footer[0], window),
                self.ui.answer_footer_visible("B", &self.footer[1], window),
            ];
            div()
                .w(px(300.))
                .children(content)
                .child(
                    div()
                        .track_focus(&self.footer[0])
                        .h(px(20.))
                        .child("Copy A"),
                )
                .child(
                    div()
                        .track_focus(&self.footer[1])
                        .h(px(20.))
                        .child("Copy B"),
                )
                .child(div().track_focus(&self.outside).h(px(20.)).child("Outside"))
        }
    }
    #[gpui::test]
    fn ansi_code_fence_renders_cleaned_styled_text(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some("\x1b[31mred\x1b[0m plain \x1b[1;4mblue\x1b[0m".into()),
            language: "ansi",
        });
        cx.simulate_resize(size(px(250.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(250.), px(300.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("code-body:surface-test").is_some());
        let body = cx.debug_bounds("code-body:surface-test").unwrap();
        assert!(
            body.size.height >= px(18.) && body.size.height < px(80.),
            "escape bytes are stripped before layout: {body:?}"
        );
    }

    #[gpui::test]
    fn lua_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "local function greet(name)\n  print(\"hi \" .. name)\nend";
        let runs = syntax_runs("lua", code, false, &theme, theme.text)
            .expect("lua highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/strings/functions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "lua",
        });
        cx.simulate_resize(size(px(300.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Lua fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn r_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "values <- c(1, 2, 3)\nmean(values)";
        let runs = syntax_runs("r", code, false, &theme, theme.text)
            .expect("R highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "functions/numbers get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "r",
        });
        cx.simulate_resize(size(px(300.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "R fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn scala_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "object Main {\n  def main(args: Array[String]): Unit = println(\"hi\")\n}";
        let runs = syntax_runs("scala", code, false, &theme, theme.text)
            .expect("Scala highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/functions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "scala",
        });
        cx.simulate_resize(size(px(300.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Scala fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn dart_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "void main() {\n  print('hello');\n}";
        let runs = syntax_runs("dart", code, false, &theme, theme.text)
            .expect("Dart highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "types/functions/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "dart",
        });
        cx.simulate_resize(size(px(300.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Dart fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn powershell_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "param([string]$Name)\nWrite-Host \"Hi $Name\"";
        let runs = syntax_runs("powershell", code, false, &theme, theme.text)
            .expect("PowerShell highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/variables get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "powershell",
        });
        cx.simulate_resize(size(px(320.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(320.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "PowerShell fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn elixir_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "defmodule Demo do\n  def greet(name), do: IO.puts(\"hi \" <> name)\nend";
        let runs = syntax_runs("elixir", code, false, &theme, theme.text)
            .expect("Elixir highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/functions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "elixir",
        });
        cx.simulate_resize(size(px(340.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(340.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Elixir fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn haskell_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "main :: IO ()\nmain = putStrLn \"hi\"";
        let runs = syntax_runs("haskell", code, false, &theme, theme.text)
            .expect("Haskell highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "functions/types/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "haskell",
        });
        cx.simulate_resize(size(px(320.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(320.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Haskell fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn clojure_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "(defn greet [name]\n  (println \"Hello, \" name))";
        let runs = syntax_runs("clojure", code, false, &theme, theme.text)
            .expect("Clojure highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/functions/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "clojure",
        });
        cx.simulate_resize(size(px(340.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(340.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Clojure fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn julia_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "function greet(name)\n  println(\"Hello, $name\")\nend";
        let runs = syntax_runs("julia", code, false, &theme, theme.text)
            .expect("Julia highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/functions/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "julia",
        });
        cx.simulate_resize(size(px(340.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(340.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Julia fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn ocaml_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "let rec map f = function\n  | [] -> []\n  | x :: xs -> f x :: map f xs";
        let runs = syntax_runs("ocaml", code, false, &theme, theme.text)
            .expect("OCaml highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/functions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "ocaml",
        });
        cx.simulate_resize(size(px(360.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(360.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "OCaml fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn erlang_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "-module(demo).\ngreet(Name) ->\n    io:format(\"Hello, ~s~n\", [Name]).";
        let runs = syntax_runs("erlang", code, false, &theme, theme.text)
            .expect("Erlang highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "attributes/functions/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "erlang",
        });
        cx.simulate_resize(size(px(380.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(380.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Erlang fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn elm_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "module Demo exposing (main)\n\nmain =\n    Html.text \"hi\"";
        let runs = syntax_runs("elm", code, false, &theme, theme.text)
            .expect("Elm highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/functions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "elm",
        });
        cx.simulate_resize(size(px(360.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(360.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Elm fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn hcl_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "resource \"aws_instance\" \"web\" {\n  count = 2\n  tags = { Name = \"web-${count.index}\" }\n}";
        let runs = syntax_runs("hcl", code, false, &theme, theme.text)
            .expect("HCL highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/attributes/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "hcl",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "HCL fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn nix_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "{ pkgs, ... }:\n\n{\n  name = \"demo\";\n  message = builtins.concatStringsSep \", \" [ \"a\" \"b\" ];\n}";
        let runs = syntax_runs("nix", code, false, &theme, theme.text)
            .expect("Nix highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "attributes/functions/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "nix",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Nix fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn puppet_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "class nginx {\n  package { 'nginx':\n    ensure => installed,\n  }\n}";
        let runs = syntax_runs("puppet", code, false, &theme, theme.text)
            .expect("Puppet highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "types/attributes/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "puppet",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Puppet fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn graphql_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "query GetUser($id: ID!) {\n  user(id: $id) {\n    id\n    name\n  }\n}";
        let runs = syntax_runs("graphql", code, false, &theme, theme.text)
            .expect("GraphQL highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/variables get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "graphql",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "GraphQL fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn vue_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<template>\n  <div>{{ message }}</div>\n</template>\n\n<script setup>\nconst message = \"hi\"\n</script>";
        let runs = syntax_runs("vue", code, false, &theme, theme.text)
            .expect("Vue highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/directives/scripts get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "vue",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Vue fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn svelte_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<script>\nlet count = 0;\n</script>\n\n{#if count}\n  <p>{count}</p>\n{/if}";
        let runs = syntax_runs("svelte", code, false, &theme, theme.text).expect("Svelte highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/blocks/scripts get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "svelte",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Svelte fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn astro_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "---\nconst title = \"Hi\";\n---\n\n<h1>{title}</h1>\n\n<style>\n.title { color: red; }\n</style>";
        let runs =
            syntax_runs("astro", code, false, &theme, theme.text).expect("Astro highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "frontmatter/tags/expressions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "astro",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Astro fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn zig_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "const std = @import(\"std\");\n\npub fn main() void {\n    const answer: u8 = 42;\n    std.debug.print(\"{d}\\n\", .{answer});\n}";
        let runs =
            syntax_runs("zig", code, false, &theme, theme.text).expect("Zig highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/builtins/types get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "zig",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Zig fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn solidity_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.24;\n\ncontract Counter {\n    uint256 public value = 42;\n\n    function set(uint256 next) external {\n        require(next > 0, \"zero\");\n        value = next;\n    }\n}";
        let runs = syntax_runs("solidity", code, false, &theme, theme.text)
            .expect("Solidity highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "comments/keywords/types get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "solidity",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Solidity fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn mdx_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "---\ntitle: \"Hello\"\n---\nimport { Card } from \"./Card\";\n\n# Hello {name}\n\n<Card count={2}>**Hi**</Card>\n\n```ts\nconst value = { ignored: true };\n```";
        let runs =
            syntax_runs("mdx", code, false, &theme, theme.text).expect("MDX highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "frontmatter/ESM/JSX/expressions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "mdx",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "MDX fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn prisma_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "datasource db {\n  provider = \"postgresql\"\n  url = env(\"DATABASE_URL\")\n}\n\nmodel User {\n  id Int @id\n  role Role @default(USER)\n}\n\nenum Role {\n  USER\n}";
        let runs = syntax_runs("prisma", code, false, &theme, theme.text)
            .expect("Prisma highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "blocks/fields/attributes get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "prisma",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Prisma fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn handlebars_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<div class=\"page\">\n{{#if user}}\n<p>{{user.name}}</p>\n{{else}}\n<p>Guest</p>\n{{/if}}\n{{> footer year=2026}}\n</div>";
        let runs = syntax_runs("handlebars", code, false, &theme, theme.text)
            .expect("Handlebars highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/mustaches/attributes get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "handlebars",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Handlebars fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn liquid_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<div>\n{% if product.available %}\n<h1>{{ product.title | escape }}</h1>\n{% else %}\n<p>{{ 'Sold out' | t }}</p>\n{% endif %}\n{% raw %}{{ literal }}{% endraw %}\n</div>";
        let runs = syntax_runs("liquid", code, false, &theme, theme.text)
            .expect("Liquid highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/objects/filters get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "liquid",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Liquid fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn blade_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<div class=\"card\">\n{{-- hidden --}}\n@if ($user->active)\n<h1>{{ $user->name }}</h1>\n@else\n<p>{!! $fallback !!}</p>\n@endif\n@include('partials.footer')\n</div>";
        let runs =
            syntax_runs("blade", code, false, &theme, theme.text).expect("Blade highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/directives/echoes get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "blade",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Blade fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn templ_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "package components\n\ntempl Hello(name string) {\n  <div class=\"hello\">Hello, { name }!</div>\n  if len(name) == 0 {\n    <p>Empty</p>\n  }\n}\n\n@Layout() {\n  <h1>Title</h1>\n}";
        let runs =
            syntax_runs("templ", code, false, &theme, theme.text).expect("Templ highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/go/expressions get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "templ",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Templ fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn pascal_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "program Hello;\n{$mode objfpc}\n\nfunction Add(A, B: Integer): Integer;\nbegin\n  Result := A + B;\n  if Result > 10 then Exit(Result);\n  WriteLn('O''Brien', #13, $FF);\nend.";
        let runs = syntax_runs("pascal", code, false, &theme, theme.text)
            .expect("Pascal highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "keywords/types/strings get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "pascal",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Pascal fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn makefile_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "# Build\nCC := clang\nSRC = $(wildcard src/*.c)\n\n.PHONY: all\nall: $(SRC)";
        let runs = syntax_runs("makefile", code, false, &theme, theme.text)
            .expect("Makefile highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "variables/targets/recipes get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "makefile",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Makefile fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn twig_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "<div class=\"page\">\n{# hidden #}\n{% if user.active %}\n<h1>{{ user.name | upper }}</h1>\n{% else %}\n<p>{{ 'Guest' }}</p>\n{% endif %}\n{% verbatim %}{{ not_twig }}{% endverbatim %}\n</div>";
        let runs =
            syntax_runs("twig", code, false, &theme, theme.text).expect("Twig highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/filters/comments get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "twig",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Twig fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn pug_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "doctype html\nhtml\n  body\n    // comment\n    h1#title Hello #{user.name}\n    if user.active\n      p Welcome !{user.name}\n    include partials/footer";
        let runs =
            syntax_runs("pug", code, false, &theme, theme.text).expect("Pug highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "tags/attributes/interpolations get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "pug",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "Pug fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn ini_code_fence_renders_highlighted_body(cx: &mut gpui::TestAppContext) {
        let theme = Theme::default();
        let code = "# config\n[server]\nhost = \"localhost\"\nport = 8080\npath = 'C:\\Program Files'\n\n[client]\ntheme = \"dark\"";
        let runs =
            syntax_runs("ini", code, false, &theme, theme.text).expect("INI highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(runs.len() > 2, "comments/sections/keys get distinct runs");

        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: true,
            code: Some(code.into()),
            language: "ini",
        });
        cx.simulate_resize(size(px(420.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("code-body:surface-test").is_some(),
            "INI fences render through the highlighted code body"
        );
    }

    #[gpui::test]
    fn answer_footer_follows_focus_in_its_own_code_table_and_copy_control(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let outside = cx.focus_handle().tab_stop(true);
            window.focus(&outside, cx);
            AnswerFocusFixture {
                ui: Default::default(),
                footer: [
                    cx.focus_handle().tab_stop(true),
                    cx.focus_handle().tab_stop(true),
                ],
                outside,
                visible: [false; 2],
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(600.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.visible), [false, false]);
        let controls = view.read_with(cx, |view, _| {
            [
                view.ui.markdown.focus["answer-a:0:wrap"].handle.clone(),
                view.ui.markdown.focus["table:answer-b:0"].handle.clone(),
                view.footer[0].clone(),
                view.ui.markdown.focus["process:0:wrap"].handle.clone(),
                view.outside.clone(),
            ]
        });
        for (focus, expected) in controls.into_iter().zip([
            [true, false],
            [false, true],
            [true, false],
            [false, false],
            [false, false],
        ]) {
            cx.update(|window, cx| window.focus(&focus, cx));
            draw(cx);
            assert_eq!(view.read_with(cx, |view, _| view.visible), expected);
        }
    }
    struct ParagraphLineHeightFixture(MarkdownUi);
    impl Render for ParagraphLineHeightFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let mut context = RenderContext {
                ui: &mut self.0,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            div()
                .w(px(300.))
                .text_size(px(14.))
                .line_height(px(22.75))
                .child(document(
                    "Line one  \nLine two",
                    "fractional-paragraph",
                    theme.text,
                    &theme,
                    &mut context,
                ))
        }
    }
    #[gpui::test]
    fn markdown_accumulates_fractional_line_height_before_snapping_the_block(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) =
            cx.add_window_view(|_, _| ParagraphLineHeightFixture(MarkdownUi::default()));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(200.)), |_, _| {
            view.into_any_element()
        });
        let bounds = cx.debug_bounds("label:fractional-paragraph:0").unwrap();
        assert_eq!(
            bounds.size.height,
            px(45.5),
            "two CSS22.75px lines must accumulate their fraction: {bounds:?}"
        );
    }
    struct TableSizingFixture;
    impl Render for TableSizingFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            // Recorded CoreText measurements from the real wide-table QA.
            let mut columns = [
                TableColumn {
                    minimum: 3021.8674,
                    maximum: 3039.8674,
                },
                TableColumn {
                    minimum: 75.32947,
                    maximum: 75.32947,
                },
                TableColumn {
                    minimum: 118.27625,
                    maximum: 136.27625,
                },
            ];
            snap_table_columns(&mut columns, 1.);
            let shrink = table_shrink_factors(&columns);
            div()
                .flex()
                .w(px(columns.iter().map(|column| column.minimum).sum()))
                .children(columns.into_iter().enumerate().map(|(index, column)| {
                    table_column(column, shrink[index])
                        .h(px(20.))
                        .debug_selector(move || format!("recorded-table-column:{index}"))
                }))
        }
    }
    #[gpui::test]
    fn table_distributes_all_negative_space_for_recorded_live_column_ranges(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| TableSizingFixture);
        cx.draw(point(px(0.), px(0.)), size(px(3300.), px(100.)), |_, _| {
            view.into_any_element()
        });
        let first = cx.debug_bounds("recorded-table-column:0").unwrap();
        let middle = cx.debug_bounds("recorded-table-column:1").unwrap();
        let last = cx.debug_bounds("recorded-table-column:2").unwrap();
        assert_eq!(first.size.width, px(3022.));
        assert_eq!(middle.size.width, px(76.));
        assert_eq!(last.size.width, px(119.));
        assert_eq!(last.right(), px(3217.));
    }
    #[test]
    fn markdown_link_handoff_uses_the_safe_https_policy() {
        for (url, expected) in [
            (
                "https://example.com/docs",
                Some("https://example.com/docs"),
            ),
            ("http://example.com", None),
            ("https://user:pass@example.com", None),
            ("javascript:alert(1)", None),
            ("data:text/html,hi", None),
            ("file:///tmp/secret", None),
            ("mailto:hello@example.com", None),
            ("vela://session/1", None),
            ("not a url", None),
        ] {
            assert_eq!(
                vela_browser::safe_external_url(url).as_deref(),
                expected,
                "{url}"
            );
        }
    }

    #[test]
    fn bare_gfm_urls_render_as_inline_link_ranges() {
        let theme = Theme::default();
        let nodes = tree(
            &mut markdown::events("Visit https://example.com/docs and www.example.org.").into_iter(),
        );
        let mut result = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut result,
            theme.text,
            &theme,
        );
        assert_eq!(result.text, "Visit https://example.com/docs and www.example.org.");
        assert_eq!(result.links.len(), 2);
        assert_eq!(&result.text[result.links[0].0.clone()], "https://example.com/docs");
        assert_eq!(result.links[0].1, "https://example.com/docs");
        assert_eq!(&result.text[result.links[1].0.clone()], "www.example.org");
        assert_eq!(result.links[1].1, "http://www.example.org");
    }

    #[test]
    fn text_entities_flatten_to_decoded_prose() {
        let nodes = tree(&mut markdown::events("Tom &amp; Jerry &lt;3").into_iter());
        let theme = Theme::default();
        let mut inline = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut inline,
            theme.text,
            &theme,
        );
        assert_eq!(inline.text, "Tom & Jerry <3");
    }

    #[test]
    fn inline_math_renders_scripts_in_the_math_font() {
        let nodes = tree(
            &mut markdown::events("before $x_i^2 + \\alpha$ after").into_iter(),
        );
        let theme = Theme::default();
        let mut inline = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut inline,
            theme.text,
            &theme,
        );
        assert!(inline.text.contains("xᵢ² + α"), "{}", inline.text);
        assert!(inline.runs.iter().any(|run| run.font.family.as_ref() == crate::math::MATH_FONT));
        assert_eq!(inline.math_sources, vec![r"x_i^2 + \alpha"]);
    }

    #[test]
    fn source_inline_math_delimiters_render_in_the_math_font() {
        let nodes = tree(&mut markdown::events("before \\(x_i\\) after").into_iter());
        let theme = Theme::default();
        let mut inline = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut inline,
            theme.text,
            &theme,
        );
        assert!(inline.text.contains("xᵢ"), "{}", inline.text);
        assert_eq!(inline.math_sources, vec!["x_i"]);
        assert!(inline.runs.iter().any(|run| run.font.family.as_ref() == crate::math::MATH_FONT));
    }

    struct EntitySurfaceFixture {
        ui: MarkdownUi,
    }
    impl Render for EntitySurfaceFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            div().w(px(420.)).child(document(
                "Tom &amp; Jerry &lt;3",
                "entity-test",
                theme.text,
                &theme,
                &mut context,
            ))
        }
    }

    #[gpui::test]
    fn decoded_entities_render_through_the_prose_label(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| EntitySurfaceFixture {
            ui: MarkdownUi::default(),
        });
        cx.simulate_resize(size(px(420.), px(160.)));
        cx.draw(point(px(0.), px(0.)), size(px(420.), px(160.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("label:entity-test:0").is_some(),
            "decoded entities render through the prose label"
        );
    }

    struct MathSurfaceFixture {
        ui: MarkdownUi,
        tex: &'static str,
        key: &'static str,
    }
    impl Render for MathSurfaceFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            let tex = format!("$${}$$", self.tex);
            div().w(px(520.)).child(document(
                &tex,
                self.key,
                theme.text,
                &theme,
                &mut context,
            ))
        }
    }

    #[gpui::test]
    fn display_math_renders_a_centered_math_block(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| MathSurfaceFixture {
            ui: MarkdownUi::default(),
            tex: "E = mc^2",
            key: "math-test",
        });
        cx.simulate_resize(size(px(520.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(520.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("math-block:math-test:0:0").is_some(),
            "display math renders through the native math block"
        );
    }

    #[gpui::test]
    fn display_math_renders_stacked_annotations_and_substacks(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| MathSurfaceFixture {
            ui: MarkdownUi::default(),
            tex: r"\substack{a \\ b} \overset{!}{=} \underset{n}{\lim}",
            key: "math-stacked",
        });
        cx.simulate_resize(size(px(520.), px(240.)));
        cx.draw(point(px(0.), px(0.)), size(px(520.), px(240.)), |_, _| {
            view.into_any_element()
        });
        assert!(
            cx.debug_bounds("math-block:math-stacked:0:0").is_some(),
            "stacked/annotated math renders through the native math block"
        );
    }

    #[test]
    fn inline_path_code_uses_the_accent_color_but_plain_code_stays_ink() {
        let nodes = tree(
            &mut markdown::events("`src/main.rs` and `plain`").into_iter(),
        );
        let theme = Theme::default();
        let mut inline = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut inline,
            theme.text,
            &theme,
        );
        assert_eq!(inline.runs[0].color, theme.accent.into());
        assert_eq!(inline.runs[2].color, theme.text.into());
        assert_eq!(inline.runs[0].font.family.as_ref(), theme::source_font());
    }

    #[test]
    fn ansi_runs_strip_escape_bytes_and_carry_sgr_styles() {
        let theme = Theme::default();
        let (display, spans) = crate::syntax::ansi(
            "\x1b[31mred\x1b[0m plain \x1b[1;4;38;5;33mblue\x1b[0m",
        );
        assert_eq!(display, "red plain blue");
        assert_eq!(crate::syntax::language("ansi"), Some("ansi"));
        let runs = ansi_runs(&display, &spans, &theme, theme.text);
        assert_eq!(
            runs.iter().map(|run| run.len).sum::<usize>(),
            display.len(),
            "runs cover the cleaned ANSI text"
        );
        assert!(runs.iter().any(|run| run.color == gpui::rgb(0xcd3131).into()));
        assert!(runs.iter().any(|run| {
            run.color == gpui::rgb(0x0087ff).into()
                && run.font.weight == FontWeight::BOLD
                && run.underline.is_some()
        }));
    }

    #[test]
    fn code_fence_highlighting_covers_text_and_skips_tabs_or_unknown_languages() {
        let theme = Theme::default();
        let text = "const answer = 42; // note";
        let runs = syntax_runs("ts", text, false, &theme, theme.text).expect("typescript highlight runs");
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
        assert!(runs.len() > 2, "keywords/strings/comments get distinct runs");
        assert!(
            syntax_runs("ts", text, true, &theme, theme.text).is_none(),
            "a streaming fence stays on the plain code surface"
        );
        assert!(syntax_runs("unknown", text, false, &theme, theme.text).is_none());
        assert!(
            syntax_runs("json", "{\n\t\"answer\": 42\n}", false, &theme, theme.text).is_none(),
            "tabbed code keeps the plain preformatted layout"
        );
        assert!(syntax_runs("json", "{\"answer\": 42}", false, &theme, theme.text).is_some());
    }

    #[test]
    fn image_size_cache_only_requests_reflow_when_a_lazy_image_resolves() {
        let mut cache = std::collections::HashMap::new();
        assert!(!cache_image_size(&mut cache, "img", None));
        assert!(!cache_image_size(&mut cache, "img", None));
        let size = gpui::size(px(200.), px(100.));
        assert!(cache_image_size(&mut cache, "img", Some(size)));
        assert!(!cache_image_size(&mut cache, "img", Some(size)));
        assert!(!cache_image_size(&mut cache, "img", None));
    }

    #[test]
    fn table_cells_expose_header_cell_roles_and_plain_accessibility_labels() {
        let nodes = tree(
            &mut markdown::events(
                "| **Name** | Picture |\n| --- | --- |\n| `code` and **bold** | ![A globe](file:///globe.png) |\n|  | Empty |\n| ![ ](file:///untitled.png) | Done |",
            )
            .into_iter(),
        );
        let Node::Container(Tag::Table(_), rows) = &nodes[0] else {
            panic!("expected a table");
        };
        fn row_cells(row: &Node) -> &[Node] {
            let Node::Container(_, cells) = row else {
                panic!("expected a row");
            };
            cells
        }
        fn cell_body(cell: &Node) -> &[Node] {
            let Node::Container(_, body) = cell else {
                panic!("expected a cell");
            };
            body
        }
        let header = row_cells(&rows[0]);
        let first = row_cells(&rows[1]);
        let second = row_cells(&rows[2]);
        let third = row_cells(&rows[3]);

        assert_eq!(table_cell_role(true), gpui::Role::ColumnHeader);
        assert_eq!(table_cell_role(false), gpui::Role::Cell);
        assert_eq!(table_cell_description(true), Some("Column header"));
        assert_eq!(table_cell_description(false), None);
        assert_eq!(table_cell_label(cell_body(&header[0]), true, 0, 0), "Name");
        assert_eq!(table_cell_label(cell_body(&header[1]), true, 0, 1), "Picture");
        assert_eq!(
            table_cell_label(cell_body(&first[0]), false, 1, 0),
            "code and bold"
        );
        assert_eq!(
            table_cell_label(cell_body(&first[1]), false, 1, 1),
            "A globe"
        );
        assert_eq!(
            table_cell_label(cell_body(&second[0]), false, 2, 0),
            "Row 3, column 1"
        );
        assert_eq!(table_cell_label(cell_body(&second[1]), false, 2, 1), "Empty");
        assert_eq!(
            table_cell_label(cell_body(&third[0]), false, 3, 0),
            "Image",
            "an image without alt text still gets a useful cell label"
        );
    }

    struct TableInlineFixture {
        ui: MarkdownUi,
    }
    impl Render for TableInlineFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let source = concat!(
                "| Mixed | Media |\n",
                "| --- | --- |\n",
                "| **bold** `code` [link](https://example.com) | before ![icon](data:image/png;base64,",
                "iVBORw0KGgoAAAANSUhEUgAAAZAAAABkCAYAAACoy2Z3AAABNUlEQVR42u3VIQEAAAjAsPcvDRkQuIkVuHlTAwBXiQCAgQBgIAAYCAAGAgAGAoCBAGAgABgIAAYCAAYCgIEAYCAAGAgABgIABgKAgQBgIAAYCAAGAgAGAoCBAGAgABgIAAYCAAYCgIEAYCAAGAgABgIABgKAgQBgIAAYCAAGAgAGAoCBAGAgABgIABgIAAYCgIEAYCAAGAgAGAgABgKAgQBgIAAYCAAYCAAGAoCBAGAgABgIABgIAAYCgIEAYCAAGAgAGAgABgKAgQBgIAAYCAAYCAAGAoCBAGAgABgIABgIAAYCgIEAYCAAGIgIABgIAAYCgIEAYCAAYCAAGAgABgKAgQBgIABgIAAYCAAGAoCBAGAgAGAgABgIAAYCgIEAYCAAYCAAGAgABgKAgQBgIABgIAD8WLhmWbpVyPVQAAAAAElFTkSuQmCC",
                ") **after** |\n",
                "| Second | Plain |"
            );
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            div().w(px(200.)).child(document(
                source,
                "table-inline",
                theme.text,
                &theme,
                &mut context,
            ))
        }
    }

    #[gpui::test]
    fn table_cells_render_styled_text_and_images_together(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| TableInlineFixture {
            ui: MarkdownUi::default(),
        });
        cx.simulate_resize(size(px(640.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(640.), px(500.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        for _ in 0..6 {
            draw(cx);
            cx.run_until_parked();
        }
        assert!(
            cx.debug_bounds("label:table-inline:0:1:0").is_some(),
            "bold, code and link runs stay in one mixed cell"
        );
        assert!(
            cx.debug_bounds("table-row:table-inline:0:0").is_some()
                && cx.debug_bounds("table-row:table-inline:0:1").is_some()
                && cx.debug_bounds("table-row:table-inline:0:2").is_some(),
            "header and body rows are rendered as separate accessibility rows"
        );
        assert!(
            cx.debug_bounds("table-cell:table-inline:0:0:0").is_some()
                && cx.debug_bounds("table-cell:table-inline:0:0:1").is_some(),
            "header cells render with their column roles"
        );
        let cell = cx
            .debug_bounds("table-cell:table-inline:0:1:1")
            .expect("image cell");
        let image = cx
            .debug_bounds("image-container:table-inline:0:1:1:1")
            .expect("inline table image is rendered, not just its alt text");
        assert!(
            cell.contains(&image.center()),
            "the image stays inside its cell: cell={cell:?} image={image:?}"
        );
        assert_eq!(
            image.size.width,
            px(200.),
            "a 400px-wide image fits Source's 200px table image cap"
        );
        assert!(
            cx.debug_bounds("label:table-inline:0:1:1:0").is_some()
                && cx.debug_bounds("label:table-inline:0:1:1:2").is_some(),
            "text before and after the image remains visible"
        );
        assert!(
            view.read_with(cx, |view, _| view
                .ui
                .image_sizes
                .get("table-inline:0:1:1:1")
                .copied()
                .flatten()
                .is_some()),
            "the cell image resolves through the shared markdown image loader"
        );
        let handle = view.read_with(cx, |view, _| {
            view.ui.horizontal["table:table-inline:0"]
                .native_handle()
                .clone()
        });
        assert!(
            handle.max_offset().x > px(0.),
            "the 200px cell image contributes to the table's intrinsic width"
        );
    }

    struct TableSurfaceFixture {
        ui: MarkdownUi,
        width: f32,
        cell: String,
    }
    impl Render for TableSurfaceFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let source = format!(
                "| Identifier | State | Last column |\n| --- | --- | --- |\n| {} | **Retained** | END_OF_TABLE_🌍 |\n| Second row | Pending | Final cell |",
                self.cell
            );
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            div().w(px(self.width)).child(document(
                &source,
                "table-test",
                theme.text,
                &theme,
                &mut context,
            ))
        }
    }
    #[gpui::test]
    fn table_horizontal_keyboard_navigation_clamps_and_resets_after_reflow(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| TableSurfaceFixture {
            ui: MarkdownUi::default(),
            width: 284.,
            cell: format!("{}🌍", "WIDE".repeat(96)),
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..5 {
                let width = view.read_with(cx, |view, _| view.width);
                cx.simulate_resize(size(px(width), px(300.)));
                cx.draw(point(px(0.), px(0.)), size(px(width), px(300.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        let (focus, handle) = view.read_with(cx, |view, _| {
            (
                view.ui
                    .focus
                    .get("table:table-test:0")
                    .expect("table is a keyboard focus stop")
                    .handle
                    .clone(),
                view.ui
                    .horizontal
                    .get("table:table-test:0")
                    .expect("table owns a horizontal viewport")
                    .native_handle(),
            )
        });
        cx.update(|window, cx| window.focus(&focus, cx));
        draw(cx);
        assert!(handle.max_offset().x > px(2000.));
        let state_cell = cx
            .debug_bounds("label:table-test:0:1:1")
            .expect("state cell is rendered");
        assert!(
            state_cell.size.height <= px(23.),
            "an unbreakable word must fit its own shaped min-content width: {state_cell:?}"
        );
        cx.simulate_keystrokes("right");
        draw(cx);
        assert_eq!(handle.offset().x, px(-40.));
        cx.simulate_keystrokes("alt-right");
        draw(cx);
        assert_eq!(handle.offset().x, -px(40.) - handle.bounds().size.width);
        cx.simulate_keystrokes("alt-left");
        cx.simulate_keystrokes("left");
        cx.simulate_keystrokes("left");
        draw(cx);
        assert_eq!(handle.offset().x, px(0.));
        handle.set_offset(point(-handle.max_offset().x + px(5.), px(0.)));
        cx.simulate_keystrokes("right");
        draw(cx);
        assert_eq!(handle.offset().x, -handle.max_offset().x);
        view.update(cx, |view, cx| {
            view.cell = "Short".into();
            view.width = 600.;
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.max_offset().x, px(0.));
        assert_eq!(handle.offset().x, px(0.));
    }
    #[gpui::test]
    fn table_wraps_breakable_prose_instead_of_forcing_max_content_overflow(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| TableSurfaceFixture {
            ui: MarkdownUi::default(),
            width: 350.,
            cell: "ordinary words ".repeat(30),
        });
        for _ in 0..6 {
            cx.draw(point(px(0.), px(0.)), size(px(350.), px(1200.)), |_, _| {
                view.clone().into_any_element()
            });
        }
        let handle = view.read_with(cx, |view, _| {
            view.ui.horizontal["table:table-test:0"].native_handle()
        });
        assert_eq!(
            handle.max_offset().x,
            px(0.),
            "a paragraph must wrap at word boundaries"
        );
        assert!(
            handle.bounds().size.height > px(200.),
            "all wrapped lines must contribute to row height"
        );
    }
    struct VirtualCodeFixture {
        ui: MarkdownUi,
        list: ListState,
        viewport: crate::scroll::ListViewportHandle,
        wrap: bool,
    }
    impl Render for VirtualCodeFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let owner = cx.weak_entity();
            let list = self.list.clone();
            div()
                .w(px(250.))
                .h(px(200.))
                .child(crate::scroll::ListViewport::new(
                    self.viewport.clone(),
                    list.clone(),
                    move |gutter| {
                        div()
                            .size_full()
                            .pr(gutter)
                            .child(
                                gpui::list(list, move |index, window, cx| {
                                    if index > 0 {
                                        return div()
                                            .h(px(if index == 1 { 300. } else { 20. }))
                                            .into_any_element();
                                    }
                                    owner
                                        .update(cx, |view, cx| {
                                            view.ui.wrap.insert("virtual-code:0".into(), view.wrap);
                                            let theme = Theme::default();
                                            let source = format!(
                                                "```text\n{}\n```",
                                                "wide 🌍 code ".repeat(80)
                                            );
                                            let mut context = RenderContext {
                                                ui: &mut view.ui,
                                                window,
                                                cx,
                                                owner: None,
                                                user: false,
                                                streaming: false,
                                                list_depth: 0,
                                                list: Some(view.list.clone()),
                                                answer: None,
                                            };
                                            document(
                                                &source,
                                                "virtual-code",
                                                theme.text,
                                                &theme,
                                                &mut context,
                                            )
                                        })
                                        .unwrap()
                                })
                                .size_full(),
                            )
                            .into_any_element()
                    },
                ))
        }
    }
    #[gpui::test]
    fn virtual_code_row_remeasures_when_its_horizontal_gutter_appears_and_disappears(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| VirtualCodeFixture {
            ui: MarkdownUi::default(),
            list: ListState::new(3, ListAlignment::Top, px(0.)).measure_all(),
            viewport: crate::scroll::ListViewportHandle::default(),
            wrap: false,
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..6 {
                cx.draw(point(px(0.), px(0.)), size(px(250.), px(200.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        for _ in 0..9 {
            cx.draw(point(px(0.), px(0.)), size(px(250.), px(200.)), |_, _| {
                view.clone().into_any_element()
            });
            view.read_with(cx, |view, _| {
                assert_eq!(
                    view.list.viewport_bounds().size.width,
                    px(242.),
                    "outer vertical gutter must stay stable"
                );
                assert!(
                    view.list.bounds_for_item(0).unwrap().size.height >= px(87.),
                    "inner horizontal gutter must not alternate with the outer width probe"
                );
            });
        }
        view.read_with(cx, |view, _| {
            let bounds = view.list.bounds_for_item(0).expect("visible code row");
            assert!(
                bounds.size.height >= px(87.) && bounds.size.height <= px(90.),
                "row must include8px horizontal gutter: {bounds:?}"
            );
            assert!(
                view.ui.horizontal["virtual-code:0"]
                    .native_handle()
                    .max_offset()
                    .x
                    > px(2000.)
            );
        });
        view.update(cx, |view, cx| {
            view.wrap = true;
            view.list.remeasure();
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            assert!(view.list.bounds_for_item(0).unwrap().size.height > px(200.));
            assert_eq!(
                view.ui.horizontal["virtual-code:0"]
                    .native_handle()
                    .max_offset()
                    .x,
                px(0.)
            );
        });
        view.update(cx, |view, cx| {
            view.wrap = false;
            view.list.remeasure();
            cx.notify();
        });
        draw(cx);
        assert!(
            view.read_with(cx, |view, _| view
                .list
                .bounds_for_item(0)
                .unwrap()
                .size
                .height)
                >= px(87.)
        );
    }
    impl Render for CodeSurfaceFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.ui.wrap.insert("surface-test".into(), self.wrap);
            let raw = self
                .code
                .clone()
                .unwrap_or_else(|| "wide 🌍 code ".repeat(80));
            let nodes = tree(
                &mut markdown::events(&format!("```{}\n{raw}\n```", self.language)).into_iter(),
            );
            let theme = Theme::default();
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: false,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            div().w(px(250.)).child(render(
                &nodes[0],
                "surface-test",
                theme.text,
                &theme,
                &mut context,
            ))
        }
    }
    #[gpui::test]
    fn actual_code_surface_exposes_intrinsic_width_and_resets_scroll_when_wrapped(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: false,
            code: None,
            language: "text",
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..5 {
                cx.draw(point(px(0.), px(0.)), size(px(250.), px(600.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        let handle = view.read_with(cx, |view, _| {
            view.ui.horizontal["surface-test"].native_handle()
        });
        assert!(
            handle.max_offset().x > px(2000.),
            "unwrapped glyphs must be reachable by scrolling"
        );
        assert!(
            handle.bounds().size.height >= px(50.) && handle.bounds().size.height <= px(52.),
            "one code line plus24px padding and8px gutter: {:?}",
            handle.bounds()
        );
        handle.set_offset(point(-handle.max_offset().x, px(0.)));
        draw(cx);
        view.update(cx, |view, cx| {
            view.wrap = true;
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.max_offset().x, px(0.));
        assert_eq!(
            handle.offset().x,
            px(0.),
            "wrapping must restore the beginning of every line"
        );
    }
    #[gpui::test]
    fn code_tabs_use_two_space_stops_in_wrapping_and_scroll_extents(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: false,
            code: Some(format!("{}END", "A ".repeat(40))),
            language: "text",
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..5 {
                cx.draw(point(px(0.), px(0.)), size(px(250.), px(600.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        let handle = view.read_with(cx, |view, _| {
            view.ui.horizontal["surface-test"].native_handle()
        });
        let expected = handle.max_offset().x;
        view.update(cx, |view, cx| {
            view.code = Some(format!("{}END", "A\t".repeat(40)));
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            handle.max_offset().x,
            expected,
            "tabs and equivalent two-space stops must have the same scroll extent"
        );
        view.update(cx, |view, cx| {
            view.wrap = true;
            cx.notify();
        });
        draw(cx);
        let tab_height = handle.bounds().size.height;
        view.update(cx, |view, cx| {
            view.code = Some(format!("{}END", "A ".repeat(40)));
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            handle.bounds().size.height,
            tab_height,
            "the same stops must wrap into the same number of rows"
        );
    }
    #[gpui::test]
    fn code_body_accumulates_fractional_leading_across_blank_lines(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| CodeSurfaceFixture {
            ui: MarkdownUi::default(),
            wrap: false,
            code: Some("One\nTwo\n\nFour".into()),
            language: "text",
        });
        for _ in 0..5 {
            cx.draw(point(px(0.), px(0.)), size(px(250.), px(400.)), |_, _| {
                view.clone().into_any_element()
            });
        }
        let bounds = view.read_with(cx, |view, _| {
            view.ui.horizontal["surface-test"].native_handle().bounds()
        });
        assert_eq!(
            bounds.size.height,
            px(99.),
            "four18.75px code lines plus24px padding: {bounds:?}"
        );
    }
    impl Render for RichBubbleFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            let mut context = RenderContext {
                ui: &mut self.ui,
                window,
                cx,
                owner: None,
                user: true,
                streaming: false,
                list_depth: 0,
                list: None,
                answer: None,
            };
            let (document, label) = user_document(&self.text, "rich-test", &theme, &mut context);
            let observed = self.observed.clone();
            div().w(px(self.width)).child(bubble(
                div()
                    .relative()
                    .child(document)
                    .child(
                        gpui::canvas(
                            move |bounds, _, _| *observed.borrow_mut() = Some(bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .into_any_element(),
                "rich-test",
                &theme,
                label,
            ))
        }
    }
    #[gpui::test]
    fn structured_user_bubble_retains_intrinsic_width_and_block_height(
        cx: &mut gpui::TestAppContext,
    ) {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(None));
        let measured = observed.clone();
        let (view,cx)=cx.add_window_view(move |_,_|RichBubbleFixture {
            width:284.,
            text:"# Formatted request\n\nKeep **strong** text and `inline code`.\n\n- first item\n- second item\n\n> Quoted detail\n\n```rust\nlet value = 42;\n```\n\nFinal paragraph.".into(),
            ui:MarkdownUi::default(),observed:measured,
        });
        for width in [284., 500.] {
            view.update(cx, |view, _| view.width = width);
            cx.draw(point(px(0.), px(0.)), size(px(width), px(600.)), |_, _| {
                view.clone().into_any_element()
            });
            let bounds = observed.borrow().expect("structured document bounds");
            assert!(
                bounds.size.width > px(150.),
                "structured bubble must keep intrinsic text width: {bounds:?}"
            );
            assert!(
                bounds.size.width <= px(width * 0.77 - 24. + 1.),
                "bubble must obey the 77 percent cap: {bounds:?}"
            );
            assert!(
                bounds.size.height > px(150.) && bounds.size.height < px(500.),
                "block content must lay out without single-character wrapping: {bounds:?}"
            );
        }
    }
    #[test]
    fn task_states_survive_tight_loose_and_nested_lists_without_text_symbols() {
        for source in [
            "- [x] Finished\n- [ ] Pending\n- Parent\n  - [x] Child",
            "- [x] Finished\n\n- [ ] Pending\n\n- Parent\n  - [x] Child",
        ] {
            let nodes = tree(&mut markdown::events(source).into_iter());
            let Node::Container(Tag::List(_), items) = &nodes[0] else {
                panic!("expected the original task list");
            };
            let states = items
                .iter()
                .map(|item| {
                    let Node::Container(Tag::Item, body) = item else {
                        panic!("expected item")
                    };
                    task_state(body)
                })
                .collect::<Vec<_>>();
            assert_eq!(states, vec![Some(true), Some(false), None]);
            let theme = Theme::default();
            let mut text = Inline::default();
            flatten(
                &nodes,
                &InlineStyle::default(),
                &mut text,
                theme.text,
                &theme,
            );
            assert!(
                !text.text.contains(['☑', '☐']),
                "task controls must not add extra prose lines"
            );
            for word in ["Finished", "Pending", "Parent", "Child"] {
                assert!(text.text.contains(word));
            }
        }
    }

    #[test]
    fn user_document_collapses_source_block_margins_without_flattening_structure() {
        let nodes = tree(
            &mut markdown::events("# Heading\n\nFirst\n\nSecond\n\n- item\n\n```rs\ncode\n```")
                .into_iter(),
        );
        assert_eq!(user_margins(&nodes[1], Some(&nodes[0])), (0., 14.));
        assert_eq!(user_margins(&nodes[2], Some(&nodes[1])), (14., 14.));
        assert_eq!(user_margins(&nodes[3], Some(&nodes[2])), (14., 14.));
        assert!(matches!(nodes[4], Node::Container(Tag::CodeBlock(_), _)));
    }

    #[test]
    fn nested_inline_styles_are_scoped_and_link_ranges_use_utf8_bytes() {
        let theme = Theme::default();
        let nodes = tree(
            &mut markdown::events(
                "**bold *nested*** plain [🌍](https://example.com) `**literal**`",
            )
            .into_iter(),
        );
        let mut result = Inline::default();
        flatten(
            &nodes,
            &InlineStyle::default(),
            &mut result,
            theme.text,
            &theme,
        );
        assert_eq!(result.text, "bold nested plain 🌍 **literal**");
        assert_eq!(&result.text[result.links[0].0.clone()], "🌍");
        assert!(
            result
                .runs
                .iter()
                .any(|run| run.font.weight == FontWeight::BOLD
                    && run.font.style == gpui::FontStyle::Italic)
        );
        assert_eq!(result.runs.last().unwrap().font.weight, FontWeight::NORMAL);
        for (base, expected) in [
            (FontWeight::NORMAL, FontWeight::BOLD),
            (FontWeight::SEMIBOLD, FontWeight::BLACK),
            (FontWeight::BOLD, FontWeight::BLACK),
        ] {
            let mut styled = Inline::default();
            flatten(
                &nodes,
                &InlineStyle {
                    weight: base,
                    ..Default::default()
                },
                &mut styled,
                theme.text,
                &theme,
            );
            assert_eq!(
                styled.runs[0].font.weight, expected,
                "strong uses CSS relative weight inside headings and headers"
            );
            assert_eq!(
                styled.runs.last().unwrap().font.weight,
                base,
                "a closed strong span restores its inherited weight"
            );
        }
    }
}
