use bevy::{ math::{Vec2, }, prelude::Color};

use crate::terminal::Tile;

#[derive(Default)]
pub struct TerminalRendererTileData {
    pub fg_colors: Vec<[f32;3]>,
    pub bg_colors: Vec<[f32;3]>,
    pub uvs: Vec<[f32; 2]>,
}

fn to_mesh_color(col: Color) -> [f32;3] {
    [col.r(), col.g(), col.b()]
}

impl TerminalRendererTileData {
    pub fn with_size(width: usize, height: usize) -> Self {
        let mut v = Self::default();
        v.resize(width,height);
        v
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        let len = width * height;

        self.fg_colors.resize(len * 4, Default::default());
        self.bg_colors.resize(len * 4, Default::default());
        self.uvs.resize(len * 4, Default::default());
    }

    pub fn update_from_tiles(&mut self, tiles: &Vec<Tile>) {
        let uv_size = Vec2::new(1.0 / 16.0, 1.0 / 16.0);
        let right = Vec2::new(uv_size.x, 0.0);
        let up = Vec2::new(0.0, uv_size.y);

        for (i, tile) in tiles.iter().enumerate() {
            let glyph = tile.glyph as usize;
            // flip y so 0 == top tile on sprite sheet
            let glyph_index = [glyph % 16, glyph / 16];
            
            let origin = Vec2::new(glyph_index[0] as f32 * uv_size.x, glyph_index[1] as f32 * uv_size.y);
    
            let vi = i * 4;
            let uvs = &mut self.uvs;
            uvs[vi + 0] = origin.into();
            uvs[vi + 1] = (origin + up).into();
            uvs[vi + 2] = (origin + right).into();
            uvs[vi + 3] = (origin + up + right).into();

            for j in vi..vi + 4 {
                self.fg_colors[j] = to_mesh_color(tile.fg_color);
                self.bg_colors[j] = to_mesh_color(tile.bg_color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Color;

    use crate::{render::renderer_tile_data::TerminalRendererTileData, terminal::Tile};

    #[test]
    fn resize_test() {
        let mut tiles: Vec<Tile> = vec![Tile::default(); 50];

        for tile in tiles.iter_mut() {
            *tile = Tile {
                fg_color: Color::BLUE,
                ..Default::default()
            }
        }
        
        let mut colors: TerminalRendererTileData = TerminalRendererTileData::with_size(25,25);
        colors.update_from_tiles(&tiles);

        assert_eq!([0.0,0.0,1.0], colors.fg_colors[0]);
    }
}