//! # Layout & timing constants
//!
//! The **Technical Details** slide specs a fixed `24x80` screen (an `18x78`
//! board + border and a `2x78` scoreboard + border). We treat those as the
//! *target/maximum* and shrink to fit smaller terminals, so the board always
//! fits the window ruscii hands us.
//!
//! Unlike the plain-`println!` build, here we don't query the terminal
//! ourselves — ruscii reports the window size via `Window::size()` each frame,
//! and we build a [`Layout`] from it. The board size is fixed once at startup
//! from the first observed size (a coral grid can't sensibly resize mid-game).

/// Maximum total screen width (slide: 80).
pub const MAX_SCREEN_W: usize = 80;
/// Maximum total screen height (slide: 24).
pub const MAX_SCREEN_H: usize = 24;

/// Smallest playable board before we ask the user to enlarge the terminal.
pub const MIN_BOARD_W: usize = 20;
/// Smallest playable board height.
pub const MIN_BOARD_H: usize = 6;

/// Rows the UI chrome occupies around the board:
/// 1 title + 1 top border + 1 divider + 2 scoreboard + 1 bottom border
/// + 1 controls hint = 7. (ruscii draws the board border as part of the grid
/// area via draw_rect, but we still budget rows for title/score/controls.)
pub const CHROME_ROWS: usize = 7;

/// Scoreboard text height (inside its border). Records the slide's 2x78 spec.
#[allow(dead_code)]
pub const SCORE_H: usize = 2;

// --- Timing ---------------------------------------------------------------

/// ruscii ticks ("steps") between Conway-style coral life updates. ruscii's
/// default framerate is ~30 fps, so 16 steps ≈ a life update about twice a
/// second — a calm, watchable pace.
pub const LIFE_PERIOD: usize = 16;
/// ruscii steps between disease decay passes.
pub const DISEASE_PERIOD: usize = 14;
/// Score awarded per *healthy* coral cell each life update.
pub const SCORE_PER_HEALTHY: i64 = 1;

/// Runtime geometry, computed from the terminal size at startup.
#[derive(Copy, Clone, Debug)]
pub struct Layout {
    /// Playable board width (inside the border).
    pub board_w: usize,
    /// Playable board height (inside the border).
    pub board_h: usize,
}

impl Layout {
    /// Derive a board that fits inside a terminal of `cols` × `rows`, capped at
    /// the slide's 80x24 target and floored at the usable minimum.
    pub fn from_size(cols: usize, rows: usize) -> Layout {
        // Reserve 2 columns for the left/right border drawn by draw_rect.
        let board_w = cols
            .saturating_sub(2)
            .min(MAX_SCREEN_W - 2)
            .max(MIN_BOARD_W);

        // Reserve chrome rows (title/score/controls) and 2 border rows.
        let board_h = rows
            .saturating_sub(CHROME_ROWS)
            .min(MAX_SCREEN_H - CHROME_ROWS)
            .max(MIN_BOARD_H);

        Layout { board_w, board_h }
    }

    /// Total framed width = board + 2 border columns.
    pub fn frame_w(&self) -> usize {
        self.board_w + 2
    }

    /// Total rows the rendered UI occupies.
    pub fn total_rows(&self) -> usize {
        self.board_h + CHROME_ROWS
    }

    /// Smallest terminal (cols, rows) at which the game renders without clipping.
    pub fn min_terminal() -> (usize, usize) {
        (MIN_BOARD_W + 2, MIN_BOARD_H + CHROME_ROWS)
    }
}
