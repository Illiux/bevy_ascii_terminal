use bevy::prelude::*;
use bevy_ascii_terminal::{*, ui::{UiBox, UiProgressBar}};
use bevy_tiled_camera::*;
use sark_grids::Pivot;

fn main() {
    App::new()
    .add_plugin(TiledCameraPlugin)
    .add_plugins(DefaultPlugins)
    .add_plugin(TerminalPlugin)
    .add_startup_system(setup)
    .run();
}

fn setup(
    mut commands: Commands,
) {
    let size = [25,25];
    let mut term_bundle = TerminalBundle::new().with_size(size);
    let term = &mut term_bundle.terminal;

    term.iter_mut().for_each(|t| t.glyph = 'n');


    let ui_box = UiBox::single_line();

    term.draw_box([0,0].pivot(Pivot::TopLeft), [4,4], &ui_box);

    let bar = UiProgressBar::default();

    term.draw_progress_bar([0,0].pivot(Pivot::TopRight), 10, &bar);


    commands.spawn_bundle(term_bundle);

    commands.spawn_bundle(TiledCameraBundle::new().with_tile_count(size));
}