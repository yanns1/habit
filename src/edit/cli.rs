use clap::Args;

use clap::ValueEnum;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Edit an existing habit.
pub struct EditCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit to edit.
    pub habit: String,

    #[clap(verbatim_doc_comment)]
    /// What to edit.
    pub what: What,
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum What {
    Name,
    Description,
    Days,
    At,
}
