use crate::new::NewCli;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    New(NewCli),
}
