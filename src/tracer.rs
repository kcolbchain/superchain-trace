use crate::types::*;
use eyre::{eyre, Result};
use reqwest::Client;
use serde_json::{json, Value};

/// L2ToL2CrossDomainMessenger event topic (SentMessage)
const SENT_MESSAGE_TOPIC: &str =
    "0x382409ac69001e11931a28435afef442cbfd20d9891907e8fa373ba7d351f320";

pub async fn trace_message(tx_hash: &str, chain: &ChainInfo) -> Result<TraceResult> {
    let client = Client::new();

    // Fetch transaction receipt
    let receipt = fetch_receipt(&client, &chain.rpc_url, tx_hash).await?;

    let block_hex = receipt["blockNumber"]
        .as_str()
        .ok_or_else(|| eyre!("No block number in receipt"))?;
    let block_number = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16)?;
    let status_code = receipt["status"].as_str().unwrap_or("0x1");
    let from = receipt["from"].as_str().unwrap_or("unknown").to_string();
    let gas_str = receipt["gasUsed"].as_str().unwrap_or("0x0");
    let gas_used = u64::from_str_radix(gas_str.trim_start_matches("0x"), 16).ok();

    // Look for cross-chain message logs
    let logs = receipt["logs"].as_array().ok_or_else(|| eyre!("No logs"))?;
    let cross_chain_log = logs.iter().find(|log| {
        log["topics"]
            .as_array()
            .map(|topics| {
                topics.first().and_then(|t| t.as_str()) == Some(SENT_MESSAGE_TOPIC)
            })
            .unwrap_or(false)
    });

    let mut lifecycle = vec![LifecycleStep {
        status: MessageStatus::Initiated,
        description: format!("Transaction submitted on {}", chain.name),
        tx_hash: Some(tx_hash.to_string()),
        chain: chain.name.clone(),
        timestamp: None,
    }];

    let (dest_chain, target, nonce) = if let Some(log) = cross_chain_log {
        lifecycle.push(LifecycleStep {
            status: MessageStatus::Indexed,
            description: "Cross-chain message detected in logs".into(),
            tx_hash: None,
            chain: chain.name.clone(),
            timestamp: None,
        });

        // Decode destination from log topics (topic[1] = destination chain ID)
        let topics = log["topics"].as_array().unwrap();
        let dest_chain_id = if topics.len() > 1 {
            let hex = topics[1].as_str().unwrap_or("0x0");
            u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
        } else {
            0
        };

        let dest_name = crate::chains::all_chains()
            .iter()
            .find(|c| c.chain_id == dest_chain_id)
            .map(|c| c.name.clone());

        (dest_name, None, None)
    } else {
        if status_code == "0x1" {
            lifecycle.push(LifecycleStep {
                status: MessageStatus::Unknown,
                description: "No cross-chain message event found. This may be a regular transaction.".into(),
                tx_hash: None,
                chain: chain.name.clone(),
                timestamp: None,
            });
        }
        (None, None, None)
    };

    let message = CrossChainMessage {
        source_chain: chain.name.clone(),
        dest_chain,
        source_tx: tx_hash.to_string(),
        dest_tx: None,
        sender: from,
        target,
        nonce,
        status: if cross_chain_log.is_some() {
            MessageStatus::Indexed
        } else {
            MessageStatus::Initiated
        },
        block_number,
        timestamp: 0,
        gas_used,
        error: if status_code == "0x0" {
            Some("Transaction reverted".into())
        } else {
            None
        },
    };

    Ok(TraceResult { message, lifecycle })
}

async fn fetch_receipt(client: &Client, rpc_url: &str, tx_hash: &str) -> Result<Value> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    resp["result"]
        .as_object()
        .map(|_| resp["result"].clone())
        .ok_or_else(|| eyre!("Transaction not found: {}", tx_hash))
}
