use anyhow::Result;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{config::Config, error::LuascanError};

#[derive(Debug)]
pub enum Command {
    Check(CheckOptions),
    Lsp(LspOptions),
}

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub target: PathBuf,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct LspOptions {
    pub config: Config,
}

#[derive(Parser, Debug)]
#[command(
    name = "luascan",
    version,
    about = "A Lua syntax checker and LSP server"
)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    // Run the type checker over a path
    Check {
        // Path to a file or directory containing Lua sources
        path: PathBuf,
    },
    // Start the Typua language server
    Lsp,
}

pub fn parse() -> Result<Command> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().map_err(|source| LuascanError::CurrentDir { source })?;
    let config = Config::load_from_dir(&cwd)?;

    let command = match cli.command {
        Subcommands::Check { path } => Command::Check(CheckOptions {
            target: path,
            config,
        }),
        Subcommands::Lsp => Command::Lsp(LspOptions { config }),
    };

    Ok(command)
}
