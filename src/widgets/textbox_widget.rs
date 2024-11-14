use ratatui::buffer::Buffer;

use ratatui::layout::Rect;

use ratatui::widgets::{Block, WidgetRef};

use ratatui::style::{Color, Stylize};

use ratatui::widgets::Borders;

pub struct TextBox {
    pub(crate) border: Borders,
    pub(crate) color: Color,
    pub(crate) text: String,
}

#[allow(dead_code)]
impl TextBox {
    pub fn new(border: Borders, color: Color, text: String) -> Self {
        TextBox {
            border,
            color,
            text,
        }
    }

    pub fn set_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }
}

impl Default for TextBox {
    fn default() -> Self {
        TextBox {
            border: Borders::ALL,
            color: Color::DarkGray,
            text: "".to_string(),
        }
    }
}

impl WidgetRef for TextBox {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(self.border).bg(self.color);
        ratatui::widgets::Paragraph::new(self.text.clone())
            .block(block)
            .render_ref(area, buf)
    }
}
