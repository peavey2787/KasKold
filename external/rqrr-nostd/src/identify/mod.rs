pub use self::grid::SkewedGridLocation;
pub use self::match_capstones::find_and_rank_possible_neighbors;

pub mod grid;
pub mod match_capstones;

/// A simple point in (some) space
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// The point at pixel `(x, y)`, each axis saturating at `MAX_COORDINATE`.
    pub(crate) fn at(x: usize, y: usize) -> Self {
        Self {
            x: crate::geometry::coordinate(x),
            y: crate::geometry::coordinate(y),
        }
    }
}
