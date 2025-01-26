use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Suspend a habit.
pub struct SuspendCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit to suspend.
    pub habit: String,
}
