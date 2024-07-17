use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Delete an existing habit.
pub struct DeleteCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit to delete.
    pub habit: String,
}
