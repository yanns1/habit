mod bowl_of_marbles;
mod heatmap;

pub use bowl_of_marbles::BowlOfMarbles;
pub use heatmap::HeatMap;

#[derive(Debug)]
pub enum ProgressVisualizer {
    HeatMap,
    BowlOfMarbles,
}
