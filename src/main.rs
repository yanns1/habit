use crate::cli::Cli;
use clap::Parser;
use engine::get_engine;

mod cli;
mod engine;
mod new;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut engine = get_engine(cli);
    engine.run()?;

    Ok(())
}
