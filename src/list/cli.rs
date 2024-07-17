use clap::Args;

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// List existing habits.
pub struct ListCli {
    #[clap(verbatim_doc_comment)]
    #[clap(long, short, action)]
    // Verbose output, i.e. show all habit info, not just its name.
    pub verbose: bool,
}
