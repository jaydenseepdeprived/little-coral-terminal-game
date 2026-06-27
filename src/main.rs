//! # Coral Reef — a terminal ASCII clicker (ruscii edition)
//!
//! Terminal Game Jam · Group A. Plant, restore, and protect a coral reef whose
//! cells live by Conway's-Game-of-Life rules and are eaten away by spreading
//! disease; you fight back by stamping fresh coral.
//!
//! This is the **ruscii** build the design slides called for. ruscii owns the
//! frame loop, the alternate screen, raw-mode setup/teardown, and keyboard
//! polling, so there is no hand-rolled Game/terminal code here — `App::run`
//! drives everything and restores the terminal automatically on exit.
//!
//! ## Architecture (MVC)
//! - [`model`]      — game state + rules (`Model`, owns `Player` and `Coral`)
//! - [`view`]       — draws the reef with ruscii's `Pencil` (rainbow coral!)
//! - [`controller`] — turns ruscii keyboard state into model mutations
//! - [`coral`]      — the reef simulation (life step, disease step, stamping)
//! - [`player`]     — the movable planting cursor
//! - [`assets`]     — `.txt` asset loader (`Coral::ImportAsset`)
//! - [`constants`]  — layout/timing numbers from the Technical Details slide
//!
//! ## Controls
//! `WASD` to move · `SPACE` to plant · `Q`/`ESC` to quit.
//!
//! ## Build notes
//! On Linux, ruscii needs the X11 dev libraries (it uses them for key
//! press/release events): `sudo apt install libx11-dev` (Debian/Ubuntu) or
//! `sudo dnf install xorg-x11-server-devel` (Fedora/RHEL). Windows and macOS
//! need nothing extra. Then: `cargo run --release`.

mod assets;
mod constants;
mod controller;
mod conway_rules;
mod coral;
mod model;
mod player;
mod rulebook;
mod rules;
mod stamps;
mod view;

use ruscii::app::{App, State};
use ruscii::terminal::Window;

use assets::Asset;
use constants::Layout;
use controller::Controller;
use conway_rules::ConwayRules;
use coral::Coral;
use model::Model;
use player::Player;
use rulebook::Rulebook;
use rules::CoralRules;
use stamps::StampSet;
use view::View;

/// Which ruleset the game runs. Change `ACTIVE_RULESET` to switch; both
/// rulebooks are real, compiled, and interchangeable.
#[derive(Copy, Clone)]
enum RuleSet {
    /// Classic health-ladder rules (`CoralRules`).
    Classic,
    /// Timer-driven rules: pure-Conway healthy cells, expiring unhealthy cells,
    /// a separate disease automaton, killable disease (`ConwayRules`).
    Conway,
}

/// The ruleset the game launches with. Flip this to `RuleSet::Classic` for the
/// original behaviour.
const ACTIVE_RULESET: RuleSet = RuleSet::Conway;

/// Build the boxed rulebook for the chosen [`RuleSet`].
fn make_rulebook(which: RuleSet) -> Box<dyn Rulebook> {
    match which {
        RuleSet::Classic => Box::new(CoralRules::default()),
        RuleSet::Conway => Box::new(ConwayRules::default()),
    }
}

/// Built-in fallbacks so the game still runs if assets are missing. Loading
/// from disk is preferred (that's the spec), but a playable fallback beats a
/// crash.
const FALLBACK_PLAYER: &str = "  o\n /|\\\n / \\\n";
const FALLBACK_STAMP: &str = " @\n@@@\n @\n";

/// How many coral stamps to drop on the board at spawn, so the player starts
/// with "a couple of coral stamps" rather than a blank reef.
const SPAWN_STAMPS: usize = 3;

/// Load an asset from `path`, falling back to an embedded default (with a
/// warning to stderr, which ruscii keeps separate from the rendered screen).
fn load_or_default(path: &str, fallback: &str) -> Asset {
    match Asset::import(path) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("warning: could not read {path}; using built-in asset");
            Asset::from_str(fallback)
        }
    }
}

fn main() {
    // Player sprite via the .txt pipeline (Coral::ImportAsset on the slides).
    let player_sprite = load_or_default("assets/player.txt", FALLBACK_PLAYER);
    // The library of coral stamps: every assets/stamps/stampN.txt.
    let stamp_set = StampSet::load("assets/stamps", FALLBACK_STAMP);

    let mut app = App::default();

    // The board is sized once, from the first frame's window size. We build the
    // model lazily inside the closure on the first tick because we need the
    // window dimensions ruscii reports, which aren't available until run().
    let mut model: Option<Model> = None;
    // Keep the loaded assets available for the deferred construction.
    let mut pending = Some((player_sprite, stamp_set));

    app.run(|state: &mut State, window: &mut Window| {
        // First frame: now that we know the window size, build the model.
        if model.is_none() {
            let size = window.size();
            let layout = Layout::from_size(size.x as usize, size.y as usize);

            // If the terminal is too small, show a message and stop.
            if (size.x as usize) < layout.frame_w()
                || (size.y as usize) < layout.total_rows()
            {
                let (min_c, min_r) = Layout::min_terminal();
                let msg = format!(
                    "Terminal too small. Resize to at least {min_c}x{min_r} and rerun.",
                );
                let mut pencil = ruscii::drawing::Pencil::new(window.canvas_mut());
                pencil.draw_text(&msg, ruscii::spatial::Vec2::xy(1, 1));
                return;
            }

            let (sprite, stamps) = pending.take().unwrap();
            let player = Player::new(Some(sprite), layout);
            // Pick the active rulebook (see ACTIVE_RULESET / make_rulebook).
            let rules = make_rulebook(ACTIVE_RULESET);
            let mut coral = Coral::new(stamps, rules, layout);
            // Start the reef with a few stamps already planted.
            coral.seed_spawn(SPAWN_STAMPS);
            model = Some(Model::new(player, coral));
        }

        let model = model.as_mut().unwrap();

        // CONTROL: translate input into model changes; quit if requested.
        if !Controller::process(state, model) {
            state.stop();
            return;
        }

        // UPDATE: advance the simulation to ruscii's current step.
        model.tick(state.step());

        // RENDER: draw the frame.
        View::render(window, model);
    });
}
