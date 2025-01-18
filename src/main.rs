use clap::Parser;
use habit::cli::Cli;
use habit::db;
use habit::engine::get_engine;
use habit::{DATA_DIR, DB_PATH};
use std::fs;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Make directories
    fs::create_dir_all(DATA_DIR.clone())?;

    // Check if the DB exists. If not, create it.
    if !DB_PATH.exists() {
        let conn = db::open_db()?;
        db::create_tables(&conn)?;
    }

    // Run engine.
    let mut engine = get_engine(cli);
    engine.run()?;

    Ok(())
}
