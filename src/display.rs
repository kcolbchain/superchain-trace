use crate::types::*;
use colored::Colorize;

pub fn print_trace(result: &TraceResult) {
    let msg = &result.message;

    println!("\n  {} {}", "Source:".dimmed(), msg.source_chain.bold());
    println!("  {} {}", "Tx:".dimmed(), msg.source_tx.cyan());
    println!("  {} {}", "Sender:".dimmed(), msg.sender);
    println!("  {} {}", "Block:".dimmed(), msg.block_number);

    if let Some(ref dest) = msg.dest_chain {
        println!("  {} {}", "Dest:".dimmed(), dest.bold().green());
    }
    if let Some(gas) = msg.gas_used {
        println!("  {} {}", "Gas:".dimmed(), gas);
    }
    if let Some(ref err) = msg.error {
        println!("  {} {}", "Error:".dimmed(), err.red());
    }

    println!("\n  {}", "Lifecycle:".bold());
    for (i, step) in result.lifecycle.iter().enumerate() {
        let icon = match step.status {
            MessageStatus::Initiated => "●".blue(),
            MessageStatus::Indexed => "●".yellow(),
            MessageStatus::Safe => "●".green(),
            MessageStatus::Finalized => "●".green().bold(),
            MessageStatus::Executed => "✓".green().bold(),
            MessageStatus::Failed => "✗".red().bold(),
            MessageStatus::Unknown => "?".dimmed(),
        };

        let connector = if i < result.lifecycle.len() - 1 { "│" } else { " " };

        println!("  {} {} {}", icon, step.status.to_string().bold(), step.description.dimmed());
        if let Some(ref tx) = step.tx_hash {
            println!("  {}   tx: {}", connector.dimmed(), tx.cyan());
        }
        println!("  {}", connector.dimmed());
    }
}

pub fn print_status(status: &StatusResult) {
    let icon = match status.status {
        MessageStatus::Executed => "✓".green().bold(),
        MessageStatus::Failed => "✗".red().bold(),
        MessageStatus::Safe | MessageStatus::Finalized => "●".green(),
        _ => "●".yellow(),
    };

    println!("\n  {} {}", icon, status.status.to_string().bold());
    println!("  {} {}", "Safety:".dimmed(), status.safety_level);
    println!("  {} {}", "Details:".dimmed(), status.details);
    println!();
}

pub fn print_chains(chains: &[ChainInfo]) {
    println!("\n  {}", "Supported Superchain networks:".bold());
    println!("  {}", "─".repeat(50).dimmed());
    for chain in chains {
        println!(
            "  {} {:12} chain_id={:<10} {}",
            "●".cyan(),
            chain.name.bold(),
            chain.chain_id,
            chain.rpc_url.dimmed()
        );
    }
    println!();
}
