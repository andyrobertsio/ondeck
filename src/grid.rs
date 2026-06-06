//! The grid coordinate primitive shared by themes (layout slot rects) and the
//! `at="…"` escape hatch.

/// A grid rectangle stored as CSS grid *line* numbers (1-indexed, end-exclusive).
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub col_start: u8,
    pub col_end: u8,
    pub row_start: u8,
    pub row_end: u8,
}

impl Rect {
    /// Build from inclusive cell coordinates (col1..=col2, row1..=row2).
    pub fn cells(c1: u8, r1: u8, c2: u8, r2: u8) -> Rect {
        Rect {
            col_start: c1,
            col_end: c2 + 1,
            row_start: r1,
            row_end: r2 + 1,
        }
    }

    pub fn style(&self) -> String {
        format!(
            "grid-column:{}/{};grid-row:{}/{};",
            self.col_start, self.col_end, self.row_start, self.row_end
        )
    }

    /// Mirror the rectangle horizontally across a `cols`-wide grid (rows
    /// unchanged). Grid lines run 1..=cols+1, so line L maps to (cols+2)-L.
    pub fn mirror_cols(&self, cols: u8) -> Rect {
        Rect {
            col_start: (cols + 2) - self.col_end,
            col_end: (cols + 2) - self.col_start,
            row_start: self.row_start,
            row_end: self.row_end,
        }
    }
}

/// Parse `x2 y5 x8 y6`: two x tokens (col start/end, inclusive) and two y tokens
/// (row start/end, inclusive). Used for both theme layout rects and `at="…"`.
pub fn parse_at(s: &str) -> Option<Rect> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for tok in s.split_whitespace() {
        if tok.len() < 2 {
            return None;
        }
        let (axis, num) = tok.split_at(1);
        let n: u8 = num.parse().ok()?;
        match axis {
            "x" | "X" => xs.push(n),
            "y" | "Y" => ys.push(n),
            _ => return None,
        }
    }
    if xs.len() == 2 && ys.len() == 2 {
        Some(Rect::cells(xs[0], ys[0], xs[1], ys[1]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_at_inclusive_to_lines() {
        let r = parse_at("x2 y5 x8 y6").unwrap();
        // inclusive cells (2..=8, 5..=6) → CSS grid lines (2/9, 5/7)
        assert_eq!(
            (r.col_start, r.col_end, r.row_start, r.row_end),
            (2, 9, 5, 7)
        );
    }

    #[test]
    fn parse_at_rejects_malformed() {
        assert!(parse_at("x2 y5").is_none()); // too few
        assert!(parse_at("2 5 8 6").is_none()); // no axis prefixes
        assert!(parse_at("x2 yz x8 y6").is_none()); // non-numeric
    }

    #[test]
    fn rect_style_emits_grid_lines() {
        assert_eq!(
            Rect::cells(2, 5, 8, 6).style(),
            "grid-column:2/9;grid-row:5/7;"
        );
    }
}
