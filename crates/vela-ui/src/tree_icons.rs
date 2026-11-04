//! Static reference glyphs, filename precedence and palette. Geometry comes
//! from FileIcons/TreeFileGlyph; these are vector assets, not font substitutes.
use gpui::{SharedString, Svg, prelude::*};
use serde_json::Value;
use std::sync::OnceLock;

fn catalog() -> &'static Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/tree-icons.json"))
            .expect("bundled reference file icons")
    })
}

pub(crate) fn asset(path: &str) -> Option<&'static [u8]> {
    if let Some(name) = path
        .strip_prefix("file-tabs/")
        .and_then(|name| name.strip_suffix(".svg"))
    {
        return catalog()["tab_glyphs"][name].as_str().map(str::as_bytes);
    }
    let name = path.strip_prefix("tree-icons/")?.strip_suffix(".svg")?;
    catalog()["glyphs"][name].as_str().map(str::as_bytes)
}

pub(crate) fn paths() -> Vec<SharedString> {
    catalog()["glyphs"]
        .as_object()
        .expect("reference glyph map")
        .keys()
        .map(|name| format!("tree-icons/{name}.svg").into())
        .chain(
            catalog()["tab_glyphs"]
                .as_object()
                .expect("reference tab glyphs")
                .keys()
                .map(|name| SharedString::from(format!("file-tabs/{name}.svg"))),
        )
        .collect()
}

fn name(file: &str) -> &'static str {
    let base = file.rsplit('/').next().unwrap_or(file).to_lowercase();
    let catalog = catalog();
    if let Some(name) = catalog["exact"][&base].as_str() {
        return name;
    }
    for rule in catalog["contains"]
        .as_array()
        .expect("reference filename rules")
    {
        if base.contains(rule[0].as_str().expect("filename needle")) {
            return rule[1].as_str().expect("filename glyph");
        }
    }
    let extension = base.rsplit('.').next().unwrap_or(&base);
    catalog["extensions"][extension]
        .as_str()
        .unwrap_or("default")
}

pub(crate) fn glyph(file: &str) -> Svg {
    let name = name(file);
    let palette = catalog()["colors"][name].as_str().unwrap_or("gray");
    let color = catalog()["palette"][palette]
        .as_str()
        .expect("icon palette color");
    gpui::svg()
        .path(format!("tree-icons/{name}.svg"))
        .size(gpui::px(16.))
        .flex_shrink_0()
        .text_color(crate::theme::color(color))
}

pub(crate) fn tab_glyph(file: &str) -> Svg {
    let base = file.rsplit('/').next().unwrap_or(file).to_lowercase();
    let extension = base.rsplit('.').next().unwrap_or(&base);
    let kind = catalog()["tab_rules"][extension]
        .as_str()
        .or_else(|| catalog()["tab_rules"][&base].as_str())
        .unwrap_or("generic");
    gpui::svg()
        .path(format!("file-tabs/{kind}.svg"))
        .size(gpui::px(13.))
        .flex_shrink_0()
        .text_color(crate::theme::Theme::default().faint)
}
