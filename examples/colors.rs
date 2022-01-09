use bevy::prelude::*;
use bevy_ascii_terminal::*;
use bevy_tiled_camera::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TerminalPlugin)
        .add_plugin(TiledCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .add_startup_system(setup.system())
        .add_system(draw_colors.system())
        .run();
}

fn setup(mut commands: Commands) {
    let size = [12, 27];
    let bundle = TerminalBundle::new().with_size(size);
    commands.spawn_bundle(bundle);

    commands.spawn_bundle(
        TiledCameraBundle::new()
            .with_pixels_per_tile(8)
            .with_tile_count(size),
    );
}

fn draw_colors(time: Res<Time>, mut q: Query<&mut Terminal>) {
    let hue_count = 24;
    let hue_max = 360.0;
    let hue_step = hue_max / hue_count as f32;

    let mut term = q.single_mut();
    term.clear();

    // Draw shaded hues
    for y in 0..hue_count {
        let hue = y as f32 * hue_step;
        let col: TileColor = Color::hsl(hue, 1.0, 0.05).into();
        term.put_bg_color([0, y], col);
        for x in 1..10 {
            let col: TileColor = Color::hsl(hue, 1.0, x as f32 / 10.0).into();
            term.put_bg_color([x, y], col);
        }
    }

    // Draw greys/sepia
    for x in 0..10 {
        let v = x as f32 / 10.0;
        let col: TileColor = Color::rgb(v, v, v).into();
        term.put_bg_color([x, 24], col);

        let mut light_scale = 0.9;
        if x <= 5 {
            light_scale = 0.75;
        };
        let col: TileColor = Color::hsl(35.0, 0.33, v * light_scale).into();
        term.put_bg_color([x, 25], col);
    }

    let a = time.time_since_startup().as_secs_f32().cos() / 2.0 + 0.5;
    let col = TileColor::rgba(u8::MAX, u8::MAX, u8::MAX, (a * 255.0) as u8);

    let t = term.get_tile_mut([11, 26]);
    t.glyph = '*';
    t.fg_color = col;
}
