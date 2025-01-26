use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Resume (converse of suspend) a habit.
pub struct ResumeCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit to resume.
    pub habit: String,
}
