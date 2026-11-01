//! An SD card in SPI mode, with sparse block storage so it can be as large as a real one.
use std::collections::HashMap;
use std::sync::Mutex;

const BLOCK: usize = 512;
const CARD_BLOCKS: usize = 16 * 1024 * 1024;
const CMD_START: u8 = 0x40;
const IDLE: u8 = 0xFF;
const R1_IDLE: u8 = 0x01;
const R1_READY: u8 = 0x00;
const R1_ILLEGAL: u8 = 0x04;
const DATA_TOKEN: u8 = 0xFE;
const DATA_ACCEPTED: u8 = 0x05;
const OCR_HIGH_CAPACITY: u8 = 0xC0;

#[derive(Default)]
struct Card {
    blocks: HashMap<usize, [u8; BLOCK]>,
    frame: Vec<u8>,
    response: std::collections::VecDeque<u8>,
    ready: bool,
    app_command: bool,
    writing: Option<usize>,
    write_buffer: Vec<u8>,
    in_write_payload: bool,
    present: bool,
}

static CARD: Mutex<Option<Card>> = Mutex::new(None);

fn with_card<T>(operation: impl FnOnce(&mut Card) -> T) -> T {
    let mut card = CARD.lock().expect("sd model poisoned");
    operation(card.get_or_insert_with(Card::default))
}

pub fn set_present(present: bool) {
    with_card(|card| card.present = present);
}

pub fn exchange(sent: u8) -> u8 {
    with_card(|card| {
        if card.present {
            card.exchange(sent)
        } else {
            IDLE
        }
    })
}

/// The written blocks and whether the card is inserted, as one byte string.
pub fn image() -> Vec<u8> {
    with_card(|card| {
        let mut image = vec![u8::from(card.present)];
        for (block, data) in &card.blocks {
            image.extend((*block as u64).to_le_bytes());
            image.extend(data);
        }
        image
    })
}

pub fn restore(image: &[u8]) {
    let (present, blocks) = image.split_first().expect("card image");
    with_card(|card| {
        card.present = *present != 0;
        for entry in blocks.chunks_exact(8 + BLOCK) {
            let (block, data) = entry.split_at(8);
            let block = u64::from_le_bytes(block.try_into().expect("eight bytes")) as usize;
            card.blocks
                .insert(block, data.try_into().expect("one block"));
        }
    });
}

/// One block of the card, as a reader on another machine would see it.
pub fn read_block(block: usize) -> [u8; BLOCK] {
    with_card(|card| card.block(block))
}

impl Card {
    fn block(&self, block: usize) -> [u8; BLOCK] {
        self.blocks.get(&block).copied().unwrap_or([0; BLOCK])
    }

    fn exchange(&mut self, sent: u8) -> u8 {
        if self.writing.is_some() {
            return self.exchange_write(sent);
        }
        let out = self.response.pop_front().unwrap_or(IDLE);
        self.collect_command(sent);
        out
    }

    fn collect_command(&mut self, sent: u8) {
        if self.frame.is_empty() && sent & 0xC0 != CMD_START {
            return;
        }
        self.frame.push(sent);
        if self.frame.len() == 6 {
            let frame = core::mem::take(&mut self.frame);
            let argument = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]);
            self.execute(frame[0] & 0x3F, argument);
        }
    }

    fn execute(&mut self, command: u8, argument: u32) {
        self.response.clear();
        self.response.push_back(IDLE);
        let app = core::mem::take(&mut self.app_command);
        match (app, command) {
            (_, 0) => self.respond(R1_IDLE, &[]),
            (_, 8) => self.respond(R1_IDLE, &[0x00, 0x00, 0x01, 0xAA]),
            (_, 55) => {
                self.app_command = true;
                self.respond(self.status(), &[]);
            }
            (true, 41) => {
                self.ready = true;
                self.respond(R1_READY, &[]);
            }
            (_, 58) => self.respond(self.status(), &[OCR_HIGH_CAPACITY, 0xFF, 0x80, 0x00]),
            (_, 13) => self.respond(self.status(), &[0x00]),
            (_, 16 | 12) => self.respond(self.status(), &[]),
            (_, 9) => self.send_csd(),
            (_, 10) => self.send_register(&[0u8; 16]),
            (_, 17) => self.send_block(argument as usize),
            (_, 24) => self.begin_write(argument as usize),
            _ => self.respond(R1_ILLEGAL, &[]),
        }
    }

    fn status(&self) -> u8 {
        if self.ready {
            R1_READY
        } else {
            R1_IDLE
        }
    }

    fn respond(&mut self, r1: u8, tail: &[u8]) {
        self.response.push_back(r1);
        self.response.extend(tail.iter().copied());
    }

    /// CSD v2: capacity = (C_SIZE + 1) * 512 KiB.
    fn send_csd(&mut self) {
        let size = (CARD_BLOCKS / 1024 - 1) as u32;
        let mut csd = [0u8; 16];
        csd[0] = 0x40;
        csd[7] = (size >> 16) as u8 & 0x3F;
        csd[8] = (size >> 8) as u8;
        csd[9] = size as u8;
        self.send_register(&csd);
    }

    fn send_register(&mut self, bytes: &[u8]) {
        self.respond(R1_READY, &[IDLE, DATA_TOKEN]);
        self.response.extend(bytes.iter().copied());
        self.response.extend([0xFF, 0xFF]);
    }

    fn send_block(&mut self, block: usize) {
        self.respond(R1_READY, &[IDLE, DATA_TOKEN]);
        self.response.extend(self.block(block));
        self.response.extend([0xFF, 0xFF]);
    }

    fn begin_write(&mut self, block: usize) {
        self.respond(R1_READY, &[]);
        self.writing = Some(block);
        self.write_buffer.clear();
        self.in_write_payload = false;
    }

    fn exchange_write(&mut self, sent: u8) -> u8 {
        if let Some(out) = self.response.pop_front() {
            return out;
        }
        if !self.in_write_payload {
            self.in_write_payload = sent == DATA_TOKEN;
            return IDLE;
        }
        self.write_buffer.push(sent);
        if self.write_buffer.len() < BLOCK + 2 {
            return IDLE;
        }
        let block = self.writing.take().unwrap_or(0);
        let mut payload = [0; BLOCK];
        payload.copy_from_slice(&self.write_buffer[..BLOCK]);
        self.blocks.insert(block, payload);
        self.response.push_back(DATA_ACCEPTED);
        IDLE
    }
}
