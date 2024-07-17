pub trait ProgressVisualizer {
    fn show_progress(&mut self, habit: &str) -> anyhow::Result<()>;
}

pub struct HeatMap {}

impl HeatMap {
    pub fn new() -> Self {
        HeatMap {}
    }
}

impl ProgressVisualizer for HeatMap {
    fn show_progress(&mut self, habit: &str) -> anyhow::Result<()> {
        println!("HeatMap for {}", habit);
        todo!();
    }
}

pub struct BowlOfMarbles {}

impl BowlOfMarbles {
    pub fn new() -> Self {
        BowlOfMarbles {}
    }
}

impl ProgressVisualizer for BowlOfMarbles {
    fn show_progress(&mut self, habit: &str) -> anyhow::Result<()> {
        println!("BowlOfMarbles for {}", habit);
        todo!();
    }
}
