//! # StampSet — the player's library of coral shapes
//!
//! The **Coral** slide describes *"stamps" — spawns an asset from a textfile*.
//! Originally there was a single stamp; now the player stamps one of *many*
//! shapes chosen at random, loaded from `assets/stamps/stamp1.txt`,
//! `stamp2.txt`, and so on.
//!
//! Drop a new `stampN.txt` into `assets/stamps/` and it's picked up
//! automatically the next run — no code change needed. Numbering should start
//! at 1 and be contiguous; loading stops at the first missing number.

use crate::assets::Asset;

/// A collection of coral stamp shapes the player can plant.
pub struct StampSet {
    /// The loaded stamps, in file-number order. Guaranteed non-empty: if no
    /// files are found, [`StampSet::load`] falls back to a built-in shape.
    stamps: Vec<Asset>,
}

impl StampSet {
    /// Load every `stamp{N}.txt` (N = 1, 2, 3, …) from `dir`, stopping at the
    /// first number that doesn't exist.
    ///
    /// If the directory is missing or empty, falls back to a single built-in
    /// stamp so the game is always playable. `fallback` is the text of that
    /// built-in shape.
    pub fn load(dir: &str, fallback: &str) -> StampSet {
        let mut stamps = Vec::new();

        let mut n = 1;
        loop {
            let path = format!("{dir}/stamp{n}.txt");
            match Asset::import(&path) {
                Ok(asset) => {
                    stamps.push(asset);
                    n += 1;
                }
                Err(_) => break,
            }
        }

        if stamps.is_empty() {
            eprintln!(
                "warning: no stamps found in {dir}/; using one built-in stamp"
            );
            stamps.push(Asset::from_str(fallback));
        }

        StampSet { stamps }
    }

    /// How many distinct stamps are available. Part of the public stamp-library
    /// API; handy for UI or debugging even though the core loop only needs
    /// [`StampSet::random`].
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.stamps.len()
    }

    /// Borrow the stamp at `index` (wrapped into range so any index is safe).
    /// Public API for deterministic selection (e.g. a future "cycle stamp"
    /// control); the core loop uses [`StampSet::random`].
    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> &Asset {
        &self.stamps[index % self.stamps.len()]
    }

    /// Borrow a uniformly random stamp.
    pub fn random(&self) -> &Asset {
        let i = rand::random::<usize>() % self.stamps.len();
        &self.stamps[i]
    }
}
