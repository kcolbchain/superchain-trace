//! Common test helpers and fixtures for cross-chain integration tests.

use serde_json::json;

use crate::supersim;

/// L2ToL2CrossDomainMessenger predeploy address (OP Stack interop).
pub const L2_TO_L2_CROSS_DOMAIN_MESSENGER: &str = "0x4200000000000000000000000000000000000023";

/// CrossL2Inbox predeploy address.
pub const CROSS_L2_INBOX: &str = "0x4200000000000000000000000000000000000022";

/// SentMessage(uint256 destination, address target, uint256 messageNonce, address sender, bytes message)
pub const SENT_MESSAGE_TOPIC: &str =
    "0x382409ac69001e11931a28435afef442cbfd20d9891907e8fa373ba7d351f320";

// ---------------------------------------------------------------------------
// Transaction helpers
// ---------------------------------------------------------------------------

/// Send a simple ETH transfer and return the tx hash.
pub async fn send_eth_transfer(
    rpc_url: &str,
    from: &str,
    to: &str,
    value_hex: &str,
) -> eyre::Result<String> {
    let tx = json!({
        "from": from,
        "to": to,
        "value": value_hex,
        "gas": "0x5208",
    });

    let result = supersim::rpc_request(rpc_url, "eth_sendTransaction", json!([tx])).await?;
    result
        .as_str()
        .map(String::from)
        .ok_or_else(|| eyre::eyre!("eth_sendTransaction did not return a tx hash"))
}

/// Wait for a transaction receipt with a timeout.
pub async fn wait_for_receipt(
    rpc_url: &str,
    tx_hash: &str,
    timeout_secs: u64,
) -> eyre::Result<serde_json::Value> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return Err(eyre::eyre!(
                "Timeout waiting for receipt of tx {}",
                tx_hash
            ));
        }

        let result = supersim::rpc_request(
            rpc_url,
            "eth_getTransactionReceipt",
            json!([tx_hash]),
        )
        .await?;

        if !result.is_null() {
            return Ok(result);
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Send a cross-chain message via the L2ToL2CrossDomainMessenger.
///
/// Calls `sendMessage(uint256 _destination, address _target, bytes _message)`.
/// Returns the transaction hash on the source chain.
pub async fn send_cross_chain_message(
    source_rpc: &str,
    destination_chain_id: u64,
    target_address: &str,
    message_data: &[u8],
) -> eyre::Result<String> {
    // Encode: sendMessage(uint256,address,bytes)
    // selector = keccak256("sendMessage(uint256,address,bytes)")[..4]
    let selector = "0x7056f41f";

    // ABI-encode parameters (simplified — uses zero-padded encoding)
    let dest_hex = format!("{:064x}", destination_chain_id);
    let target_clean = target_address.trim_start_matches("0x");
    let target_padded = format!("{:0>64}", target_clean);
    // bytes offset (3 * 32 = 96 = 0x60)
    let offset = format!("{:064x}", 96u64);
    // bytes length
    let data_len = format!("{:064x}", message_data.len());
    // bytes data, padded to 32 bytes
    let data_hex = hex::encode(message_data);
    let padded_len = ((message_data.len() + 31) / 32) * 64;
    let data_padded = format!("{:0<width$}", data_hex, width = padded_len);

    let calldata = format!(
        "{}{}{}{}{}{}",
        selector, dest_hex, target_padded, offset, data_len, data_padded,
    );

    let tx = json!({
        "from": supersim::DEV_ADDRESS,
        "to": L2_TO_L2_CROSS_DOMAIN_MESSENGER,
        "data": calldata,
        "gas": "0x30000",
    });

    let result = supersim::rpc_request(source_rpc, "eth_sendTransaction", json!([tx])).await?;
    result
        .as_str()
        .map(String::from)
        .ok_or_else(|| eyre::eyre!("sendMessage did not return a tx hash"))
}

/// Check whether a transaction receipt contains a SentMessage log.
pub fn receipt_has_sent_message_log(receipt: &serde_json::Value) -> bool {
    receipt["logs"]
        .as_array()
        .map(|logs| {
            logs.iter().any(|log| {
                log["topics"]
                    .as_array()
                    .and_then(|t| t.first())
                    .and_then(|t| t.as_str())
                    .map(|t| t == SENT_MESSAGE_TOPIC)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Extract the destination chain ID from a SentMessage log.
pub fn extract_destination_chain_id(receipt: &serde_json::Value) -> Option<u64> {
    receipt["logs"].as_array().and_then(|logs| {
        logs.iter().find_map(|log| {
            let topics = log["topics"].as_array()?;
            let first = topics.first()?.as_str()?;
            if first != SENT_MESSAGE_TOPIC {
                return None;
            }
            let dest_topic = topics.get(1)?.as_str()?;
            u64::from_str_radix(dest_topic.trim_start_matches("0x"), 16).ok()
        })
    })
}

/// Build a `ChainInfo`-compatible struct for a supersim chain (for use with the
/// library's `trace_message`).
pub fn supersim_chain_info(
    name: &str,
    chain_id: u64,
    rpc_url: &str,
) -> serde_json::Value {
    json!({
        "name": name,
        "chain_id": chain_id,
        "rpc_url": rpc_url,
        "explorer_url": format!("http://localhost:0/{}", name),
    })
}
