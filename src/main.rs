mod cli;
mod config;
mod core;
mod models;
mod sources;

#[cfg(feature = "gui")]
mod gui;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(e) = cli::run(cli) {
        eprintln!("오류: {:#}", e);
        std::process::exit(1);
    }
}
