use std::io;
use std::io::Error;
use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::KeyCode;

use crossterm::event::KeyEventKind;

use crossterm::event::Event;
use ratatui::style::Color::Red;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::EventHandler;

use ratatui::layout::Margin;

use ratatui::widgets::Borders;

use ratatui::widgets::Block;

use ratatui::buffer::Buffer;

use ratatui::layout::Rect;

use ratatui::widgets::WidgetRef;

use crate::PickerItem;

#[derive(Debug)]
pub struct Picker {
    pub(crate) items: Vec<Arc<Mutex<PickerItem>>>,
    pub(crate) index: u32,
    pub(crate) title: String,
    pub focussed: bool,
}

impl Picker {
    pub fn new(
        items: Vec<Arc<Mutex<PickerItem>>>,
        title: String,
        index: Option<u32>,
        focussed: bool,
    ) -> Self {
        Self {
            items,
            index: index.unwrap_or(0),
            title,
            focussed,
        }
    }

    pub fn get_title(&self) -> io::Result<&str> {
        Ok(&self.title)
    }

    pub fn increment_index(&mut self) -> io::Result<()> {
        let max_index = self.items.len() as u32 - 1;

        self.index = match self.index {
            i if i == max_index => 0, // If the current index is the max, wrap around to 0
            i if i < max_index => i + 1, // Otherwise, increment the index
            _ => self.index,          // Default case (although this shouldn't be needed)
        };
        self.change_selected()?;
        Ok(())
    }

    pub fn decrement_index(&mut self) -> io::Result<()> {
        let max_index = self.items.len() as u32 - 1;

        self.index = match self.index {
            0 => max_index,      // If index is 0, wrap around to the max index
            i if i > 0 => i - 1, // Otherwise, decrement the index
            _ => self.index,     // Default case (shouldn't be needed)
        };
        self.change_selected()?;
        Ok(())
    }

    pub fn change_selected(&mut self) -> io::Result<()> {
        for (i, item) in self.items.iter_mut().enumerate() {
            let is_selected = i as u32 == self.index;

            let mut selected_item = match item.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(), // Recover from a poisoned lock
            };

            selected_item.selected = is_selected;
            if is_selected {
                selected_item.set_style(Style::default().bg(Red))?;
            } else {
                selected_item.set_style(Style::default().bg(ratatui::style::Color::Reset))?;
            }

            // item.lock().unwrap().selected = i as u32 == self.index;
        }
        Ok(())
    }
}

impl WidgetRef for Picker {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Block::default()
            .borders(Borders::ALL)
            .title(self.get_title().unwrap_or("Title Error"))
            .render(area, buf);

        if self.items.is_empty() {
            return;
        }

        let items_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let item_height = items_area.height / self.items.len() as u16;

        for (i, item) in self.items.iter().enumerate() {
            let item_area = Rect {
                x: items_area.x,
                y: items_area.y + i as u16 * item_height,
                width: items_area.width,
                height: item_height,
            };

            item.lock().unwrap().render(item_area, buf);
        }
    }
}

impl EventHandler for Picker {
    fn handle_event(&mut self, event: &Event) -> Result<(), Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Up => &self.decrement_index()?,
                    KeyCode::Down => &self.increment_index()?,
                    _ => &{},
                }
            }
            _ => &{},
        };
        Ok(())
    }
}
