use calamine::{Data, Range};
use ratatui::{style::Style, widgets::WidgetRef};

const ITEM_HEIGHT: usize = 4;

pub struct XlsxTableWidget {
    range: Range<Data>,
    column_widths: Vec<usize>,
}

#[allow(dead_code)]
impl XlsxTableWidget {
    pub fn new(range: Range<Data>) -> Self {
        let mut column_widths = vec![0; range.width()];
        for row in range.rows() {
            for (col, cell) in row.iter().enumerate() {
                let width = match cell {
                    Data::String(s) => s.len(),
                    Data::Int(_) | Data::Float(_) | Data::Bool(_) => 8,
                    _ => 0,
                };
                column_widths[col] = column_widths[col].max(width);
            }
        }
        Self {
            range,
            column_widths,
        }
    }
}

impl WidgetRef for XlsxTableWidget {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let max_visible_rows = area.height as usize / ITEM_HEIGHT;

        for (row_idx, row) in self.range.rows().take(max_visible_rows).enumerate() {
            let y = area.top() + (row_idx as u16 * ITEM_HEIGHT as u16);

            if y >= area.bottom() {
                break;
            }

            let mut x = area.left();

            for (col_idx, cell) in row.iter().enumerate() {
                let text = match cell {
                    Data::String(s) => s.clone(),
                    Data::Int(i) => i.to_string(),
                    Data::Float(f) => format!("{:.2}", f),
                    Data::Bool(b) => b.to_string(),
                    _ => String::new(),
                };

                let width = self.column_widths[col_idx];
                let display_text = if text.len() > width {
                    text[..width].to_string()
                } else {
                    format!("{:width$}", text, width = width)
                };

                buf.set_string(x, y, &display_text, Style::default());

                x += width as u16 + 1;
                if x >= area.right() {
                    break;
                }
            }
        }
    }
}
