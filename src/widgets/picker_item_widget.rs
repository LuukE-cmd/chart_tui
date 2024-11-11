use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, WidgetRef},
};
use std::{
    io,
    sync::{Arc, Mutex, Weak},
};

use crate::Picker;

pub type CallbackFunction = Option<Arc<Mutex<dyn FnMut() -> io::Result<()> + Send + Sync>>>;

pub struct PickerItem {
    pub text: String,
    pub selected: bool,
    parent: Weak<Mutex<Picker>>,
    borders: Borders,
    style: Style,
    callback: CallbackFunction,
}

impl PickerItem {
    pub fn new(
        input_text: &str,
        selected: bool,
        parent: Weak<Mutex<Picker>>,
        callback: CallbackFunction,
    ) -> Self {
        let mut style = Style::default();
        let mut borders = Borders::NONE;

        if selected {
            style = Style::default().bg(Color::Red);
            borders = Borders::ALL;
        }

        PickerItem {
            text: input_text.to_string(),
            selected,
            parent,
            borders,
            style,
            callback,
        }
    }

    pub fn set_style(&mut self, new_style: Style) -> io::Result<()> {
        self.style = new_style;
        Ok(())
    }

    pub fn set_borders(&mut self, new_borders: Borders) -> io::Result<()> {
        self.borders = new_borders;
        Ok(())
    }

    pub fn set_callback(&mut self, new_callback: CallbackFunction) -> &Self {
        self.callback = new_callback;
        self
    }
}

impl WidgetRef for PickerItem {
    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let block_style = Block::default().borders(self.borders).style(self.style);

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
                if key_event.code == KeyCode::Enter
                    && self.selected
                    && self.parent.upgrade().unwrap().lock().unwrap().focussed
                {
                    self.text = "yeet".to_string();
                    self.style = Style::default().bg(Color::Green);

                    if let Some(callback) = &mut self.callback {
                        callback.lock().unwrap()()?;
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}

pub trait EventHandler {
    fn handle_event(&mut self, event: &Event) -> Result<(), io::Error>;
}
