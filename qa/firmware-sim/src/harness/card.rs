//! A card used on another machine: files put on it and read off it the way a
//! computer would, without going through any firmware code.
use esp_hal::board::sd;

const BLOCK: usize = 512;
const DIRECTORY_ENTRY: usize = 32;
const FAT_ENTRY: usize = 4;
const ARCHIVE: u8 = 0x20;
const FREE_ENTRY: u8 = 0x00;
const DELETED_ENTRY: u8 = 0xE5;
const END_OF_CHAIN: u32 = 0x0FFF_FFFF;
const FIRST_FILE_CLUSTER: u32 = 3;
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
        let (block_at, offset) = self
            .root_slots()
            .into_iter()
            .find(|(block, offset)| {
                matches!(sd::read_block(*block)[*offset], FREE_ENTRY | DELETED_ENTRY)
            })
            .expect("room in the root directory");
        let mut block = sd::read_block(block_at);
        let entry = &mut block[offset..offset + DIRECTORY_ENTRY];
        entry.fill(0);
        entry[..11].copy_from_slice(name);
        entry[11] = ARCHIVE;
        entry[20..22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
        entry[26..28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
        entry[28..32].copy_from_slice(&(size as u32).to_le_bytes());
        sd::write_block(block_at, &block);
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
