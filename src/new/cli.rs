use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Make a new habit.
pub struct NewCli {}
