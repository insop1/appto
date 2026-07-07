mod container;
mod desktop;
mod paths;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "appto")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(visible_aliases = ["install", "i"])]
    Add {
        path: PathBuf,

        #[arg(short, long)]
        force: bool,
    },
    #[command(visible_alias = "rm")]
    Remove {
        id: String,

        #[arg(short, long)]
        force: bool,
    },
    #[command(visible_alias = "ls")]
    List,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Add { path, force } => run::add(&path, force)?,
        Commands::Remove { id, force } => run::remove(&id, force)?,
        Commands::List => run::list()?,
    };
    Ok(())
}
