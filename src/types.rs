use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub explorer_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    Initiated, // Tx sent on source chain
    Indexed,   // OP-Supervisor has indexed the log
    Safe,      // Message reached safe safety level
    Finalized, // Message is finalized on L1
    Executed,  // Relayed and executed on destination chain
    Failed,    // Execution failed on destination
    Unknown,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initiated => write!(f, "INITIATED"),
            Self::Indexed => write!(f, "INDEXED"),
            Self::Safe => write!(f, "SAFE"),
            Self::Finalized => write!(f, "FINALIZED"),
            Self::Executed => write!(f, "EXECUTED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainMessage {
    pub source_chain: String,
    pub dest_chain: Option<String>,
    pub source_tx: String,
    pub dest_tx: Option<String>,
    pub sender: String,
    pub target: Option<String>,
    pub nonce: Option<u64>,
    pub status: MessageStatus,
    pub block_number: u64,
    pub timestamp: u64,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    pub message: CrossChainMessage,
    pub lifecycle: Vec<LifecycleStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleStep {
    pub status: MessageStatus,
    pub description: String,
    pub tx_hash: Option<String>,
    pub chain: String,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    pub msg_id: String,
    pub status: MessageStatus,
    pub safety_level: String,
    pub details: String,
}
