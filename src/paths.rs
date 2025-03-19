#[cfg(not(debug_assertions))]
use clap::crate_name;
#[cfg(not(debug_assertions))]
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
    #[cfg(debug_assertions)]
    {
        let mut data_dir_path = PathBuf::new();
        data_dir_path.push(".data");
        fs::create_dir_all(data_dir_path.clone()).expect("Failed to create debug data directory.");

        data_dir_path
    }

    #[cfg(not(debug_assertions))]
    {
        let data_dir_path = ProjectDirs::from("", crate_name!(), crate_name!())
            .expect("Failed to construct the path of the data directory.")
            .data_local_dir()
            .to_path_buf();

        // Make data directory if does not already exist.
        fs::create_dir_all(data_dir_path.clone()).expect("Failed to create data directory.");

        data_dir_path
    }
});
