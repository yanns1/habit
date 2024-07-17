use crate::{edit::EditCli, new::NewCli};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
#[clap(verbatim_doc_comment)]
/// A command-line habit tracker.
///
/// Create a habit, log your reps and see your progress via
/// cool terminal-based visualizations (yes, there is ASCII art!).
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    New(NewCli),
    Edit(EditCli),
}
