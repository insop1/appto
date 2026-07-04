mod run;
mod paths;
mod desktop;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Result};

#[derive(Parser)]
#[command(name = "appto")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        path: PathBuf,

        #[arg(long)]
        edit: bool,

        #[arg(short, long)]
        force: bool,
    },
    Remove {
        name: String,
    },
    List
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Add { path, edit, force } => run::add(path, edit, force)?,
        Commands::Remove { name } => todo!(),
        Commands::List => todo!(),
    };
    Ok(())
}
