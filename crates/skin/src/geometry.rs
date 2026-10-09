/// An axis-aligned rectangle in pixels. Width and height are never negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// Creates a rectangle; negative sizes are clamped to zero.
    #[must_use]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w: w.max(0),
            h: h.max(0),
        }
    }

    #[must_use]
    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    #[must_use]
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// The overlap of two rectangles (empty when they do not overlap).
    #[must_use]
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    /// The same rectangle moved by `(dx, dy)`.
    #[must_use]
    pub fn offset(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{} {}x{}", self.x, self.y, self.w, self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;

    #[test]
    fn intersection_and_containment() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.intersect(&b), Rect::new(5, 5, 5, 5));
        assert!(a.intersect(&Rect::new(20, 20, 1, 1)).is_empty());
        assert!(a.contains(9, 9));
        assert!(!a.contains(10, 0));
        assert_eq!(Rect::new(1, 2, -3, 4).w, 0);
    }
}
