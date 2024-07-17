mod cli;
mod db;
mod edit;
mod engine;
mod habit;
mod new;
mod prompt;
mod utils;

use crate::cli::Cli;
use clap::crate_name;
use clap::Parser;
use directories::ProjectDirs;
use engine::get_engine;
use lazy_static::lazy_static;
use std::fs;
use std::path::PathBuf;

lazy_static! {
    static ref DATA_DIR: PathBuf = ProjectDirs::from("", crate_name!(), crate_name!())
        .unwrap()
        .data_local_dir()
        .to_path_buf();
    static ref DB_PATH: PathBuf = {
        let mut db_path = ProjectDirs::from("", crate_name!(), crate_name!())
            .unwrap()
            .data_local_dir()
            .to_path_buf();
        db_path.push("habit.db");
        db_path
    };
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Make directories
    fs::create_dir_all(DATA_DIR.clone())?;

    // Check if the DB is made, if not create it.
    if !DB_PATH.exists() {
        let conn = db::open_db()?;
        // Make the tables.
        db::habit_create_table(&conn)?;
        // TODO: Make the log table
    }

    let mut engine = get_engine(cli);
    engine.run()?;

    Ok(())
}
