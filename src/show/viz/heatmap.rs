use crate::show::ui::centered_rect;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::{Block, Paragraph, Widget};

pub struct HeatMap {}

impl HeatMap {
    pub fn new() -> Self {
        HeatMap {}
    }
}

impl Widget for HeatMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = centered_rect(area, 50, 50);
        let para = Paragraph::new("Heatmap").block(Block::bordered());
        para.render(rect, buf);
    }
}
