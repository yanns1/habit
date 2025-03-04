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
pub mod resume;
pub mod show;
pub mod suspend;
pub mod tui;
pub mod utils;

use clap::crate_name;
use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::LazyLock;

pub static DATA_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    ProjectDirs::from("", crate_name!(), crate_name!())
        .expect("Failed to construct the path of the data directory.")
        .data_local_dir()
        .to_path_buf()
});
