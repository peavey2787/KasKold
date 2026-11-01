//! Everything that survives losing power, so a later process can be the same device rebooted.
use std::fs;
use std::io;
use std::path::Path;

use super::{flash, sd};

pub fn save(path: &Path) -> io::Result<()> {
    let flash = flash::image();
    let card = sd::image();
    let mut file = Vec::with_capacity(flash.len() + card.len() + 8);
    file.extend((flash.len() as u64).to_le_bytes());
    file.extend(flash);
    file.extend(card);
    fs::write(path, file)
}

pub fn load(path: &Path) -> io::Result<()> {
    let file = fs::read(path)?;
    let (length, rest) = file.split_at(8);
    let length = u64::from_le_bytes(length.try_into().expect("eight bytes")) as usize;
    let (image, card) = rest.split_at(length);
    flash::restore(image);
    sd::restore(card);
    Ok(())
}
