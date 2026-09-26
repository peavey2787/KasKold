use crate::seed_entropy::{
    mix_additive_dice, mix_additive_touch, mix_additive_touch_transcript, MAX_ADDITIVE_DICE_ROLLS,
};

#[test]
fn additive_dice_validates_bounds_and_changes_pool() {
    let original = [0x11u8; 32];
    let mut pool = original;
    assert!(mix_additive_dice(&mut pool, &[1, 2, 3, 4, 5, 6]));
    assert_ne!(pool, original);

    let mut invalid = original;
    assert!(!mix_additive_dice(&mut invalid, &[]));
    assert_eq!(invalid, original);
    assert!(!mix_additive_dice(&mut invalid, &[0]));
    assert!(!mix_additive_dice(&mut invalid, &[7]));
    let too_many_rolls = [1u8; MAX_ADDITIVE_DICE_ROLLS + 1];
    assert!(!mix_additive_dice(&mut invalid, &too_many_rolls));
}

#[test]
fn additive_touch_digest_is_consumed_and_zeroized() {
    let mut pool = [0x22u8; 32];
    let original = pool;
    let mut digest = [0x33u8; 32];
    mix_additive_touch(&mut pool, &mut digest);
    assert_ne!(pool, original);
    assert_eq!(digest, [0u8; 32]);
}

#[test]
fn web_touch_transcript_is_additive_and_bounded() {
    let mut pool = [0x44u8; 32];
    let original = pool;
    assert!(mix_additive_touch_transcript(
        &mut pool,
        b"pointer transcript"
    ));
    assert_ne!(pool, original);

    let mut empty = original;
    assert!(!mix_additive_touch_transcript(&mut empty, b""));
    assert_eq!(empty, original);
}
