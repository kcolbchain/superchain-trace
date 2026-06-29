use superchain_trace::{tracer, types::ChainInfo};

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn supersim_chain() -> ChainInfo {
    ChainInfo {
        name: env_or_default("SUPERSIM_CHAIN_NAME", "supersim-op"),
        chain_id: std::env::var("SUPERSIM_CHAIN_ID")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(901),
        rpc_url: env_or_default("SUPERSIM_RPC_URL", "http://127.0.0.1:9545"),
        explorer_url: String::new(),
    }
}

fn supersim_l2_chain() -> ChainInfo {
    ChainInfo {
        name: env_or_default("SUPERSIM_L2_NAME", "supersim-l2"),
        chain_id: std::env::var("SUPERSIM_L2_CHAIN_ID")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(902),
        rpc_url: env_or_default("SUPERSIM_L2_RPC_URL", "http://127.0.0.1:9546"),
        explorer_url: String::new(),
    }
}

fn supersim_l3_chain() -> ChainInfo {
    ChainInfo {
        name: env_or_default("SUPERSIM_L3_NAME", "supersim-l3"),
        chain_id: std::env::var("SUPERSIM_L3_CHAIN_ID")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(903),
        rpc_url: env_or_default("SUPERSIM_L3_RPC_URL", "http://127.0.0.1:9547"),
        explorer_url: String::new(),
    }
}

#[tokio::test]
#[ignore = "requires a running ethereum-optimism/supersim instance and SUPERSIM_TRACE_TX_HASH"]
async fn traces_supersim_cross_chain_message_receipt() {
    let tx_hash = std::env::var("SUPERSIM_TRACE_TX_HASH")
        .expect("set SUPERSIM_TRACE_TX_HASH to a source-chain cross-chain message transaction");
    let trace = tracer::trace_message(&tx_hash, &supersim_chain())
        .await
        .expect("supersim trace should decode");

    assert_eq!(trace.message.source_tx, tx_hash);
    assert!(
        !trace.lifecycle.is_empty(),
        "trace should include at least the source-chain lifecycle step"
    );
}

#[tokio::test]
#[ignore = "requires a running ethereum-optimism/supersim instance with at least 2 chains"]
async fn traces_cross_chain_message_between_l2_and_l3() {
    let source_chain = supersim_l2_chain();
    let dest_chain = supersim_l3_chain();

    let tx_hash = std::env::var("SUPERSIM_CROSS_CHAIN_TX_HASH")
        .expect("set SUPERSIM_CROSS_CHAIN_TX_HASH to a cross-chain message tx on L2");

    let trace = tracer::trace_message(&tx_hash, &source_chain)
        .await
        .expect("trace should decode cross-chain message");

    assert_eq!(trace.message.source_chain, source_chain.name);
    if let Some(ref dest_name) = trace.message.dest_chain {
        assert_eq!(dest_name, &dest_chain.name);
    }
    assert!(trace.message.block_number > 0);
}

#[tokio::test]
#[ignore = "requires a running supersim instance — checks basic RPC connectivity"]
async fn supersim_rpc_connectivity() {
    let chain = supersim_chain();
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_chainId",
        "params": [],
        "id": 1
    });

    let resp = client
        .post(&chain.rpc_url)
        .json(&body)
        .send()
        .await
        .expect("supersim RPC should be reachable");

    assert!(resp.status().is_success(), "RPC should return 200");

    let json: serde_json::Value = resp.json().await.expect("response should be valid JSON");

    assert_eq!(json["jsonrpc"], "2.0");
    let chain_id_hex = json["result"]
        .as_str()
        .expect("result should be a hex string");
    let chain_id = u64::from_str_radix(chain_id_hex.trim_start_matches("0x"), 16)
        .expect("chain ID should be a valid hex number");
    assert!(chain_id > 0, "chain ID should be non-zero");
}

#[tokio::test]
#[ignore = "requires supersim with at least 2 chains — tests L2-to-L3 cross-chain"]
async fn supersim_multichain_connectivity() {
    let chains = [supersim_l2_chain(), supersim_l3_chain()];
    let client = reqwest::Client::new();

    for chain in &chains {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let resp = client
            .post(&chain.rpc_url)
            .json(&body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("RPC {} should be reachable: {e}", chain.name));

        assert!(
            resp.status().is_success(),
            "{} RPC should return 200",
            chain.name
        );

        let json: serde_json::Value = resp.json().await.unwrap();
        let block_hex = json["result"]
            .as_str()
            .expect("result should be a hex string");
        let block_num = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16)
            .expect("block number should be a valid hex");
        assert!(
            block_num > 0,
            "{} should have mined at least block 0",
            chain.name
        );
    }
}

#[tokio::test]
#[ignore = "requires supersim — tests the supervisor status check against a live instance"]
async fn supersim_supervisor_status() {
    let chain = supersim_chain();
    let result = superchain_trace::supervisor::check_status("test-msg-001", &chain)
        .await
        .expect("supervisor check should not error");

    assert_eq!(result.msg_id, "test-msg-001");
    assert!(
        !result.details.is_empty(),
        "status details should be non-empty"
    );
}
