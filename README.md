# superchain-trace
[![repo](https://img.shields.io/badge/repo-superchain-trace-blue)](https://github.com/kcolbchain/superchain-trace)

Cross-chain message debugger for the OP Superchain. By [kcolbchain](https://kcolbchain.com) (est. 2015).

## Why this exists

The OP Superchain enables cross-chain messaging between Optimism, Base, Zora, Mode, and other OP Stack chains via the interop system. But when a cross-chain message fails or gets stuck, there's no developer-facing tool to trace what happened. This tool fills that gap.

## Commands

```bash
# Trace a cross-chain message by its initiating tx hash
superchain-trace trace 0xabc123... --chain op

# Check message status and safety level
superchain-trace status 0xdef456... --chain base

# List supported Superchain networks
superchain-trace chains
```

## Message lifecycle

```
Source chain: tx submitted
    ↓
OP-Supervisor: log indexed
    ↓
Safety level: unsafe → safe → finalized
    ↓
Destination chain: message relayed and executed
```

This tool shows you exactly where your message is in this lifecycle and why it might be stuck.

## Install

```bash
cargo install --git https://github.com/kcolbchain/superchain-trace
```

Or build from source:

```bash
git clone https://github.com/kcolbchain/superchain-trace
cd superchain-trace
cargo build --release
```

## Supported chains

- OP Mainnet (chain ID 10)
- Base (8453)
- Zora (7777777)
- Mode (34443)
- Fraxtal (252)

## Architecture

```
CLI (clap)
  ↓
Tracer — fetches tx receipt, identifies L2ToL2CrossDomainMessenger events
  ↓
Supervisor client — queries OP-Supervisor for safety level
  ↓
Display — pretty terminal output with lifecycle visualization
```

## Supersim integration tests

The repository includes an ignored integration test for local OP Stack cross-chain message traces against [ethereum-optimism/supersim](https://github.com/ethereum-optimism/supersim).

```bash
# Start supersim in another terminal and submit a source-chain message tx, then:
export SUPERSIM_RPC_URL=http://127.0.0.1:9545
export SUPERSIM_CHAIN_ID=901
export SUPERSIM_CHAIN_NAME=supersim-op
export SUPERSIM_TRACE_TX_HASH=0x...
cargo test --test supersim_trace -- --ignored
```

The test is ignored by default so regular CI and local unit tests do not require a running supersim process or funded dev account.

## License

MIT

## Contributing

Issues and PRs welcome. Built for the OP Foundation Interop Mission.
