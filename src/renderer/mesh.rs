use std::iter::repeat;

use bevy::{
    app::{Plugin, PostUpdate},
    asset::{AssetEvent, Assets, Handle},
    ecs::{
        change_detection::DetectChangesMut,
        component::Component,
        event::EventReader,
        query::{Added, Changed, With},
        schedule::{IntoSystemConfigs, SystemSet},
        system::{Query, Res, ResMut},
    },
    math::{bounding::Aabb2d, IRect, IVec2, Rect, Vec2},
    render::{
        color::Color,
        mesh::{Indices, Mesh, MeshVertexAttribute, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::{PrimitiveTopology, VertexFormat},
        texture::Image,
    },
    sprite::Mesh2dHandle,
};

use crate::{GridPoint, GridRect, Pivot, Terminal};

use super::{material::TerminalMaterial, uv_mapping::UvMapping, TerminalRenderSettings};

pub const ATTRIBUTE_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv", 1123131, VertexFormat::Float32x2);
pub const ATTRIBUTE_COLOR_BG: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color_Bg", 1123132, VertexFormat::Float32x4);
pub const ATTRIBUTE_COLOR_FG: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color_Fg", 1123133, VertexFormat::Float32x4);

pub struct TerminalMeshPlugin;

impl Plugin for TerminalMeshPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            PostUpdate,
            (
                init_mesh,
                on_image_load,
                on_image_change,
                on_renderer_change,
                on_terminal_change,
                reset_terminal_state,
            )
                .chain()
                .in_set(TerminalMeshSystems),
        );
    }
}

/// System for tracking camera and cursor data.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, SystemSet)]
pub struct TerminalMeshSystems;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct UpdateMeshVerts;

#[derive(Component)]
pub struct TerminalMeshRenderer {
    pub mesh_pivot: Pivot,
    pixels_per_tile: IVec2,
    /// The size of a tile of the terminal mesh in world space, as read from
    /// previous mesh rebuild.
    tile_size_world: Vec2,
    /// Terminal grid size as read from previous mesh rebuild.
    term_grid_size: IVec2,
    mesh_bounds: Aabb2d,
}

impl TerminalMeshRenderer {
    /// The local 2d bounds of the rendered terminal mesh in local
    /// space, as derived from the most previous mesh rebuild.
    pub fn mesh_bounds(&self) -> Aabb2d {
        self.mesh_bounds
    }

    /// Returns the world position (bottom left corner) of a mesh tile in the
    /// terminal from it's tile index. Note this ignores bounds.
    ///
    /// Tile indices range from 0 at the bottom/left to size-1 at the top/right.
    pub fn tile_position_world(&self, xy: impl GridPoint) -> Vec2 {
        self.mesh_bounds.min + xy.as_vec2() * self.tile_size_world
    }

    /// The grid size of the terminal
    pub fn terminal_grid_size(&self) -> IVec2 {
        self.term_grid_size
    }

    /// Update cached mesh data. Called whenever a terminal's font image changes.
    fn update_data(&mut self, term_size: IVec2, tile_size_pixels: IVec2, tile_size_world: Vec2) {
        self.term_grid_size = term_size;
        self.tile_size_world = tile_size_world;
        self.pixels_per_tile = tile_size_pixels;

        // Calculate mesh bounds
        let size = term_size.as_vec2() * tile_size_world;
        let pivot = self.mesh_pivot.normalized();
        // Truncate to a grid position
        let min = -(size * pivot).as_ivec2().as_vec2();
        let max = min + size;
        let bounds = Aabb2d { min, max };
        self.mesh_bounds = bounds;
    }

    pub fn mesh_origin(&self) -> Vec2 {
        self.mesh_bounds.min
    }

    pub fn tile_size_world(&self) -> Vec2 {
        self.tile_size_world
    }

    pub fn world_to_tile(&self, world_pos: impl Into<Vec2>) -> Option<IVec2> {
        let world_pos: Vec2 = world_pos.into();
        let pos = ((world_pos - self.mesh_bounds.min) / self.tile_size_world)
            .floor()
            .as_ivec2();
        if pos.cmplt(IVec2::ZERO).any() || pos.cmpge(self.term_grid_size).any() {
            return None;
        }
        Some(pos)
    }

    pub fn world_grid(&self, world_pos: Vec2) -> GridRect {
        let size = self.term_grid_size.as_vec2();
        let bl = world_pos + (size * self.mesh_pivot.normalized()).floor();
        GridRect::new(bl.as_ivec2(), size.as_ivec2())
    }

    pub fn pixels_per_tile(&self) -> IVec2 {
        self.pixels_per_tile
    }

    // pub fn pixel_bounds(&self, world_pos: Vec2) -> Rect {

    // }
}

fn init_mesh(
    q_term: Query<&Mesh2dHandle, (Added<Mesh2dHandle>, With<Terminal>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for mesh_handle in &q_term {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_indices(Indices::U32(Vec::new()));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
        mesh.insert_attribute(ATTRIBUTE_UV, Vec::<[f32; 2]>::new());
        mesh.insert_attribute(ATTRIBUTE_COLOR_FG, Vec::<[f32; 4]>::new());
        mesh.insert_attribute(ATTRIBUTE_COLOR_BG, Vec::<[f32; 4]>::new());
        meshes.insert(mesh_handle.0.clone(), mesh);
    }
}

fn on_image_load(
    mut q_term: Query<(
        &mut TerminalMeshRenderer,
        &Terminal,
        &Handle<TerminalMaterial>,
    )>,
    materials: Res<Assets<TerminalMaterial>>,
    images: Res<Assets<Image>>,
    mut img_evt: EventReader<AssetEvent<Image>>,
    settings: Res<TerminalRenderSettings>,
) {
    for evt in img_evt.read() {
        let id = match evt {
            AssetEvent::LoadedWithDependencies { id } => id,
            _ => continue,
        };
        for (mut renderer, term, image) in
            q_term
                .iter_mut()
                .filter_map(|(renderer, term, mat_handle)| {
                    let mat = materials
                        .get(mat_handle.clone())
                        .expect("Error getting terminal material");
                    let image = mat
                        .texture
                        .clone()
                        .filter(|img_handle| img_handle.clone().id() == *id)
                        .and_then(|img_handle| images.get(img_handle.clone()))?;

                    Some((renderer, term, image))
                })
        {
            let pixels_per_tile = (image.size() / 16).as_ivec2();
            let tile_size_world = settings.tile_scaling.tile_size_world(image.size());
            renderer.update_data(term.size(), pixels_per_tile, tile_size_world);
        }
    }
}

fn on_image_change(
    mut q_term: Query<(
        &mut TerminalMeshRenderer,
        &Terminal,
        &Handle<TerminalMaterial>,
    )>,
    mut mat_evt: EventReader<AssetEvent<TerminalMaterial>>,
    materials: Res<Assets<TerminalMaterial>>,
    images: Res<Assets<Image>>,
    settings: Res<TerminalRenderSettings>,
) {
    for evt in mat_evt.read() {
        let event_id = match evt {
            AssetEvent::Modified { id } => id,
            _ => continue,
        };
        for (mut renderer, term, image) in
            q_term
                .iter_mut()
                .filter_map(|(renderer, term, mat_handle)| {
                    let mat = (mat_handle.clone().id() == *event_id).then(|| {
                        materials
                            .get(mat_handle.clone())
                            .expect("Error getting terminal material")
                    })?;
                    let image = mat
                        .texture
                        .clone()
                        .and_then(|img_handle| images.get(img_handle.clone()))?;
                    Some((renderer, term, image))
                })
        {
            let tile_size_pixels = (image.size() / 16).as_ivec2();
            let tile_size_world = settings.tile_scaling.tile_size_world(image.size());
            renderer.update_data(term.size(), tile_size_pixels, tile_size_world);
        }
    }
}

fn on_renderer_change(
    mut q_term: Query<
        (
            &Terminal,
            &Mesh2dHandle,
            &TerminalMeshRenderer,
            &Handle<UvMapping>,
        ),
        Changed<TerminalMeshRenderer>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mappings: Res<Assets<UvMapping>>,
) {
    for (term, mesh_handle, renderer, mapping) in &mut q_term {
        let mesh = meshes
            .get_mut(mesh_handle.0.clone())
            .expect("Error getting terminal mesh");
        resize_mesh_data(mesh, 0);

        let mapping = mappings
            .get(mapping.clone())
            .expect("Couldn't find terminal uv mapping");

        let origin = renderer.mesh_origin();
        let tile_size = renderer.tile_size_world();
        VertMesher::build_mesh_verts(origin, tile_size, mesh, |mesher| {
            for (p, _) in term.iter_xy() {
                mesher.add_tile(p.x, p.y);
            }
        });
        UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
            for t in term.tiles().iter() {
                mesher.add_tile(t.glyph, t.fg_color, t.bg_color);
            }
        });
        if let Some(border) = term.get_border() {
            VertMesher::build_mesh_verts(origin, tile_size, mesh, |mesher| {
                for (p, _) in border.iter() {
                    mesher.add_tile(p.x, p.y);
                }
            });

            UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
                for (_, t) in border.iter() {
                    mesher.add_tile(t.glyph, t.fg_color, t.bg_color);
                }
            });
        }
    }
}

#[allow(clippy::type_complexity)]
fn on_terminal_change(
    q_term: Query<
        (
            &Terminal,
            &Mesh2dHandle,
            &TerminalMeshRenderer,
            &Handle<UvMapping>,
        ),
        Changed<Terminal>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mappings: Res<Assets<UvMapping>>,
) {
    for (term, mesh_handle, renderer, mapping) in &q_term {
        let mesh = meshes
            .get_mut(mesh_handle.0.clone())
            .expect("Couldn't find terminal mesh");
        let mapping = mappings
            .get(mapping.clone())
            .expect("Couldn't find terminal uv mapping");

        UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
            for (i, t) in term.tiles().iter().enumerate() {
                mesher.set_tile(t.glyph, t.fg_color, t.bg_color, i);
            }
        });

        let mesh_tile_count = mesh_tile_count(mesh);
        if let Some(border) = term.get_border() {
            if mesh_tile_count == term.tile_count() {
                let origin = renderer.mesh_origin();
                let tile_size = renderer.tile_size_world();
                VertMesher::build_mesh_verts(origin, tile_size, mesh, |mesher| {
                    for (p, _) in border.iter() {
                        mesher.add_tile(p.x, p.y);
                    }
                });

                UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
                    for (_, t) in border.iter() {
                        mesher.add_tile(t.glyph, t.fg_color, t.bg_color);
                    }
                });
            } else if border.changed() {
                let mut i = term.tile_count();
                UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
                    for (_, t) in border.iter() {
                        mesher.set_tile(t.glyph, t.fg_color, t.bg_color, i);
                        i += 1;
                    }
                });
            }
        } else if mesh_tile_count != term.tile_count() {
            resize_mesh_data(mesh, term.tile_count());
        }
    }
}

fn reset_terminal_state(mut q_term: Query<&mut Terminal>) {
    for mut term in &mut q_term {
        if let Some(mut border) = term.bypass_change_detection().get_border_mut() {
            border.reset_changed_state();
        }
    }
}

/// Utility for updating terminal mesh vertices
pub struct VertMesher {
    origin: Vec2,
    tile_size: Vec2,
    indices: Vec<u32>,
    verts: Vec<[f32; 3]>,
}

impl VertMesher {
    pub fn build_mesh_verts(
        origin: Vec2,
        tile_size: Vec2,
        mesh: &mut Mesh,
        func: impl FnOnce(&mut Self),
    ) {
        let Some(Indices::U32(indices)) = mesh.remove_indices() else {
            panic!("Incorrect terminal mesh indices format");
        };
        let Some(VertexAttributeValues::Float32x3(verts)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Incorrect mesh terminal vertex format");
        };

        let mut mesher = Self {
            origin,
            tile_size,
            indices,
            verts,
        };
        func(&mut mesher);
        mesh.insert_indices(Indices::U32(mesher.indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesher.verts);
    }

    // #[inline]
    // pub fn set_tile(&mut self, x: i32, y: i32, index: usize) {
    //     let p = (self.origin + Vec2::new(x as f32, y as f32) * self.tile_size).extend(0.0);
    //     let right = (Vec2::X * self.tile_size).extend(0.0);
    //     let up = (Vec2::Y * self.tile_size).extend(0.0);

    //     let i = index * 4;
    //     self.verts[i] = (p + up).into();
    //     self.verts[i + 1] = p.into();
    //     self.verts[i + 2] = (p + right + up).into();
    //     self.verts[i + 3] = (p + right).into();

    //     let vi = i as u32;
    //     self.indices[i] = vi;
    //     self.indices[i + 1] = vi + 1;
    //     self.indices[i + 2] = vi + 2;
    //     self.indices[i + 3] = vi + 3;
    //     self.indices[i + 4] = vi + 2;
    //     self.indices[i + 5] = vi + 1;
    // }

    fn add_tile(&mut self, x: i32, y: i32) {
        let p = (self.origin + Vec2::new(x as f32, y as f32) * self.tile_size).extend(0.0);
        let right = (Vec2::X * self.tile_size).extend(0.0);
        let up = (Vec2::Y * self.tile_size).extend(0.0);

        let i = self.verts.len() / 4;
        self.verts
            .extend([p + up, p, p + right + up, p + right].map(|v| v.to_array()));

        let i = i as u32;
        self.indices.extend([i, i + 1, i + 2, i + 3, i + 2, i + 1]);
    }
}

/// Utility for updating terminal mesh vertex data
pub struct UVMesher<'a> {
    mapping: &'a UvMapping,
    uvs: Vec<[f32; 2]>,
    fg: Vec<[f32; 4]>,
    bg: Vec<[f32; 4]>,
}

impl<'a> UVMesher<'a> {
    pub fn build_mesh_tile_data(
        mapping: &'a UvMapping,
        mesh: &mut Mesh,
        func: impl FnOnce(&mut Self),
    ) {
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.remove_attribute(ATTRIBUTE_UV)
        else {
            panic!("Incorrect terminal mesh uv format");
        };
        let Some(VertexAttributeValues::Float32x4(fg)) = mesh.remove_attribute(ATTRIBUTE_COLOR_FG)
        else {
            panic!("Incorrect terminal mesh fg color format");
        };
        let Some(VertexAttributeValues::Float32x4(bg)) = mesh.remove_attribute(ATTRIBUTE_COLOR_BG)
        else {
            panic!("Incorrect terminal mesh bg color format");
        };

        let mut mesher = Self {
            mapping,
            uvs,
            fg,
            bg,
        };

        func(&mut mesher);

        mesh.insert_attribute(ATTRIBUTE_UV, mesher.uvs);
        mesh.insert_attribute(ATTRIBUTE_COLOR_FG, mesher.fg);
        mesh.insert_attribute(ATTRIBUTE_COLOR_BG, mesher.bg);
    }

    #[inline]
    pub fn set_tile(&mut self, glyph: impl Into<char>, fg: Color, bg: Color, index: usize) {
        let uvs = self.mapping.uvs_from_glyph(glyph.into());
        let i = index * 4;

        self.uvs[i..i + 4]
            .iter_mut()
            .zip(uvs)
            .for_each(|(tuv, uv)| *tuv = *uv);

        self.fg[i..i + 4].fill(fg.as_linear_rgba_f32());
        self.bg[i..i + 4].fill(bg.as_linear_rgba_f32());
    }

    fn add_tile(&mut self, glyph: impl Into<char>, fg: Color, bg: Color) {
        let uvs = self.mapping.uvs_from_glyph(glyph.into());
        self.uvs.extend(uvs);
        self.fg.extend(repeat(fg.as_linear_rgba_f32()).take(4));
        self.bg.extend(repeat(bg.as_linear_rgba_f32()).take(4));
    }
}

fn mesh_vertex_count(mesh: &Mesh) -> usize {
    let Some(VertexAttributeValues::Float32x3(verts)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("Incorrect mesh terminal vertex format");
    };
    verts.len()
}

fn mesh_tile_count(mesh: &Mesh) -> usize {
    mesh_vertex_count(mesh) / 4
}

fn resize_mesh_data(mesh: &mut Mesh, tile_count: usize) {
    let Some(Indices::U32(indices)) = mesh.indices_mut() else {
        panic!("Incorrect terminal mesh indices format");
    };
    indices.resize(tile_count * 6, 0);
    let Some(VertexAttributeValues::Float32x3(verts)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("Incorrect mesh terminal vertex format");
    };
    verts.resize(tile_count * 4, [0.0; 3]);
    let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(ATTRIBUTE_UV) else {
        panic!("Incorrect terminal mesh uv format");
    };
    uvs.resize(tile_count * 4, [0.0; 2]);
    let Some(VertexAttributeValues::Float32x4(fg)) = mesh.attribute_mut(ATTRIBUTE_COLOR_FG) else {
        panic!("Incorrect terminal mesh fg color format");
    };
    fg.resize(tile_count * 4, [0.0; 4]);
    let Some(VertexAttributeValues::Float32x4(bg)) = mesh.attribute_mut(ATTRIBUTE_COLOR_BG) else {
        panic!("Incorrect terminal mesh bg color format");
    };
    bg.resize(tile_count * 4, [0.0; 4]);
}
