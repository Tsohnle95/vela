//! Palette data copied from Vela's `appearances.ts`, keeping the original IDs.
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Debug, Deserialize)]
pub struct Appearance {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scheme: String,
    pub colors: HashMap<String, String>,
}

pub fn profiles() -> &'static [Appearance] {
    static PROFILES: OnceLock<Vec<Appearance>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        serde_json::from_str(include_str!("appearances.json")).expect("bundled Vela palettes")
    })
}
pub fn profile(id: &str) -> &'static Appearance {
    profiles()
        .iter()
        .find(|p| p.id == id)
        .unwrap_or(&profiles()[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_all_original_palettes_and_required_tokens() {
        assert_eq!(profiles().len(), 15);
        let mut ids = std::collections::HashSet::new();
        for profile in profiles() {
            assert!(ids.insert(&profile.id));
            for key in [
                "base",
                "top",
                "pane",
                "editor",
                "agent",
                "rail",
                "tab",
                "console",
                "ink",
                "muted",
                "faint",
                "line",
                "lineStrong",
                "accent",
                "good",
                "danger",
                "compose",
                "user",
                "card",
                "sendInk",
            ] {
                assert!(!profile.colors[key].is_empty(), "{}: {key}", profile.id);
            }
        }
        assert_eq!(profile("codex-dark").name, "Midnight");
        assert_eq!(profile("unknown-old-setting").id, "codex");
    }
}
