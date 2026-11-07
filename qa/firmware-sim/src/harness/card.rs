//! A card used on another machine: files put on it and read off it the way a
//! computer would, without going through any firmware code.
use esp_hal::board::sd;

const BLOCK: usize = 512;
const DIRECTORY_ENTRY: usize = 32;
const FAT_ENTRY: usize = 4;
const ARCHIVE: u8 = 0x20;
const VOLUME_LABEL: u8 = 0x08;
const DIRECTORY: u8 = 0x10;
const LONG_NAME: u8 = 0x0F;
const LAST_LONG_PART: u8 = 0x40;
const LONG_PART_CHARS: usize = 13;
const LONG_PART_SLOTS: [usize; LONG_PART_CHARS] = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
const COMPUTER_PARTITION_START: usize = 8192;
const COMPUTER_RESERVED_BLOCKS: u16 = 32;
const COMPUTER_BLOCKS_PER_CLUSTER: u8 = 64;
const COMPUTER_FAT_BLOCKS: u32 = 2048;
const COMPUTER_TOTAL_BLOCKS: u32 = 16 * 1024 * 1024 - COMPUTER_PARTITION_START as u32;
const ROOT_CLUSTER: u32 = 2;
const BOOT_SIGNATURE_AT: usize = 510;
const FAT32_LBA_PARTITION: u8 = 0x0C;
const FREE_ENTRY: u8 = 0x00;
const DELETED_ENTRY: u8 = 0xE5;
const END_OF_CHAIN: u32 = 0x0FFF_FFFF;
const FIRST_FILE_CLUSTER: u32 = 3;
const FAT_BLOCKS_CHECKED: usize = 64;
const FILESYSTEM_MARK: &[u8; 5] = b"FAT32";
const FILESYSTEM_MARK_AT: usize = 82;
const PARTITION_START_AT: usize = 454;

struct Volume {
    fat: usize,
    fat_blocks: usize,
    fats: usize,
    data: usize,
    per_cluster: usize,
    root_cluster: u32,
}

fn little_u16(block: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([block[at], block[at + 1]]))
}

fn little_u32(block: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(block[at..at + 4].try_into().expect("four bytes"))
}

fn volume() -> Volume {
    let first = sd::read_block(0);
    let start = if &first[FILESYSTEM_MARK_AT..FILESYSTEM_MARK_AT + 5] == FILESYSTEM_MARK {
        0
    } else {
        little_u32(&first, PARTITION_START_AT) as usize
    };
    let boot = sd::read_block(start);
    assert_eq!(
        &boot[FILESYSTEM_MARK_AT..FILESYSTEM_MARK_AT + 5],
        FILESYSTEM_MARK,
        "the card holds no FAT32 volume"
    );
    let fat = start + little_u16(&boot, 14);
    let fats = usize::from(boot[16]);
    let fat_blocks = little_u32(&boot, 36) as usize;
    Volume {
        fat,
        fat_blocks,
        fats,
        data: fat + fats * fat_blocks,
        per_cluster: usize::from(boot[13]),
        root_cluster: little_u32(&boot, 44),
    }
}

impl Volume {
    fn first_block(&self, cluster: u32) -> usize {
        self.data + (cluster as usize - 2) * self.per_cluster
    }

    fn cluster_bytes(&self) -> usize {
        self.per_cluster * BLOCK
    }

    fn next(&self, cluster: u32) -> u32 {
        let position = cluster as usize * FAT_ENTRY;
        let block = sd::read_block(self.fat + position / BLOCK);
        little_u32(&block, position % BLOCK) & END_OF_CHAIN
    }

    fn link(&self, cluster: u32, next: u32) {
        let position = cluster as usize * FAT_ENTRY;
        for copy in 0..self.fats {
            let at = self.fat + copy * self.fat_blocks + position / BLOCK;
            let mut block = sd::read_block(at);
            block[position % BLOCK..position % BLOCK + FAT_ENTRY]
                .copy_from_slice(&next.to_le_bytes());
            sd::write_block(at, &block);
        }
    }

    fn free_clusters(&self, count: usize) -> Vec<u32> {
        (FIRST_FILE_CLUSTER..)
            .filter(|cluster| self.next(*cluster) == 0)
            .take(count)
            .collect()
    }

    /// Every directory slot of the root, as (block, offset within it).
    fn root_slots(&self) -> Vec<(usize, usize)> {
        let mut slots = Vec::new();
        let mut cluster = self.root_cluster;
        while cluster < END_OF_CHAIN & !7 {
            let first = self.first_block(cluster);
            for block in first..first + self.per_cluster {
                slots.extend(
                    (0..BLOCK / DIRECTORY_ENTRY).map(|slot| (block, slot * DIRECTORY_ENTRY)),
                );
            }
            cluster = self.next(cluster);
        }
        slots
    }

    fn write_chain(&self, clusters: &[u32], contents: &[u8]) {
        for (cluster, part) in clusters.iter().zip(contents.chunks(self.cluster_bytes())) {
            let first = self.first_block(*cluster);
            for (offset, piece) in part.chunks(BLOCK).enumerate() {
                let mut block = [0u8; BLOCK];
                block[..piece.len()].copy_from_slice(piece);
                sd::write_block(first + offset, &block);
            }
        }
        for pair in clusters.windows(2) {
            self.link(pair[0], pair[1]);
        }
        self.link(
            *clusters.last().expect("a file holds a cluster"),
            END_OF_CHAIN,
        );
    }

    fn write_entry(&self, name: &[u8; 11], first_cluster: u32, size: usize) {
        self.write_entry_with(name, ARCHIVE, first_cluster, size);
    }

    fn write_entry_with(&self, name: &[u8; 11], kind: u8, first_cluster: u32, size: usize) {
        let mut entry = [0u8; DIRECTORY_ENTRY];
        entry[..11].copy_from_slice(name);
        entry[11] = kind;
        entry[20..22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
        entry[26..28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
        entry[28..32].copy_from_slice(&(size as u32).to_le_bytes());
        self.append_raw_entry(&entry);
    }

    fn append_raw_entry(&self, raw: &[u8; DIRECTORY_ENTRY]) {
        let (block_at, offset) = self
            .root_slots()
            .into_iter()
            .find(|(block, offset)| {
                matches!(sd::read_block(*block)[*offset], FREE_ENTRY | DELETED_ENTRY)
            })
            .expect("room in the root directory");
        let mut block = sd::read_block(block_at);
        block[offset..offset + DIRECTORY_ENTRY].copy_from_slice(raw);
        sd::write_block(block_at, &block);
    }

    /// The long-name entries a computer writes just before a file's own entry.
    fn append_long_name(&self, long: &str, short: &[u8; 11]) {
        let checksum = short
            .iter()
            .fold(0u8, |sum, byte| sum.rotate_right(1).wrapping_add(*byte));
        let characters: Vec<u16> = long.encode_utf16().chain([0]).collect();
        let parts = long.encode_utf16().count().div_ceil(LONG_PART_CHARS);
        for part in (0..parts).rev() {
            let mut entry = [0xFFu8; DIRECTORY_ENTRY];
            let sequence = part as u8 + 1;
            entry[0] = sequence | if part + 1 == parts { LAST_LONG_PART } else { 0 };
            entry[11] = LONG_NAME;
            entry[12] = 0;
            entry[13] = checksum;
            entry[26..28].fill(0);
            let piece = part * LONG_PART_CHARS..(part + 1) * LONG_PART_CHARS;
            let piece = characters.get(piece.start..piece.end.min(characters.len()));
            for (slot, character) in LONG_PART_SLOTS.iter().zip(piece.unwrap_or_default()) {
                entry[*slot..*slot + 2].copy_from_slice(&character.to_le_bytes());
            }
            self.append_raw_entry(&entry);
        }
    }

    /// Every cluster a file's chain walks through.
    fn chain_of(&self, first: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut cluster = first;
        while (ROOT_CLUSTER..END_OF_CHAIN & !7).contains(&cluster) {
            chain.push(cluster);
            cluster = self.next(cluster);
        }
        chain
    }

    /// What a disk checker looks at: one owner per cluster, chains that fit the
    /// sizes, and both copies of the table alike. Nothing is repaired.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut owners = std::collections::HashMap::new();
        let mut long_parts_waiting = false;
        for (block_at, offset) in self.root_slots() {
            let block = sd::read_block(block_at);
            let entry = &block[offset..offset + DIRECTORY_ENTRY];
            if entry[0] == FREE_ENTRY {
                break;
            }
            if entry[0] == DELETED_ENTRY {
                if long_parts_waiting {
                    problems.push("a long name is left over from a deleted file".to_string());
                }
                long_parts_waiting = false;
                continue;
            }
            if entry[11] == LONG_NAME {
                long_parts_waiting = true;
                continue;
            }
            long_parts_waiting = false;
            if entry[11] & VOLUME_LABEL != 0 {
                continue;
            }
            let name = String::from_utf8_lossy(&entry[..11]).into_owned();
            let first = (little_u16(entry, 20) as u32) << 16 | little_u16(entry, 26) as u32;
            let chain = self.chain_of(first);
            for cluster in &chain {
                if let Some(other) = owners.insert(*cluster, name.clone()) {
                    problems.push(format!("{name} and {other} share cluster {cluster}"));
                }
            }
            let size = little_u32(entry, 28) as usize;
            if entry[11] & DIRECTORY == 0 && chain.len() != size.div_ceil(self.cluster_bytes()) {
                problems.push(format!(
                    "{name} holds {size} bytes in {} clusters",
                    chain.len()
                ));
            }
        }
        for at in 0..self.fat_blocks.min(FAT_BLOCKS_CHECKED) {
            for copy in 1..self.fats {
                if sd::read_block(self.fat + at)
                    != sd::read_block(self.fat + copy * self.fat_blocks + at)
                {
                    problems.push(format!("copy {copy} of the table differs at block {at}"));
                }
            }
        }
        problems
    }

    fn delete_entry(&self, name: &[u8; 11]) {
        for (block_at, offset) in self.root_slots() {
            let mut block = sd::read_block(block_at);
            if &block[offset..offset + 11] == name {
                block[offset] = DELETED_ENTRY;
                sd::write_block(block_at, &block);
            }
        }
    }

    fn find_entry(&self, name: &[u8; 11]) -> Option<(u32, usize)> {
        self.root_slots()
            .into_iter()
            .find_map(|(block_at, offset)| {
                let block = sd::read_block(block_at);
                let entry = &block[offset..offset + DIRECTORY_ENTRY];
                (&entry[..11] == name).then(|| {
                    let high = little_u16(entry, 20) as u32;
                    let low = little_u16(entry, 26) as u32;
                    (high << 16 | low, little_u32(entry, 28) as usize)
                })
            })
    }
}

/// Copies a file onto the card's root directory, replacing any file of that name.
/// The name is its 8.3 directory form.
pub(crate) fn put_file(name: &[u8; 11], contents: &[u8]) {
    let volume = volume();
    volume.delete_entry(name);
    let clusters = volume.free_clusters(contents.len().div_ceil(volume.cluster_bytes()).max(1));
    volume.write_chain(&clusters, contents);
    volume.write_entry(name, clusters[0], contents.len());
}

/// The contents of a file in the card's root directory, if it is there.
pub(crate) fn read_file(name: &[u8; 11]) -> Option<Vec<u8>> {
    let volume = volume();
    let (mut cluster, size) = volume.find_entry(name)?;
    let mut contents = Vec::with_capacity(size);
    while contents.len() < size {
        let first = volume.first_block(cluster);
        for block in first..first + volume.per_cluster {
            contents.extend_from_slice(&sd::read_block(block));
        }
        cluster = volume.next(cluster);
    }
    contents.truncate(size);
    Some(contents)
}

/// Every block a file in the card's root directory occupies.
pub(crate) fn blocks_of(name: &[u8; 11]) -> Vec<usize> {
    let volume = volume();
    let (first, _) = volume.find_entry(name).expect("the file is on the card");
    volume
        .chain_of(first)
        .into_iter()
        .flat_map(|cluster| {
            let start = volume.first_block(cluster);
            start..start + volume.per_cluster
        })
        .collect()
}

/// Fails unless every one of these blocks holds only zeros.
pub(crate) fn expect_blank(blocks: &[usize]) {
    for &block in blocks {
        assert!(
            sd::read_block(block).iter().all(|byte| *byte == 0),
            "block {block} still holds a deleted file's bytes"
        );
    }
}

/// Erases the card and formats it the way a computer does: a partition table, and a
/// volume that starts later on the card with large clusters.
pub(crate) fn format_like_a_computer() {
    let mut table = [0u8; BLOCK];
    table[446 + 4] = FAT32_LBA_PARTITION;
    table[PARTITION_START_AT..PARTITION_START_AT + 4]
        .copy_from_slice(&(COMPUTER_PARTITION_START as u32).to_le_bytes());
    table[PARTITION_START_AT + 4..PARTITION_START_AT + 8]
        .copy_from_slice(&COMPUTER_TOTAL_BLOCKS.to_le_bytes());
    table[BOOT_SIGNATURE_AT..].copy_from_slice(&[0x55, 0xAA]);
    sd::write_block(0, &table);

    let mut boot = [0u8; BLOCK];
    boot[..3].copy_from_slice(&[0xEB, 0x58, 0x90]);
    boot[3..11].copy_from_slice(b"MSDOS5.0");
    boot[11..13].copy_from_slice(&(BLOCK as u16).to_le_bytes());
    boot[13] = COMPUTER_BLOCKS_PER_CLUSTER;
    boot[14..16].copy_from_slice(&COMPUTER_RESERVED_BLOCKS.to_le_bytes());
    boot[16] = 2;
    boot[21] = 0xF8;
    boot[32..36].copy_from_slice(&COMPUTER_TOTAL_BLOCKS.to_le_bytes());
    boot[36..40].copy_from_slice(&COMPUTER_FAT_BLOCKS.to_le_bytes());
    boot[44..48].copy_from_slice(&ROOT_CLUSTER.to_le_bytes());
    boot[48..50].copy_from_slice(&1u16.to_le_bytes());
    boot[50..52].copy_from_slice(&6u16.to_le_bytes());
    boot[FILESYSTEM_MARK_AT..FILESYSTEM_MARK_AT + 5].copy_from_slice(FILESYSTEM_MARK);
    boot[BOOT_SIGNATURE_AT..].copy_from_slice(&[0x55, 0xAA]);
    sd::write_block(COMPUTER_PARTITION_START, &boot);

    let reserved = COMPUTER_PARTITION_START + usize::from(COMPUTER_RESERVED_BLOCKS);
    let blank = [0u8; BLOCK];
    for at in reserved..reserved + 2 * COMPUTER_FAT_BLOCKS as usize {
        sd::write_block(at, &blank);
    }
    let volume = volume();
    for cluster in [0, 1] {
        volume.link(cluster, END_OF_CHAIN);
    }
    let first = volume.first_block(ROOT_CLUSTER);
    for at in first..first + volume.per_cluster {
        sd::write_block(at, &blank);
    }
    volume.link(ROOT_CLUSTER, END_OF_CHAIN);
}

/// Puts a file on the card under a long name; `short` is the name the computer derived.
pub(crate) fn put_long_named_file(long: &str, short: &[u8; 11], contents: &[u8]) {
    let volume = volume();
    let clusters = volume.free_clusters(contents.len().div_ceil(volume.cluster_bytes()).max(1));
    volume.write_chain(&clusters, contents);
    volume.append_long_name(long, short);
    volume.write_entry(short, clusters[0], contents.len());
}

/// As a card that has been used for a while: the file's clusters are not side by side.
pub(crate) fn put_scattered_file(long: &str, short: &[u8; 11], contents: &[u8]) {
    let volume = volume();
    let count = contents.len().div_ceil(volume.cluster_bytes()).max(1);
    let clusters: Vec<u32> = volume
        .free_clusters(2 * count)
        .into_iter()
        .step_by(2)
        .collect();
    volume.write_chain(&clusters, contents);
    volume.append_long_name(long, short);
    volume.write_entry(short, clusters[0], contents.len());
}

pub(crate) fn put_volume_label(label: &[u8; 11]) {
    volume().write_entry_with(label, VOLUME_LABEL, 0, 0);
}

/// A folder the firmware has no reason to enter.
pub(crate) fn put_folder(long: &str, short: &[u8; 11]) {
    let volume = volume();
    let cluster = volume.free_clusters(1)[0];
    volume.write_chain(&[cluster], &[0u8; DIRECTORY_ENTRY]);
    volume.append_long_name(long, short);
    volume.write_entry_with(short, DIRECTORY, cluster, 0);
}

/// A file a computer already deleted: its entries remain, marked free.
pub(crate) fn put_deleted_file(long: &str, short: &[u8; 11]) {
    let volume = volume();
    volume.append_long_name(long, short);
    volume.write_entry(short, 0, 0);
    for (block_at, offset) in volume.root_slots() {
        let mut block = sd::read_block(block_at);
        let entry = &block[offset..offset + DIRECTORY_ENTRY];
        if entry[0] != FREE_ENTRY && (entry[11] == LONG_NAME || &entry[..11] == short) {
            block[offset] = DELETED_ENTRY;
            sd::write_block(block_at, &block);
        }
    }
}

/// Panics on anything a disk checker on a computer would report about this card.
pub(crate) fn expect_a_sound_volume() {
    let problems = volume().problems();
    assert!(problems.is_empty(), "the card is damaged: {problems:#?}");
}
