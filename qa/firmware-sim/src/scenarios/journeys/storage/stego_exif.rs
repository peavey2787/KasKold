//! A photo from a real camera arrives with its own EXIF: hiding a phrase keeps it readable.
use super::stego::{
    hide_portably, restore_portably_from_the_photo, DESCRIPTOR, LOWER_CARD, NOISE_PHOTO,
    PHOTO_FILE, UPPER_CARD,
};
use crate::harness::card;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};

const CAMERA_MAKER: &[u8] = b"Acme Camera Co\0";
const TAKEN_AT: &[u8] = b"2024:06:01 18:30:00\0";
const OLD_CAPTION: &[u8] = b"the caption the camera wrote\0";
const LANDSCAPE: u16 = 6;
const TAG_IMAGE_DESCRIPTION: u16 = 0x010E;
const TAG_MAKE: u16 = 0x010F;
const TAG_ORIENTATION: u16 = 0x0112;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_TAKEN_AT: u16 = 0x9003;
const TAG_USER_COMMENT: u16 = 0x9286;
const ASCII: u16 = 2;
const SHORT: u16 = 3;
const LONG: u16 = 4;
const ENTRY_SIZE: usize = 12;
const JFIF_END: usize = 20;
const COMMENT_CHARSET_SIZE: usize = 8;

journey!(
    a_phone_photo_keeps_its_details_and_gives_the_phrase_back:
    restore_the_known_wallet,
    hide_in_a_little_endian_photo,
    walk::factory_reset,
    restore_portably_from_the_photo,
);
journey!(
    a_camera_photo_keeps_its_details_and_gives_the_phrase_back:
    restore_the_known_wallet,
    hide_in_a_big_endian_photo,
    walk::factory_reset,
    restore_portably_from_the_photo,
);
journey!(
    a_phrase_hidden_in_the_picture_leaves_the_photo_details_alone:
    restore_the_known_wallet,
    hide_in_the_picture_of_a_camera_photo,
    walk::factory_reset,
    restore_portably_from_the_photo,
);
journey!(
    hiding_again_leaves_nothing_of_the_earlier_phrase:
    restore_the_known_wallet,
    hide_in_a_little_endian_photo,
    hide_again_in_the_same_photo,
    walk::factory_reset,
    restore_portably_from_the_photo,
);

#[derive(Clone, Copy)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    value: u32,
}

/// The EXIF of a photo, read the way any other program reads it.
struct Details<'a> {
    tiff: &'a [u8],
    little_endian: bool,
}

impl<'a> Details<'a> {
    fn of(photo: &'a [u8]) -> Self {
        let tiff = exif_segment(photo).expect("the photo lost its EXIF");
        let little_endian = match &tiff[..2] {
            b"II" => true,
            b"MM" => false,
            order => panic!("the photo has no byte order: {order:?}"),
        };
        Self {
            tiff,
            little_endian,
        }
    }

    fn u16_at(&self, offset: usize) -> u16 {
        let bytes = [self.tiff[offset], self.tiff[offset + 1]];
        if self.little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        }
    }

    fn u32_at(&self, offset: usize) -> u32 {
        let bytes: [u8; 4] = self.tiff[offset..offset + 4]
            .try_into()
            .expect("four bytes");
        if self.little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    }

    fn entries_at(&self, directory: usize) -> Vec<Entry> {
        (0..usize::from(self.u16_at(directory)))
            .map(|index| {
                let at = directory + 2 + index * ENTRY_SIZE;
                Entry {
                    tag: self.u16_at(at),
                    kind: self.u16_at(at + 2),
                    count: self.u32_at(at + 4),
                    value: self.u32_at(at + 8),
                }
            })
            .collect()
    }

    fn first_directory(&self) -> Vec<Entry> {
        self.entries_at(self.u32_at(4) as usize)
    }

    fn value_of(&self, entry: Entry) -> &'a [u8] {
        let start = entry.value as usize;
        &self.tiff[start..start + entry.count as usize]
    }

    fn lookup(&self, entries: &[Entry], tag: u16) -> Entry {
        let found = entries.iter().filter(|entry| entry.tag == tag);
        let mut matches = found.copied();
        let entry = matches
            .next()
            .unwrap_or_else(|| panic!("the photo lost tag {tag:#06x}"));
        assert!(matches.next().is_none(), "tag {tag:#06x} appears twice");
        entry
    }

    fn expect_the_camera_details_survived(&self) {
        let first = self.first_directory();
        let tags: Vec<u16> = first.iter().map(|entry| entry.tag).collect();
        assert!(
            tags.windows(2).all(|pair| pair[0] < pair[1]),
            "other programs need the tags in order: {tags:04x?}"
        );
        assert_eq!(self.value_of(self.lookup(&first, TAG_MAKE)), CAMERA_MAKER);
        let orientation = self.lookup(&first, TAG_ORIENTATION);
        assert_eq!(orientation.kind, SHORT);
        let inline_at = self.inline_value_offset(TAG_ORIENTATION);
        assert_eq!(self.u16_at(inline_at), LANDSCAPE);
        let exif_ifd = self.lookup(&first, TAG_EXIF_IFD).value as usize;
        let exif = self.entries_at(exif_ifd);
        assert_eq!(self.value_of(self.lookup(&exif, TAG_TAKEN_AT)), TAKEN_AT);
    }

    fn inline_value_offset(&self, tag: u16) -> usize {
        let directory = self.u32_at(4) as usize;
        let index = self
            .first_directory()
            .iter()
            .position(|entry| entry.tag == tag)
            .expect("the tag is present");
        directory + 2 + index * ENTRY_SIZE + 8
    }

    fn expect_the_new_caption(&self) {
        let first = self.first_directory();
        let caption = self.value_of(self.lookup(&first, TAG_IMAGE_DESCRIPTION));
        assert_eq!(&caption[..caption.len() - 1], DESCRIPTOR.as_bytes());
    }

    fn hidden_comment(&self) -> Vec<u8> {
        let first = self.first_directory();
        let comment = self.value_of(self.lookup(&first, TAG_USER_COMMENT));
        comment[COMMENT_CHARSET_SIZE..].to_vec()
    }
}

fn exif_segment(photo: &[u8]) -> Option<&[u8]> {
    let mut position = 2;
    while photo.get(position) == Some(&0xFF) && photo[position + 1] != 0xDA {
        let size = usize::from(u16::from_be_bytes([
            photo[position + 2],
            photo[position + 3],
        ]));
        let segment = &photo[position + 4..position + 2 + size];
        if photo[position + 1] == 0xE1 && segment.starts_with(b"Exif\0\0") {
            return Some(&segment[6..]);
        }
        position += 2 + size;
    }
    None
}

/// The test photo as a camera would save it, in the camera's byte order.
fn camera_photo(little_endian: bool) -> Vec<u8> {
    let u16_bytes = |value: u16| {
        if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    let u32_bytes = |value: u32| {
        if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    const FIRST_DIRECTORY: usize = 8;
    const FIRST_ENTRIES: usize = 4;
    const EXIF_ENTRIES: usize = 1;
    let exif_directory = FIRST_DIRECTORY + 2 + FIRST_ENTRIES * ENTRY_SIZE + 4;
    let maker_at = exif_directory + 2 + EXIF_ENTRIES * ENTRY_SIZE + 4;
    let caption_at = maker_at + CAMERA_MAKER.len();
    let taken_at = caption_at + OLD_CAPTION.len();

    let mut tiff = Vec::new();
    tiff.extend_from_slice(if little_endian { b"II" } else { b"MM" });
    tiff.extend_from_slice(&u16_bytes(42));
    tiff.extend_from_slice(&u32_bytes(u32::try_from(FIRST_DIRECTORY).unwrap()));
    let mut entry = |tiff: &mut Vec<u8>, tag: u16, kind: u16, count: usize, value: [u8; 4]| {
        tiff.extend_from_slice(&u16_bytes(tag));
        tiff.extend_from_slice(&u16_bytes(kind));
        tiff.extend_from_slice(&u32_bytes(u32::try_from(count).unwrap()));
        tiff.extend_from_slice(&value);
    };
    tiff.extend_from_slice(&u16_bytes(u16::try_from(FIRST_ENTRIES).unwrap()));
    entry(
        &mut tiff,
        TAG_IMAGE_DESCRIPTION,
        ASCII,
        OLD_CAPTION.len(),
        u32_bytes(u32::try_from(caption_at).unwrap()),
    );
    entry(
        &mut tiff,
        TAG_MAKE,
        ASCII,
        CAMERA_MAKER.len(),
        u32_bytes(u32::try_from(maker_at).unwrap()),
    );
    let [low, high] = u16_bytes(LANDSCAPE);
    entry(&mut tiff, TAG_ORIENTATION, SHORT, 1, [low, high, 0, 0]);
    entry(
        &mut tiff,
        TAG_EXIF_IFD,
        LONG,
        1,
        u32_bytes(u32::try_from(exif_directory).unwrap()),
    );
    tiff.extend_from_slice(&[0; 4]);
    tiff.extend_from_slice(&u16_bytes(u16::try_from(EXIF_ENTRIES).unwrap()));
    entry(
        &mut tiff,
        TAG_TAKEN_AT,
        ASCII,
        TAKEN_AT.len(),
        u32_bytes(u32::try_from(taken_at).unwrap()),
    );
    tiff.extend_from_slice(&[0; 4]);
    tiff.extend_from_slice(CAMERA_MAKER);
    tiff.extend_from_slice(OLD_CAPTION);
    tiff.extend_from_slice(TAKEN_AT);

    let mut photo = NOISE_PHOTO[..JFIF_END].to_vec();
    photo.extend_from_slice(&[0xFF, 0xE1]);
    photo.extend_from_slice(&(u16::try_from(2 + 6 + tiff.len()).unwrap()).to_be_bytes());
    photo.extend_from_slice(b"Exif\0\0");
    photo.extend_from_slice(&tiff);
    photo.extend_from_slice(&NOISE_PHOTO[JFIF_END..]);
    photo
}

fn saved_photo() -> Vec<u8> {
    card::read_file(PHOTO_FILE).expect("the photo is still on the card")
}

fn hide_in_descriptor_of(little_endian: bool) {
    hide_portably(&camera_photo(little_endian), UPPER_CARD);
    let photo = saved_photo();
    let details = Details::of(&photo);
    details.expect_the_camera_details_survived();
    details.expect_the_new_caption();
    assert!(!details.hidden_comment().is_empty());
    assert!(
        !walk::contains(&photo, OLD_CAPTION),
        "the camera's caption was replaced but is still in the file"
    );
}

fn hide_in_a_little_endian_photo() {
    hide_in_descriptor_of(true);
}

fn hide_in_a_big_endian_photo() {
    hide_in_descriptor_of(false);
}

fn hide_in_the_picture_of_a_camera_photo() {
    hide_portably(&camera_photo(false), LOWER_CARD);
    Details::of(&saved_photo()).expect_the_camera_details_survived();
}

fn hide_again_in_the_same_photo() {
    let earlier = saved_photo();
    let earlier_comment = Details::of(&earlier).hidden_comment();
    hide_portably(&earlier, UPPER_CARD);
    let photo = saved_photo();
    let details = Details::of(&photo);
    details.expect_the_camera_details_survived();
    assert_ne!(details.hidden_comment(), earlier_comment);
    assert!(
        !walk::contains(&photo, &earlier_comment),
        "the earlier backup is still in the photo"
    );
}
