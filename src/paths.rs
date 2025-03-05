use clap::crate_name;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

/// The path to the data directory of the app.
///
/// It is lazily initialized.
///
/// If the data directory does not exist, it is created as part of the initialization.
pub static DATA_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let data_dir_path = ProjectDirs::from("", crate_name!(), crate_name!())
        .expect("Failed to construct the path of the data directory.")
        .data_local_dir()
        .to_path_buf();

    // Make data directory if does not already exist.
    fs::create_dir_all(data_dir_path.clone()).expect("Failed to create data directory.");

    data_dir_path
});

/// The path to the SQLite database file.
///
/// It is lazily initialized.
///
/// If the database does not exist, it is _not_ created as part of the initialization.
pub static DB_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut db_path = DATA_DIR_PATH.clone();
    db_path.push("habit.db");
    db_path
});
