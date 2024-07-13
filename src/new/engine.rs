use crate::engine::Engine;
use crate::new::cli::NewCli;

pub fn get_engine(_cli: NewCli) -> Box<dyn Engine> {
    Box::new(NewEngine {})
}

struct NewEngine {}

impl Engine for NewEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        println!("new");

        Ok(())
    }
}
