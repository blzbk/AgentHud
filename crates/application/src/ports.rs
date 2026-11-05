//! Ports the use cases need, implemented by the adapters.

use std::path::PathBuf;
use std::sync::Arc;

use domain::{AgentSession, EpochSeconds, Provider, UsageSnapshot};

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

impl<T: UsageSource + ?Sized> UsageSource for Arc<T> {
    fn provider(&self) -> Provider {
        (**self).provider()
    }

    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        (**self).latest()
    }
}

/// Why a remote source gave no usage. Messages never carry a credential.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// The login of the source has expired or was refused.
    #[error("login expired")]
    AuthExpired,
    /// Too many requests; `retry_after` is the delay asked for, in seconds.
    #[error("rate limited")]
    RateLimited { retry_after: Option<EpochSeconds> },
    /// Any other failure: missing tool, network, timeout, unreadable answer.
    #[error("unavailable: {0}")]
    Unavailable(String),
}

/// Asks a remote source (process or network) for the current usage of one
/// provider. Slow: called on a schedule, never on each refresh.
pub trait UsageFetcher: Send + Sync {
    fn provider(&self) -> Provider;

    /// Current usage, observed at `now`.
    ///
    /// # Errors
    ///
    /// Fails when the source cannot tell the usage.
    fn fetch(&self, now: EpochSeconds) -> Result<UsageSnapshot, FetchError>;
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
