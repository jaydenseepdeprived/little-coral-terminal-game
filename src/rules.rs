//! # CoralRules — the tunable rulebook
//!
//! Everything you'd want to tweak to change how the reef *behaves* and *looks*
//! lives here, in one place, so you don't have to read the simulation code in
//! `coral.rs` to rebalance the game. [`Coral`](crate::coral::Coral) holds a
//! `CoralRules` and asks it the questions ("is this cell born? does it
//! survive? what colour is it?"); the simulation loop itself stays generic.
//!
//! ## What you can change here
//! - **Conway thresholds** — which neighbour counts cause birth and survival
//!   ([`CoralRules::birth_counts`], [`CoralRules::survival_counts`]).
//! - **Health ladder** — how a stressed/diseased cell heals or declines
//!   ([`CoralRules::heal`], [`CoralRules::decline`]).
//! - **Disease behaviour** — whether infection spreads and how diseased cells
//!   end ([`CoralRules::disease_spreads`], [`CoralRules::diseased_next`]).
//! - **Colours & glyphs** — the on-screen look of each cell state
//!   ([`CoralRules::cell_glyph`], [`CoralRules::cell_color`]).
//!
//! ## How to retune
//! Construct a custom rulebook and pass it to `Coral::new`. For the classic
//! Conway rule "born on 3, survives on 2 or 3", the defaults already match; to
//! make coral hardier you might widen `survival_counts` to `2..=4`, etc. Want
//! a different palette? Edit [`CoralRules::cell_color`] only — nothing else
//! needs to change.

use ruscii::terminal::Color;

use crate::coral::Cell;
use crate::rulebook::{Board, Rulebook};

/// A self-contained description of the reef's rules and appearance.
///
/// `Clone` so the model can keep one and the view can be handed a copy or a
/// borrow cheaply. All fields are public for direct tweaking, but the
/// behaviour is also exposed through methods so call sites read as intent
/// ("does this survive?") rather than raw comparisons.
#[derive(Clone, Debug)]
pub struct CoralRules {
    /// Neighbour counts at which an **empty** cell becomes newly healthy coral.
    /// Classic Conway birth is exactly 3.
    pub birth_counts: Vec<u8>,

    /// Neighbour counts at which **living** coral is "comfortable" and heals
    /// one step up the health ladder. Outside this set, coral is over- or
    /// under-crowded and declines one step. Classic Conway survival is 2 or 3.
    pub survival_counts: Vec<u8>,

    /// Whether disease spreads to healthy/unhealthy neighbours of a diseased
    /// cell. Turn off for a gentler game where disease only kills what's
    /// already infected.
    pub disease_spreads: bool,

    /// Whether a fresh infection is periodically seeded onto a random healthy
    /// cell (keeps pressure on a pristine reef).
    pub disease_seeds: bool,

    /// Xterm colour codes cycled through for **healthy** coral, indexed by
    /// board position so neighbours differ and the reef looks like a rainbow.
    /// Replace with a fixed single code for a monochrome reef.
    pub healthy_palette: Vec<u8>,
}

impl Default for CoralRules {
    /// The shipping ruleset: classic Conway thresholds, disease that spreads
    /// and self-seeds, and a full-spectrum rainbow for healthy coral.
    fn default() -> CoralRules {
        CoralRules {
            birth_counts: vec![3],
            survival_counts: vec![2, 3],
            disease_spreads: true,
            disease_seeds: true,
            // The Xterm 6×6×6 colour cube is codes 16..=231; we sample a
            // spread of vivid hues across it for the rainbow.
            healthy_palette: (16u8..=231u8).collect(),
        }
    }
}

impl CoralRules {
    /// Should an empty cell with `n` live neighbours be born as healthy coral?
    #[inline]
    pub fn is_born(&self, n: u8) -> bool {
        self.birth_counts.contains(&n)
    }

    /// Is `n` neighbours a comfortable count (coral heals) vs. stressful
    /// (coral declines)?
    #[inline]
    pub fn survives(&self, n: u8) -> bool {
        self.survival_counts.contains(&n)
    }

    /// Move a cell one step *up* the health ladder.
    ///
    /// `Diseased → Unhealthy → Healthy`; `Healthy` stays healthy, `Empty`
    /// stays empty. Change this to alter how recovery works (e.g. make
    /// diseased coral unrecoverable by returning `Cell::Diseased` here).
    #[inline]
    pub fn heal(&self, cell: Cell) -> Cell {
        match cell {
            Cell::Diseased => Cell::Unhealthy,
            Cell::Unhealthy => Cell::Healthy,
            Cell::Healthy => Cell::Healthy,
            Cell::Empty => Cell::Empty,
        }
    }

    /// Move a cell one step *down* the health ladder (final step is death).
    ///
    /// `Healthy → Unhealthy → Diseased → Empty`. Change this to make coral
    /// hardier (e.g. `Healthy → Healthy` to ignore crowding stress) or more
    /// fragile.
    #[inline]
    pub fn decline(&self, cell: Cell) -> Cell {
        match cell {
            Cell::Healthy => Cell::Unhealthy,
            Cell::Unhealthy => Cell::Diseased,
            Cell::Diseased => Cell::Empty,
            Cell::Empty => Cell::Empty,
        }
    }

    /// What a diseased cell becomes on each disease step. Default: it dies
    /// (`Empty`). Set this to `Cell::Diseased` for permanent infection, or
    /// `Cell::Unhealthy` for self-recovering disease.
    #[inline]
    pub fn diseased_next(&self, _cell: Cell) -> Cell {
        Cell::Empty
    }

    /// Pick a stable rainbow colour for a healthy cell from its position.
    ///
    /// Hashing the position (rather than using a frame counter) keeps each
    /// cell's colour steady across frames instead of flickering. Falls back to
    /// white if the palette is somehow empty.
    fn rainbow(&self, x: usize, y: usize) -> Color {
        if self.healthy_palette.is_empty() {
            return Color::White;
        }
        let idx = (x.wrapping_mul(7).wrapping_add(y.wrapping_mul(13)))
            % self.healthy_palette.len();
        Color::Xterm(self.healthy_palette[idx])
    }
}

/// The classic ruleset as a [`Rulebook`]: a health-ladder life step and a
/// spreading, self-seeding disease, with the rainbow/white/olive palette.
impl Rulebook for CoralRules {
    /// Life step: empty cells are born on `birth_counts`; living cells heal on
    /// `survival_counts` and decline otherwise, walking the health ladder
    /// (`heal`/`decline`). Snapshotted so updates are simultaneous.
    fn life_step(&self, board: &mut dyn Board, _frame: usize) {
        let w = board.width();
        let h = board.height();

        let mut current = vec![Cell::Empty; w * h];
        for y in 0..h {
            for x in 0..w {
                current[y * w + x] = board.cell(x as isize, y as isize);
            }
        }
        let live_n = |x: usize, y: usize| -> u8 {
            let mut n = 0u8;
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    if current[ny as usize * w + nx as usize] != Cell::Empty {
                        n += 1;
                    }
                }
            }
            n
        };

        for y in 0..h {
            for x in 0..w {
                let cell = current[y * w + x];
                let n = live_n(x, y);
                let new_cell = match cell {
                    Cell::Empty => {
                        if self.is_born(n) {
                            Cell::Healthy
                        } else {
                            Cell::Empty
                        }
                    }
                    _ => {
                        if self.survives(n) {
                            self.heal(cell)
                        } else {
                            self.decline(cell)
                        }
                    }
                };
                board.set_cell(x, y, new_cell);
            }
        }
    }

    /// Disease step: infection spreads to healthy/unhealthy neighbours of
    /// diseased cells (if `disease_spreads`), diseased cells advance via
    /// `diseased_next`, and a fresh infection is seeded if `disease_seeds`.
    fn disease_step(&self, board: &mut dyn Board, _frame: usize) {
        let w = board.width();
        let h = board.height();

        let mut current = vec![Cell::Empty; w * h];
        for y in 0..h {
            for x in 0..w {
                current[y * w + x] = board.cell(x as isize, y as isize);
            }
        }
        let has_diseased_neighbor = |x: usize, y: usize| -> bool {
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    if current[ny as usize * w + nx as usize] == Cell::Diseased {
                        return true;
                    }
                }
            }
            false
        };

        for y in 0..h {
            for x in 0..w {
                match current[y * w + x] {
                    Cell::Healthy | Cell::Unhealthy => {
                        if self.disease_spreads && has_diseased_neighbor(x, y) {
                            board.set_cell(x, y, Cell::Diseased);
                        }
                    }
                    Cell::Diseased => {
                        board.set_cell(x, y, self.diseased_next(Cell::Diseased));
                    }
                    Cell::Empty => {}
                }
            }
        }
    }

    fn cell_glyph(&self, cell: Cell) -> char {
        match cell {
            Cell::Empty => ' ',
            Cell::Healthy => '@',
            Cell::Unhealthy => 'o',
            Cell::Diseased => '#',
        }
    }

    fn cell_color(&self, cell: Cell, x: usize, y: usize) -> Color {
        match cell {
            Cell::Healthy => self.rainbow(x, y),
            Cell::Unhealthy => Color::White,
            Cell::Diseased => Color::Xterm(64), // dull olive / sickly green
            Cell::Empty => Color::Black,
        }
    }
}
