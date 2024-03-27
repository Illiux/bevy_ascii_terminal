use std::iter::repeat;

use bevy::{
    app::Plugin,
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, Changed, Or, With},
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::BuildChildren,
    math::{bounding::Aabb2d, IVec2, Vec2},
    render::{
        color::Color,
        mesh::{Indices, Mesh, MeshVertexAttribute, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::{PrimitiveTopology, VertexFormat},
    },
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};

use crate::{GridPoint, Pivot, Terminal};

use super::{material::TerminalMaterial, uv_mapping::UvMapping};

pub const ATTRIBUTE_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv", 1123131, VertexFormat::Float32x2);
pub const ATTRIBUTE_COLOR_BG: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color_Bg", 1123132, VertexFormat::Float32x4);
pub const ATTRIBUTE_COLOR_FG: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color_Fg", 1123133, VertexFormat::Float32x4);

pub struct TerminalMeshPlugin;

impl Plugin for TerminalMeshPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        //app.add_systems(Update, (add_and_remove, update).chain());
    }
}

#[derive(Component)]
pub struct HasBorder;

#[derive(Component)]
pub struct TerminalMeshRenderer {
    pub mesh_pivot: Pivot,
    /// The size of a tile of the terminal mesh in world space, as read from
    /// previous mesh rebuild.
    tile_size_world: Vec2,
    /// Terminal grid size as read from previous mesh rebuild.
    term_size_grid: IVec2,
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
        self.term_size_grid
    }

    /// Update cached mesh data.
    fn update_data(&mut self, term_size: IVec2, tile_size: Vec2) {
        self.term_size_grid = term_size;
        self.tile_size_world = tile_size;

        // Calculate mesh bounds
        let size = term_size.as_vec2() * tile_size;
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

    pub fn tile_size(&self) -> Vec2 {
        self.tile_size_world
    }
}

// fn add_remove_border(
//     mut commands: Commands,
//     mut q_term: Query<(Entity, &Terminal)>,
//     mut meshes: ResMut<Assets<Mesh>>,
// ) {
//     for (entity, term, mut border_entity) in &mut q_term {
//         if term.border().is_some() && border_entity.0.is_none() {
//             let mesh = new_tile_mesh();
//             let mesh_bundle = MaterialMesh2dBundle::<TerminalMaterial> {
//                 mesh: Mesh2dHandle(meshes.add(mesh)),
//                 ..Default::default()
//             };
//             let border = commands.spawn((mesh_bundle, BorderMesh)).id();
//             let border = commands.entity(entity).add_child(border).id();
//             border_entity.0 = Some(border);
//         }

//         if let Some(border_entity) = border_entity.0 {
//             if term.border().is_none() {
//                 commands.entity(border_entity).despawn();
//             }
//         }
//     }
// }

fn init_mesh(
    mut q_term: Query<(&Terminal, &mut Mesh2dHandle, &TerminalMeshRenderer), Added<Mesh2dHandle>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (term, mut handle, renderer) in &mut q_term {
        let len = term.tile_count();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_indices(Indices::U32(vec![0; len * 6]));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]; len * 4]);
        mesh.insert_attribute(ATTRIBUTE_UV, vec![[0.0; 2]; len * 4]);
        mesh.insert_attribute(ATTRIBUTE_COLOR_FG, vec![[0.0; 4]; len * 4]);
        mesh.insert_attribute(ATTRIBUTE_COLOR_BG, vec![[0.0; 4]; len * 4]);

        let origin = renderer.mesh_origin();
        let tile_size = renderer.tile_size();
        VertMesher::build_mesh_verts(origin, tile_size, &mut mesh, |mesher| {
            for (i, (p, _)) in term.iter_xy().enumerate() {
                mesher.set_tile(p.x, p.y, i);
            }
        });
        handle.0 = meshes.add(mesh);
    }
}

#[allow(clippy::type_complexity)]
fn update_mesh(
    q_term: Query<
        (
            Entity,
            &Terminal,
            &Mesh2dHandle,
            &TerminalMeshRenderer,
            &Handle<UvMapping>,
            Option<&HasBorder>,
        ),
        Changed<Terminal>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mappings: Res<Assets<UvMapping>>,
    mut commands: Commands,
) {
    for (term_entity, term, mesh_handle, renderer, mapping, has_border) in &q_term {
        let Some(mesh) = meshes.get_mut(mesh_handle.0.clone()) else {
            panic!("Couldn't find terminal mesh");
        };
        let Some(mapping) = mappings.get(mapping.clone()) else {
            panic!("Couldn't find terminal uv mapping");
        };
        UVMesher::build_mesh_tile_data(mapping, mesh, |mesher| {
            for (i, t) in term.tiles().iter().enumerate() {
                mesher.set_tile(t.glyph, t.fg_color, t.bg_color, i);
            }
        });

        let tile_count = term.tile_count();
        if term.get_border().is_some() {
            commands.entity(term_entity).insert(HasBorder);
        } else if mesh.count_vertices() > tile_count * 4 {
        }
    }
}

fn resize_mesh(mesh: &mut Mesh, tile_count: usize) {
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
        let Some(Indices::U32(mut indices)) = mesh.remove_indices() else {
            panic!("Incorrect terminal mesh indices format");
        };
        let Some(VertexAttributeValues::Float32x3(mut verts)) =
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

    #[inline]
    pub fn set_tile(&mut self, x: i32, y: i32, index: usize) {
        let p = (self.origin + Vec2::new(x as f32, y as f32) * self.tile_size).extend(0.0);
        let right = (Vec2::X * self.tile_size).extend(0.0);
        let up = (Vec2::Y * self.tile_size).extend(0.0);

        let i = index * 4;
        self.verts[i] = (p + up).into();
        self.verts[i + 1] = p.into();
        self.verts[i + 2] = (p + right + up).into();
        self.verts[i + 3] = (p + right).into();

        let vi = i as u32;
        self.indices[i] = vi;
        self.indices[i + 1] = vi + 1;
        self.indices[i + 2] = vi + 2;
        self.indices[i + 3] = vi + 3;
        self.indices[i + 4] = vi + 2;
        self.indices[i + 5] = vi + 1;
    }

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
