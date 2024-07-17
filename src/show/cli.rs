use clap::Args;

use super::How;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Visualize your progress.
pub struct ShowCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit for which to show progress.
    pub habit: String,

    #[clap(verbatim_doc_comment)]
    #[clap(value_enum, default_value_t = How::HeatMap)]
    /// What kind of visualization to use.
    pub how: How,
}
