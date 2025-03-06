use crate::delete::DeleteCli;
use crate::edit::EditCli;
use crate::list::ListCli;
use crate::log::LogCli;
use crate::new::NewCli;
use crate::resume::ResumeCli;
use crate::show::ShowCli;
use crate::suspend::SuspendCli;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
#[clap(verbatim_doc_comment)]
/// A command-line habit tracker.
///
/// Create habits, log your reps and see your progress via
/// cool terminal-based visualizations.
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
    Suspend(SuspendCli),
    Resume(ResumeCli),
    Show(ShowCli),
}
