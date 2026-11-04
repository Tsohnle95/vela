//! Document-word completion for the native source editor.
//!
//! This is a bounded LSP-style word provider: identifiers are collected from
//! the active buffer, ranked by prefix match, frequency and length, and the
//! current word range is replaced on acceptance.

use std::collections::HashMap;
use std::ops::Range;

pub(super) const MAX_COMPLETIONS: usize = 8;

fn word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn language_extra(rel: &str, ch: char) -> bool {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "css" | "scss" | "less" => ch == '-' || (extension == "scss" && ch == '$') || (extension == "less" && ch == '@'),
        "html" | "htm" | "xml" => matches!(ch, '-' | ':' | '.'),
        _ => false,
    }
}

fn word_char_for(ch: char, rel: &str) -> bool {
    word_char(ch) || language_extra(rel, ch)
}

pub(super) fn word_range_at_lang(text: &str, caret: usize, rel: &str) -> Range<usize> {
    word_range_with(text, caret, |ch| word_char_for(ch, rel))
}

fn word_range_with(text: &str, caret: usize, word: impl Fn(char) -> bool) -> Range<usize> {
    let mut caret = caret.min(text.len());
    while caret > 0 && !text.is_char_boundary(caret) {
        caret -= 1;
    }
    let mut start = caret;
    while start > 0 {
        let ch = text[..start].chars().next_back().expect("non-empty prefix");
        if !word(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = caret;
    while end < text.len() {
        let ch = text[end..].chars().next().expect("non-empty suffix");
        if !word(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    start..end
}

/// Full identifier around the caret. The caret may be inside the word; the
/// returned range still covers the trailing characters so accepting a candidate
/// replaces the whole identifier rather than duplicating its suffix.
pub(super) fn word_range_at(text: &str, caret: usize) -> Range<usize> {
    word_range_with(text, caret, word_char)
}

/// Rank unique document identifiers for `prefix`. Starts-with matches outrank
/// contains matches; frequency, shorter labels and case-insensitive spelling
/// break ties. Numbers and one-character words are ignored.
#[cfg(test)]
pub(super) fn completions(text: &str, prefix: &str, limit: usize) -> Vec<String> {
    completions_with(text, prefix, limit, word_char)
}

#[cfg(test)]
pub(super) fn completions_lang(
    text: &str,
    prefix: &str,
    limit: usize,
    rel: &str,
) -> Vec<String> {
    completions_with(text, prefix, limit, |ch| word_char_for(ch, rel))
}

const CSS_PROPERTIES: &[&str] = &[
    "accent-color", "align-content", "align-items", "align-self", "all", "animation",
    "animation-delay", "animation-direction", "animation-duration", "animation-fill-mode",
    "animation-iteration-count", "animation-name", "animation-play-state",
    "animation-timing-function", "appearance", "aspect-ratio", "backdrop-filter",
    "backface-visibility", "background", "background-attachment", "background-clip",
    "background-color", "background-image", "background-origin", "background-position",
    "background-repeat", "background-size", "block-size", "border", "border-bottom",
    "border-bottom-color", "border-bottom-left-radius", "border-bottom-right-radius",
    "border-bottom-style", "border-bottom-width", "border-collapse", "border-color",
    "border-image", "border-left", "border-left-color", "border-left-style",
    "border-left-width", "border-radius", "border-right", "border-right-color",
    "border-right-style", "border-right-width", "border-spacing", "border-style",
    "border-top", "border-top-color", "border-top-left-radius",
    "border-top-right-radius", "border-top-style", "border-top-width", "border-width",
    "bottom", "box-shadow", "box-sizing", "break-after", "break-before", "break-inside",
    "caption-side", "caret-color", "clear", "clip", "clip-path", "color", "column-count",
    "column-gap", "column-rule", "column-rule-color", "column-rule-style",
    "column-rule-width", "column-span", "column-width", "columns", "contain", "content",
    "counter-increment", "counter-reset", "cursor", "direction", "display", "empty-cells",
    "filter", "flex", "flex-basis", "flex-direction", "flex-flow", "flex-grow",
    "flex-shrink", "flex-wrap", "float", "font", "font-family", "font-feature-settings",
    "font-kerning", "font-size", "font-stretch", "font-style", "font-variant",
    "font-weight", "gap", "grid", "grid-area", "grid-auto-columns", "grid-auto-flow",
    "grid-auto-rows", "grid-column", "grid-column-end", "grid-column-start", "grid-row",
    "grid-row-end", "grid-row-start", "grid-template", "grid-template-areas",
    "grid-template-columns", "grid-template-rows", "height", "hyphens", "inline-size",
    "inset", "inset-block", "inset-inline", "isolation", "justify-content",
    "justify-items", "justify-self", "left", "letter-spacing", "line-height",
    "list-style", "list-style-image", "list-style-position", "list-style-type", "margin",
    "margin-block", "margin-bottom", "margin-inline", "margin-left", "margin-right",
    "margin-top", "mask", "max-height", "max-width", "min-height", "min-width",
    "mix-blend-mode", "object-fit", "object-position", "opacity", "order", "outline",
    "outline-color", "outline-offset", "outline-style", "outline-width", "overflow",
    "overflow-wrap", "overflow-x", "overflow-y", "padding", "padding-block",
    "padding-bottom", "padding-inline", "padding-left", "padding-right", "padding-top",
    "perspective", "perspective-origin", "place-content", "place-items", "place-self",
    "pointer-events", "position", "quotes", "resize", "right", "rotate", "row-gap",
    "scale", "scroll-behavior", "scroll-margin", "scroll-padding", "scroll-snap-align",
    "scroll-snap-type", "table-layout", "text-align", "text-decoration",
    "text-decoration-color", "text-decoration-line", "text-decoration-style",
    "text-indent", "text-overflow", "text-rendering", "text-shadow", "text-transform",
    "top", "touch-action", "transform", "transform-origin", "transform-style",
    "transition", "transition-delay", "transition-duration", "transition-property",
    "transition-timing-function", "translate", "unicode-bidi", "user-select",
    "vertical-align", "visibility", "white-space", "width", "will-change", "word-break",
    "word-spacing", "word-wrap", "writing-mode", "z-index",
];

const CSS_AT_RULES: &[&str] = &[
    "charset", "container", "counter-style", "font-face", "font-feature-values",
    "import", "keyframes", "layer", "media", "namespace", "page", "property", "scope",
    "starting-style", "supports", "viewport",
];

const CSS_MATH_FUNCTIONS: &[&str] = &[
    "var(", "calc(", "min(", "max(", "clamp(", "env(", "attr(", "minmax(", "repeat(",
    "fit-content(",
];

const CSS_COLOR_FUNCTIONS: &[&str] = &[
    "var(", "calc(", "env(", "rgb(", "rgba(", "hsl(", "hsla(", "hwb(", "lab(", "lch(",
    "oklab(", "oklch(", "color(", "color-mix(",
];

const CSS_IMAGE_FUNCTIONS: &[&str] = &[
    "var(", "url(", "image-set(", "cross-fade(", "element(", "paint(",
    "linear-gradient(", "radial-gradient(", "conic-gradient(",
    "repeating-linear-gradient(", "repeating-radial-gradient(",
    "repeating-conic-gradient(", "counter(", "counters(",
];

const CSS_FILTER_FUNCTIONS: &[&str] = &[
    "var(", "blur(", "brightness(", "contrast(", "drop-shadow(", "grayscale(",
    "hue-rotate(", "invert(", "opacity(", "saturate(", "sepia(",
];

const CSS_TRANSFORM_FUNCTIONS: &[&str] = &[
    "var(", "matrix(", "matrix3d(", "perspective(", "rotate(", "rotate3d(", "scale(",
    "scale3d(", "skew(", "translate(", "translate3d(",
];

const CSS_EASING_FUNCTIONS: &[&str] = &[
    "var(", "cubic-bezier(", "steps(", "linear(",
];

const CSS_MEDIA_FEATURES: &[&str] = &[
    "width", "min-width", "max-width", "height", "min-height", "max-height",
    "aspect-ratio", "min-aspect-ratio", "max-aspect-ratio", "orientation",
    "resolution", "min-resolution", "max-resolution", "color", "color-gamut",
    "hover", "any-hover", "pointer", "any-pointer", "prefers-color-scheme",
    "prefers-reduced-motion", "prefers-contrast", "forced-colors", "display-mode",
    "scripting", "update", "overflow-block", "overflow-inline", "grid", "monochrome",
    "scan",
];

fn media_feature_values(feature: &str) -> &'static [&'static str] {
    match feature.trim().to_ascii_lowercase().as_str() {
        "orientation" => &["portrait", "landscape"],
        "hover" | "any-hover" => &["none", "hover"],
        "pointer" | "any-pointer" => &["none", "coarse", "fine"],
        "prefers-color-scheme" => &["light", "dark"],
        "prefers-reduced-motion" => &["no-preference", "reduce"],
        "prefers-contrast" => &["no-preference", "more", "less", "custom"],
        "forced-colors" => &["none", "active"],
        "display-mode" => &[
            "browser",
            "fullscreen",
            "standalone",
            "minimal-ui",
            "picture-in-picture",
        ],
        "scripting" => &["none", "initial-only", "enabled"],
        "update" => &["none", "slow", "fast"],
        "overflow-block" => &["none", "scroll", "paged"],
        "overflow-inline" => &["none", "scroll"],
        "color-gamut" => &["srgb", "p3", "rec2020"],
        "scan" => &["interlace", "progressive"],
        "grid" => &["0", "1"],
        _ => &[],
    }
}

fn css_conditional_candidates(before: &str) -> Option<Vec<String>> {
    if let Some(start) = before.rfind("@media")
        && !before[start..].contains('{')
        && let Some(open) = before[start..].rfind('(')
    {
        let segment = &before[start + open + 1..];
        if segment.contains(')') {
            return None;
        }
        if let Some(colon) = segment.rfind(':') {
            return Some(
                media_feature_values(segment[..colon].trim())
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect(),
            );
        }
        return Some(
            CSS_MEDIA_FEATURES
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        );
    }
    if let Some(start) = before.rfind("@supports")
        && !before[start..].contains('{')
        && let Some(open) = before[start..].rfind('(')
    {
        let segment = &before[start + open + 1..];
        if segment.contains(')') {
            return None;
        }
        if let Some(colon) = segment.rfind(':') {
            let property = segment[..colon].trim();
            let specific = css_property_values(property);
            let functions = css_property_functions(property);
            return Some(if specific.is_empty() {
                functions
                    .iter()
                    .chain(CSS_VALUES.iter())
                    .map(|item| (*item).to_owned())
                    .collect()
            } else {
                specific
                    .iter()
                    .chain(functions.iter())
                    .map(|item| (*item).to_owned())
                    .collect()
            });
        }
        return Some(
            CSS_PROPERTIES
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        );
    }
    None
}

fn css_custom_properties(text: &str) -> Vec<String> {
    let masked = crate::object_keys::mask_strings(&crate::semantic::mask_comments(text));
    let mut seen = std::collections::HashSet::new();
    let mut found = Vec::new();
    let mut index = 0usize;
    while let Some(offset) = masked[index..].find("--") {
        let start = index + offset;
        let boundary = masked[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '-')));
        let mut end = start + 2;
        while end < masked.len() {
            let ch = masked[end..].chars().next().unwrap_or('\0');
            if ch.is_alphanumeric() || matches!(ch, '_' | '-') {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        let name = &masked[start..end];
        if boundary
            && end > start + 2
            && masked[end..].trim_start().starts_with(':')
            && seen.insert(name.to_ascii_lowercase())
        {
            found.push(name.to_owned());
        }
        index = end.max(start + 2);
    }
    found
}

fn css_property_functions(property: &str) -> &'static [&'static str] {
    match property.to_ascii_lowercase().as_str() {
        "color" | "background-color" | "border-color" | "border-top-color"
        | "border-right-color" | "border-bottom-color" | "border-left-color"
        | "outline-color" | "text-decoration-color" | "caret-color" | "fill" | "stroke" => {
            CSS_COLOR_FUNCTIONS
        }
        "background" | "background-image" | "border-image" | "content" | "cursor"
        | "list-style-image" | "mask" => CSS_IMAGE_FUNCTIONS,
        "filter" | "backdrop-filter" => CSS_FILTER_FUNCTIONS,
        "transform" | "translate" | "scale" | "rotate" => CSS_TRANSFORM_FUNCTIONS,
        "animation-timing-function" | "transition-timing-function" => CSS_EASING_FUNCTIONS,
        _ => CSS_MATH_FUNCTIONS,
    }
}

const CSS_VALUES: &[&str] = &[
    "absolute", "all", "auto", "baseline", "block", "block-end", "block-start", "bold",
    "border-box", "both", "bottom", "break-word", "center", "clip", "collapse",
    "column", "column-reverse", "contain", "content-box", "cover", "crisp-edges",
    "currentColor", "dashed", "default", "disc", "dotted", "double", "ease", "ease-in",
    "ease-in-out", "ease-out", "end", "fill", "fixed", "flex", "flex-end", "flex-start",
    "grid", "groove", "hidden", "inherit", "initial", "inline", "inline-block",
    "inline-flex", "inline-grid", "inset", "italic", "justify", "left", "line-through",
    "linear", "lowercase", "max-content", "min-content", "none", "normal", "no-repeat",
    "nowrap", "oblique", "outset", "overlay", "pointer", "relative", "repeat",
    "repeat-x", "repeat-y", "reverse", "ridge", "right", "row", "row-reverse",
    "scale-down", "scroll", "solid", "space-around", "space-between", "space-evenly",
    "static", "sticky", "stretch", "text", "transparent", "underline", "unset",
    "uppercase", "visible", "wrap", "wrap-reverse",
];

fn css_property_values(property: &str) -> &'static [&'static str] {
    match property.to_ascii_lowercase().as_str() {
        "align-content" | "justify-content" => &[
            "normal", "start", "end", "left", "right", "center", "flex-start",
            "flex-end", "space-between", "space-around", "space-evenly", "stretch",
        ],
        "align-items" | "align-self" | "justify-items" | "justify-self" => &[
            "auto", "normal", "stretch", "start", "end", "center", "baseline",
            "flex-start", "flex-end", "self-start", "self-end",
        ],
        "animation-direction" => &["normal", "reverse", "alternate", "alternate-reverse"],
        "animation-fill-mode" => &["none", "forwards", "backwards", "both"],
        "background-repeat" => &["repeat", "repeat-x", "repeat-y", "no-repeat", "space", "round"],
        "background-size" => &["auto", "cover", "contain"],
        "border-style" | "border-top-style" | "border-right-style"
        | "border-bottom-style" | "border-left-style" | "outline-style" => &[
            "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge",
            "inset", "outset",
        ],
        "box-sizing" => &["content-box", "border-box"],
        "cursor" => &[
            "auto", "default", "pointer", "text", "move", "not-allowed", "wait", "help",
            "crosshair", "grab", "grabbing", "zoom-in", "zoom-out", "col-resize",
            "row-resize", "n-resize", "e-resize", "s-resize", "w-resize",
        ],
        "display" => &[
            "block", "inline", "inline-block", "flex", "inline-flex", "grid",
            "inline-grid", "none", "contents", "flow-root", "table", "table-cell",
            "table-row", "list-item",
        ],
        "flex-direction" => &["row", "row-reverse", "column", "column-reverse"],
        "flex-wrap" => &["nowrap", "wrap", "wrap-reverse"],
        "font-style" => &["normal", "italic", "oblique"],
        "font-weight" => &[
            "normal", "bold", "bolder", "lighter", "100", "200", "300", "400", "500",
            "600", "700", "800", "900",
        ],
        "grid-auto-flow" => &["row", "column", "dense", "row dense", "column dense"],
        "list-style" | "list-style-type" => &[
            "none", "disc", "circle", "square", "decimal", "lower-alpha", "upper-alpha",
            "lower-roman", "upper-roman", "lower-greek", "decimal-leading-zero",
        ],
        "mix-blend-mode" => &[
            "normal", "multiply", "screen", "overlay", "darken", "lighten",
            "color-dodge", "color-burn", "hard-light", "soft-light", "difference",
            "exclusion", "hue", "saturation", "color", "luminosity",
        ],
        "object-fit" => &["fill", "contain", "cover", "none", "scale-down"],
        "overflow" | "overflow-x" | "overflow-y" => &[
            "visible", "hidden", "clip", "scroll", "auto",
        ],
        "pointer-events" => &["auto", "none"],
        "position" => &["static", "relative", "absolute", "fixed", "sticky"],
        "resize" => &["none", "both", "horizontal", "vertical"],
        "text-align" => &["left", "right", "center", "justify", "start", "end"],
        "text-decoration" | "text-decoration-line" => &[
            "none", "underline", "overline", "line-through", "underline overline",
            "underline line-through",
        ],
        "text-transform" => &["none", "capitalize", "uppercase", "lowercase", "full-width"],
        "transition-timing-function" => &[
            "ease", "linear", "ease-in", "ease-out", "ease-in-out", "step-start",
            "step-end",
        ],
        "user-select" => &["auto", "text", "none", "contain", "all"],
        "visibility" => &["visible", "hidden", "collapse"],
        "white-space" => &[
            "normal", "nowrap", "pre", "pre-wrap", "pre-line", "break-spaces",
        ],
        "word-break" => &["normal", "break-all", "keep-all", "break-word"],
        "writing-mode" => &["horizontal-tb", "vertical-rl", "vertical-lr"],
        _ => &[],
    }
}

const HTML_TAGS: &[&str] = &[
    "a", "abbr", "address", "article", "aside", "audio", "b", "blockquote", "body", "br",
    "button", "canvas", "caption", "code", "col", "colgroup", "datalist", "dd", "del",
    "details", "dfn", "div", "dialog", "dl", "dt", "em", "embed", "fieldset",
    "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6",
    "head", "header", "html", "i", "iframe", "img", "input", "ins", "kbd", "label",
    "legend", "li", "link", "main", "map", "mark", "menu", "meta", "meter", "nav",
    "noscript", "object", "ol", "optgroup", "option", "output", "p", "picture", "pre",
    "progress", "q", "rp", "rt", "ruby", "s", "samp", "script", "section", "select",
    "small", "source", "span", "strong", "style", "sub", "summary", "sup", "svg",
    "table", "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "time",
    "title", "tr", "track", "u", "ul", "var", "video", "wbr",
];

const HTML_ATTRIBUTES: &[&str] = &[
    "accept", "action", "alt", "aria-hidden", "aria-label", "autocomplete", "autofocus",
    "checked", "class", "cols", "colspan", "content", "controls", "crossorigin",
    "datetime", "decoding", "default", "dir", "disabled", "download", "draggable",
    "enctype", "for", "form", "headers", "height", "hidden", "href", "id", "ismap",
    "lang", "loading", "loop", "max", "maxlength", "method", "min", "multiple",
    "muted", "name", "novalidate", "pattern", "placeholder", "poster", "preload",
    "readonly", "rel", "required", "role", "rows", "rowspan", "scope", "selected",
    "size", "spellcheck", "src", "srcset", "step", "style", "tabindex", "target",
    "title", "type", "usemap", "value", "width", "wrap",
];

const HTML_VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

fn html_tag_name(content: &str) -> Option<&str> {
    let content = content.trim_start_matches('/').trim_start();
    let name = content
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '/' | '>'))
        .next()?;
    (!name.is_empty()).then_some(name)
}

fn html_unclosed_tag(text: &str) -> Option<String> {
    let mut stack = Vec::<String>::new();
    let mut index = 0usize;
    while let Some(offset) = text[index..].find('<') {
        let start = index + offset;
        if text[start..].starts_with("<!--") {
            let Some(close) = text[start + 4..].find("-->") else {
                break;
            };
            index = start + 4 + close + 3;
            continue;
        }
        let Some(close) = text[start..].find('>') else {
            break;
        };
        let end = start + close;
        let content = &text[start + 1..end];
        if content.starts_with(['!', '?']) {
            index = end + 1;
            continue;
        }
        if content.starts_with('/') {
            let Some(name) = html_tag_name(content) else {
                index = end + 1;
                continue;
            };
            let name = name.to_ascii_lowercase();
            if let Some(position) = stack.iter().rposition(|tag| *tag == name) {
                stack.truncate(position);
            }
        } else if !content.ends_with('/')
            && let Some(name) = html_tag_name(content)
        {
            let name = name.to_ascii_lowercase();
            if !HTML_VOID_TAGS.contains(&name.as_str()) {
                stack.push(name);
            }
        }
        index = end + 1;
    }
    stack.pop()
}

fn html_attribute_values(attribute: &str) -> &'static [&'static str] {
    match attribute.to_ascii_lowercase().as_str() {
        "autocomplete" => &[
            "off", "on", "name", "email", "username", "new-password", "current-password",
            "one-time-code", "organization", "street-address", "address-line1",
            "address-line2", "address-level1", "address-level2", "postal-code",
            "country", "cc-name", "cc-number", "cc-exp", "cc-csc", "bday", "tel",
            "url", "photo",
        ],
        "contenteditable" => &["true", "false", "plaintext-only"],
        "crossorigin" => &["anonymous", "use-credentials"],
        "decoding" => &["sync", "async", "auto"],
        "dir" => &["ltr", "rtl", "auto"],
        "draggable" => &["true", "false"],
        "enctype" => &[
            "application/x-www-form-urlencoded",
            "multipart/form-data",
            "text/plain",
        ],
        "inputmode" => &["none", "text", "decimal", "numeric", "tel", "search", "email", "url"],
        "loading" => &["eager", "lazy"],
        "method" => &["get", "post", "dialog"],
        "rel" => &[
            "alternate", "author", "bookmark", "canonical", "external", "help", "icon",
            "license", "manifest", "modulepreload", "next", "nofollow", "noopener",
            "noreferrer", "opener", "pingback", "preconnect", "prefetch", "preload",
            "prerender", "prev", "search", "stylesheet", "tag",
        ],
        "scope" => &["row", "col", "rowgroup", "colgroup"],
        "shape" => &["rect", "circle", "poly", "default"],
        "spellcheck" => &["true", "false"],
        "target" => &["_self", "_blank", "_parent", "_top"],
        "translate" => &["yes", "no"],
        "type" => &[
            "button", "checkbox", "color", "date", "datetime-local", "email", "file",
            "hidden", "image", "month", "number", "password", "radio", "range", "reset",
            "search", "submit", "tel", "text", "time", "url", "week",
        ],
        "wrap" => &["soft", "hard"],
        _ => &[],
    }
}

fn html_attribute_value_context(content: &str) -> Option<&str> {
    let equals = content.rfind('=')?;
    let after = &content[equals + 1..];
    let inside = if let Some(rest) = after.strip_prefix('"') {
        !rest.contains('"')
    } else if let Some(rest) = after.strip_prefix('\'') {
        !rest.contains('\'')
    } else {
        !after.chars().any(char::is_whitespace)
    };
    if !inside {
        return None;
    }
    let before = content[..equals].trim_end();
    let attribute = before
        .rsplit(|ch: char| ch.is_whitespace() || ch == '<')
        .next()
        .unwrap_or_default();
    (!attribute.is_empty()).then_some(attribute)
}

fn semantic_candidates(text: &str, caret: usize, rel: &str) -> Vec<String> {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let caret = caret.min(text.len());
    let before = &text[..caret];
    match extension.as_str() {
        "css" | "scss" | "less" => {
            let statement = before
                .rsplit([';', '{', '}'])
                .next()
                .unwrap_or_default()
                .trim_start();
            if let Some(at_rule) = statement.strip_prefix('@')
                && !at_rule.contains(char::is_whitespace)
                && !at_rule.contains(':')
            {
                return CSS_AT_RULES
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect();
            }
            if let Some(items) = css_conditional_candidates(before) {
                return items;
            }
            let open = before.rfind('{').map_or(0, |index| index + 1);
            let close = before.rfind('}').map_or(0, |index| index + 1);
            if open <= close {
                return Vec::new();
            }
            let colon = before.rfind(':').map_or(0, |index| index + 1);
            let semicolon = before.rfind(';').map_or(0, |index| index + 1);
            let values = colon > semicolon && colon > open;
            let custom = css_custom_properties(text);
            if values {
                let property = before[..colon - 1]
                    .rsplit([';', '{'])
                    .next()
                    .unwrap_or_default()
                    .trim();
                let specific = css_property_values(property);
                let functions = css_property_functions(property);
                let mut candidates = if specific.is_empty() {
                    functions
                        .iter()
                        .chain(CSS_VALUES.iter())
                        .map(|item| (*item).to_owned())
                        .collect::<Vec<_>>()
                } else {
                    specific
                        .iter()
                        .chain(functions.iter())
                        .map(|item| (*item).to_owned())
                        .collect()
                };
                let in_var = before
                    .rfind("var(")
                    .is_some_and(|start| before.rfind(')').is_none_or(|close| start > close));
                if in_var {
                    custom.into_iter().chain(candidates).collect()
                } else {
                    candidates.extend(custom);
                    candidates
                }
            } else {
                let mut candidates = CSS_PROPERTIES
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect::<Vec<_>>();
                candidates.extend(custom);
                candidates
            }
        }
        "html" | "htm" | "xml" => {
            let Some(tag) = before.rfind('<') else {
                return Vec::new();
            };
            if before[tag..].contains('>') {
                if !before.ends_with('>') {
                    return Vec::new();
                }
                let content = &before[tag + 1..before.len() - 1];
                if content.starts_with(['/', '!', '?']) || content.ends_with('/') {
                    return Vec::new();
                }
                let Some(name) = html_tag_name(content) else {
                    return Vec::new();
                };
                if HTML_VOID_TAGS
                    .iter()
                    .any(|void| void.eq_ignore_ascii_case(name))
                {
                    return Vec::new();
                }
                return vec![format!("</{name}>")];
            }
            let raw = &before[tag + 1..];
            if raw.starts_with('/') && !raw.contains('>') {
                if let Some(unclosed) = html_unclosed_tag(&before[..tag]) {
                    return vec![format!("{unclosed}>")];
                }
                return HTML_TAGS
                    .iter()
                    .map(|item| format!("{item}>"))
                    .collect();
            }
            let content = raw.trim_start_matches('/');
            if let Some(attribute) = html_attribute_value_context(content) {
                return html_attribute_values(attribute)
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect();
            }
            let attributes = content.chars().any(char::is_whitespace);
            let source = if attributes { HTML_ATTRIBUTES } else { HTML_TAGS };
            source.iter().map(|item| (*item).to_owned()).collect()
        }
        _ => Vec::new(),
    }
}

/// Language-aware completion: CSS properties/values and HTML tags/attributes
/// from the bounded worker-equivalent sets, followed by document words.
pub(super) fn completions_at(
    text: &str,
    caret: usize,
    prefix: &str,
    limit: usize,
    rel: &str,
) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let prefix_lower = prefix.to_lowercase();
    let mut candidates = semantic_candidates(text, caret, rel);
    candidates.extend(completions_with(text, prefix, limit, |ch| {
        word_char_for(ch, rel)
    }));
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|item| seen.insert(item.to_lowercase()))
        .filter(|item| {
            prefix_lower.is_empty() || item.to_lowercase().starts_with(&prefix_lower)
        })
        .take(limit)
        .collect()
}

fn completions_with(
    text: &str,
    prefix: &str,
    limit: usize,
    word: impl Fn(char) -> bool,
) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let prefix_lower = prefix.to_lowercase();
    let mut words = HashMap::<String, (usize, String)>::new();
    for word in text.split(|ch: char| !word(ch)) {
        if word.chars().count() < 2 || word.chars().all(|ch| ch.is_numeric()) {
            continue;
        }
        let lower = word.to_lowercase();
        words
            .entry(lower)
            .and_modify(|(count, _)| *count += 1)
            .or_insert((1, word.to_owned()));
    }
    let mut ranked = words
        .into_iter()
        .filter_map(|(lower, (count, spelling))| {
            (prefix_lower.is_empty() || lower.starts_with(&prefix_lower))
                .then_some((count, spelling))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.len().cmp(&right.1.len()))
            .then_with(|| left.1.to_lowercase().cmp(&right.1.to_lowercase()))
            .then_with(|| left.1.cmp(&right.1))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, spelling)| spelling)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_and_html_semantic_completions_precede_document_words() {
        let css = ".x {\n  dis";
        let items = completions_at(css, css.len(), "dis", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("display"));

        let value = ".x {\n  display: f";
        let items = completions_at(value, value.len(), "f", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("flex"));

        let tag = "<di";
        let items = completions_at(tag, tag.len(), "di", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("div"));

        let attribute = "<div cla";
        let items = completions_at(attribute, attribute.len(), "cla", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("class"));
        assert_eq!(
            items.iter().filter(|item| *item == "class").count(),
            1,
            "semantic and document-word candidates deduplicate"
        );
    }

    #[test]
    fn css_value_functions_complete_before_generic_values() {
        for (source, prefix, expected) in [
            (".x { color: rg", "rg", "rgb("),
            (".x { width: ca", "ca", "calc("),
            (".x { background: lin", "lin", "linear-gradient("),
            (".x { filter: bl", "bl", "blur("),
            (".x { transform: tr", "tr", "translate("),
        ] {
            let items = completions_at(source, source.len(), prefix, 8, "x.css");
            assert_eq!(
                items.first().map(String::as_str),
                Some(expected),
                "{source}"
            );
        }

        let display = ".x { display: f";
        let items = completions_at(display, display.len(), "f", 8, "x.css");
        assert_eq!(
            items.first().map(String::as_str),
            Some("flex"),
            "property-specific keywords still precede functions"
        );
    }

    #[test]
    fn css_custom_properties_complete_from_the_current_document() {
        let text = ":root { --brand-color: #fff; --space: 1rem; }\n\
                    .x { /* --fake: no */ content: \"--string: no\"; }\n\
                    .y { color: var(--br";
        assert_eq!(
            css_custom_properties(text),
            vec!["--brand-color".to_owned(), "--space".to_owned()]
        );

        let items = completions_at(text, text.len(), "--br", 8, "x.css");
        assert_eq!(
            items.first().map(String::as_str),
            Some("--brand-color")
        );

        let var = ".x { --brand: red; --space: 1rem; color: var(";
        let items = completions_at(var, var.len(), "", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("--brand"));

        let property = ".x { --brand: red; --space: 1rem; --sp";
        let items = completions_at(property, property.len(), "--sp", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("--space"));
    }

    #[test]
    fn css_media_features_and_supports_conditions_complete() {
        for (source, prefix, expected) in [
            ("@media (min-w", "min-w", "min-width"),
            ("@media (orientation: l", "l", "landscape"),
            ("@media (prefers-color-scheme: d", "d", "dark"),
            ("@media (hover: n", "n", "none"),
            ("@supports (disp", "disp", "display"),
            ("@supports (display: f", "f", "flex"),
        ] {
            let items = completions_at(source, source.len(), prefix, 8, "x.css");
            assert_eq!(
                items.first().map(String::as_str),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn css_at_rule_completions_follow_the_statement_context() {
        for (source, prefix, expected) in [
            ("@med", "med", "media"),
            ("@sup", "sup", "supports"),
            ("@font-f", "font-f", "font-face"),
            ("@keyfr", "keyfr", "keyframes"),
            ("@start", "start", "starting-style"),
        ] {
            let items = completions_at(source, source.len(), prefix, 8, "x.css");
            assert_eq!(
                items.first().map(String::as_str),
                Some(expected),
                "{source}"
            );
        }

        let inside = "@media (min-width: 10";
        let items = completions_at(inside, inside.len(), "10", 8, "x.css");
        assert!(
            !items.iter().any(|item| item == "media"),
            "media feature values do not fall back to at-rule names"
        );

        let declaration = ".x { dis";
        let items = completions_at(declaration, declaration.len(), "dis", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("display"));
    }

    #[test]
    fn css_property_values_follow_the_declaration_name() {
        for (source, prefix, expected) in [
            (".x { display: f", "f", "flex"),
            (".x { position: r", "r", "relative"),
            (".x { cursor: po", "po", "pointer"),
            (".x { overflow: h", "h", "hidden"),
            (".x { font-weight: b", "b", "bold"),
            (".x { grid-auto-flow: c", "c", "column"),
            (".x { text-transform: up", "up", "uppercase"),
        ] {
            let items = completions_at(source, source.len(), prefix, 8, "x.css");
            assert_eq!(
                items.first().map(String::as_str),
                Some(expected),
                "{source}"
            );
        }

        let generic = ".x { color: tr";
        let items = completions_at(generic, generic.len(), "tr", 8, "x.css");
        assert_eq!(items.first().map(String::as_str), Some("transparent"));

        for (source, prefix, expected) in [
            (".x { aspec", "aspec", "aspect-ratio"),
            (".x { grid-auto-f", "grid-auto-f", "grid-auto-flow"),
            (".x { pointer-e", "pointer-e", "pointer-events"),
            (".x { scroll-snap-t", "scroll-snap-t", "scroll-snap-type"),
        ] {
            let items = completions_at(source, source.len(), prefix, 8, "x.css");
            assert_eq!(
                items.first().map(String::as_str),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn html_closing_tag_completion_uses_the_unclosed_ancestor() {
        let nested = "<section><div></di";
        let items = completions_at(nested, nested.len(), "di", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("div>"));

        let span = "<div><span></sp";
        let items = completions_at(span, span.len(), "sp", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("span>"));

        let self_closing = "<div/><span></sp";
        let items = completions_at(self_closing, self_closing.len(), "sp", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("span>"));

        let no_ancestor = "</";
        let items = completions_at(no_ancestor, no_ancestor.len(), "", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("a>"));

        let started = "<div>";
        let items = completions_at(started, started.len(), "", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("</div>"));

        let void = "<img>";
        let items = completions_at(void, void.len(), "", 8, "x.html");
        assert!(!items.iter().any(|item| item == "</img>"));

        let self_closing = "<div/>";
        let items = completions_at(self_closing, self_closing.len(), "", 8, "x.html");
        assert!(!items.iter().any(|item| item == "</div>"));
    }

    #[test]
    fn html_attribute_value_completions_follow_the_attribute_context() {
        let input_type = "<input type=\"te";
        let items = completions_at(input_type, input_type.len(), "te", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("tel"));
        assert!(items.iter().any(|item| item == "text"));

        let target = "<a target=\"_b";
        let items = completions_at(target, target.len(), "_b", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("_blank"));

        let rel = "<link rel=\"sty";
        let items = completions_at(rel, rel.len(), "sty", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("stylesheet"));

        let method = "<form method='p";
        let items = completions_at(method, method.len(), "p", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("post"));

        let complete = "<input type=\"text\"";
        let items = completions_at(complete, complete.len(), "ty", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("type"));

        let tag = "<inp";
        let items = completions_at(tag, tag.len(), "inp", 8, "x.html");
        assert_eq!(items.first().map(String::as_str), Some("input"));
    }

    #[test]
    fn language_word_ranges_keep_css_and_markup_punctuation() {
        assert_eq!(word_range_at_lang("background-color", 10, "x.css"), 0..16);
        assert_eq!(word_range_at_lang("background-color", 12, "x.ts"), 11..16);
        assert_eq!(word_range_at_lang("aria-label", 4, "x.html"), 0..10);
        assert_eq!(word_range_at_lang("$brand-color", 6, "x.scss"), 0..12);
        let text = "background-color: red; border-color: blue;";
        assert!(completions_lang(text, "background-", 8, "x.css").contains(&"background-color".into()));
        assert!(!completions(text, "background-", 8).contains(&"background-color".into()));
    }

    #[test]
    fn word_ranges_cover_the_whole_identifier_around_the_caret() {
        let text = "alpha beta_gamma delta";
        assert_eq!(word_range_at(text, 2), 0..5);
        assert_eq!(word_range_at(text, 0), 0..5);
        assert_eq!(word_range_at(text, 5), 0..5);
        assert_eq!(word_range_at(text, 6), 6..16);
        assert_eq!(word_range_at(text, 10), 6..16);
        assert_eq!(word_range_at(text, 16), 6..16);
        assert_eq!(word_range_at(text, 17), 17..22);
        assert_eq!(word_range_at("café au lait", 2), 0..5);
        assert_eq!(word_range_at("café au lait", 5), 0..5);
    }

    #[test]
    fn completion_filters_and_ranks_document_words() {
        let text = "alpha beta alpha gamma alphabet beta2 42 café";
        assert_eq!(
            completions(text, "al", MAX_COMPLETIONS),
            vec!["alpha".to_owned(), "alphabet".to_owned()]
        );
        assert_eq!(
            completions(text, "bet", MAX_COMPLETIONS),
            vec!["beta".to_owned(), "beta2".to_owned()]
        );
        assert_eq!(completions(text, "caf", MAX_COMPLETIONS), vec!["café"]);
        assert!(completions(text, "zzz", MAX_COMPLETIONS).is_empty());
        assert!(completions(text, "42", MAX_COMPLETIONS).is_empty());
        assert!(completions(text, "a", 0).is_empty());

        let all = completions(text, "", MAX_COMPLETIONS);
        assert_eq!(all[0], "alpha", "frequency wins for empty prefixes");
        assert!(all.contains(&"beta".to_owned()));
        assert_eq!(completions(text, "", 2).len(), 2);
    }

    #[test]
    fn duplicate_case_variants_keep_the_first_spelling_and_count() {
        let text = "Value value VALUE value";
        let items = completions(text, "val", MAX_COMPLETIONS);
        assert_eq!(items, vec!["Value".to_owned()]);
    }
}
