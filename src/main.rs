use std::{
    fs,
    io::{self},
    path::PathBuf,
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

type WidgetVector = Arc<Mutex<Vec<Arc<Mutex<dyn WidgetRef>>>>>;

#[derive(Default)]
pub struct App {
    exit: bool,
    event_queue: Vec<Weak<Mutex<dyn EventHandler>>>,
    widgets: WidgetVector,
}

impl App {
    pub fn init(&mut self) -> io::Result<()> {
        let picker = self.file_picker();
        if let Ok(mut widget_list) = self.widgets.try_lock() {
            widget_list.push(picker.clone())
        }
        self.add_event_handler(picker);
        Ok(())
    }

    fn file_picker(&mut self) -> Arc<Mutex<Picker>> {
        let title = "test".to_string();
        let picker = Arc::new(Mutex::new(Picker::new(
            vec![],
            title.clone(),
            Some(0),
            true,
        )));

        let current_dir = std::env::current_dir().expect("Failed to open current directory");

        let items = self.xlsx_picker_items(current_dir, &picker);

        for item in &items {
            self.add_event_handler(item.clone());
        }

        picker.lock().unwrap().items = items;
        picker
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.init()?;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.propagate_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    fn propagate_events(&mut self) -> io::Result<()> {
        let read_event = event::read()?;

        if let Event::Key(key_event) = &read_event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
                self.exit(); // Set the exit flag to true
                return Ok(()); // Return early to quit immediately
            }
        }

        // Collect valid strong references before processing
        let handlers: Vec<_> = self.event_queue.iter().filter_map(Weak::upgrade).collect();

        for handler in handlers {
            if let Ok(mut handler) = handler.try_lock() {
                handler.handle_event(&read_event)?;
            }
        }

        // Clean up any stale weak references
        self.event_queue
            .retain(|weak_handler| weak_handler.strong_count() > 0);

        Ok(())
    }

    fn add_event_handler(&mut self, handler: Arc<Mutex<dyn EventHandler>>) {
        self.event_queue.push(Arc::downgrade(&handler));
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn xlsx_picker_items(
        &mut self,
        current_dir: PathBuf,
        picker: &Arc<Mutex<Picker>>,
    ) -> Vec<Arc<Mutex<PickerItem>>> {
        let xlsx_files: Vec<PathBuf> = fs::read_dir(&current_dir)
            .expect("Failed to read directory")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "xlsx" {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        let open_file_functions: Vec<CallbackFunction> = xlsx_files
            .iter()
            .map(|path| build_open_file_func(Arc::new(path.clone()), self.widgets.clone()))
            .collect();

        let items: Vec<Arc<Mutex<PickerItem>>> = xlsx_files
            .iter()
            .zip(open_file_functions.iter())
            .enumerate()
            .map(|(index, (path, open_file_function))| {
                Arc::new(Mutex::new(PickerItem::new(
                    path.file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or("Unknown"),
                    index == 0, // Mark the first item as selected
                    Arc::downgrade(picker),
                    open_file_function.clone(),
                )))
            })
            .collect();
        items
    }
}

fn build_open_file_func(path: Arc<PathBuf>, widgets: WidgetVector) -> CallbackFunction {
    let open_file_function: CallbackFunction = Some(Arc::new(Mutex::new(move || {
        //widgets.println("test: {} ", path.to_str().unwrap_or("Err"));
        Ok(())
    })));
    open_file_function
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Ok(widgets) = self.widgets.try_lock() else {
            return;
        };
        // Determine the height each widget should occupy within the given area
        let total_height = area.height;
        let widget_height = total_height / widgets.len() as u16;

        for (i, widget) in widgets.iter().enumerate() {
            if let Ok(widget_ref) = widget.try_lock() {
                let widget_area = Rect {
                    x: area.x,
                    y: area.y + i as u16 * widget_height,
                    width: area.width,
                    height: widget_height,
                };
                widget_ref.render_ref(widget_area, buf);
            }
        }
    }
}
