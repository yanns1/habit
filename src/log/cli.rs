use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Log a rep for a habit.
pub struct LogCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit for which to log a rep.
    pub habit: String,
}
