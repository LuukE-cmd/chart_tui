use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, WidgetRef},
};
use std::{fmt::Debug, io};

#[derive(Debug)]
pub struct PickerItem {
    pub text: String,
    pub selected: bool,
}

impl PickerItem {
    pub fn new(input_text: &str, selected: bool) -> Self {
        PickerItem {
            text: input_text.to_string(),
            selected,
        }
    }
}

impl WidgetRef for PickerItem {
    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let block_style = if self.selected {
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red))
        } else {
            Block::default().borders(Borders::NONE)
        };

        // Use the item’s own rendering logic, styled based on selection
        Paragraph::new(self.text.as_str())
            .block(block_style)
            .render(area, buf);
    }
}

impl EventHandler for PickerItem {
    fn handle_event(&mut self, event: &Event) -> Result<(), io::Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if key_event.code == KeyCode::Enter && self.selected {
                    println!(
                        "Enter Pressed on {} resulting in event: {}",
                        self.text, key_event.code
                    )
                }
            }
            _ => {}
        };
        Ok(())
    }
}

pub trait EventHandler: Debug {
    fn handle_event(&mut self, event: &Event) -> Result<(), io::Error>;
}
