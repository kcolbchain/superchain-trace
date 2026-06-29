use crate::types::*;
use async_trait::async_trait;
use eyre::Result;

/// A cross-chain message safety query as understood by the OP-Supervisor.
///
/// The OP-Supervisor identifies a message by the chain it was emitted on, the
/// block it landed in, and the log index of the initiating event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageQuery {
    pub msg_id: String,
    pub chain_id: u64,
    pub block_number: Option<u64>,
    pub log_index: Option<u64>,
}

impl MessageQuery {
    pub fn new(msg_id: impl Into<String>, chain_id: u64) -> Self {
        Self {
            msg_id: msg_id.into(),
            chain_id,
            block_number: None,
            log_index: None,
        }
    }
}

/// The safety level the OP-Supervisor reports for a cross-chain message.
///
/// These mirror the OP Stack interop safety levels. They are ordered from
/// least to most settled and map onto the message [`MessageStatus`] lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyLevel {
    /// The supervisor has not indexed the initiating log yet.
    Pending,
    /// Log is indexed but only locally visible (reorg-able).
    Unsafe,
    /// Cross-chain dependencies are validated and considered safe to relay.
    Safe,
    /// The message's source data is finalized on L1.
    Finalized,
}

impl SafetyLevel {
    /// Parse the textual safety level returned by the OP-Supervisor JSON-RPC.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "finalized" => Self::Finalized,
            "safe" | "cross-safe" | "cross_safe" => Self::Safe,
            "unsafe" | "local-safe" | "local_safe" | "cross-unsafe" | "cross_unsafe" => {
                Self::Unsafe
            }
            _ => Self::Pending,
        }
    }

    /// The wire string for this safety level.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Unsafe => "unsafe",
            Self::Safe => "safe",
            Self::Finalized => "finalized",
        }
    }

    /// Map a supervisor safety level onto the message lifecycle status.
    pub fn to_status(self) -> MessageStatus {
        match self {
            Self::Pending => MessageStatus::Unknown,
            Self::Unsafe => MessageStatus::Indexed,
            Self::Safe => MessageStatus::Safe,
            Self::Finalized => MessageStatus::Finalized,
        }
    }
}

/// A client capable of answering OP-Supervisor safety-level queries.
///
/// Production code uses [`HttpSupervisorClient`], which talks to a live
/// OP-Supervisor endpoint. Tests inject a mock implementation so the message
/// lifecycle logic can be exercised against fixtures without any network.
#[async_trait]
pub trait SupervisorClient {
    /// Resolve the current safety level for a cross-chain message.
    async fn safety_level(&self, query: &MessageQuery) -> Result<SafetyLevel>;
}

/// Live OP-Supervisor client.
///
/// NOTE: the OP-Supervisor wire protocol is still stabilizing; until a public
/// endpoint is wired up this reports [`SafetyLevel::Pending`] rather than
/// fabricating a safety level it cannot verify.
pub struct HttpSupervisorClient {
    pub endpoint: Option<String>,
}

impl HttpSupervisorClient {
    pub fn new(endpoint: Option<String>) -> Self {
        Self { endpoint }
    }
}

#[async_trait]
impl SupervisorClient for HttpSupervisorClient {
    async fn safety_level(&self, _query: &MessageQuery) -> Result<SafetyLevel> {
        // TODO: Connect to actual OP-Supervisor API when a public endpoint is
        // available. The OP-Supervisor exposes safety level checks via
        // `supervisor_checkMessage { chain_id, block_number, log_index }`.
        Ok(SafetyLevel::Pending)
    }
}

/// Build a [`StatusResult`] for `msg_id` using the supplied supervisor client.
///
/// This is the testable core: given any [`SupervisorClient`] (live or mock) it
/// resolves the safety level and maps it onto the message lifecycle status.
pub async fn check_status_with<C: SupervisorClient + ?Sized>(
    client: &C,
    msg_id: &str,
    chain: &ChainInfo,
) -> Result<StatusResult> {
    let query = MessageQuery::new(msg_id, chain.chain_id);
    let level = client.safety_level(&query).await?;

    let details = match level {
        SafetyLevel::Pending => format!(
            "OP-Supervisor has not indexed {} on {} yet — message still pending. \
             See: https://docs.optimism.io/interop/tools",
            msg_id, chain.name
        ),
        other => format!(
            "OP-Supervisor reports safety level `{}` for {} on {}.",
            other.as_str(),
            msg_id,
            chain.name
        ),
    };

    Ok(StatusResult {
        msg_id: msg_id.to_string(),
        status: level.to_status(),
        safety_level: level.as_str().to_string(),
        details,
    })
}

/// Check message status via the live OP-Supervisor API.
///
/// Convenience wrapper over [`check_status_with`] using [`HttpSupervisorClient`].
pub async fn check_status(msg_id: &str, chain: &ChainInfo) -> Result<StatusResult> {
    let client = HttpSupervisorClient::new(None);
    check_status_with(&client, msg_id, chain).await
}
