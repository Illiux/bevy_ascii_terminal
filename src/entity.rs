use bevy::prelude::{Bundle, GlobalTransform, Transform};
use sark_grids::{GridPoint, Size2d};

use crate::{renderer, Terminal, TerminalFont};

/// A bundle with all the required components for a terminal.
///
/// Can specify some properties of the terminal on initilaization.
#[derive(Bundle, Default)]
pub struct TerminalBundle {
    pub terminal: Terminal,
    pub transform: Transform,
    pub global_transform: GlobalTransform,

    #[bundle]
    pub renderer: renderer::TerminalRendererBundle,
}

impl TerminalBundle {
    pub fn new() -> Self {
        TerminalBundle::default()
    }

    /// Set the initial size of the terminal.
    pub fn with_size(mut self, size: impl Size2d) -> Self {
        self.terminal.resize(size.as_array());
        self
    }

    pub fn with_font(mut self, font: TerminalFont) -> Self {
        self.renderer.font = font;
        self
    }

    pub fn with_position(mut self, pos: impl GridPoint) -> Self {
        let p = self.transform.translation;
        self.transform.translation = pos.as_vec2().extend(p.z);
        self
    }

    /// Sets the intial z position for the terminal.
    pub fn with_depth(mut self, depth: i32) -> Self {
        self.transform.translation.z = depth as f32;
        self
    }
}
