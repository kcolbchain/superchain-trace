//! Supersim process management and RPC endpoint configuration.
//!
//! Supersim (<https://github.com/ethereum-optimism/supersim>) spins up a local
//! multi-chain OP Stack environment with an L1 and two L2 chains that support
//! cross-chain interop messaging.

use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;

// ---------------------------------------------------------------------------
// RPC endpoints — supersim defaults
// ---------------------------------------------------------------------------

/// L1 RPC endpoint (anvil fork started by supersim).
pub const L1_RPC_URL: &str = "http://127.0.0.1:8545";

/// First L2 chain (OPChainA) — default supersim port.
pub const L2_CHAIN_A_RPC_URL: &str = "http://127.0.0.1:9545";

/// Second L2 chain (OPChainB) — default supersim port.
pub const L2_CHAIN_B_RPC_URL: &str = "http://127.0.0.1:9546";

/// Supersim admin / orchestrator API.
pub const SUPERSIM_ADMIN_URL: &str = "http://127.0.0.1:8420";

/// Chain IDs used by supersim.
pub const L1_CHAIN_ID: u64 = 900;
pub const L2_CHAIN_A_ID: u64 = 901;
pub const L2_CHAIN_B_ID: u64 = 902;

/// Pre-funded dev account (anvil account 0).
pub const DEV_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
pub const DEV_PRIVATE_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

// ---------------------------------------------------------------------------
// Supersim process helpers
// ---------------------------------------------------------------------------

/// Handle that stops the supersim process on drop.
pub struct SupersimProcess {
    child: Option<Child>,
}

impl SupersimProcess {
    /// Start a new supersim instance.
    ///
    /// Waits up to 30 seconds for the L1 and L2 RPC endpoints to become
    /// reachable before returning.
    pub async fn start() -> eyre::Result<Self> {
        let child = Command::new("supersim")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut proc = Self { child: Some(child) };

        // Wait for RPC endpoints to be available
        let urls = [L1_RPC_URL, L2_CHAIN_A_RPC_URL, L2_CHAIN_B_RPC_URL];
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();

        for url in &urls {
            loop {
                if start.elapsed() > timeout {
                    proc.stop().await;
                    return Err(eyre::eyre!(
                        "Timeout waiting for supersim endpoint: {}",
                        url
                    ));
                }
                if is_rpc_ready(url).await {
                    break;
                }
                sleep(Duration::from_millis(500)).await;
            }
        }

        Ok(proc)
    }

    /// Kill the supersim process.
    pub async fn stop(&mut self) {
        if let Some(ref mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

impl Drop for SupersimProcess {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // Best-effort synchronous kill in drop
            let _ = child.start_kill();
        }
    }
}

// ---------------------------------------------------------------------------
// Health-check helpers
// ---------------------------------------------------------------------------

/// Check if an RPC endpoint responds to `eth_chainId`.
pub async fn is_rpc_ready(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_chainId",
        "params": [],
        "id": 1,
    });

    client
        .post(url)
        .json(&body)
        .send()
        .await
        .and_then(|r| Ok(r.status().is_success()))
        .unwrap_or(false)
}

/// Assert that all supersim RPC endpoints are reachable.
pub async fn assert_supersim_healthy() {
    for (name, url) in [
        ("L1", L1_RPC_URL),
        ("L2 Chain A", L2_CHAIN_A_RPC_URL),
        ("L2 Chain B", L2_CHAIN_B_RPC_URL),
    ] {
        assert!(
            is_rpc_ready(url).await,
            "Supersim {} endpoint not reachable at {}. Is supersim running?",
            name,
            url,
        );
    }
}

/// Fetch the chain ID from an RPC endpoint.
pub async fn get_chain_id(rpc_url: &str) -> eyre::Result<u64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_chainId",
        "params": [],
        "id": 1,
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("No result in eth_chainId response"))?;
    Ok(u64::from_str_radix(hex.trim_start_matches("0x"), 16)?)
}

/// Fetch the latest block number from an RPC endpoint.
pub async fn get_block_number(rpc_url: &str) -> eyre::Result<u64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1,
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("No result in eth_blockNumber response"))?;
    Ok(u64::from_str_radix(hex.trim_start_matches("0x"), 16)?)
}

/// Send a raw JSON-RPC request and return the result.
pub async fn rpc_request(
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> eyre::Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        return Err(eyre::eyre!("RPC error: {}", err));
    }

    Ok(resp["result"].clone())
}
