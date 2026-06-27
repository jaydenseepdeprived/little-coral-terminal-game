//! # ConwayRules — a stricter, timer-driven rulebook
//!
//! An alternative ruleset (selectable in `main.rs`) with different dynamics
//! from the default [`CoralRules`](crate::rules::CoralRules):
//!
//! - **Healthy cells are pure Conway's Game of Life.** Birth on exactly 3 live
//!   neighbours; survival on 2 or 3. They're rainbow-coloured. When a healthy
//!   cell *dies* by Conway (over- or under-crowding), it doesn't vanish — it
//!   becomes an **Unhealthy** cell.
//! - **Unhealthy cells are inert and temporary.** They don't participate in the
//!   Conway birth/survival counts' fate themselves beyond being "live" for
//!   neighbour counting; they just sit there. The player can **fix** one by
//!   stamping coral over it (that turns it Healthy again, handled by the stamp
//!   path). If left alone, an unhealthy cell **disappears after
//!   `unhealthy_lifespan` frames** — tracked via the board's per-cell age.
//! - **Disease is a separate automaton that attacks only healthy cells.** A
//!   diseased cell infects adjacent **Healthy** neighbours (marking them
//!   Diseased / red), and each diseased cell dies (→ Empty) after
//!   `disease_lifespan` frames. Disease ignores unhealthy and empty cells. The
//!   player can **kill** a diseased cell at the cursor.
//! - **Colours:** healthy = rainbow, unhealthy = grey (fading), diseased = red.
//!
//! The age buffer on the [`Board`] is what makes the timers possible: when a
//! cell *enters* the Unhealthy or Diseased state we stamp the current frame as
//! its "birth frame" into the age slot, and expire it once
//! `frame - birth_frame` exceeds the lifespan.

use ruscii::terminal::Color;

use crate::coral::Cell;
use crate::rulebook::{Board, Rulebook};

/// The timer-driven, separate-disease-automaton ruleset.
pub struct ConwayRules {
    /// Frames an Unhealthy cell persists before disappearing (if not fixed).
    pub unhealthy_lifespan: u32,
    /// Frames a Diseased cell persists before dying off to Empty.
    pub disease_lifespan: u32,
    /// Xterm colour codes cycled for healthy coral (the rainbow).
    pub healthy_palette: Vec<u8>,
}

impl Default for ConwayRules {
    fn default() -> ConwayRules {
        ConwayRules {
            unhealthy_lifespan: 6,
            disease_lifespan: 4,
            // Xterm 6×6×6 colour cube (codes 16..=231) for the rainbow.
            healthy_palette: (16u8..=231u8).collect(),
        }
    }
}

impl ConwayRules {
    /// Stable rainbow colour for a healthy cell from its position.
    fn rainbow(&self, x: usize, y: usize) -> Color {
        if self.healthy_palette.is_empty() {
            return Color::White;
        }
        let idx = (x.wrapping_mul(7).wrapping_add(y.wrapping_mul(13)))
            % self.healthy_palette.len();
        Color::Xterm(self.healthy_palette[idx])
    }
}

impl Rulebook for ConwayRules {
    /// Life automaton.
    ///
    /// Healthy cells follow classic Conway against *all* live cells as
    /// neighbours, but death routes to Unhealthy rather than Empty:
    ///
    /// - Empty with exactly 3 live neighbours → Healthy (born).
    /// - Healthy with 2 or 3 live neighbours → stays Healthy.
    /// - Healthy otherwise → becomes Unhealthy, and we stamp `frame` as its
    ///   birth frame so its lifespan timer starts now.
    /// - Unhealthy → stays Unhealthy until its lifespan elapses, then Empty.
    /// - Diseased cells are left to the disease automaton (untouched here).
    fn life_step(&self, board: &mut dyn Board, frame: usize) {
        let w = board.width();
        let h = board.height();

        // Snapshot the whole grid first so updates are simultaneous (Conway's
        // defining property). We read `current` and write to `board`.
        let mut current = vec![Cell::Empty; w * h];
        for y in 0..h {
            for x in 0..w {
                current[y * w + x] = board.cell(x as isize, y as isize);
            }
        }
        let neighbors_live = |x: usize, y: usize| -> u8 {
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
                let n = neighbors_live(x, y);

                match cell {
                    Cell::Empty => {
                        if n == 3 {
                            board.set_cell(x, y, Cell::Healthy);
                        }
                    }
                    Cell::Healthy => {
                        if n != 2 && n != 3 {
                            // Conway death → becomes Unhealthy and starts its
                            // disappearance timer at this frame.
                            board.set_cell(x, y, Cell::Unhealthy);
                            board.set_age(x, y, frame as u32);
                        }
                        // else: survives, unchanged.
                    }
                    Cell::Unhealthy => {
                        // Disappear after its lifespan unless the player fixed
                        // it (fixing turns it Healthy via the stamp path, so a
                        // still-unhealthy cell here was never fixed).
                        let born = board.age(x, y);
                        if (frame as u32).saturating_sub(born)
                            >= self.unhealthy_lifespan
                        {
                            board.set_cell(x, y, Cell::Empty);
                        }
                    }
                    Cell::Diseased => {
                        // Handled by disease_step; leave alone here.
                    }
                }
            }
        }
    }

    /// Disease automaton — runs independently of the life rules and touches
    /// only healthy and diseased cells.
    ///
    /// - A Healthy cell adjacent to any Diseased cell becomes Diseased (red),
    ///   with its disease timer started at `frame`.
    /// - A Diseased cell dies to Empty once `disease_lifespan` frames elapse.
    ///
    /// Unhealthy and Empty cells are immune, so disease can only chew through
    /// living (healthy) reef — exactly the requested behaviour.
    fn disease_step(&self, board: &mut dyn Board, frame: usize) {
        let w = board.width();
        let h = board.height();

        // Snapshot so spread is simultaneous (no chain-reaction within a tick).
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
                    Cell::Healthy => {
                        if has_diseased_neighbor(x, y) {
                            board.set_cell(x, y, Cell::Diseased);
                            board.set_age(x, y, frame as u32);
                        }
                    }
                    Cell::Diseased => {
                        let born = board.age(x, y);
                        if (frame as u32).saturating_sub(born)
                            >= self.disease_lifespan
                        {
                            board.set_cell(x, y, Cell::Empty);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn cell_glyph(&self, cell: Cell) -> char {
        match cell {
            Cell::Empty => ' ',
            Cell::Healthy => '@',
            Cell::Unhealthy => 'O',
            Cell::Diseased => '#',
        }
    }

    fn cell_color(&self, cell: Cell, x: usize, y: usize) -> Color {
        match cell {
            Cell::Healthy => self.rainbow(x, y),
            Cell::Unhealthy => Color::Grey, // fading, washed-out
            Cell::Diseased => Color::Red,   // disease is red, per the request
            Cell::Empty => Color::Black,
        }
    }

    /// This ruleset gives the player a verb to kill diseased cells.
    fn player_can_kill_disease(&self) -> bool {
        true
    }
}
