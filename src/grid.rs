//! The grid coordinate primitive shared by themes (block rects) and the
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

    /// Span in grid lines: (cols, rows).
    pub fn extent(&self) -> (i16, i16) {
        (
            self.col_end as i16 - self.col_start as i16,
            self.row_end as i16 - self.row_start as i16,
        )
    }

    /// Shift by (dcols, drows) grid lines.
    pub fn translate(&self, dc: i16, dr: i16) -> Rect {
        Rect {
            col_start: (self.col_start as i16 + dc).max(1) as u8,
            col_end: (self.col_end as i16 + dc).max(1) as u8,
            row_start: (self.row_start as i16 + dr).max(1) as u8,
            row_end: (self.row_end as i16 + dr).max(1) as u8,
        }
    }
}

/// Direction a repeatable block flows from its anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatDir {
    Up,
    Down,
    Left,
    Right,
}

/// How rendered copies sit within the limit-sized track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatAlign {
    Start,
    Center,
    End,
}

/// Place `count` copies of `anchor`, each offset along `dir` by
/// (extent-along-axis + `margin`). `limit` sizes the track that `align`
/// positions the copies within (centering/ending partial counts); when
/// `limit <= count` there is no slack and `align` is a no-op.
pub fn repeat_rects(
    anchor: &Rect,
    dir: RepeatDir,
    margin: u8,
    count: usize,
    limit: usize,
    align: RepeatAlign,
) -> Vec<Rect> {
    let (w, h) = anchor.extent();
    let (is_col, sign): (bool, i16) = match dir {
        RepeatDir::Right => (true, 1),
        RepeatDir::Left => (true, -1),
        RepeatDir::Down => (false, 1),
        RepeatDir::Up => (false, -1),
    };
    let step = (if is_col { w } else { h }) + margin as i16;
    let n = count.min(limit); // limit caps; extras are dropped
    let slack = (limit.saturating_sub(n)) as i16 * step;
    let align_off = match align {
        RepeatAlign::Start => 0,
        RepeatAlign::Center => sign * (slack / 2),
        RepeatAlign::End => sign * slack,
    };
    (0..n)
        .map(|i| {
            let off = align_off + sign * (i as i16) * step;
            if is_col {
                anchor.translate(off, 0)
            } else {
                anchor.translate(0, off)
            }
        })
        .collect()
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

    fn starts(rects: &[Rect]) -> Vec<(u8, u8)> {
        rects.iter().map(|r| (r.col_start, r.row_start)).collect()
    }

    #[test]
    fn repeat_right_flows_by_extent_plus_margin() {
        // anchor 6 cells wide (cols 4..=9 → lines 4/10, extent 6), margin 1 → step 7.
        let a = Rect::cells(4, 7, 9, 13);
        let rs = repeat_rects(&a, RepeatDir::Right, 1, 3, 3, RepeatAlign::Start);
        assert_eq!(starts(&rs), vec![(4, 7), (11, 7), (18, 7)]);
    }

    #[test]
    fn repeat_down_flows_vertically() {
        let a = Rect::cells(4, 3, 9, 5); // height extent 3 (rows 3/6), margin 0 → step 3
        let rs = repeat_rects(&a, RepeatDir::Down, 0, 2, 2, RepeatAlign::Start);
        assert_eq!(starts(&rs), vec![(4, 3), (4, 6)]);
    }

    #[test]
    fn repeat_center_aligns_partial_count_in_track() {
        // limit 4, count 2, step 7 → slack = 2*7 = 14, center shifts by 7.
        let a = Rect::cells(4, 7, 9, 13);
        let rs = repeat_rects(&a, RepeatDir::Right, 1, 2, 4, RepeatAlign::Center);
        assert_eq!(starts(&rs), vec![(11, 7), (18, 7)]);
    }

    #[test]
    fn repeat_left_reverses() {
        let a = Rect::cells(20, 7, 25, 13); // extent 6, margin 0 → step 6
        let rs = repeat_rects(&a, RepeatDir::Left, 0, 2, 2, RepeatAlign::Start);
        assert_eq!(starts(&rs), vec![(20, 7), (14, 7)]);
    }
}
