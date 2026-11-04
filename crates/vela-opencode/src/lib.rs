//! Native OpenCode V2 adapter.
//!
//! The adapter speaks OpenCode's documented HTTP/SSE surface directly instead
//! of embedding a Node client. It owns service discovery, lifecycle, the
//! authenticated client, and a reconnecting, bounded event stream. It knows
//! nothing about GPUI.

pub mod inventory;
pub mod client;
pub mod commands;
pub mod forms;
pub mod providers;
pub mod service;
pub mod stream;

pub use client::{
    AgentInfo, Client, InboxItem, ModelInfo, OpencodeError, PermissionReply, PromptFile,
    PromptMention,
};
pub use commands::{CommandKind, CommandOption};
pub use forms::{FormField, FormKind, FormOption, FormRequest};
pub use service::{Endpoint, discover, ensure, service_file};
pub use stream::spawn_event_stream;
