use clap::Parser;
use habit::cli::Cli;
use habit::engine::get_engine;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Run engine.
    let mut engine = get_engine(cli);
    engine.run()?;

    Ok(())
}
