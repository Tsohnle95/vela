//! Vela Native's GPUI presentation layer.

mod app;
mod auto_import;
mod browser_downloads;
mod brackets;
mod calls;
mod changes_view;
mod class_members;
mod code_actions;
mod completion;
mod destructuring;
mod diagnostics;
mod editor;
mod editor_groups;
mod emmet;
mod enums;
mod export_duplicates;
mod focus;
mod formatting;
mod highlights;
mod hover;
mod icons;
mod import_duplicates;
#[cfg(test)]
mod inline_code;
mod input;
mod math;
mod member_access;
mod interfaces;
mod multiple_selection;
mod object_keys;
mod object_types;
mod panel_layout;
mod panels;
mod pixel_art;
mod platform_menu;
mod platform_prompt;
mod platform_quit;
mod preformatted;
mod provider_oauth;
mod provider_usage;
mod provider_validation;
mod providers;
mod rename;
mod scroll;
mod semantic;
mod settings;
mod signature;
mod slash_args;
mod subagent;
mod surface;
mod switch_cases;
mod symbols;
mod syntax;
pub mod theme;
mod toasts;
mod tree_icons;
mod transcript_viewport;
mod type_literals;
mod type_members;
mod type_parameters;
mod type_unions;
mod typography;
mod window_state;

pub use app::run;
#[cfg(feature = "transcript-bench")]
pub use app::transcript_native_bench::{LiveProbe, run_native_transcript_bench};

mod turn_anchor;


#[cfg(test)]
mod build_profile_tests {
    /// Interactive builds must stay optimized: an unoptimized GPUI frame costs
    /// ~15x more, which shows up as lag while scrolling the editor.
    #[test]
    fn dev_profile_stays_optimized() {
        let manifest = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../Cargo.toml"
        ))
        .expect("workspace manifest");
        let mut in_dev = false;
        let mut opt_level = None;
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_dev = line == "[profile.dev]";
                continue;
            }
            if in_dev
                && let Some(value) = line.strip_prefix("opt-level")
                && let Some(value) = value.split('=').nth(1)
            {
                opt_level = value.trim().parse::<u8>().ok();
            }
        }
        let level = opt_level.expect("[profile.dev] opt-level must be set for interactive builds");
        assert!(
            level >= 1,
            "dev builds run the real frame loop; opt-level {level} reintroduces input lag"
        );
    }
}
