//! # View (the "V" in MVC) — ruscii edition
//!
//! Draws the model onto a ruscii [`Window`] each frame using a [`Pencil`].
//! This is the part that the original `println!` build couldn't do properly:
//! ruscii's per-call foreground colour lets us honour the **Coral** slide's
//! spec directly —
//!
//! - **healthy: rainbow coloured** → each healthy cell gets an `Xterm` colour
//!   chosen from its board position, so the reef shimmers across the spectrum.
//! - **unhealthy: white** → exactly as the slide says.
//! - diseased coral is drawn in a dim sickly green so the threat reads at a
//!   glance, and the player cursor in bright yellow on top of everything.
//!
//! The View only *reads* the model (MVC discipline); all geometry comes from
//! the reef's runtime width/height so the board always matches the grid.

use ruscii::drawing::{Pencil, RectCharset};
use ruscii::spatial::Vec2;
use ruscii::terminal::{Color, Window};

use crate::coral::Cell;
use crate::model::Model;

/// Stateless renderer.
pub struct View;

impl View {
    /// Draw a full frame of the game to `window` from `model`.
    ///
    /// Two layouts: the normal play screen (title, bordered board, scoreboard,
    /// controls) and — once the reef has died — a dedicated game-over screen
    /// (a skull replacing the board, plus the final score). The simulation is
    /// already frozen by the model at that point; this just shows the result.
    pub fn render(window: &mut Window, model: &Model) {
        if model.game_over {
            Self::render_game_over(window, model);
        } else {
            Self::render_play(window, model);
        }
    }

    /// The normal in-play screen.
    fn render_play(window: &mut Window, model: &Model) {
        let board_w = model.coral.width() as i32;
        let board_h = model.coral.height() as i32;

        let mut pencil = Pencil::new(window.canvas_mut());

        // --- Title (row 0) ----------------------------------------------
        pencil.set_foreground(Color::Yellow);
        pencil.draw_text(">))@> fih grow coral :D <@((<", Vec2::xy(26, 0));

        // --- Board border (rows 1..) ------------------------------------
        // draw_rect's dimension includes the border chars, so a board of
        // (board_w x board_h) playable cells needs a rect of (board_w+2 x
        // board_h+2). The top-left corner sits at (0, 1).
        let board_origin = Vec2::xy(0, 1);
        pencil.set_foreground(Color::Grey);
        pencil.draw_rect(
            &RectCharset::simple_lines(),
            board_origin,
            Vec2::xy(board_w + 2, board_h + 2),
        );

        // --- Coral cells ------------------------------------------------
        // Cell (x, y) on the board draws at (x+1, y+2): +1 for the left
        // border column, +2 for the title row plus the top border row.
        // Glyph and colour both come from the coral's rules, so the reef's
        // appearance is tuned in one place (see `rules.rs`).
        let w = model.coral.width();
        let h = model.coral.height();
        let rules = &model.coral.rules;
        let grid = model.coral.grid();
        for y in 0..h {
            for x in 0..w {
                let cell = grid[y * w + x];
                if cell == Cell::Empty {
                    continue; // leave empties blank (transparent water)
                }
                let pos = Vec2::xy(x as i32 + 1, y as i32 + 2);
                pencil.set_foreground(rules.cell_color(cell, x, y));
                pencil.draw_char(rules.cell_glyph(cell), pos);
            }
        }

        // --- Player cursor (drawn last, on top) -------------------------
        let px = model.player.x as i32 + 1;
        let py = model.player.y as i32 + 2;
        pencil.set_foreground(Color::Yellow);
        pencil.draw_char(model.player.glyph_overlay(), Vec2::xy(px, py));

        // --- Scoreboard -------------------------------------------------
        // Two lines just below the board's bottom border.
        let score_y = board_h + 3; // title(1) + border rows(2) + board + 0-index
        let (healthy, unhealthy, diseased) = model.coral.census();
        pencil.set_foreground(Color::White);
        pencil.draw_text(
            &format!("SCORE {:>7}    FRAME {:>6}", model.score, model.frames),
            Vec2::xy(1, score_y),
        );
        pencil.draw_text(
            &format!(
                "healthy {:>4}   unhealthy {:>4}   diseased {:>4}",
                healthy, unhealthy, diseased
            ),
            Vec2::xy(1, score_y + 1),
        );

        // --- Controls / status ------------------------------------------
        let status_y = score_y + 3;
        pencil.set_foreground(Color::DarkGrey);
        // Show the kill-disease control only when the rulebook allows it.
        let controls = if model.coral.player_can_kill_disease() {
            "WASD: move   SPACE: plant/fix   ENTER: kill disease   Q/Esc: quit"
        } else {
            "WASD: move    SPACE: plant    Q/Esc: quit"
        };
        pencil.draw_text(controls, Vec2::xy(1, status_y));
    }

    /// The game-over screen: a skull centred where the board was, the final
    /// score beneath it, and a quit prompt. The simulation is frozen, so this
    /// is a static result screen.
    fn render_game_over(window: &mut Window, model: &Model) {
        let board_w = model.coral.width() as i32;
        let board_h = model.coral.height() as i32;
        // The framed area spans columns 0..=board_w+1 and rows 1..=board_h+1.
        // We centre content within that footprint.
        let frame_w = board_w + 2;

        let mut pencil = Pencil::new(window.canvas_mut());

        // Keep the title and the board border so the screen still reads as the
        // same game, just with a skull inside the frame.
        pencil.set_foreground(Color::Cyan);
        pencil.draw_text("------------------------------------------------------------------------------", Vec2::xy(1, 0));

        pencil.set_foreground(Color::Grey);
        pencil.draw_rect(
            &RectCharset::simple_lines(),
            Vec2::xy(0, 1),
            Vec2::xy(frame_w, board_h + 2),
        );

        // Vertically position the skull block so it (plus the text lines under
        // it) sits roughly centred in the board area. Board interior rows run
        // 2..=board_h+1.
        let block_height = SKULL.len() as i32 + 3; // skull + blank + 2 text lines
        let interior_top = 2;
        let mut y = interior_top + ((board_h - block_height).max(0)) / 2;

        // Draw the skull, each line horizontally centred in the frame.
        pencil.set_foreground(Color::White);
        for line in SKULL.iter() {
            let x = ((frame_w - line.chars().count() as i32) / 2).max(0);
            pencil.draw_text(line, Vec2::xy(x, y));
            y += 1;
        }

        // Final score, centred, just below the skull.
        y += 1;
        let score_line = format!("GAME OVER   -   FINAL SCORE: {}", model.score);
        let sx = ((frame_w - score_line.chars().count() as i32) / 2).max(0);
        pencil.set_foreground(Color::Red);
        pencil.draw_text(&score_line, Vec2::xy(sx, y));

        // Quit prompt, centred, below the score.
        y += 1;
        let prompt = "Press Q or Esc to quit";
        let qx = ((frame_w - prompt.chars().count() as i32) / 2).max(0);
        pencil.set_foreground(Color::DarkGrey);
        pencil.draw_text(prompt, Vec2::xy(qx, y));
    }
}

/// ASCII-art skull shown on the game-over screen. Each entry is one row; the
/// view centres them horizontally within the board frame.
const SKULL: [&str; 16] = [
     "    _______________   ",
     "   /               \\  ",
     "  /                 \\ ",
     " /                   \\",
     " |   XXXX     XXXX   |",
     " |   XXXX     XXXX   |",
     " |   XXX       XXX   |",
     " |         X         |",
     " \\__      XXX     __/ ",
     "   |\\     XXX     /|  ",
     "   | |           | |  ",
     "   | I I I I I I I |  ",
     "   |  I I I I I I  |  ",
     "    \\_           _/   ",
     "     \\_         _/    ",
     "       \\_______/      ",
];
