use crate::cli::Cli;
use crate::new;

pub trait Engine {
    fn run(&mut self) -> anyhow::Result<()>;
}

pub fn get_engine(cli: Cli) -> Box<dyn Engine> {
    match cli.command {
        crate::cli::Command::New(cli) => new::get_engine(cli),
    }
}
