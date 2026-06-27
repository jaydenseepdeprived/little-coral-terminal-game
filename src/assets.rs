//! # Assets
//!
//! The **Technical Details** slide states *"All assets are stored as .txt
//! files"* and the **Code Structure** slide gives `Coral::ImportAsset()`.
//! This module is that loader, generalised so the player sprite and coral
//! "stamps" all flow through one code path.
//!
//! An asset is just a rectangular-ish block of text. We load it into a
//! `Vec<Vec<char>>` (rows of chars) so the renderer can blit it onto the
//! board at an (x, y) origin.

use std::fs;
use std::path::Path;

/// A loaded text asset: a grid of characters plus its bounding size.
///
/// Rows are padded to equal width on load so callers never have to worry
/// about ragged lines when stamping.
#[derive(Clone, Debug)]
pub struct Asset {
    /// Character grid, indexed `[row][col]`.
    pub rows: Vec<Vec<char>>,
    /// Width of the widest row (all rows padded to this).
    pub width: usize,
    /// Number of rows.
    pub height: usize,
}

impl Asset {
    /// `Coral::ImportAsset()` from the slides.
    ///
    /// Reads a `.txt` file from disk and returns it as an [`Asset`]. Tabs are
    /// expanded to spaces (4 wide) so alignment survives, and every row is
    /// right-padded with spaces to the maximum width.
    ///
    /// # Errors
    /// Returns the underlying `std::io::Error` if the file can't be read,
    /// so the caller can decide whether to fall back to a built-in sprite.
    pub fn import<P: AsRef<Path>>(path: P) -> std::io::Result<Asset> {
        let raw = fs::read_to_string(path)?;
        Ok(Asset::from_str(&raw))
    }

    /// Build an asset directly from an in-memory string. Used both by
    /// [`Asset::import`] and by the built-in fallback sprites, so the padding
    /// logic lives in exactly one place.
    pub fn from_str(raw: &str) -> Asset {
        // Expand tabs and split into rows of chars.
        let mut rows: Vec<Vec<char>> = raw
            .replace('\t', "    ")
            .lines()
            .map(|line| line.chars().collect())
            .collect();

        // Drop a trailing empty line (common when files end in "\n").
        if rows.last().map(|r| r.is_empty()).unwrap_or(false) {
            rows.pop();
        }

        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);

        // Pad every row to the same width so blitting is uniform.
        for row in &mut rows {
            while row.len() < width {
                row.push(' ');
            }
        }

        let height = rows.len();
        Asset { rows, width, height }
    }
}
