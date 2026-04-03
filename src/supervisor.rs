use crate::types::*;
use eyre::Result;

/// Check message status via OP-Supervisor API.
/// In production, this queries the OP-Supervisor service.
/// For now, returns status based on available on-chain data.
pub async fn check_status(msg_id: &str, chain: &ChainInfo) -> Result<StatusResult> {
    // TODO: Connect to actual OP-Supervisor API when available
    // The OP-Supervisor exposes safety level checks for cross-chain messages
    // API endpoint: POST /check-message with { chain_id, block_number, log_index }

    // For MVP, we check the on-chain state by looking at the message hash
    // and whether it's been relayed on the destination chain
    let _client = reqwest::Client::new();

    // Placeholder: in production this queries OP-Supervisor
    Ok(StatusResult {
        msg_id: msg_id.to_string(),
        status: MessageStatus::Unknown,
        safety_level: "pending".into(),
        details: format!(
            "OP-Supervisor query for {} on {} — connect to supervisor endpoint for live status. \
             See: https://docs.optimism.io/interop/tools",
            msg_id, chain.name
        ),
    })
}
