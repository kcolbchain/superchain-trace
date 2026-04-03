use clap::{Parser, Subcommand};
use colored::Colorize;

mod chains;
mod display;
mod supervisor;
mod tracer;
mod types;

#[derive(Parser)]
#[command(name = "superchain-trace", about = "Cross-chain message debugger for the OP Superchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Trace a cross-chain message by initiating transaction hash
    Trace {
        /// Transaction hash of the initiating message
        tx_hash: String,
        /// Source chain name (op, base, zora, mode)
        #[arg(short, long, default_value = "op")]
        chain: String,
    },
    /// Check the status of a cross-chain message
    Status {
        /// Message identifier (log index or message hash)
        msg_id: String,
        /// Source chain
        #[arg(short, long, default_value = "op")]
        chain: String,
    },
    /// List supported Superchain chains
    Chains,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Trace { tx_hash, chain } => {
            println!("{}", "superchain-trace".bold().cyan());
            println!("{}", "─".repeat(50).dimmed());
            let chain_info = chains::get_chain(&chain)?;
            let result = tracer::trace_message(&tx_hash, &chain_info).await?;
            display::print_trace(&result);
        }
        Commands::Status { msg_id, chain } => {
            let chain_info = chains::get_chain(&chain)?;
            let status = supervisor::check_status(&msg_id, &chain_info).await?;
            display::print_status(&status);
        }
        Commands::Chains => {
            display::print_chains(&chains::all_chains());
        }
    }

    Ok(())
}
