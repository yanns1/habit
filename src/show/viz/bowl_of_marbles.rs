use crate::show::ui::centered_rect;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::{Block, Paragraph, Widget};

pub struct BowlOfMarbles {}

impl BowlOfMarbles {
    pub fn new() -> Self {
        BowlOfMarbles {}
    }
}

impl Widget for BowlOfMarbles {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = centered_rect(area, 50, 50);
        let para = Paragraph::new("Bowl of marbles").block(Block::bordered());
        para.render(rect, buf);
    }
}
