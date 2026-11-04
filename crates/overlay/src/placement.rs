//! Keeps a saved window position on a screen that still exists, so a window
//! moved to a monitor since unplugged never comes back out of sight.

use serde::{Deserialize, Serialize};

/// A position in physical pixels, the window's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Saved form of the position, as written to the window file.
    ///
    /// # Errors
    ///
    /// Fails only if serialization itself fails, which a plain point cannot.
    pub fn to_json(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }

    /// Reads a position saved by [`Point::to_json`].
    ///
    /// # Errors
    ///
    /// Fails if `json` is not a saved position.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// An area of the desktop in physical pixels, such as a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    fn left(self) -> i64 {
        i64::from(self.origin.x)
    }

    fn top(self) -> i64 {
        i64::from(self.origin.y)
    }

    fn right(self) -> i64 {
        self.left() + i64::from(self.size.width)
    }

    fn bottom(self) -> i64 {
        self.top() + i64::from(self.size.height)
    }

    fn contains(self, other: Self) -> bool {
        other.left() >= self.left()
            && other.top() >= self.top()
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    fn overlap_area(self, other: Self) -> i64 {
        let width = self.right().min(other.right()) - self.left().max(other.left());
        let height = self.bottom().min(other.bottom()) - self.top().max(other.top());
        width.max(0) * height.max(0)
    }
}

/// Where to put a window of `window_size` saved at `saved`.
///
/// Keeps `saved` when the window fits entirely on one of `screens`. Otherwise
/// moves it inside the screen it overlaps the most, or inside `primary` when
/// it overlaps none. Returns `None` when no screen is known.
#[must_use]
pub fn fit_on_screens(
    saved: Point,
    window_size: Size,
    screens: &[Rect],
    primary: Option<Rect>,
) -> Option<Point> {
    let window = Rect {
        origin: saved,
        size: window_size,
    };
    if screens.iter().any(|screen| screen.contains(window)) {
        return Some(saved);
    }
    let target = screens
        .iter()
        .copied()
        .filter(|screen| screen.overlap_area(window) > 0)
        .max_by_key(|screen| screen.overlap_area(window))
        .or(primary)
        .or_else(|| screens.first().copied())?;
    Some(Point {
        x: clamp_axis(
            saved.x,
            window_size.width,
            target.origin.x,
            target.size.width,
        ),
        y: clamp_axis(
            saved.y,
            window_size.height,
            target.origin.y,
            target.size.height,
        ),
    })
}

/// Clamps one coordinate so the window stays within the screen on that axis,
/// pinned to the screen start when the window is larger than the screen.
fn clamp_axis(position: i32, window_length: u32, screen_start: i32, screen_length: u32) -> i32 {
    let start = i64::from(screen_start);
    let end = (start + i64::from(screen_length) - i64::from(window_length)).max(start);
    let clamped = i64::from(position).clamp(start, end);
    // Both bounds derive from an `i32` screen origin, so this cannot fail.
    i32::try_from(clamped).unwrap_or(screen_start)
}

#[cfg(test)]
mod tests {
    use super::{fit_on_screens, Point, Rect, Size};

    const WINDOW: Size = Size {
        width: 320,
        height: 160,
    };

    const PRIMARY: Rect = Rect {
        origin: Point { x: 0, y: 0 },
        size: Size {
            width: 1920,
            height: 1080,
        },
    };

    const RIGHT_SCREEN: Rect = Rect {
        origin: Point { x: 1920, y: 0 },
        size: Size {
            width: 2560,
            height: 1440,
        },
    };

    #[test]
    fn a_saved_position_reads_back_identical() {
        let saved = Point { x: -1200, y: 340 };

        let json = saved.to_json().expect("a point serializes");

        assert_eq!(json, r#"{"x":-1200,"y":340}"#);
        assert_eq!(Point::from_json(&json).ok(), Some(saved));
    }

    #[test]
    fn rejects_an_unreadable_saved_position() {
        assert!(Point::from_json("").is_err());
        assert!(Point::from_json(r#"{"x":"left"}"#).is_err());
    }

    #[test]
    fn keeps_a_position_fully_on_a_known_screen() {
        let saved = Point { x: 2500, y: 300 };

        let placed = fit_on_screens(saved, WINDOW, &[PRIMARY, RIGHT_SCREEN], Some(PRIMARY));

        assert_eq!(placed, Some(saved));
    }

    #[test]
    fn brings_a_position_from_a_lost_screen_back_onto_the_primary() {
        let saved = Point { x: 3000, y: 1300 };

        let placed = fit_on_screens(saved, WINDOW, &[PRIMARY], Some(PRIMARY));

        assert_eq!(placed, Some(Point { x: 1600, y: 920 }));
    }

    #[test]
    fn shifts_a_window_straddling_the_right_edge_fully_into_view() {
        let saved = Point { x: 1800, y: 100 };

        let placed = fit_on_screens(saved, WINDOW, &[PRIMARY], Some(PRIMARY));

        assert_eq!(placed, Some(Point { x: 1600, y: 100 }));
    }

    #[test]
    fn imposes_no_position_without_any_known_screen() {
        let placed = fit_on_screens(Point { x: 10, y: 10 }, WINDOW, &[], None);

        assert_eq!(placed, None);
    }

    #[test]
    fn pins_a_window_larger_than_the_screen_to_its_corner() {
        let small = Rect {
            origin: Point { x: -800, y: 0 },
            size: Size {
                width: 200,
                height: 100,
            },
        };

        let placed = fit_on_screens(Point { x: -900, y: 50 }, WINDOW, &[small], None);

        assert_eq!(placed, Some(Point { x: -800, y: 0 }));
    }
}
