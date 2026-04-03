use crate::types::ChainInfo;
use eyre::{eyre, Result};

pub fn all_chains() -> Vec<ChainInfo> {
    vec![
        ChainInfo {
            name: "op".into(),
            chain_id: 10,
            rpc_url: "https://mainnet.optimism.io".into(),
            explorer_url: "https://optimistic.etherscan.io".into(),
        },
        ChainInfo {
            name: "base".into(),
            chain_id: 8453,
            rpc_url: "https://mainnet.base.org".into(),
            explorer_url: "https://basescan.org".into(),
        },
        ChainInfo {
            name: "zora".into(),
            chain_id: 7777777,
            rpc_url: "https://rpc.zora.energy".into(),
            explorer_url: "https://explorer.zora.energy".into(),
        },
        ChainInfo {
            name: "mode".into(),
            chain_id: 34443,
            rpc_url: "https://mainnet.mode.network".into(),
            explorer_url: "https://explorer.mode.network".into(),
        },
        ChainInfo {
            name: "fraxtal".into(),
            chain_id: 252,
            rpc_url: "https://rpc.frax.com".into(),
            explorer_url: "https://fraxscan.com".into(),
        },
    ]
}

pub fn get_chain(name: &str) -> Result<ChainInfo> {
    all_chains()
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| eyre!("Unknown chain: {}. Use `superchain-trace chains` to list.", name))
}
