use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Visualize your progress.
pub struct ShowCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit for which to show progress.
    ///
    /// Defaults to the habit you most recently logged a rep for.
    pub habit: Option<String>,
}
