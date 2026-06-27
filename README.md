# Coral Reef — Terminal ASCII Clicker (ruscii edition)

Terminal Game Jam · Group A. Plant, restore, and protect a coral reef that
lives by Conway's-Game-of-Life rules and is eaten by spreading disease. Built
on the **ruscii** terminal engine, as the design slides specified.

This is a port of the earlier plain-`println!` build. The game logic (coral
simulation, player, assets, layout) is unchanged; what changed is the
rendering and the loop: ruscii now owns the frame loop, the screen, raw-mode
setup/teardown, and keyboard input, and its `Pencil` gives us per-cell colour
— so the slide's spec finally renders properly: **healthy coral in rainbow,
unhealthy in white.**

## Build & run

ruscii needs the X11 development libraries on Linux (it uses them for key
press/release events). Windows and macOS need nothing extra.

```bash
# Linux (Debian/Ubuntu):
sudo apt install libx11-dev
# Linux (Fedora/RHEL/CentOS):
sudo dnf install xorg-x11-server-devel

cargo run --release
```

Run from the project root (or alongside `assets/`) so the `.txt` assets load;
if they're missing the game uses built-in fallback sprites.

## Controls

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` | move the planting cursor (hold to glide) |
| `Space` | plant (stamp) coral |
| `Q` / `Esc` | quit |

## How it plays

Steer the yellow `X` cursor and stamp coral. The reef then evolves on its own:

- **Conway-style health.** Each coral cell checks its 8 neighbours. With 2–3 it
  heals a step; otherwise it declines a step. Empty cells with exactly 3
  neighbours sprout new coral. Health ladders
  `Diseased → Unhealthy → Healthy` and back down to death.
- **Disease.** Independently, disease spreads from infected cells and a fresh
  infection is seeded periodically. Re-stamping heals diseased cells — that's
  how you fight back.
- **Score** rises with healthy-coral count on each life update, plus a little
  per manual plant. Game over only if the reef once lived and then died out.

Glyphs: `@` healthy (rainbow) · `o` unhealthy (white) · `#` diseased (sickly
green) · `X` you (yellow).

## Architecture (MVC)

| Module | Role |
|--------|------|
| `model.rs` | state + score; owns Player & Coral; `tick(step)` |
| `view.rs` | draws board/coral/scoreboard with ruscii `Pencil` |
| `controller.rs` | ruscii keyboard state → model mutations |
| `main.rs` | `App::run` loop wiring; picks the active rulebook |
| `coral.rs` | the grid + per-cell ages; delegates stepping to a `Rulebook` |
| `rulebook.rs` | **`Rulebook`** trait + **`Board`** interface (see below) |
| `rules.rs` | **`CoralRules`** — the classic health-ladder ruleset |
| `conway_rules.rs` | **`ConwayRules`** — the timer-driven ruleset (active) |
| `stamps.rs` | **`StampSet`** — the library of coral shapes (see below) |
| `player.rs` | the movable cursor |
| `assets.rs` | `.txt` asset loader (`Coral::ImportAsset`) |
| `constants.rs` | geometry (24×80 target) + timing |

## Controls

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` | move the cursor (hold to glide) |
| `Space` | plant a random coral / **fix** an unhealthy cell (stamp it healthy) |
| `X` | **kill** the diseased cell at the cursor (if the rulebook allows) |
| `Q` / `Esc` | quit |

## Coral stamps (`assets/stamps/`)

The player plants a **random** coral shape on each press, chosen from every
`assets/stamps/stamp{N}.txt` (numbered from 1, contiguous). Five ship by
default (seed cluster, branching, fan, brain, staghorn). Drop in a
`stamp6.txt` and it's picked up automatically next run — no code change. The
reef also starts with a few stamps already planted (`SPAWN_STAMPS` in
`main.rs`).

Stamp files are plain ASCII: non-space characters become coral, spaces are
left as open water, so the shape is preserved when stamped over existing reef.

## Rulebooks (`rulebook.rs`, `rules.rs`, `conway_rules.rs`)

The coral's behaviour and appearance live behind the **`Rulebook`** trait, so
completely different rulesets — different automata, per-cell timers, new player
verbs — can coexist. `Coral` just holds the grid (plus a parallel per-cell
**age** buffer for timers) and delegates each step to its boxed rulebook via
the **`Board`** interface. Two rulebooks ship:

**`CoralRules`** (`rules.rs`) — the original health-ladder ruleset. Living coral
heals (`2–3` neighbours) or declines (`Diseased → Unhealthy → Healthy ↔ Empty`)
each step; disease spreads to any coral and diseased cells die off; healthy is
rainbow, unhealthy white, diseased olive. Tunable via its public fields
(`birth_counts`, `survival_counts`, etc.).

**`ConwayRules`** (`conway_rules.rs`) — the **active** ruleset, with the
timer-driven semantics requested:

- **Healthy cells are pure Conway's Game of Life** (born on 3, survive on 2–3),
  rainbow-coloured. When a healthy cell dies by Conway, it becomes *Unhealthy*
  rather than vanishing.
- **Unhealthy cells are inert and temporary.** The player can *fix* one by
  planting over it (`Space` → healthy again); left alone, it disappears after
  `unhealthy_lifespan` frames (tracked via the per-cell age).
- **Disease is a separate automaton** that attacks **only healthy cells**:
  diseased cells infect adjacent healthy neighbours (marking them red) and die
  off after `disease_lifespan` frames. The player can **kill** a diseased cell
  with `X`.
- **Colours:** healthy = rainbow, unhealthy = grey, diseased = red.

### Switching or writing a rulebook

The active rulebook is chosen in `main.rs`:

```rust
let rules: Box<dyn Rulebook> = Box::new(ConwayRules::default());
// or: Box::new(rules::CoralRules::default());
```

To write your own: make a struct in a new file, `impl Rulebook for It` (define
`life_step` / `disease_step` against the `Board` interface, plus
`cell_glyph` / `cell_color`, and optionally `player_can_kill_disease`), then box
it in `main.rs`. The `Board` interface gives you neighbour-aware cell access and
the per-cell age buffer for timers.

## What ruscii replaced

The previous build hand-rolled the loop, raw-mode handling, a `Drop` guard to
restore the terminal, and string-buffer rendering via `println!`. ruscii
provides all of that:

- `App::run(|state, window| ...)` — the frame loop and terminal lifecycle.
- `State::keyboard().get_keys_down()` / `.last_key_events()` — held keys (for
  smooth movement) and discrete press events (for plant/quit).
- `State::step()` — the frame counter that paces the slow life/disease updates.
- `Window::canvas_mut()` + `Pencil` — drawing with `draw_char` / `draw_text` /
  `draw_rect`, and `set_foreground(Color::Xterm(..))` for the rainbow reef.

## Notes & possible extensions

- The board sizes itself to the terminal at startup (capped at the slide's
  80×24), so lines never wrap. Very small terminals get a "resize" message.
- ruscii's `Key` also exposes vim-style and numpad keys (its examples use
  `H/J/K/L`, `Numpad4/6`); arrow-key support can be added to the controller's
  match arms the same way as the WASD arms.
- The slide's starred future objects (debris, boats, yachts) would become
  extra vectors on `Model` and extra draw passes in `View`.
```
