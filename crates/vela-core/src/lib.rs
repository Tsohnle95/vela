//! Framework-independent Vela domain.
//!
//! This crate deliberately has no GPUI, network, or platform dependency. It
//! owns the transcript projection, event normalization, markdown block
//! splitting, row-layout arithmetic, and the bounded warm-session cache. Those
//! are the pieces that most directly determine navigation and streaming cost,
//! so they are unit-testable without a display server.

pub mod event;
pub mod layout;
pub mod markdown;
pub mod session;
pub mod transcript;
pub mod warm;

pub use event::{AgentEvent, EventEnvelope, ExecutionOutcome, PermissionRequest, TodoItem};
pub use layout::LayoutCache;
pub use session::{SessionPhase, SessionSummary, SessionUsage};
pub use transcript::{Block, BlockKind, Message, Part, PartKind, ToolPart, ToolStatus, Transcript};
pub use warm::{WarmCache, WarmEntry};
pub mod appearance;
pub mod document;
pub mod text;
pub mod search;

pub mod tray;
