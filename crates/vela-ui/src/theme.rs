use gpui::{Rgba, rgb};

/// Vela's default "Daybreak" (Codex) appearance, taken from the original
/// appearance profile. Tokens keep the product's light paper surfaces, blue
/// signal, and hairline dividers.
#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    kitty: bool,
    paper: bool,
    pub bg: Rgba,
    pub top: Rgba,
    pub rail: Rgba,
    pub panel: Rgba,
    pub agent: Rgba,
    pub editor: Rgba,
    pub inset: Rgba,
    pub elevated: Rgba,
    pub selected: Rgba,
    pub text: Rgba,
    pub dim: Rgba,
    pub faint: Rgba,
    pub accent: Rgba,
    pub send_ink: Rgba,
    pub accent_soft: Rgba,
    pub green: Rgba,
    pub red: Rgba,
    pub yellow: Rgba,
    pub border: Rgba,
    pub border_soft: Rgba,
    pub border_strong: Rgba,
    pub user_bubble: Rgba,
    pub code_bg: Rgba,
}

thread_local! { static ACTIVE_APPEARANCE: std::cell::RefCell<String> = std::cell::RefCell::new("codex".into()); }

pub fn set_appearance(id: &str) {
    ACTIVE_APPEARANCE
        .with(|active| *active.borrow_mut() = vela_core::appearance::profile(id).id.clone());
}

impl Default for Theme {
    fn default() -> Self {
        ACTIVE_APPEARANCE
            .with(|active| Self::from_profile(vela_core::appearance::profile(&active.borrow())))
    }
}

impl Theme {
    pub fn from_profile(profile: &vela_core::appearance::Appearance) -> Self {
        let c = |key: &str| {
            color(
                profile
                    .colors
                    .get(key)
                    .map(String::as_str)
                    .unwrap_or("#000000"),
            )
        };
        let accent = c("accent");
        Self {
            dark: profile.scheme == "dark",
            kitty: profile.id == "kitty",
            paper: profile.id == "paper",
            bg: c("base"),
            top: c("top"),
            rail: c("rail"),
            panel: c("pane"),
            agent: c("agent"),
            editor: c("editor"),
            inset: c("tab"),
            elevated: c("card"),
            selected: c("selected"),
            text: c("ink"),
            dim: c("muted"),
            faint: c("faint"),
            accent,
            send_ink: c("sendInk"),
            accent_soft: Rgba { a: 0.10, ..accent },
            green: c("good"),
            red: c("danger"),
            yellow: c("syntaxString"),
            border: c("line"),
            border_soft: c("lineSoft"),
            border_strong: c("lineStrong"),
            user_bubble: c("user"),
            code_bg: c("console"),
        }
    }
    pub fn small_shadow(&self) -> Vec<gpui::BoxShadow> {
        vec![
            gpui::BoxShadow::new(
                gpui::px(0.),
                gpui::px(1.),
                rgba(0, 0, 0, if self.kitty { 0.30 } else { 0.18 }).into(),
            )
            .blur_radius(gpui::px(2.)),
            gpui::BoxShadow {
                offset: gpui::point(gpui::px(0.), gpui::px(1.)),
                blur_radius: gpui::px(0.),
                spread_radius: gpui::px(0.),
                inset: true,
                color: if self.kitty {
                    rgba(231, 231, 238, 0.05).into()
                } else {
                    rgba(255, 255, 255, 0.03).into()
                },
            },
        ]
    }

    pub fn medium_shadow(&self) -> Vec<gpui::BoxShadow> {
        let (ink, first, y, blur, second) = if self.kitty {
            ((0, 0, 0), 0.24, 14., 36., 0.32)
        } else if self.paper {
            ((67, 48, 33), 0.08, 16., 38., 0.10)
        } else {
            ((0, 0, 0), 0.10, 10., 30., 0.18)
        };
        vec![
            gpui::BoxShadow::new(
                gpui::px(0.),
                gpui::px(2.),
                rgba(ink.0, ink.1, ink.2, first).into(),
            )
            .blur_radius(gpui::px(8.)),
            gpui::BoxShadow::new(
                gpui::px(0.),
                gpui::px(y),
                rgba(ink.0, ink.1, ink.2, second).into(),
            )
            .blur_radius(gpui::px(blur)),
        ]
    }

    pub fn large_shadow(&self) -> Vec<gpui::BoxShadow> {
        let (ink, first, y, blur, second) = if self.kitty {
            ((0, 0, 0), 0.22, 12., 30., 0.28)
        } else if self.paper {
            ((67, 48, 33), 0.06, 12., 28., 0.08)
        } else {
            ((0, 0, 0), 0.07, 10., 24., 0.09)
        };
        vec![
            gpui::BoxShadow::new(
                gpui::px(0.),
                gpui::px(2.),
                rgba(ink.0, ink.1, ink.2, first).into(),
            )
            .blur_radius(gpui::px(6.)),
            gpui::BoxShadow::new(
                gpui::px(0.),
                gpui::px(y),
                rgba(ink.0, ink.1, ink.2, second).into(),
            )
            .blur_radius(gpui::px(blur)),
        ]
    }
}

pub fn color(value: &str) -> Rgba {
    if let Some(value) = value.strip_prefix('#') {
        return rgb(u32::from_str_radix(value, 16).unwrap_or(0));
    }
    if let Some(values) = value
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let channels = values
            .split(',')
            .filter_map(|v| v.trim().parse::<f32>().ok())
            .collect::<Vec<_>>();
        if let [r, g, b, a] = channels.as_slice() {
            return rgba(*r as u8, *g as u8, *b as u8, *a);
        }
    }
    rgba(0, 0, 0, 0.)
}

pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Rgba {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

pub const FONT_UI: &str = ".AppleSystemUIFont";
pub const FONT_MONO: &str = "Menlo";
thread_local! {static SOURCE_FONT: std::cell::Cell<&'static str> = const {std::cell::Cell::new(FONT_MONO)};}

pub fn initialize_source_font(cx: &gpui::App) {
    // Chromium's ui-monospace resolves to the system mono family on macOS.
    // Select the family through GPUI's actual font loader; PostScript-name
    // requests for the private system face can silently return a serif font.
    #[cfg(target_os = "macos")]
    {
        // Terminal ships public SF Mono faces on macOS. Load the installed fonts
        // locally; never redistribute Apple's font files with the application.
        let fonts = ["Regular", "RegularItalic", "Bold", "BoldItalic"].into_iter()
        .filter_map(|face|std::fs::read(format!("/System/Applications/Utilities/Terminal.app/Contents/Resources/Fonts/SF-Mono-{face}.otf")).ok())
        .map(std::borrow::Cow::Owned).collect::<Vec<_>>();
        if !fonts.is_empty() {
            let _ = cx.text_system().add_fonts(fonts);
        }
        for name in ["SF Mono", FONT_MONO] {
            let id = cx.text_system().resolve_font(&gpui::font(name));
            if cx
                .text_system()
                .get_font_for_id(id)
                .is_some_and(|font| font.family.as_ref() == name)
            {
                SOURCE_FONT.with(|font| font.set(name));
                tracing::debug!(family = name, "selected source editor font");
                break;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = cx;
}

pub fn source_font() -> &'static str {
    SOURCE_FONT.with(std::cell::Cell::get)
}

pub fn ui_font() -> gpui::Font {
    gpui::Font {
        fallbacks: Some(gpui::FontFallbacks::from_fonts(
            [
                // CoreText otherwise recursively chooses compact UI emoji
                // through Helvetica before reaching the browser's emoji face.
                "Apple Color Emoji",
                "Segoe UI",
                "Helvetica",
                "Arial",
                "Segoe UI Emoji",
                "Segoe UI Symbol",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        )),
        ..gpui::font(FONT_UI)
    }
}

pub fn source_editor_font() -> gpui::Font {
    gpui::Font {
        fallbacks: Some(gpui::FontFallbacks::from_fonts(
            ["Apple Color Emoji", "Menlo", "Consolas", "Liberation Mono"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )),
        ..gpui::font(source_font())
    }
}

/// The reference Markdown CSS resolves its monospace fallback to Menlo on
/// macOS (7.5256348px per character at 12.5px), independently of the editor.
pub fn markdown_code_font() -> gpui::Font {
    gpui::Font {
        family: FONT_MONO.into(),
        ..source_editor_font()
    }
}

pub const TITLEBAR_HEIGHT: f32 = 44.0;
pub const RAIL_WIDTH: f32 = 50.0;
pub const SIDEBAR_WIDTH: f32 = 230.0;
pub const AGENT_WIDTH: f32 = 325.0;
pub const PANEL_RADIUS: f32 = 16.0;
