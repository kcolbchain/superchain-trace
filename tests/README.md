# Integration Tests

These integration tests verify **superchain-trace** against a local multi-chain
OP Stack environment powered by
[supersim](https://github.com/ethereum-optimism/supersim).

## Prerequisites

| Tool | Install |
|------|---------|
| Rust / Cargo | <https://rustup.rs> |
| supersim | `go install github.com/ethereum-optimism/supersim@latest` |
| Foundry (anvil) | `curl -L https://foundry.paradigm.xyz \| bash && foundryup` |

## Quick start

```bash
# 1. Start supersim in one terminal
supersim

# 2. Run the integration tests in another terminal
cargo test --test integration -- --ignored
```

## Test structure

```
tests/
├── README.md                  # ← you are here
└── integration/
    ├── mod.rs                 # Test crate root
    ├── supersim.rs            # Supersim process management & RPC helpers
    ├── helpers.rs             # Common test fixtures & transaction helpers
    └── cross_chain.rs         # Cross-chain test scenarios
```

### Test categories

| Test | What it covers |
|------|---------------|
| `test_supersim_connectivity` | Verifies all three supersim endpoints respond with correct chain IDs |
| `test_trace_l1_to_l2_basic_deposit` | Traces a plain L1 tx (no cross-chain event) |
| `test_trace_l2_regular_transaction` | Traces a plain L2 tx (no cross-chain event) |
| `test_trace_l2_to_l1_nonexistent_tx` | Handles querying a non-existent tx hash |
| `test_cross_l2_send_message_a_to_b` | Sends a cross-chain message A → B and checks SentMessage log |
| `test_cross_l2_send_message_b_to_a` | Sends a cross-chain message B → A |
| `test_cross_l2_invalid_destination` | Error handling for invalid destination chain |
| `test_unreachable_rpc_endpoint` | Error handling for unreachable RPC |
| `test_message_status_block_progression` | Verifies chains produce blocks |
| `test_message_status_receipt_fields` | Validates receipt data the tracer depends on |
| `test_message_status_log_detection` | End-to-end log detection for cross-chain vs regular txs |

## Supersim endpoints

| Chain | RPC URL | Chain ID |
|-------|---------|----------|
| L1 | `http://127.0.0.1:8545` | 900 |
| L2 Chain A | `http://127.0.0.1:9545` | 901 |
| L2 Chain B | `http://127.0.0.1:9546` | 902 |

## CI integration

To run these tests in CI you need to:

1. Install supersim and its dependencies (Go ≥ 1.21, Foundry)
2. Start supersim as a background process
3. Wait for RPC endpoints to be ready
4. Run `cargo test --test integration -- --ignored`

Example GitHub Actions step:

```yaml
- name: Install Foundry
  uses: foundry-rs/foundry-toolchain@v1

- name: Install supersim
  run: go install github.com/ethereum-optimism/supersim@latest

- name: Start supersim
  run: |
    supersim &
    sleep 10  # wait for chains to spin up

- name: Run integration tests
  run: cargo test --test integration -- --ignored
```

## Notes

- All tests are `#[ignore]` by default so that `cargo test` (without flags) always passes.
- Tests use the pre-funded anvil dev account (`0xf39F…2266`).
- The `SupersimProcess` helper in `supersim.rs` can also manage the process
  programmatically if you prefer to start/stop supersim within a test fixture.
