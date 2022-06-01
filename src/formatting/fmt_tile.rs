use arrayvec::{ArrayVec};
use bevy::prelude::*;
use sark_grids::GridPoint;

use crate::{Tile, Terminal};

/// Formatting that can be applied to a terminal tile.
/// 
/// Formatting allows you to create an object that specifies certain aspects 
/// to modify without necessarily replacing an entire tile.
#[derive(Debug, Default, Clone)]
pub struct TileFormat {
    /// Modifications to be applied to a tile.
    modifications: ArrayVec<TileModification, 3>,
}

/// Modifications that can be applied to a terminal tile.
#[derive(Debug, Clone, Copy)]
pub enum TileModification {
    /// Change the glyph of a tile.
    Glyph(char),
    /// Change the foreground color of a tile.
    FGColor(Color),
    /// Change the background color of a tile.
    BGColor(Color),
}

/// A trait for building a [TileFormat].
pub trait TileModifier: Clone {
    /// Change the glyph of a tile.
    fn glyph(self, glyph: char) -> TileFormat;
    /// Change the foreground color of a tile.
    fn fg(self, color: Color) -> TileFormat;
    /// Change the background color of a tile.
    fn bg(self, color: Color) -> TileFormat;

    /// Get the [TileFormat] which can be used to apply tile modifications.
    fn format(self) -> TileFormat;
}

impl TileFormat {
    #[inline]
    /// Apply formatting to an existing tile without necessarily replacing it completely.
    pub fn apply(&self, tile: &mut Tile) {
        for write in self.modifications.iter() {
            match write {
                TileModification::Glyph(glyph) => tile.glyph = *glyph,
                TileModification::FGColor(col) => tile.fg_color = *col,
                TileModification::BGColor(col) => tile.bg_color = *col,
            }
        }
    }

    /// Create a [TileFormat] to clear a tile to default.
    pub fn clear() -> TileFormat {
        TileFormat::from(Tile::default())
    }

    /// Iterate over tile modifications.
    pub fn iter(&self) -> impl Iterator<Item=&TileModification> {
        self.modifications.iter()
    }

    #[inline]
    pub(crate) fn draw(&self, xy: impl GridPoint, term: &mut Terminal) {
        let t = term.get_tile_mut(xy);
        self.apply(t);
    }
}

impl TileModifier for TileFormat {
    /// Change the forergound color of a tile.
    fn fg(mut self, color: Color) -> TileFormat {
        self.modifications.push(TileModification::FGColor(color));
        self
    }

    /// Change the background color of a tile.
    fn bg(mut self, color: Color) -> TileFormat {
        self.modifications.push(TileModification::BGColor(color));
        self
    }

    /// Change the glyph of a tile.
    fn glyph(mut self, glyph: char) -> TileFormat {
        self.modifications.push(TileModification::Glyph(glyph));
        self
    }
    /// Get the [TileFormat] which can be used to apply tile modifications.
    fn format(self) -> TileFormat {
        self
    }
}

// impl TileModifier for &TileFormat {
//     fn glyph(mut self, glyph: char) -> TileFormat {
//         todo!()
//     }

//     fn fg(self, color: Color) -> TileFormat {
//         *self
//     }

//     fn bg(self, color: Color) -> TileFormat {
//         todo!()
//     }

//     fn format(self) -> TileFormat {
//         todo!()
//     }
// }

impl TileModifier for char {
    /// Replace the original character with a given one.
    /// 
    /// This is pointless.
    fn glyph(self, glyph: char) -> TileFormat {
        TileFormat::default().glyph(glyph)
    }

    /// Modify the foreground color of the tile.
    fn fg(self, color: Color) -> TileFormat {
        TileFormat::default().glyph(self).fg(color)
    }

    /// Modify the background color of the tile.
    fn bg(self, color: Color) -> TileFormat {
        TileFormat::default().glyph(self).bg(color)
    }

    /// Get the [TileFormat] for this character.
    fn format(self) -> TileFormat {
        TileFormat::default().glyph(self)
    }
}

impl From<TileFormat> for Tile {
    fn from(fmt: TileFormat) -> Self {
        let mut tile = Tile::default();
        fmt.apply(&mut tile);
        tile
    }
}

impl From<Tile> for TileFormat {
    fn from(tile: Tile) -> Self {
        TileFormat::default().glyph(tile.glyph).fg(tile.fg_color).bg(tile.bg_color)
    }
}