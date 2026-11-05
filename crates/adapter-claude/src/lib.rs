//! Claude Code adapter: reads the files under `~/.claude` and implements the
//! `application` ports for the Claude provider.

mod agent_sessions;
mod running_sessions;
mod transcript;
mod usage_block;
mod usage_source;

pub use agent_sessions::ClaudeAgentSessions;
pub use usage_source::ClaudeUsageSource;

/// Folder of the Claude Code data, under each home.
pub const CLAUDE_DIR: &str = ".claude";

const PROJECTS_DIR: &str = "projects";
const TRANSCRIPT_EXTENSION: &str = "jsonl";
