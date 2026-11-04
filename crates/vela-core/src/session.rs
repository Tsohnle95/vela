use serde::{Deserialize, Serialize};

/// The only runtime Vela Native enables today. Kept explicit so provider
/// adapters can be added without leaking runtime details into the transcript.
pub const OPENCODE_RUNTIME_ID: &str = "opencode";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub directory: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
}

impl SessionSummary {
    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            "Untitled session"
        } else {
            &self.title
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionPhase {
    #[default]
    Idle,
    Busy,
    Retry,
    Error,
}

impl SessionPhase {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Busy | Self::Retry)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionUsage {
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub reasoning_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
}

impl SessionUsage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.output_tokens)
            .saturating_add(self.reasoning_tokens)
            .saturating_add(self.cache_read_tokens)
            .saturating_add(self.cache_write_tokens)
    }
}
