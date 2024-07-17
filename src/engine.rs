use crate::cli;
use crate::delete;
use crate::edit;
use crate::list;
use crate::log;
use crate::new;

pub trait Engine {
    fn run(&mut self) -> anyhow::Result<()>;
}

pub fn get_engine(cli: cli::Cli) -> Box<dyn Engine> {
    match cli.command {
        crate::cli::Command::New(cli) => new::get_engine(cli),
        crate::cli::Command::Edit(cli) => edit::get_engine(cli),
        crate::cli::Command::Delete(cli) => delete::get_engine(cli),
        crate::cli::Command::List(cli) => list::get_engine(cli),
        crate::cli::Command::Log(cli) => log::get_engine(cli),
    }
}
