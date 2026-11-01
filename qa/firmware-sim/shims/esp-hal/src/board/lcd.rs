//! ILI9342C: decodes the command stream the real driver sends into pixels.
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;

const COLUMN_ADDRESS_SET: u8 = 0x2A;
const ROW_ADDRESS_SET: u8 = 0x2B;
const MEMORY_WRITE: u8 = 0x2C;

struct Panel {
    pixels: Vec<u16>,
    command: u8,
    parameters: Vec<u8>,
    columns: (usize, usize),
    rows: (usize, usize),
    cursor: (usize, usize),
    high_byte: Option<u8>,
}

static PANEL: Mutex<Option<Panel>> = Mutex::new(None);
static PIXEL_WRITES: AtomicU64 = AtomicU64::new(0);

fn with_panel<T>(operation: impl FnOnce(&mut Panel) -> T) -> T {
    let mut panel = PANEL.lock().expect("panel model poisoned");
    operation(panel.get_or_insert_with(|| Panel {
        pixels: vec![0; WIDTH * HEIGHT],
        command: 0,
        parameters: Vec::new(),
        columns: (0, WIDTH - 1),
        rows: (0, HEIGHT - 1),
        cursor: (0, 0),
        high_byte: None,
    }))
}

pub fn receive(byte: u8, is_data: bool) {
    with_panel(|panel| if is_data { panel.data(byte) } else { panel.begin(byte) });
}

/// A count that moves whenever pixels change, so a settled screen is detectable.
pub fn pixel_writes() -> u64 {
    PIXEL_WRITES.load(Ordering::Relaxed)
}

/// RGB565 pixels in reading order.
pub fn snapshot() -> Vec<u16> {
    with_panel(|panel| panel.pixels.clone())
}

impl Panel {
    fn begin(&mut self, command: u8) {
        self.command = command;
        self.parameters.clear();
        self.high_byte = None;
        if command == MEMORY_WRITE {
            self.cursor = (self.columns.0, self.rows.0);
        }
    }

    fn data(&mut self, byte: u8) {
        match self.command {
            COLUMN_ADDRESS_SET => self.window_parameter(byte, true),
            ROW_ADDRESS_SET => self.window_parameter(byte, false),
            MEMORY_WRITE => self.pixel_byte(byte),
            _ => {}
        }
    }

    fn window_parameter(&mut self, byte: u8, is_column: bool) {
        self.parameters.push(byte);
        if let [start_high, start_low, end_high, end_low] = self.parameters[..] {
            let start = usize::from(u16::from_be_bytes([start_high, start_low]));
            let end = usize::from(u16::from_be_bytes([end_high, end_low]));
            if is_column {
                self.columns = (start, end);
            } else {
                self.rows = (start, end);
            }
        }
    }

    fn pixel_byte(&mut self, byte: u8) {
        let Some(high) = self.high_byte.take() else {
            self.high_byte = Some(byte);
            return;
        };
        let (x, y) = self.cursor;
        if x < WIDTH && y < HEIGHT {
            self.pixels[y * WIDTH + x] = u16::from_be_bytes([high, byte]);
            PIXEL_WRITES.fetch_add(1, Ordering::Relaxed);
        }
        self.cursor = if x >= self.columns.1 { (self.columns.0, y + 1) } else { (x + 1, y) };
    }
}
