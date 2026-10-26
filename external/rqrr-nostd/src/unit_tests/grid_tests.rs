use super::*;

#[test]
fn capstones_that_degenerate_when_rotated_are_rejected() {
    let square = [
        Point { x: 0, y: 0 },
        Point { x: 7, y: 0 },
        Point { x: 7, y: 7 },
        Point { x: 0, y: 7 },
    ];
    let c = geometry::Perspective::create(&square, 7.0, 7.0).expect("square maps");
    let origin = Point { x: 0, y: 0 };
    let across = Point { x: 1, y: 1 };

    let mut sound = CapStone {
        corners: square,
        center: Point { x: 3, y: 3 },
        c: c.clone(),
    };
    assert_eq!(rotate_capstone(&mut sound, &origin, &across), Some(()));

    let point = Point { x: 3, y: 3 };
    let mut collapsed = CapStone {
        corners: [point; 4],
        center: point,
        c,
    };
    assert_eq!(rotate_capstone(&mut collapsed, &origin, &across), None);
}
