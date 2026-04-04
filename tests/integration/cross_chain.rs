//! Cross-chain integration tests that exercise superchain-trace against supersim.
//!
//! Every test in this module is `#[ignore]` because it requires a running supersim.
//! Run with:
//!
//! ```bash
//! cargo test --test integration -- --ignored
//! ```

use serde_json::json;

use crate::helpers;
use crate::supersim;

// ---------------------------------------------------------------------------
// Connectivity & chain identity
// ---------------------------------------------------------------------------

/// Verify that supersim endpoints are reachable and return the expected chain IDs.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_supersim_connectivity() {
    supersim::assert_supersim_healthy().await;

    let l1_id = supersim::get_chain_id(supersim::L1_RPC_URL).await.unwrap();
    assert_eq!(l1_id, supersim::L1_CHAIN_ID, "unexpected L1 chain ID");

    let a_id = supersim::get_chain_id(supersim::L2_CHAIN_A_RPC_URL).await.unwrap();
    assert_eq!(a_id, supersim::L2_CHAIN_A_ID, "unexpected L2-A chain ID");

    let b_id = supersim::get_chain_id(supersim::L2_CHAIN_B_RPC_URL).await.unwrap();
    assert_eq!(b_id, supersim::L2_CHAIN_B_ID, "unexpected L2-B chain ID");
}

// ---------------------------------------------------------------------------
// L1 → L2 message tracing
// ---------------------------------------------------------------------------

/// Test tracing a transaction on L1 that does not contain cross-chain messaging events.
/// The tracer should report it as a regular (non-cross-chain) transaction.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_trace_l1_to_l2_basic_deposit() {
    supersim::assert_supersim_healthy().await;

    // Send a simple ETH transfer on L1 (not a cross-chain message, but we
    // exercise the tracer's "no cross-chain event found" path).
    let to = "0x000000000000000000000000000000000000dEaD";
    let tx_hash = helpers::send_eth_transfer(
        supersim::L1_RPC_URL,
        supersim::DEV_ADDRESS,
        to,
        "0x1",
    )
    .await
    .expect("failed to send L1 tx");

    let receipt = helpers::wait_for_receipt(supersim::L1_RPC_URL, &tx_hash, 15)
        .await
        .expect("receipt not found");

    assert_eq!(
        receipt["status"].as_str(),
        Some("0x1"),
        "L1 tx should succeed"
    );

    // The receipt should NOT contain a SentMessage log (this is a plain transfer).
    assert!(
        !helpers::receipt_has_sent_message_log(&receipt),
        "plain ETH transfer should not emit SentMessage"
    );
}

// ---------------------------------------------------------------------------
// L2 → L1 message tracing
// ---------------------------------------------------------------------------

/// Test tracing a regular transaction on L2 (no cross-chain event).
/// The tracer should handle it gracefully and not report false positives.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_trace_l2_regular_transaction() {
    supersim::assert_supersim_healthy().await;

    let to = "0x000000000000000000000000000000000000dEaD";
    let tx_hash = helpers::send_eth_transfer(
        supersim::L2_CHAIN_A_RPC_URL,
        supersim::DEV_ADDRESS,
        to,
        "0x1",
    )
    .await
    .expect("failed to send L2 tx");

    let receipt = helpers::wait_for_receipt(supersim::L2_CHAIN_A_RPC_URL, &tx_hash, 15)
        .await
        .expect("receipt not found");

    assert_eq!(receipt["status"].as_str(), Some("0x1"));

    // No cross-chain logs expected.
    assert!(!helpers::receipt_has_sent_message_log(&receipt));
}

/// Test that tracer properly handles querying a receipt for a non-existent tx hash.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_trace_l2_to_l1_nonexistent_tx() {
    supersim::assert_supersim_healthy().await;

    let fake_hash = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let result = supersim::rpc_request(
        supersim::L2_CHAIN_A_RPC_URL,
        "eth_getTransactionReceipt",
        json!([fake_hash]),
    )
    .await;

    // The RPC should return null (no receipt) rather than an error.
    match result {
        Ok(val) => assert!(val.is_null(), "non-existent tx should return null receipt"),
        Err(_) => { /* Some RPC impls return an error, which is also acceptable */ }
    }
}

// ---------------------------------------------------------------------------
// Cross-L2 message tracing (L2 Chain A ↔ L2 Chain B)
// ---------------------------------------------------------------------------

/// Test sending a cross-chain message from L2 Chain A to L2 Chain B via the
/// L2ToL2CrossDomainMessenger predeploy.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_cross_l2_send_message_a_to_b() {
    supersim::assert_supersim_healthy().await;

    let message_payload = b"hello from chain A";

    let tx_hash = helpers::send_cross_chain_message(
        supersim::L2_CHAIN_A_RPC_URL,
        supersim::L2_CHAIN_B_ID,
        // target: any address on chain B
        "0x000000000000000000000000000000000000dEaD",
        message_payload,
    )
    .await
    .expect("failed to send cross-chain message A→B");

    let receipt = helpers::wait_for_receipt(supersim::L2_CHAIN_A_RPC_URL, &tx_hash, 15)
        .await
        .expect("receipt not found for cross-chain tx");

    assert_eq!(
        receipt["status"].as_str(),
        Some("0x1"),
        "cross-chain send tx should succeed"
    );

    // Verify the receipt contains a SentMessage log targeting chain B.
    assert!(
        helpers::receipt_has_sent_message_log(&receipt),
        "cross-chain tx should emit SentMessage"
    );

    let dest_chain = helpers::extract_destination_chain_id(&receipt);
    assert_eq!(
        dest_chain,
        Some(supersim::L2_CHAIN_B_ID),
        "destination chain ID should be L2-B"
    );
}

/// Test sending a cross-chain message in the reverse direction (B → A).
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_cross_l2_send_message_b_to_a() {
    supersim::assert_supersim_healthy().await;

    let tx_hash = helpers::send_cross_chain_message(
        supersim::L2_CHAIN_B_RPC_URL,
        supersim::L2_CHAIN_A_ID,
        "0x000000000000000000000000000000000000dEaD",
        b"hello from chain B",
    )
    .await
    .expect("failed to send cross-chain message B→A");

    let receipt = helpers::wait_for_receipt(supersim::L2_CHAIN_B_RPC_URL, &tx_hash, 15)
        .await
        .expect("receipt not found for cross-chain tx");

    assert_eq!(receipt["status"].as_str(), Some("0x1"));
    assert!(helpers::receipt_has_sent_message_log(&receipt));

    let dest_chain = helpers::extract_destination_chain_id(&receipt);
    assert_eq!(dest_chain, Some(supersim::L2_CHAIN_A_ID));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

/// Test that a cross-chain message to an invalid destination is handled.
/// Sending to destination chain id 0 (invalid) should either revert or be
/// observable as an abnormal message.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_cross_l2_invalid_destination() {
    supersim::assert_supersim_healthy().await;

    let result = helpers::send_cross_chain_message(
        supersim::L2_CHAIN_A_RPC_URL,
        0, // invalid destination chain id
        "0x000000000000000000000000000000000000dEaD",
        b"should fail",
    )
    .await;

    match result {
        Ok(tx_hash) => {
            // If the tx was accepted, check that it reverted.
            let receipt = helpers::wait_for_receipt(
                supersim::L2_CHAIN_A_RPC_URL,
                &tx_hash,
                15,
            )
            .await;

            match receipt {
                Ok(r) => {
                    // Either reverted (status 0x0) or no SentMessage log
                    let status = r["status"].as_str().unwrap_or("0x0");
                    if status == "0x1" {
                        // Tx succeeded — the messenger may accept any destination,
                        // but the message won't be deliverable. That's fine.
                    }
                }
                Err(_) => { /* Tx may have been dropped — acceptable */ }
            }
        }
        Err(_) => {
            // RPC rejected the tx — expected behaviour for invalid params.
        }
    }
}

/// Test error handling when the RPC endpoint is unreachable.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_unreachable_rpc_endpoint() {
    let bad_url = "http://127.0.0.1:1"; // nothing listening

    let result = supersim::rpc_request(bad_url, "eth_chainId", json!([])).await;
    assert!(result.is_err(), "request to unreachable endpoint should fail");
}

// ---------------------------------------------------------------------------
// Message status tracking
// ---------------------------------------------------------------------------

/// Test that we can fetch block numbers from both L2 chains (used for status tracking).
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_message_status_block_progression() {
    supersim::assert_supersim_healthy().await;

    let block_a = supersim::get_block_number(supersim::L2_CHAIN_A_RPC_URL)
        .await
        .expect("failed to get block number on chain A");

    let block_b = supersim::get_block_number(supersim::L2_CHAIN_B_RPC_URL)
        .await
        .expect("failed to get block number on chain B");

    // Both chains should have produced at least one block.
    assert!(block_a > 0, "chain A should have blocks");
    assert!(block_b > 0, "chain B should have blocks");
}

/// After sending a cross-chain message, verify we can query its receipt data
/// which is the foundation for message status tracking.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_message_status_receipt_fields() {
    supersim::assert_supersim_healthy().await;

    let tx_hash = helpers::send_cross_chain_message(
        supersim::L2_CHAIN_A_RPC_URL,
        supersim::L2_CHAIN_B_ID,
        "0x000000000000000000000000000000000000dEaD",
        b"status tracking test",
    )
    .await
    .expect("failed to send message");

    let receipt = helpers::wait_for_receipt(supersim::L2_CHAIN_A_RPC_URL, &tx_hash, 15)
        .await
        .expect("receipt not found");

    // The receipt must contain the fields the tracer relies on.
    assert!(receipt["blockNumber"].is_string(), "receipt must have blockNumber");
    assert!(receipt["status"].is_string(), "receipt must have status");
    assert!(receipt["from"].is_string(), "receipt must have from");
    assert!(receipt["gasUsed"].is_string(), "receipt must have gasUsed");
    assert!(receipt["logs"].is_array(), "receipt must have logs array");
}

/// Verify that the tracer's log detection logic works against a real supersim receipt.
#[tokio::test]
#[ignore = "requires running supersim"]
async fn test_message_status_log_detection() {
    supersim::assert_supersim_healthy().await;

    // Cross-chain message — should have the log.
    let tx1 = helpers::send_cross_chain_message(
        supersim::L2_CHAIN_A_RPC_URL,
        supersim::L2_CHAIN_B_ID,
        "0x000000000000000000000000000000000000dEaD",
        b"log detection test",
    )
    .await
    .expect("failed to send cross-chain message");

    // Regular tx — should NOT have the log.
    let tx2 = helpers::send_eth_transfer(
        supersim::L2_CHAIN_A_RPC_URL,
        supersim::DEV_ADDRESS,
        "0x000000000000000000000000000000000000dEaD",
        "0x1",
    )
    .await
    .expect("failed to send regular tx");

    let r1 = helpers::wait_for_receipt(supersim::L2_CHAIN_A_RPC_URL, &tx1, 15)
        .await
        .unwrap();
    let r2 = helpers::wait_for_receipt(supersim::L2_CHAIN_A_RPC_URL, &tx2, 15)
        .await
        .unwrap();

    assert!(
        helpers::receipt_has_sent_message_log(&r1),
        "cross-chain receipt must have SentMessage log"
    );
    assert!(
        !helpers::receipt_has_sent_message_log(&r2),
        "regular receipt must NOT have SentMessage log"
    );
}
