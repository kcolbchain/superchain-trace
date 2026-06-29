//! Message-lifecycle tracing tests.
//!
//! These exercise the network-free core of the debugger:
//!
//! * `tracer::decode_trace` — parsing/decoding an `eth_getTransactionReceipt`
//!   payload into a cross-domain message and its lifecycle, driven by JSON
//!   fixtures in `tests/fixtures/`.
//! * `supervisor::check_status_with` — mapping an OP-Supervisor safety level
//!   onto the message lifecycle status, driven by a mock `SupervisorClient`.
//!
//! Unlike `supersim_trace.rs` (which is `#[ignore]`d and needs a live node),
//! everything here runs deterministically with no network.

use async_trait::async_trait;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};

use superchain_trace::supervisor::{
    check_status_with, MessageQuery, SafetyLevel, SupervisorClient,
};
use superchain_trace::tracer::{decode_trace, SENT_MESSAGE_TOPIC};
use superchain_trace::types::{ChainInfo, MessageStatus};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn load_fixture(name: &str) -> Value {
    let path = format!("{FIXTURE_DIR}/{name}");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("fixture {path} is not valid JSON: {e}"))
}

fn op_chain() -> ChainInfo {
    ChainInfo {
        name: "op".into(),
        chain_id: 10,
        rpc_url: "http://localhost:0".into(),
        explorer_url: "https://optimistic.etherscan.io".into(),
    }
}

// --------------------------------------------------------------------------
// tracer::decode_trace — parse / decode a cross-domain message from a receipt
// --------------------------------------------------------------------------

#[test]
fn decodes_cross_chain_message_to_base() {
    let receipt = load_fixture("receipt_cross_chain_to_base.json");
    let tx = "0xaaaa000000000000000000000000000000000000000000000000000000000001";

    let trace = decode_trace(tx, &op_chain(), &receipt).expect("decode should succeed");
    let msg = &trace.message;

    // Basic message decoding.
    assert_eq!(msg.source_chain, "op");
    assert_eq!(msg.source_tx, tx);
    assert_eq!(msg.sender, "0x1111111111111111111111111111111111111111");
    assert_eq!(msg.block_number, 0x10f2c8);
    assert_eq!(msg.gas_used, Some(0x1a2b3));
    assert!(msg.error.is_none());

    // topic[1] = 0x2105 = 8453 = Base, resolved by chain id.
    assert_eq!(msg.dest_chain.as_deref(), Some("base"));

    // A detected cross-chain message lands in the Indexed state.
    assert_eq!(msg.status, MessageStatus::Indexed);
}

#[test]
fn cross_chain_lifecycle_has_initiated_then_indexed() {
    let receipt = load_fixture("receipt_cross_chain_to_base.json");
    let trace = decode_trace(
        "0xaaaa000000000000000000000000000000000000000000000000000000000001",
        &op_chain(),
        &receipt,
    )
    .expect("decode should succeed");

    // State transition: Initiated -> Indexed for a real cross-chain message.
    let states: Vec<MessageStatus> = trace.lifecycle.iter().map(|s| s.status).collect();
    assert_eq!(
        states,
        vec![MessageStatus::Initiated, MessageStatus::Indexed]
    );

    // First step carries the source tx hash and source chain.
    let first = &trace.lifecycle[0];
    assert_eq!(first.chain, "op");
    assert_eq!(
        first.tx_hash.as_deref(),
        Some("0xaaaa000000000000000000000000000000000000000000000000000000000001")
    );

    // No UNKNOWN/diagnostic step should appear when a message is detected.
    assert!(
        !trace
            .lifecycle
            .iter()
            .any(|s| s.status == MessageStatus::Unknown),
        "a decoded cross-chain message must not emit an Unknown lifecycle step"
    );
}

#[test]
fn decodes_cross_chain_message_with_unknown_destination() {
    let receipt = load_fixture("receipt_cross_chain_unknown_dest.json");
    let trace = decode_trace(
        "0xeeee000000000000000000000000000000000000000000000000000000000005",
        &op_chain(),
        &receipt,
    )
    .expect("decode should succeed");

    // The SentMessage log is present, so the message is Indexed...
    assert_eq!(trace.message.status, MessageStatus::Indexed);
    // ...but topic[1] is a chain id we don't know, so dest stays unresolved.
    assert_eq!(trace.message.dest_chain, None);

    let states: Vec<MessageStatus> = trace.lifecycle.iter().map(|s| s.status).collect();
    assert_eq!(
        states,
        vec![MessageStatus::Initiated, MessageStatus::Indexed]
    );
}

#[test]
fn regular_tx_is_initiated_and_flagged_unknown() {
    let receipt = load_fixture("receipt_regular_tx.json");
    let trace = decode_trace(
        "0xcccc000000000000000000000000000000000000000000000000000000000003",
        &op_chain(),
        &receipt,
    )
    .expect("decode should succeed");

    // No SentMessage log -> message stays Initiated, no destination.
    assert_eq!(trace.message.status, MessageStatus::Initiated);
    assert_eq!(trace.message.dest_chain, None);
    assert!(trace.message.error.is_none());

    // A successful non-cross-chain tx gets a diagnostic Unknown step.
    let states: Vec<MessageStatus> = trace.lifecycle.iter().map(|s| s.status).collect();
    assert_eq!(
        states,
        vec![MessageStatus::Initiated, MessageStatus::Unknown]
    );
}

#[test]
fn reverted_tx_records_error_and_no_diagnostic_step() {
    let receipt = load_fixture("receipt_reverted.json");
    let trace = decode_trace(
        "0xbbbb000000000000000000000000000000000000000000000000000000000002",
        &op_chain(),
        &receipt,
    )
    .expect("decode should succeed even for a reverted tx");

    // status == 0x0 -> error recorded, message never leaves Initiated.
    assert_eq!(trace.message.status, MessageStatus::Initiated);
    assert_eq!(trace.message.error.as_deref(), Some("Transaction reverted"));

    // A reverted tx with no logs only has the Initiated step (status != 0x1
    // so the "regular transaction" diagnostic step is intentionally skipped).
    let states: Vec<MessageStatus> = trace.lifecycle.iter().map(|s| s.status).collect();
    assert_eq!(states, vec![MessageStatus::Initiated]);
}

#[test]
fn missing_block_number_is_an_error() {
    let receipt = load_fixture("receipt_missing_block.json");
    let err = decode_trace(
        "0xdddd000000000000000000000000000000000000000000000000000000000004",
        &op_chain(),
        &receipt,
    )
    .expect_err("a receipt with no blockNumber must fail to decode");

    assert!(
        err.to_string().contains("block number"),
        "error should mention the missing block number, got: {err}"
    );
}

#[test]
fn sent_message_topic_is_the_documented_selector() {
    // Guards against an accidental edit to the event topic the tracer keys on.
    assert_eq!(
        SENT_MESSAGE_TOPIC,
        "0x382409ac69001e11931a28435afef442cbfd20d9891907e8fa373ba7d351f320"
    );
}

// --------------------------------------------------------------------------
// supervisor::check_status_with — lifecycle status via a mock supervisor
// --------------------------------------------------------------------------

/// A mock OP-Supervisor that returns a fixed safety level and counts calls.
struct MockSupervisor {
    level: SafetyLevel,
    calls: AtomicUsize,
}

impl MockSupervisor {
    fn new(level: SafetyLevel) -> Self {
        Self {
            level,
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl SupervisorClient for MockSupervisor {
    async fn safety_level(&self, query: &MessageQuery) -> eyre::Result<SafetyLevel> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // The query should carry the chain id the caller asked about.
        assert_eq!(query.chain_id, 10);
        Ok(self.level)
    }
}

/// A mock supervisor that always errors, to verify error propagation.
struct ErroringSupervisor;

#[async_trait]
impl SupervisorClient for ErroringSupervisor {
    async fn safety_level(&self, _query: &MessageQuery) -> eyre::Result<SafetyLevel> {
        Err(eyre::eyre!("supervisor unreachable"))
    }
}

#[tokio::test]
async fn safety_levels_map_onto_lifecycle_status() {
    let cases = [
        (SafetyLevel::Pending, MessageStatus::Unknown, "pending"),
        (SafetyLevel::Unsafe, MessageStatus::Indexed, "unsafe"),
        (SafetyLevel::Safe, MessageStatus::Safe, "safe"),
        (
            SafetyLevel::Finalized,
            MessageStatus::Finalized,
            "finalized",
        ),
    ];

    for (level, expected_status, expected_label) in cases {
        let supervisor = MockSupervisor::new(level);
        let result = check_status_with(&supervisor, "msg-001", &op_chain())
            .await
            .expect("mock supervisor check should not error");

        assert_eq!(result.msg_id, "msg-001");
        assert_eq!(result.status, expected_status, "status for {level:?}");
        assert_eq!(result.safety_level, expected_label, "label for {level:?}");
        assert!(!result.details.is_empty());
        assert_eq!(
            supervisor.calls.load(Ordering::SeqCst),
            1,
            "supervisor should be queried exactly once"
        );
    }
}

#[tokio::test]
async fn supervisor_error_propagates() {
    let result = check_status_with(&ErroringSupervisor, "msg-002", &op_chain()).await;
    assert!(
        result.is_err(),
        "supervisor errors must propagate to caller"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("supervisor unreachable"));
}

#[test]
fn safety_level_parsing_is_case_and_alias_tolerant() {
    assert_eq!(SafetyLevel::parse("finalized"), SafetyLevel::Finalized);
    assert_eq!(SafetyLevel::parse("FINALIZED"), SafetyLevel::Finalized);
    assert_eq!(SafetyLevel::parse("safe"), SafetyLevel::Safe);
    assert_eq!(SafetyLevel::parse("cross-safe"), SafetyLevel::Safe);
    assert_eq!(SafetyLevel::parse(" unsafe "), SafetyLevel::Unsafe);
    assert_eq!(SafetyLevel::parse("local-safe"), SafetyLevel::Unsafe);
    assert_eq!(SafetyLevel::parse("cross-unsafe"), SafetyLevel::Unsafe);
    // Anything unrecognized is treated as not-yet-indexed.
    assert_eq!(SafetyLevel::parse("garbage"), SafetyLevel::Pending);
    assert_eq!(SafetyLevel::parse(""), SafetyLevel::Pending);
}

#[test]
fn safety_level_status_mapping_round_trips_labels() {
    // The label exposed to the CLI should match the wire string the parser
    // accepts (so re-querying a reported level is stable).
    for level in [
        SafetyLevel::Pending,
        SafetyLevel::Unsafe,
        SafetyLevel::Safe,
        SafetyLevel::Finalized,
    ] {
        assert_eq!(SafetyLevel::parse(level.as_str()), level);
    }
}
