//! Vela's application engine.
//!
//! The engine owns the OpenCode connection, session registry, transcript
//! projection, warm-session retention, permissions, and durable local settings.
//! It publishes small change notifications; the UI reads immutable snapshots
//! and never performs network, filesystem, or agent work on the render thread.

pub mod provider_usage;
pub mod engine;
pub mod changes;
pub mod detach;
pub mod files;
mod file_workspace;
pub mod git;
pub mod imports;
mod legacy_pins;
pub mod message_queue;
pub mod migration;
pub mod observation;
pub mod persist;
pub mod preview;
pub mod provider_oauth;
pub mod recovery;
pub mod runtime;
mod sass_state;
mod sass_watch;
mod sass_preferences;
mod sass_sources;
mod sass_output;
mod sass_compiler;
pub mod store;
mod terminal_relay;
pub mod trash;
pub mod validation;
pub mod window_state;

pub use engine::{
    ConnectionState, Engine, EngineConfig, EngineUpdate, ProviderCredentialRemoval,
    ProviderKeyConnection, SassLine, SassLogLevel, TerminalCommand,
};
pub use files::FileEntry;
pub use file_workspace::FileWorkspace;
pub use git::GitEntry;
pub use persist::Settings;
pub use provider_oauth::{ProviderOAuthRequest, ProviderOAuthState};
pub use store::{SessionView, Store};
pub use sass_state::{SassStatus, SassWatchState};
