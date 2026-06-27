//! # Player
//!
//! The diver / cursor the user steers around the reef. The **Overview** slide
//! lists *"WASD or Arrow Keys for Movement"* and the **Player** slide shows a
//! little sprite with bubbles. Here the player is a single board cell (the
//! planting cursor); the multi-line sprite from the slide is loaded as an
//! asset and could be blitted instead — see `assets/player.txt`.
//!
//! We keep the player deliberately small (one cell) because the core loop is
//! a *clicker*: you move the cursor and plant. A large multi-cell sprite
//! would obscure the cells you're trying to plant on.

use crate::assets::Asset;
use crate::constants::Layout;

/// A cardinal movement direction, produced by the controller from key input.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The player-controlled cursor.
pub struct Player {
    /// Column on the board, `0..board_w`.
    pub x: usize,
    /// Row on the board, `0..board_h`.
    pub y: usize,
    /// Base glyph for the player. Currently the renderer uses
    /// [`Player::glyph_overlay`] instead; kept for future use (e.g. a
    /// configurable cursor). Allowed to be unread for now.
    #[allow(dead_code)]
    pub glyph: char,
    /// The loaded sprite asset (from `assets/player.txt`). Unused by the
    /// single-cell renderer but kept so a future View can blit it; this is
    /// the `Player` ↔ asset link the slides imply. Allowed to be unread for
    /// now without a compiler warning.
    #[allow(dead_code)]
    pub sprite: Option<Asset>,
}

impl Player {
    /// Create a player centred on the board described by `layout`.
    pub fn new(sprite: Option<Asset>, layout: Layout) -> Player {
        Player {
            x: layout.board_w / 2,
            y: layout.board_h / 2,
            glyph: '@',
            sprite,
        }
    }

    /// The glyph drawn for the cursor when it overlays a board cell. Distinct
    /// from coral glyphs so the player can always find themselves; `X` reads
    /// clearly against empty space and coral alike.
    pub fn glyph_overlay(&self) -> char {
        'X'
    }

    /// Move one cell in `dir`, clamped to a board of size `board_w` × `board_h`
    /// so the cursor can never leave the playable area. The bounds are passed
    /// in (rather than stored) so the board dimensions have a single owner
    /// (`Coral`). Clamping keeps movement forgiving, which suits a casual
    /// clicker.
    pub fn move_in(&mut self, dir: Direction, board_w: usize, board_h: usize) {
        match dir {
            Direction::Up => {
                if self.y > 0 {
                    self.y -= 1;
                }
            }
            Direction::Down => {
                if self.y + 1 < board_h {
                    self.y += 1;
                }
            }
            Direction::Left => {
                if self.x > 0 {
                    self.x -= 1;
                }
            }
            Direction::Right => {
                if self.x + 1 < board_w {
                    self.x += 1;
                }
            }
        }
    }
}
