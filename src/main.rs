use std::{
    io,
    sync::{Arc, Mutex, Weak},
};

mod widgets;
use widgets::picker_item_widget::*;
use widgets::picker_widget::*;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Widget, WidgetRef},
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
            PickerItem::new("Item 1", true),
            PickerItem::new("Item 2", false), // Marking this item as selected
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
