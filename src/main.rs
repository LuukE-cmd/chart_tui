use core::str;
use std::{
    fmt::Debug,
    io,
    sync::{Arc, Mutex, Weak},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, WidgetRef},
    DefaultTerminal, Frame,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Default)]
pub struct App {
    exit: bool,
    event_queue: Vec<Weak<Mutex<dyn EventHandler>>>,
    widgets: Vec<Arc<Mutex<dyn WidgetRef>>>,
}

impl App {
    pub fn init(&mut self) -> io::Result<()> {
        let items = vec![
            PickerItem::new("Item 1", false),
            PickerItem::new("Item 2", true), // Marking this item as selected
            PickerItem::new("Item 3", false),
        ];
        let title = "test".to_string();
        let picker = Arc::new(Mutex::new(Picker::new(items, title, Some(0))));

        self.widgets.push(picker.clone());
        self.add_event_handler(picker);
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.init()?;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.propogate_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    fn propogate_events(&mut self) -> io::Result<()> {
        let read_event = event::read()?;

        if let Event::Key(key_event) = &read_event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
                self.exit(); // Set the exit flag to true
                return Ok(()); // Return early to quit immediately
            }
        }

        self.event_queue
            .retain(|weak_handler| weak_handler.strong_count() > 0);

        for handler in self.event_queue.iter().filter_map(Weak::upgrade) {
            handler.lock().unwrap().handle_event(&read_event)?;
        }

        Ok(())
    }

    fn add_event_handler(&mut self, handler: Arc<Mutex<dyn EventHandler>>) {
        self.event_queue.push(Arc::downgrade(&handler));
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Determine the height each widget should occupy within the given area
        let total_height = area.height;
        let widget_height = total_height / self.widgets.len() as u16;

        for (i, widget) in self.widgets.iter().enumerate() {
            // Lock the widget and calculate its specific area
            let widget_ref = widget.lock().unwrap();

            // Define a region for each widget based on its index
            let widget_area = Rect {
                x: area.x,
                y: area.y + i as u16 * widget_height,
                width: area.width,
                height: widget_height,
            };

            // Render the widget in its designated area
            widget_ref.render_ref(widget_area, buf);
        }
    }
}

#[derive(Debug)]
pub struct Picker {
    items: Vec<PickerItem>,
    index: u32,
    title: String,
}

impl Picker {
    pub fn new(items: Vec<PickerItem>, title: String, index: Option<u32>) -> Self {
        Self {
            items,
            index: index.unwrap_or(0),
            title,
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
            item.selected = i as u32 == self.index;
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

            item.render(item_area, buf);
        }
    }
}

impl EventHandler for Picker {
    fn handle_event(&mut self, event: &Event) -> Result<(), io::Error> {
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

#[derive(Debug)]
pub struct PickerItem {
    text: String,
    selected: bool,
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
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
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
                if key_event.code == KeyCode::Enter {
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
