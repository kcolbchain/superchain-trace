//! Integration tests for superchain-trace using supersim.
//!
//! These tests require a running [supersim](https://github.com/ethereum-optimism/supersim)
//! instance. They are `#[ignore]` by default so `cargo test` passes without supersim.
//!
//! ## Running
//!
//! ```bash
//! # Terminal 1 — start supersim
//! supersim
//!
//! # Terminal 2 — run integration tests
//! cargo test --test integration -- --ignored
//! ```

mod cross_chain;
mod helpers;
mod supersim;
