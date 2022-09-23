use std::default;

use bevy::{
    math::vec2,
    prelude::{
        info, Assets, Bundle, Changed, Children, Color, Component, CoreStage, Mesh,
        ParallelSystemDescriptorCoercion, Plugin, Query, Res, ResMut, UVec2, With,
    },
    render::mesh::Indices,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use sark_grids::Size2d;

use crate::{code_page_437, renderer::uv_mapping::UvMapping, Terminal, TerminalMaterial};

use super::{
    tile_data::TileData, vertex_data::VertexData,
};

#[derive(Component)]
pub(crate) struct TerminalBorder {
    glyphs: [char; 6],
    size: UVec2,
}

#[derive(Bundle)]
pub struct TerminalBorderBundle {
    border: TerminalBorder,
    renderer: MaterialMesh2dBundle<TerminalMaterial>,
    vert_data: VertexData,
    tile_data: TileData,
}

impl TerminalBorderBundle {
    pub fn with_size(size: impl Size2d) -> Self {
        TerminalBorderBundle {
            border: TerminalBorder {
                glyphs: ['a', 'b', 'c', 'd', 'e', 'f'],
                size: size.as_uvec2(),
            },
            renderer: Default::default(),
            vert_data: VertexData::with_size(size),
            tile_data: TileData::border_tiles(size),
        }
    }
}

fn update_border(
    mut q_term: Query<(&Terminal, &Children), Changed<Terminal>>,
    mut q_border: Query<(&mut TerminalBorder, &mut Mesh2dHandle)>,
    meshes: Res<Assets<Mesh>>,
) {
    for (term, children) in &mut q_term {
        let w = term.width();
        let h = term.height();

        if children.is_empty() {
            continue;
        }

        for child in children {
            if let Ok((mut border, mut mesh)) = q_border.get_mut(*child) {
                // Size unchanged
                if border.size == term.size() {
                    continue;
                }

                border.size = term.size();
            }
        }
    }
}

fn resize_verts(size: UVec2, vert_data: &mut VertexData) {
    let len = size.x as usize * 2 + ((size.y as usize - 2) * 2);
}

fn update_mesh(
    mut q_border: Query<
        (
            &TerminalBorder,
            &mut VertexData,
            &mut TileData,
            &Mesh2dHandle,
        ),
        Changed<TerminalBorder>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (border, mut vert_data, mut tile_data, mesh) in &mut q_border {
        info!("Resizing border mesh");
        vert_data.border_resize([0, 0], border.size, vec2(1.0, 1.0));

        tile_data.border_resize(border.size);
        let mesh = meshes
            .get_mut(&mesh.0)
            .expect("Error retrieving mesh from terminal renderer");

        tile_data.border_update(
            border.size,
            Color::WHITE,
            Color::BLACK,
            &border.glyphs,
            &UvMapping::code_page_437(),
        );

        //info!("Vert len {}, uv len {}", vert_data.verts.len() / 4, tile_data.uvs.len() / 4);

        mesh.set_indices(Some(Indices::U32(vert_data.indices.clone())));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vert_data.verts.clone());
    }
}

pub(crate) struct BorderPlugin;

impl Plugin for BorderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(
            update_mesh
                .before(super::TERMINAL_UPDATE_TILE_DATA)
                .after(super::TERMINAL_INIT),
        );
    }
}