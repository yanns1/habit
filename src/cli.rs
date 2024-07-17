use crate::{
    delete::DeleteCli, edit::EditCli, list::ListCli, log::LogCli, new::NewCli, show::ShowCli,
};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
#[clap(verbatim_doc_comment)]
/// A command-line habit tracker.
///
/// Create habits, log your reps and see your progress via
/// cool terminal-based visualizations (yes, there is ASCII art!).
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    New(NewCli),
    Edit(EditCli),
    Delete(DeleteCli),
    List(ListCli),
    Log(LogCli),
    Show(ShowCli),
}
