//! # Coral
//!
//! The living reef. This is the simulation core and the most spec-dense part
//! of the game. From the **Coral** slide:
//!
//! - *healthy: rainbow colored* → here, [`Cell::Healthy`] (we don't color in
//!   the plain-println build; see the View for where color would attach).
//! - *unhealthy: white* → [`Cell::Unhealthy`].
//! - *"stamps" — spawns an asset from a textfile* → [`Coral::stamp`].
//! - *becomes unhealthy by having too many neighbors or too few, Conway's
//!   Game of Life style* → [`Coral::life_step`].
//! - *but also slowly dies off from "diseases" too* → [`Coral::disease_step`].
//!
//! ## Why a state machine instead of a plain bool grid
//! Classic Conway is binary (alive/dead). The slide wants three meaningful
//! states — healthy, unhealthy, empty — plus a disease overlay. Modelling the
//! cell as an enum makes the rules readable and makes "unhealthy" a real
//! intermediate stage (stressed coral that recovers or dies) rather than an
//! instant flip, which feels more like an ecosystem than a math toy.

use rand::Rng;

use crate::assets::Asset;
use crate::constants::Layout;
use crate::rulebook::{Board, Rulebook};
use crate::stamps::StampSet;

/// The state of a single board cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    /// No coral here.
    Empty,
    /// Thriving coral. Drawn in rainbow colours by the view.
    Healthy,
    /// Stressed/dying coral. Its exact behaviour depends on the active
    /// rulebook (the classic rules decline it down a health ladder; the Conway
    /// rules make it a temporary cell that expires after a timer).
    Unhealthy,
    /// Diseased coral. Behaviour again depends on the rulebook.
    Diseased,
}

/// The reef grid plus its active rulebook.
pub struct Coral {
    /// Flat `w * h` grid, row-major.
    grid: Vec<Cell>,
    /// Per-cell age buffer, parallel to `grid`. Its meaning is rulebook-defined
    /// (typically the frame a cell entered its current state, for timers).
    ages: Vec<u32>,
    /// Board width in cells (from the runtime [`Layout`]).
    w: usize,
    /// Board height in cells (from the runtime [`Layout`]).
    h: usize,
    /// The library of coral shapes the player can stamp.
    stamps: StampSet,
    /// The active ruleset: its automata and appearance. Boxed so different
    /// rulebooks (with different logic) can be swapped in at startup.
    pub rules: Box<dyn Rulebook>,
}

impl Coral {
    /// Build an empty reef sized to the given [`Layout`], using `stamps` for
    /// planting and the boxed `rules` for behaviour and appearance.
    pub fn new(stamps: StampSet, rules: Box<dyn Rulebook>, layout: Layout) -> Coral {
        Coral {
            grid: vec![Cell::Empty; layout.board_w * layout.board_h],
            ages: vec![0; layout.board_w * layout.board_h],
            w: layout.board_w,
            h: layout.board_h,
            stamps,
            rules,
        }
    }

    /// Board width in cells.
    #[inline]
    pub fn width(&self) -> usize {
        self.w
    }

    /// Board height in cells.
    #[inline]
    pub fn height(&self) -> usize {
        self.h
    }

    /// Convert (x, y) into a flat index. Private; all access goes through
    /// [`Coral::get`] / [`Coral::set`] so bounds logic lives in one spot.
    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    /// Read a cell. Out-of-bounds reads return `Empty`, which makes the
    /// neighbor loop in `life_step` simpler (edges behave as dead space, the
    /// standard finite-grid Conway convention).
    #[inline]
    pub fn get(&self, x: isize, y: isize) -> Cell {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            Cell::Empty
        } else {
            self.grid[self.idx(x as usize, y as usize)]
        }
    }

    /// Write a cell via the *player/stamp* path: bounds-checked, and resets the
    /// cell's age to 0 (a freshly planted or seeded cell starts its clock
    /// clean). Rulebooks that need precise timer control use the `Board`
    /// interface's `set_cell` + `set_age` instead, which don't auto-reset.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.grid[i] = cell;
            self.ages[i] = 0;
        }
    }

    /// `"stamps" — spawns an asset from a textfile`.
    ///
    /// Blit `asset` onto the board with its top-left at `(ox, oy)`. Non-space
    /// characters become **Healthy** coral; spaces are skipped so the stamp's
    /// shape is preserved and existing coral around it isn't erased. Stamping
    /// over diseased/unhealthy cells heals them — the player's main verb for
    /// *restoring* the reef (Overview slide: "Plant/Restore/Protect").
    pub fn stamp_asset(&mut self, asset: &Asset, ox: usize, oy: usize) {
        // Collect target cells first so the immutable borrow of `asset` ends
        // before the mutable `self.set` writes begin (avoids E0502).
        let cells: Vec<(usize, usize)> = asset
            .rows
            .iter()
            .enumerate()
            .flat_map(|(dy, row)| {
                row.iter().enumerate().filter_map(move |(dx, &ch)| {
                    if ch != ' ' {
                        Some((ox + dx, oy + dy))
                    } else {
                        None
                    }
                })
            })
            .collect();

        for (x, y) in cells {
            self.set(x, y, Cell::Healthy);
        }
    }

    /// Stamp a **random** coral shape from the library at `(ox, oy)`. This is
    /// the player's plant action: each press drops a different kind of coral.
    pub fn stamp_random(&mut self, ox: usize, oy: usize) {
        // Clone the chosen asset so we don't hold a borrow of `self.stamps`
        // across the mutable `stamp_asset` call.
        let asset = self.stamps.random().clone();
        self.stamp_asset(&asset, ox, oy);
    }

    /// Seed the reef with `count` random stamps at random positions, used at
    /// spawn so the player starts with "a couple of coral stamps" already on
    /// the board (per the refinement request) rather than a blank reef.
    ///
    /// Positions are chosen so each stamp fits fully on the board.
    pub fn seed_spawn(&mut self, count: usize) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            let asset = self.stamps.random().clone();
            // Keep the whole stamp on-board: origin range leaves room for its
            // width/height. saturating_sub guards tiny boards.
            let max_x = self.w.saturating_sub(asset.width).max(1);
            let max_y = self.h.saturating_sub(asset.height).max(1);
            let ox = rng.gen_range(0..max_x);
            let oy = rng.gen_range(0..max_y);
            self.stamp_asset(&asset, ox, oy);
        }
    }

    /// Run the rulebook's life automaton for `frame`.
    ///
    /// The boxed rulebook needs `&mut self` (as a `Board`) while we also borrow
    /// `self.rules`. We sidestep that by temporarily moving the rules out,
    /// stepping, then putting them back — the standard pattern for calling
    /// through a field that needs the whole struct.
    pub fn life_step(&mut self, frame: usize) {
        let rules = std::mem::replace(&mut self.rules, Box::new(NullRules));
        rules.life_step(self, frame);
        self.rules = rules;
    }

    /// Run the rulebook's disease automaton for `frame`.
    pub fn disease_step(&mut self, frame: usize) {
        let rules = std::mem::replace(&mut self.rules, Box::new(NullRules));
        rules.disease_step(self, frame);
        self.rules = rules;
    }

    /// Player verb: kill every diseased cell in the 3×3 area centred on
    /// `(x, y)` (the cursor cell plus its 8 neighbours), turning each to Empty.
    /// Only acts when the active rulebook permits it. Returns how many diseased
    /// cells were removed, so the caller can scale score/feedback by the count.
    pub fn kill_disease(&mut self, x: usize, y: usize) -> usize {
        if !self.rules.player_can_kill_disease() {
            return 0;
        }
        let mut killed = 0;
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.w || ny as usize >= self.h
                {
                    continue;
                }
                let i = self.idx(nx as usize, ny as usize);
                if self.grid[i] == Cell::Diseased {
                    self.grid[i] = Cell::Empty;
                    self.ages[i] = 0;
                    killed += 1;
                }
            }
        }
        killed
    }

    /// Seed one new infection on a random healthy cell, if any exists. Kept as
    /// a `Coral` helper (rather than a rulebook concern) because seeding is an
    /// external pressure the model can apply on whatever cadence it likes.
    pub fn seed_disease(&mut self, frame: usize) {
        if let Some((sx, sy)) = self.random_healthy() {
            let i = self.idx(sx, sy);
            self.grid[i] = Cell::Diseased;
            self.ages[i] = frame as u32;
        }
    }

    /// Pick a uniformly random healthy cell, or `None` if there are none.
    fn random_healthy(&self) -> Option<(usize, usize)> {
        let healthy: Vec<usize> = self
            .grid
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == Cell::Healthy)
            .map(|(i, _)| i)
            .collect();
        if healthy.is_empty() {
            return None;
        }
        let pick = healthy[rand::thread_rng().gen_range(0..healthy.len())];
        Some((pick % self.w, pick / self.w))
    }

    /// Count cells by state. Returned as `(healthy, unhealthy, diseased)`,
    /// used by the model to update score and by the View for the scoreboard.
    pub fn census(&self) -> (usize, usize, usize) {
        let mut h = 0;
        let mut u = 0;
        let mut d = 0;
        for &c in &self.grid {
            match c {
                Cell::Healthy => h += 1,
                Cell::Unhealthy => u += 1,
                Cell::Diseased => d += 1,
                Cell::Empty => {}
            }
        }
        (h, u, d)
    }

    /// Borrow the raw grid for rendering. Read-only on purpose: the View must
    /// not mutate the model (MVC discipline).
    pub fn grid(&self) -> &[Cell] {
        &self.grid
    }

    /// Whether the active rulebook lets the player kill diseased cells. The
    /// view/controller use this to show the right controls.
    pub fn player_can_kill_disease(&self) -> bool {
        self.rules.player_can_kill_disease()
    }
}

/// [`Board`] is the surface rulebooks read and write during a step. `Coral`
/// implements it so a boxed rulebook can drive the grid without knowing about
/// stamps, the model, or anything else `Coral` holds.
///
/// Note `set_cell` here deliberately does **not** reset age (unlike
/// [`Coral::set`], the player/stamp path): rulebooks manage ages explicitly
/// via [`Board::set_age`] so they can implement timers precisely.
impl Board for Coral {
    fn width(&self) -> usize {
        self.w
    }

    fn height(&self) -> usize {
        self.h
    }

    fn cell(&self, x: isize, y: isize) -> Cell {
        self.get(x, y)
    }

    fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.grid[i] = cell;
        }
    }

    fn age(&self, x: usize, y: usize) -> u32 {
        if x < self.w && y < self.h {
            self.ages[self.idx(x, y)]
        } else {
            0
        }
    }

    fn set_age(&mut self, x: usize, y: usize, age: u32) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.ages[i] = age;
        }
    }
}

/// A do-nothing rulebook used only as a temporary placeholder while we move the
/// real boxed rulebook out of `Coral` to step it (see [`Coral::life_step`]).
/// It is never actually stepped.
struct NullRules;

impl Rulebook for NullRules {
    fn life_step(&self, _board: &mut dyn Board, _frame: usize) {}
    fn disease_step(&self, _board: &mut dyn Board, _frame: usize) {}
    fn cell_glyph(&self, _cell: Cell) -> char {
        ' '
    }
    fn cell_color(&self, _cell: Cell, _x: usize, _y: usize) -> ruscii::terminal::Color {
        ruscii::terminal::Color::Black
    }
}
