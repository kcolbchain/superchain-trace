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
