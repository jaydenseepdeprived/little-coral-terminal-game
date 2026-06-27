//! # Controller (the "C" in MVC) — ruscii edition
//!
//! ruscii's `App` owns the loop, so the controller is no longer a loop driver;
//! it's a pure translator from ruscii keyboard state to model mutations. It is
//! the only place key input becomes game state.
//!
//! ## Two input styles, used deliberately
//! ruscii exposes both:
//! - `keyboard().get_keys_down()` — keys currently *held*. We use this for
//!   movement so holding a direction glides the cursor smoothly (as in the
//!   official space_invaders example).
//! - `keyboard().last_key_events()` — discrete press/release *events*. We use
//!   `Pressed` events for plant and quit so each tap is one action and holding
//!   the key doesn't machine-gun.
//!
//! ## Keys
//! Movement: `W/A/S/D`. (ruscii's `Key` also has vim-style `H/J/K/L` and
//! numpad variants used by its examples; arrow-key variants exist too and can
//! be added to the match arms the same way if desired.)
//! Plant/fix: `Space`. Kill diseased cell: `Enter` (if the rulebook allows it).
//! Quit: `Q` / `Esc`.

use ruscii::app::State;
use ruscii::keyboard::{Key, KeyEvent};

use crate::model::Model;
use crate::player::Direction;

/// Stateless input translator.
pub struct Controller;

impl Controller {
    /// Apply this frame's input to the model. Returns `false` if the player
    /// asked to quit (the caller then stops the ruscii app).
    pub fn process(state: &mut State, model: &mut Model) -> bool {
        let mut keep_running = true;

        // Discrete presses: plant, kill-disease, and quit.
        for event in state.keyboard().last_key_events() {
            match event {
                KeyEvent::Pressed(Key::Q) | KeyEvent::Pressed(Key::Esc) => {
                    keep_running = false;
                }
                KeyEvent::Pressed(Key::Space) => {
                    model.plant();
                }
                KeyEvent::Pressed(Key::Enter) => {
                    // Kill a diseased cell at the cursor (no-op unless the
                    // active rulebook allows it and a diseased cell is there).
                    model.kill_disease();
                }
                _ => {}
            }
        }

        // Held keys: movement. Holding a direction moves the cursor each frame.
        for key in state.keyboard().get_keys_down() {
            match key {
                Key::W => model.move_player(Direction::Up),
                Key::S => model.move_player(Direction::Down),
                Key::A => model.move_player(Direction::Left),
                Key::D => model.move_player(Direction::Right),
                _ => {}
            }
        }

        keep_running
    }
}
