//! # Model (the "M" in MVC)
//!
//! The **Code Structure** slide hangs everything off `Game()` with `player`,
//! `objects[]`, `score`, and `frames`. We keep the data+rules here, separate
//! from drawing (`view`) and from the loop (which ruscii's `App` now owns).
//! The Model owns the player, the coral reef, and the running totals.
//!
//! `objects[]` from the slide is the coral grid plus the player. The starred
//! "future" objects (debris, boats, yachts) would become extra vectors here
//! without disturbing the rest of the architecture.
//!
//! ## Ticking
//! In the ruscii port we don't keep our own frame counter — ruscii exposes a
//! monotonic `State::step()` we pass into [`Model::tick`]. Periodic events
//! (life step, disease step) fire when the step count crosses a period
//! boundary, so the simulation pace is independent of the render framerate.

use crate::constants::{DISEASE_PERIOD, LIFE_PERIOD, SCORE_PER_HEALTHY};
use crate::coral::Coral;
use crate::player::Player;

/// All mutable game state and the rules that evolve it.
pub struct Model {
    /// The player cursor.
    pub player: Player,
    /// The living reef (the bulk of `objects[]`).
    pub coral: Coral,
    /// Running score (slide: `int score`).
    pub score: i64,
    /// Last ruscii step we processed, so we can detect period boundaries even
    /// if the closure is called at an irregular cadence. Mirrors the slide's
    /// `int frames`.
    pub frames: usize,
    /// Set once the reef has lived and then died out completely.
    pub game_over: bool,
    /// Whether the reef has ever held coral, so an empty *starting* board
    /// doesn't instantly read as "game over".
    had_coral: bool,
}

impl Model {
    /// Assemble a model from its parts (built in `main.rs` after assets load).
    pub fn new(player: Player, coral: Coral) -> Model {
        Model {
            player,
            coral,
            score: 0,
            frames: 0,
            game_over: false,
            had_coral: false,
        }
    }

    /// Move the player cursor, reading the board bounds from the reef. Kept on
    /// the model so the simultaneous borrows of `player` (mut) and `coral`
    /// (shared, for its size) are resolved in one place. Inert after game over.
    pub fn move_player(&mut self, dir: crate::player::Direction) {
        if self.game_over {
            return;
        }
        let w = self.coral.width();
        let h = self.coral.height();
        self.player.move_in(dir, w, h);
    }

    /// Plant/stamp a random coral shape at the cursor — the cookie-clicker
    /// verb. Each press drops a different kind of coral (from the stamp
    /// library). A small immediate score bump makes spamming the key feel
    /// responsive before the life sim catches up.
    ///
    /// Because stamping writes **Healthy** coral (via the age-resetting player
    /// path), this is also how the player *fixes* an unhealthy cell under the
    /// Conway rules: stamp over it and it becomes healthy again. Inert after
    /// game over.
    pub fn plant(&mut self) {
        if self.game_over {
            return;
        }
        self.coral.stamp_random(self.player.x, self.player.y);
        self.score += 2;
    }

    /// Kill the diseased cell at the cursor, if the active rulebook allows it
    /// and a diseased cell is there. Awards score for a successful cleanup.
    /// Inert after game over.
    pub fn kill_disease(&mut self) {
        if self.game_over {
            return;
        }
        let killed = self.coral.kill_disease(self.player.x, self.player.y);
        self.score += killed as i64 * 3;
        
    }

    /// Advance the simulation to ruscii step `step`.
    ///
    /// Runs the periodic life and disease updates when `step` crosses their
    /// period boundaries, accrues score for healthy coral *on each life
    /// update* (not every frame, which would inflate the score), and checks
    /// the lose condition.
    pub fn tick(&mut self, step: usize) {
        // Once the reef is gone, the game is over: freeze the simulation
        // entirely (no more life/disease steps, no score changes).
        if self.game_over {
            return;
        }

        // Guard against being called twice for the same step.
        if step == self.frames {
            return;
        }
        self.frames = step;

        let mut ran_life = false;

        // Life automaton (rulebook-defined). Passes the frame so timer-based
        // rules can measure elapsed time.
        if step % LIFE_PERIOD == 0 {
            self.coral.life_step(step);
            ran_life = true;
        }

        // Disease automaton (rulebook-defined), plus an externally-applied
        // fresh infection on the same cadence to keep pressure on the reef.
        if step % DISEASE_PERIOD == 0 {
            self.coral.disease_step(step);
            self.coral.seed_disease(step);
        }

        let (healthy, unhealthy, diseased) = self.coral.census();

        // Score for healthy coral accrues once per life update.
        if ran_life {
            self.score += healthy as i64 * SCORE_PER_HEALTHY;
        }

        let total = healthy + unhealthy + diseased;
        if total > 0 {
            self.had_coral = true;
        }
        if self.had_coral && healthy == 0 {
            self.game_over = true;
        }
    }
}
