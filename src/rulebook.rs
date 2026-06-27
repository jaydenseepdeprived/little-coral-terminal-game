//! # Rulebook — the interface every coral ruleset implements
//!
//! Earlier the game had a single `CoralRules` parameter bag whose *logic* was
//! fixed and only its *values* varied. That's fine for retuning thresholds and
//! colours, but some rulesets need genuinely different dynamics — a different
//! disease automaton, per-cell timers, new player verbs. Those don't fit a
//! shared-logic parameter bag.
//!
//! So stepping is now defined behind this trait. A `Rulebook` owns its whole
//! life and disease automata and its own appearance; [`Coral`](crate::coral::Coral)
//! just holds the grid (and a parallel age buffer) and delegates each tick to
//! its boxed rulebook. Multiple rulebooks with completely different behaviour
//! can coexist — pick one at startup.
//!
//! ## Implementing a new rulebook
//! 1. Make a struct in its own file and `impl Rulebook for YourStruct`.
//! 2. Implement `life_step` / `disease_step` against the [`Board`] interface
//!    (read neighbours, read/write cells, read/write per-cell ages).
//! 3. Implement `cell_glyph` / `cell_color` for how each state looks.
//! 4. Construct it in `main.rs` and hand it to `Coral::new`.
//!
//! The classic ruleset lives in [`crate::rules::CoralRules`]; a second,
//! age-and-disease-automaton ruleset lives in
//! [`crate::conway_rules::ConwayRules`].

use ruscii::terminal::Color;

use crate::coral::Cell;

/// The grid surface a rulebook reads and writes during a step.
///
/// This is implemented by [`Coral`](crate::coral::Coral). It exposes only what
/// an automaton needs: dimensions, neighbour-aware cell access, and a parallel
/// per-cell **age** buffer (frames the cell has spent in its current state),
/// which timer-based rules (e.g. unhealthy cells expiring) rely on.
pub trait Board {
    /// Board width in cells.
    fn width(&self) -> usize;
    /// Board height in cells.
    fn height(&self) -> usize;

    /// Read a cell; out-of-bounds reads return [`Cell::Empty`].
    fn cell(&self, x: isize, y: isize) -> Cell;
    /// Write a cell (bounds-checked; out-of-range writes ignored). Setting a
    /// cell does **not** itself reset its age — the rulebook decides when to
    /// reset, via [`Board::set_age`].
    fn set_cell(&mut self, x: usize, y: usize, cell: Cell);

    /// Read a cell's age (frames in current state). Out-of-bounds → 0.
    fn age(&self, x: usize, y: usize) -> u32;
    /// Set a cell's age.
    fn set_age(&mut self, x: usize, y: usize, age: u32);

    /// Count live (non-empty) Moore neighbours of `(x, y)`. Provided helper so
    /// every rulebook doesn't re-implement the 8-direction loop.
    fn live_neighbors(&self, x: usize, y: usize) -> u8 {
        let mut n = 0u8;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.cell(x as isize + dx, y as isize + dy) != Cell::Empty {
                    n += 1;
                }
            }
        }
        n
    }

    /// Count Moore neighbours that are exactly `target`.
    fn neighbors_of(&self, x: usize, y: usize, target: Cell) -> u8 {
        let mut n = 0u8;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.cell(x as isize + dx, y as isize + dy) == target {
                    n += 1;
                }
            }
        }
        n
    }
}

/// A complete coral ruleset: its automata and its appearance.
///
/// `life_step` and `disease_step` are called by [`Coral::step`] on their own
/// cadences (from the model). Each receives the board and the current frame
/// number (so timer logic can compute elapsed frames), and mutates the board
/// in place. Implementations should use a snapshot/double-buffer internally if
/// they need simultaneous updates (the [`Board`] doesn't do that for them).
pub trait Rulebook {
    /// Advance the life automaton one step.
    fn life_step(&self, board: &mut dyn Board, frame: usize);

    /// Advance the disease automaton one step.
    fn disease_step(&self, board: &mut dyn Board, frame: usize);

    /// The glyph used to draw a cell of the given state.
    fn cell_glyph(&self, cell: Cell) -> char;

    /// The colour used to draw a cell at board position `(x, y)`.
    fn cell_color(&self, cell: Cell, x: usize, y: usize) -> Color;

    /// Whether this rulebook lets the player **kill** a diseased cell at the
    /// cursor (a new verb some rulesets want). Default: no.
    fn player_can_kill_disease(&self) -> bool {
        false
    }
}
