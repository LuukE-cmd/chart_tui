use std::{
    fs,
    io::{self},
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
};

mod widgets;
use calamine::{open_workbook, Reader, Xlsx};
use widgets::picker_item_widget::*;
use widgets::picker_widget::*;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Styled,
    widgets::{Widget, WidgetRef},
    DefaultTerminal, Frame,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

type WidgetVector = Arc<Mutex<Vec<Arc<Mutex<dyn WidgetRef + Send + Sync>>>>>;

type EventQueueVector = Arc<Mutex<Vec<Weak<Mutex<dyn EventHandler + Send + Sync>>>>>;

#[derive(Default)]
pub struct App {
    exit: AtomicBool,
    event_queue: EventQueueVector,
    widgets: WidgetVector,
}

impl App {
    pub fn init(&mut self) -> io::Result<()> {
        let picker = self.file_picker()?;
        {
            let mut widget_list = self
                .widgets
                .try_lock()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock widget list"))?;
            widget_list.push(picker.clone())
        }
        self.add_event_handler(picker)?;
        Ok(())
    }

    fn file_picker(&mut self) -> io::Result<Arc<Mutex<Picker>>> {
        let title = "test".to_string();
        let picker = Arc::new(Mutex::new(Picker::new(
            vec![],
            title.clone(),
            Some(0),
            true,
        )));

        let current_dir = std::env::current_dir().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open current directory: {}", e),
            )
        })?;

        let items = self.xlsx_picker_items(current_dir, &picker);

        for item in &items {
            self.add_event_handler(item.clone())?;
        }

        picker.lock().unwrap().items = items;
        Ok(picker)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.init()?;
        while !self.exit.load(std::sync::atomic::Ordering::Acquire) {
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
        let handlers: Vec<_> = {
            let event_queue = self
                .event_queue
                .try_lock()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock event queue"))?;
            event_queue.iter().filter_map(Weak::upgrade).collect()
        };

        for handler in handlers {
            if let Ok(mut handler) = handler.try_lock() {
                handler.handle_event(&read_event)?;
            }
        }
        // Clean up any stale weak references
        self.event_queue
            .lock()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to lock event queue for cleanup",
                )
            })?
            .retain(|weak_handler| weak_handler.strong_count() > 0);
        Ok(())
    }

    fn add_event_handler(
        &mut self,
        handler: Arc<Mutex<dyn EventHandler + Send + Sync>>,
    ) -> io::Result<()> {
        let mut event_queue = self
            .event_queue
            .try_lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock event queue"))?;
        event_queue.push(Arc::downgrade(&handler));
        Ok(())
    }

    fn exit(&mut self) {
        self.exit.store(true, std::sync::atomic::Ordering::Relaxed);
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
            .map(|path| {
                build_open_file_func(
                    Arc::new(path.clone()),
                    self.widgets.clone(),
                    self.event_queue.clone(),
                )
            })
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

fn build_open_file_func(
    path: Arc<PathBuf>,
    widgets: WidgetVector,
    event_queue: EventQueueVector,
) -> CallbackFunction {
    let event_queue = event_queue.clone();
    let open_file_function: CallbackFunction = Some(Arc::new(Mutex::new(move || {
        //widgets.println("test: {} ", path.to_str().unwrap_or("Err"));
        let Some(path_str) = path.to_str() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "File path error"));
        };

        let mut workbook: Xlsx<_> = open_workbook(path_str).map_err(|e: calamine::XlsxError| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;

        let sheets = workbook.sheet_names();

        let title = "Pick worksheet".to_string();
        let picker: Arc<Mutex<Picker>> = Arc::new(Mutex::new(Picker::new(
            vec![],
            title.clone(),
            Some(0),
            true,
        )));

        let items: Vec<Arc<Mutex<PickerItem>>> = sheets
            .iter()
            .map(|sheet| {
                Arc::new(Mutex::new(PickerItem::new(
                    sheet,
                    false,
                    Arc::downgrade(&picker),
                    None,
                )))
            })
            .collect();
        if let Ok(mut locked_picker) = picker.try_lock() {
            locked_picker.items = items;
            locked_picker
                .items
                .first()
                .unwrap()
                .lock()
                .unwrap()
                .selected = true;
            locked_picker.update_selection()?;
        }

        add_event_handler(event_queue.clone(), picker.clone());

        if let Ok(mut locked_widgets) = widgets.lock() {
            locked_widgets.clear();
            locked_widgets.push(picker);
        }
        Ok(())
    })));
    open_file_function
}

fn add_event_handler(
    event_queue: EventQueueVector,
    handler: Arc<Mutex<dyn EventHandler + Send + Sync>>,
) {
    if let Ok(mut event_queue) = event_queue.try_lock() {
        event_queue.push(Arc::downgrade(&handler));
        eprintln!("{:?}", event_queue);
    } else {
        eprintln!("Poisoned lock in add event handler");
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Ok(widgets) = self.widgets.try_lock() else {
            return;
        };

        if widgets.len() == 0 {
            let default =
                ratatui::widgets::Block::new().set_style(ratatui::style::Style::default());
            default.render(area, buf);
            return;
        }

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
