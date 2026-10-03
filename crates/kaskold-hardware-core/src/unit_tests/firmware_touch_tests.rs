use crate::input::{
    camera_touch::{
        classify_immediate_touch, CameraTouchEffect, CameraTouchEvent, CameraTouchInput,
    },
    touch::{
        decode_gesture_byte, decode_rotated_single_touch, decode_touch_event_flag, HwGesture,
        ImmediateTouchTracker, TouchAction, TouchEventType, TouchPoint, TouchState, TouchTracker,
        TouchZone,
    },
};

fn point(x: u16, y: u16, event: TouchEventType) -> TouchState {
    TouchState::One(TouchPoint { x, y, event })
}

#[test]
fn touch_zone_edges_are_half_open_and_saturating() {
    let zone = TouchZone::new(100, 50, 120, 60);
    assert!(zone.contains(100, 50));
    assert!(zone.contains(219, 109));
    assert!(!zone.contains(220, 50));
    assert!(!zone.contains(100, 110));

    let edge = TouchZone::new(u16::MAX - 1, u16::MAX - 1, 10, 10);
    assert!(edge.contains(u16::MAX - 1, u16::MAX - 1));
    assert!(edge.contains(u16::MAX, u16::MAX));
    assert!(!TouchZone::new(u16::MAX - 1, 0, 1, 1).contains(u16::MAX, 0));
    assert!(!TouchZone::new(0, 0, 0, 10).contains(0, 0));
}

#[test]
fn press_contact_release_emits_one_tap() {
    let mut tracker = TouchTracker::new();
    assert_eq!(
        tracker.update(point(10, 20, TouchEventType::PressDown), HwGesture::None),
        TouchAction::None
    );
    assert_eq!(
        tracker.update(point(10, 20, TouchEventType::Contact), HwGesture::None),
        TouchAction::None
    );
    assert_eq!(
        tracker.update(point(10, 20, TouchEventType::LiftUp), HwGesture::None),
        TouchAction::Tap { x: 10, y: 20 }
    );
    assert_eq!(
        tracker.update(TouchState::NoTouch, HwGesture::None),
        TouchAction::None
    );
}

#[test]
fn missed_press_is_recovered_from_contact() {
    let mut tracker = TouchTracker::new();
    assert_eq!(
        tracker.update(point(30, 40, TouchEventType::Contact), HwGesture::None),
        TouchAction::None
    );
    assert_eq!(
        tracker.update(TouchState::NoTouch, HwGesture::None),
        TouchAction::Tap { x: 30, y: 40 }
    );
}

#[test]
fn drag_threshold_and_hardware_gestures_are_stable() {
    let mut tracker = TouchTracker::new();
    tracker.update(point(50, 50, TouchEventType::PressDown), HwGesture::None);
    assert_eq!(
        tracker.update(point(52, 48, TouchEventType::Contact), HwGesture::None),
        TouchAction::None
    );
    assert_eq!(
        tracker.update(point(56, 45, TouchEventType::Contact), HwGesture::None),
        TouchAction::Drag {
            x: 56,
            y: 45,
            dx: 4,
            dy: -3
        }
    );

    let cases = [
        (HwGesture::SwipeUp, TouchAction::SwipeRight),
        (HwGesture::SwipeDown, TouchAction::SwipeLeft),
        (HwGesture::SwipeLeft, TouchAction::SwipeUp),
        (HwGesture::SwipeRight, TouchAction::SwipeDown),
    ];
    for (gesture, expected) in cases {
        let mut tracker = TouchTracker::new();
        assert_eq!(
            tracker.update(point(5, 6, TouchEventType::Contact), gesture),
            expected
        );
        assert_eq!(
            tracker.update(point(5, 6, TouchEventType::Contact), gesture),
            TouchAction::None
        );
    }
}

#[test]
fn hardware_tap_rejects_ghosts_and_long_press_keeps_coordinates() {
    let mut tracker = TouchTracker::new();
    assert_eq!(
        tracker.update(point(1, 2, TouchEventType::NoEvent), HwGesture::SingleTap),
        TouchAction::None
    );
    tracker.update(point(1, 2, TouchEventType::PressDown), HwGesture::None);
    assert_eq!(
        tracker.update(point(1, 2, TouchEventType::Contact), HwGesture::SingleTap),
        TouchAction::Tap { x: 1, y: 2 }
    );
    let mut tracker = TouchTracker::new();
    assert_eq!(
        tracker.update(point(77, 88, TouchEventType::Contact), HwGesture::LongPress),
        TouchAction::Hold { x: 77, y: 88 }
    );
}

fn camera_input(x: u16, y: u16) -> CameraTouchInput {
    CameraTouchInput {
        x,
        y,
        event: CameraTouchEvent::PressDown,
        tap_pending: false,
        tune_active: true,
        selected_parameter: 0,
    }
}

#[test]
fn immediate_tracker_emits_once_per_contact_session() {
    let mut tracker = ImmediateTouchTracker::new();
    assert_eq!(tracker.update(TouchState::NoTouch), TouchAction::None);
    assert_eq!(
        tracker.update(point(100, 100, TouchEventType::PressDown)),
        TouchAction::Tap { x: 100, y: 100 },
    );
    assert_eq!(
        tracker.update(point(150, 120, TouchEventType::Contact)),
        TouchAction::None,
    );
    assert_eq!(tracker.update(TouchState::NoTouch), TouchAction::None);
    assert_eq!(
        tracker.update(point(200, 50, TouchEventType::Contact)),
        TouchAction::Tap { x: 200, y: 50 },
    );
}

#[test]
fn camera_touch_priority_and_grid_mapping_are_stable() {
    assert_eq!(
        classify_immediate_touch(CameraTouchInput {
            tap_pending: true,
            ..camera_input(10, 10)
        }),
        CameraTouchEffect::Ignore
    );
    assert_eq!(
        classify_immediate_touch(camera_input(48, 48)),
        CameraTouchEffect::Back
    );
    assert_eq!(
        classify_immediate_touch(camera_input(200, 30)),
        CameraTouchEffect::ExitTuning
    );
    assert_eq!(
        classify_immediate_touch(camera_input(210, 100)),
        CameraTouchEffect::SelectParameter(Some(2))
    );
    assert_eq!(
        classify_immediate_touch(camera_input(280, 150)),
        CameraTouchEffect::SelectParameter(Some(5))
    );
    assert_eq!(
        classify_immediate_touch(CameraTouchInput {
            event: CameraTouchEvent::Contact,
            ..camera_input(210, 100)
        }),
        CameraTouchEffect::PassThrough
    );
}

#[test]
fn camera_touch_ignores_inactive_and_invalid_events() {
    assert_eq!(
        classify_immediate_touch(CameraTouchInput {
            tune_active: false,
            ..camera_input(210, 100)
        }),
        CameraTouchEffect::Ignore,
    );
    assert_eq!(
        classify_immediate_touch(CameraTouchInput {
            event: CameraTouchEvent::Other,
            ..camera_input(10, 10)
        }),
        CameraTouchEffect::Ignore,
    );
    assert_eq!(
        classify_immediate_touch(CameraTouchInput {
            selected_parameter: 2,
            ..camera_input(210, 100)
        }),
        CameraTouchEffect::SelectParameter(None),
    );
}

#[test]
fn touch_register_decoders_cover_every_known_value() {
    assert_eq!(decode_touch_event_flag(0), TouchEventType::PressDown);
    assert_eq!(decode_touch_event_flag(1), TouchEventType::LiftUp);
    assert_eq!(decode_touch_event_flag(2), TouchEventType::Contact);
    assert_eq!(decode_touch_event_flag(3), TouchEventType::NoEvent);
    assert_eq!(decode_touch_event_flag(u8::MAX), TouchEventType::NoEvent);

    let gestures = [
        (0x00, HwGesture::None),
        (0x01, HwGesture::SwipeUp),
        (0x02, HwGesture::SwipeDown),
        (0x03, HwGesture::SwipeLeft),
        (0x04, HwGesture::SwipeRight),
        (0x05, HwGesture::SingleTap),
        (0x0B, HwGesture::DoubleTap),
        (0x0C, HwGesture::LongPress),
        (0x7F, HwGesture::Unknown(0x7F)),
    ];
    for (raw, expected) in gestures {
        assert_eq!(decode_gesture_byte(raw), expected);
    }
}

#[test]
fn rotated_touch_decoder_rejects_zero_and_multi_touch_and_saturates_coordinates() {
    assert_eq!(
        decode_rotated_single_touch([0, 0, 0, 0, 0], 319, 239),
        TouchState::NoTouch
    );
    assert_eq!(
        decode_rotated_single_touch([2, 0, 0, 0, 0], 319, 239),
        TouchState::NoTouch
    );

    let press = decode_rotated_single_touch([1, 0x00, 19, 0x00, 39], 319, 239);
    assert_eq!(press, point(300, 200, TouchEventType::PressDown));
    let contact = decode_rotated_single_touch([1, 0x80, 0xff, 0x00, 0xff], 319, 239);
    assert_eq!(contact, point(64, 0, TouchEventType::Contact));
    let beyond = decode_rotated_single_touch([1, 0x4f, 0xff, 0x0f, 0xff], 319, 239);
    assert_eq!(beyond, point(0, 0, TouchEventType::LiftUp));
}

#[test]
fn touch_tracker_defaults_match_fresh_trackers() {
    let mut immediate = ImmediateTouchTracker::default();
    assert_eq!(immediate.update(TouchState::NoTouch), TouchAction::None);
    let mut tracker = TouchTracker::default();
    assert_eq!(
        tracker.update(TouchState::NoTouch, HwGesture::None),
        TouchAction::None
    );
}

#[test]
fn camera_parameter_grid_covers_every_row_column_and_dead_zone() {
    let select = |x, y, current| {
        classify_immediate_touch(CameraTouchInput {
            selected_parameter: current,
            ..camera_input(x, y)
        })
    };
    assert_eq!(
        select(210, 40, 9),
        CameraTouchEffect::SelectParameter(Some(0))
    );
    assert_eq!(
        select(270, 40, 9),
        CameraTouchEffect::SelectParameter(Some(1))
    );
    assert_eq!(
        select(270, 100, 9),
        CameraTouchEffect::SelectParameter(Some(3))
    );
    assert_eq!(
        select(210, 140, 9),
        CameraTouchEffect::SelectParameter(Some(4))
    );
    // Gaps between rows, the strip below the grid, and the current row are inert.
    assert_eq!(select(210, 83, 9), CameraTouchEffect::SelectParameter(None));
    assert_eq!(
        select(210, 180, 9),
        CameraTouchEffect::SelectParameter(None)
    );
    assert_eq!(select(210, 40, 0), CameraTouchEffect::SelectParameter(None));
    // The tuning-exit strip ends at y = 36 and parameters end above y = 190.
    assert_eq!(select(197, 30, 9), CameraTouchEffect::PassThrough);
    assert_eq!(select(210, 190, 9), CameraTouchEffect::PassThrough);
    assert_eq!(select(197, 100, 9), CameraTouchEffect::PassThrough);
}

#[test]
fn transaction_review_and_confirm_regions_map_to_exact_effects() {
    use crate::presentation::transaction::{reduce_touch, TransactionEffect, TransactionScreen};

    let effect = |screen, x, y| reduce_touch(screen, x, y, false).effect;
    assert_eq!(
        effect(TransactionScreen::Review, 250, 200),
        TransactionEffect::ReviewAdvance
    );
    assert_eq!(
        effect(TransactionScreen::Review, 250, 100),
        TransactionEffect::None
    );
    assert_eq!(
        effect(TransactionScreen::Review, 100, 200),
        TransactionEffect::None
    );
    assert_eq!(
        effect(TransactionScreen::Confirm, 50, 200),
        TransactionEffect::ConfirmChoice(0)
    );
    assert_eq!(
        effect(TransactionScreen::Confirm, 250, 200),
        TransactionEffect::ConfirmChoice(1)
    );
    assert_eq!(
        effect(TransactionScreen::Confirm, 150, 200),
        TransactionEffect::ConfirmChoice(2)
    );
    assert_eq!(
        effect(TransactionScreen::Confirm, 150, 100),
        TransactionEffect::None
    );
    assert_eq!(
        effect(TransactionScreen::Confirm, 310, 200),
        TransactionEffect::None
    );
    assert_eq!(
        reduce_touch(TransactionScreen::Confirm, 0, 0, true).effect,
        TransactionEffect::ConfirmBack
    );
}
