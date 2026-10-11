// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the bubble goes, given the caret. Same rule as the GNOME extension:
//! right of the caret and vertically centred on it, flipped to the left when
//! it would run off the monitor, then clamped into the work area.

/// Gap between caret and bubble at 100% scale. Matches the extension's `GAP`.
pub const GAP: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Top-left corner for a bubble of `size` (w, h) next to `caret`, kept inside
/// `area`. `gap` is already scaled for the monitor's DPI.
pub fn place_bubble(caret: Rect, size: (i32, i32), area: Rect, gap: i32) -> (i32, i32) {
    let (w, h) = size;
    let mut x = caret.x + caret.w + gap;
    if x + w > area.x + area.w {
        x = caret.x - gap - w;
    }
    let x = x.max(area.x);
    let y = (caret.y + (caret.h - h) / 2)
        .max(area.y)
        .min(area.y + area.h - h);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };

    #[test]
    fn right_of_caret_and_centred() {
        let caret = Rect {
            x: 500,
            y: 300,
            w: 1,
            h: 20,
        };
        assert_eq!(place_bubble(caret, (60, 24), SCREEN, GAP), (513, 298));
    }

    #[test]
    fn flips_left_at_the_right_edge() {
        let caret = Rect {
            x: 1900,
            y: 300,
            w: 0,
            h: 20,
        };
        assert_eq!(place_bubble(caret, (60, 24), SCREEN, GAP), (1828, 298));
    }

    #[test]
    fn clamped_vertically() {
        let caret = Rect {
            x: 500,
            y: 1075,
            w: 0,
            h: 20,
        };
        assert_eq!(place_bubble(caret, (60, 24), SCREEN, GAP).1, 1056);
    }

    #[test]
    fn second_monitor_offsets() {
        let area = Rect {
            x: -1920,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let caret = Rect {
            x: -30,
            y: 10,
            w: 0,
            h: 20,
        };
        assert_eq!(place_bubble(caret, (60, 24), area, GAP), (-102, 8));
    }
}
