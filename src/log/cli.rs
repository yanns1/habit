use clap::Args;
use clap::ValueEnum;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Log a rep for a habit.
pub struct LogCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit for which to log a rep.
    pub habit: String,

    #[clap(verbatim_doc_comment)]
    /// A past date for which to log, in case you forgot to.
    pub date: Option<PastDate>,
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
// TODO: Can I provide a custom parser for this enum?
// Because I want the user to enter the date in dd-mm-yyyy
// say, but store it as `DateTime`.
pub enum PastDate {
    Last,
    // Date(DateTime<Local>),
}
