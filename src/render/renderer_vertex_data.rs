use bevy::math::Vec3;

#[derive(Default)]
pub struct TerminalRendererVertexData {
    pub verts: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl TerminalRendererVertexData {
    pub fn with_size(width: usize, height: usize) -> Self {
        let mut v = Self::default();
        v.resize(width, height, (1, 1));
        v
    }

    pub fn resize(&mut self, width: usize, height: usize, tile_size: (usize, usize)) {
        let len = width * height;

        self.verts.resize(len * 4, Default::default());
        self.indices.resize(len * 6, 0);

        let (tx, ty) = (tile_size.0 as f32, tile_size.1 as f32);

        for i in 0..len {
            let x = (i % width) as f32 * tx;
            let y = (i / width) as f32 * ty;
            let origin = Vec3::new(x, y, 0.0);
            let right = Vec3::X * tx;
            let up = Vec3::Y * ty;

            let vi = i * 4;
            // 0---2
            // | / |
            // 1---3
            let verts = &mut self.verts;
            verts[vi] = (origin + up).into();
            verts[vi + 1] = origin.into();
            verts[vi + 2] = (origin + right + up).into();
            verts[vi + 3] = (origin + right).into();

            let ii = i * 6;
            let vi = vi as u32;
            let indices = &mut self.indices;
            indices[ii] = vi + 0;
            indices[ii + 1] = vi + 1;
            indices[ii + 2] = vi + 2;
            indices[ii + 3] = vi + 3;
            indices[ii + 4] = vi + 2;
            indices[ii + 5] = vi + 1;
        }
    }
}
