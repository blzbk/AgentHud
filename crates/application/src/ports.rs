//! Ports the use cases need, implemented by the adapters.

use std::path::PathBuf;

use domain::{AgentSession, Provider, UsageSnapshot};

/// Why a source could not provide its usage.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Gives the latest known usage of one provider.
pub trait UsageSource: Send + Sync {
    fn provider(&self) -> Provider;

    /// Most recent usage, or `None` when the provider left no usage data.
    ///
    /// # Errors
    ///
    /// Fails when the usage data exists but cannot be read.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError>;
}

/// Lists the sessions of one provider's agents.
pub trait AgentStateSource: Send + Sync {
    fn provider(&self) -> Provider;

    /// Sessions written recently, in no particular order. An empty list
    /// means no agent of this provider is running.
    ///
    /// # Errors
    ///
    /// Fails when the sessions exist but cannot be listed.
    fn sessions(&self) -> Result<Vec<AgentSession>, SourceError>;
}
