pub mod cli;
pub mod db;
pub mod delete;
pub mod edit;
pub mod engine;
pub mod habit;
pub mod list;
pub mod log;
pub mod new;
pub mod prompt;
pub mod show;
pub mod tui;
pub mod utils;

use clap::crate_name;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use std::path::PathBuf;

lazy_static! {
    pub static ref DATA_DIR: PathBuf = ProjectDirs::from("", crate_name!(), crate_name!())
        .unwrap()
        .data_local_dir()
        .to_path_buf();
    pub static ref DB_PATH: PathBuf = {
        let mut db_path = ProjectDirs::from("", crate_name!(), crate_name!())
            .unwrap()
            .data_local_dir()
            .to_path_buf();
        db_path.push("habit.db");
        db_path
    };
}
