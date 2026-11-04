//! Level loading vignette: the source progress-bar screen drawn while a map loads.
//! See notes/research/loading-vignette-2026-11-04.md for the loader evidence.
use super::{AssetRef, texture::Texture};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// One level's `<level>pgb.bin` Vignette section (0050ef30), with its images
/// bound to `Vignette.cnt`. Geometry is in the source 640x480 basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadingVignette {
    /// Stored image name (`epoque1.bmp`); 0050e310 swaps the extension for GF.
    pub image_name: String,
    pub image: AssetRef<Texture>,
    /// Bitmap bar art (`SetBitmapBarMode`); absent selects the drawn default bar.
    pub bar: Option<LoadingBarArt>,
    /// `SetBitmapBarPosition(frame_y, fill_y, min_x, max_x)`: source rows of
    /// the frame and fill states, and the fill track within the bar art.
    pub frame_y: i32,
    pub fill_y: i32,
    pub min_x: i32,
    pub max_x: i32,
    pub outline_color: [f32; 4],
    pub inside_color: [f32; 4],
    /// Drawn-bar corner colours, in source order.
    pub bar_colors: [[f32; 4]; 4],
    /// `CreateBar(x1, y1, x2, y2)` as stored, inclusive. Bitmap mode subtracts
    /// `min_x` from x1/x2 to place the bar art (0050ef30).
    pub bar_rect: [i32; 4],
    /// `MaxValueBar`: the step count that fills the track.
    pub max_value: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadingBarArt {
    /// Stored bar image name (`bar.bmp`).
    pub name: String,
    pub image: AssetRef<Texture>,
}

impl LoadingVignette {
    pub fn validate(&self) -> Result<()> {
        let [x1, y1, x2, y2] = self.bar_rect;
        ensure!(x1 <= x2 && y1 <= y2, "loading bar rectangle is inverted");
        ensure!(
            self.min_x <= self.max_x,
            "loading bar fill track is inverted"
        );
        ensure!(self.max_value > 0, "loading bar maximum must be positive");
        ensure!(
            !self.image_name.is_empty(),
            "loading vignette image name is empty"
        );
        if let Some(bar) = &self.bar {
            ensure!(!bar.name.is_empty(), "loading bar image name is empty");
        }
        Ok(())
    }

    /// Screen rectangle of the bar art in bitmap mode: (x, y, width, height).
    pub fn bar_art_rect(&self) -> [i32; 4] {
        let [x1, y1, x2, y2] = self.bar_rect;
        [x1 - self.min_x, y1, x2 - x1 + 1, y2 - y1 + 1]
    }
}
