pub mod cli;
mod db;
mod delete;
mod edit;
pub mod engine;
mod habit;
mod list;
mod log;
mod new;
mod paths;
mod prompt;
mod resume;
mod show;
mod suspend;
mod tui;
mod utils;

use chrono::DateTime;
use chrono::Local;
use std::sync::LazyLock;

static TODAY: LazyLock<DateTime<Local>> = LazyLock::new(Local::now);
