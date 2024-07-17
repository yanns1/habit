mod cli;
mod engine;
mod viz;

use clap::ValueEnum;
pub use cli::ShowCli;
pub use engine::get_engine;

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum How {
    #[value(name = "heatmap")]
    HeatMap,
    BowlOfMarbles,
}
