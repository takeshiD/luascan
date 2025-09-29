mod cli;
mod config;
mod error;
mod lsp;
mod parser;
mod workspace;

use crate::cli::{CheckOptions, Command, LspOptions};
use crate::error::LuascanError;
use anyhow::Result;
use std::fs::File;
use std::process;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    match cli::parse()? {
        Command::Check(options) => handle_check(options),
        Command::Lsp(options) => handle_lsp(options),
    }
}

fn handle_check(options: CheckOptions) -> Result<()> {
    // let report = checker::run(&options)?;
    //
    // if report.diagnostics.is_empty() {
    //     println!("Checked {} file(s); no issues found.", report.files_checked);
    //     return Ok(());
    // }
    //
    // for diagnostic in &report.diagnostics {
    //     println!("{diagnostic}");
    // }
    //
    unimplemented!("handle check")
}

fn handle_lsp(options: LspOptions) -> Result<()> {
    let xdg_dir = xdg::BaseDirectories::with_prefix("luascan");
    let log_path = xdg_dir
        .place_cache_file("log.json")
        .unwrap_or_else(|e| panic!("falied to create log dir '{:?}': {}", xdg_dir, e));
    let log_file = if !log_path.exists() {
        Arc::new(
            File::create(log_path.clone())
                .unwrap_or_else(|e| panic!("failed to create log file '{:?}': {}", log_path, e)),
        )
    } else {
        Arc::new(
            File::options()
                .append(true)
                .open(log_path.clone())
                .unwrap_or_else(|e| panic!("failed to open log file '{:?}': {}", log_path, e)),
        )
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(log_file)
        .json()
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| LuascanError::Runtime { source })?;

    runtime.block_on(lsp::run(options))
}
