use bevy::prelude::{BackgroundColor, Bundle, Color, Component, default, Display, GridTrack, JustifyContent, NodeBundle, Style, UiImage, UiRect, Val};
use bevy::ui::node_bundles;

#[derive(Debug, Component)]
pub struct Board;

#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub struct Position {
    row: u32,
    col: u32,
}

impl Position {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    pub fn row(&self) -> u32 {
        self.row
    }

    pub fn col(&self) -> u32 {
        self.col
    }
}

// Bundles

#[derive(Debug, Bundle)]
pub struct BoardBundle {
    pub node: NodeBundle,
    pub board: Board,
}

impl Default for BoardBundle {
    fn default() -> Self {
        Self {
            node: NodeBundle {
                style: Style {
                    height: Val::Percent(100.0),
                    aspect_ratio: Some(1.0),
                    display: Display::Grid,
                    margin: UiRect::all(Val::Px(10.0)),
                    padding: UiRect::all(Val::Px(20.0)),
                    grid_template_columns: vec![
                        GridTrack::flex(1.0),
                        GridTrack::min_content(),
                        GridTrack::flex(1.0),
                        GridTrack::min_content(),
                        GridTrack::flex(1.0),
                    ],
                    grid_template_rows: vec![
                        GridTrack::flex(1.0),
                        GridTrack::min_content(),
                        GridTrack::flex(1.0),
                        GridTrack::min_content(),
                        GridTrack::flex(1.0),
                    ],
                    row_gap: Val::Px(12.0),
                    column_gap: Val::Px(12.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::GRAY),
                ..default()
            },
            board: Board,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct ButtonBundle {
    pub button: node_bundles::ButtonBundle,
    pub position: Position,
}

#[derive(Debug, Bundle)]
pub struct ContentBundle {
    pub node: NodeBundle,
    pub image: UiImage,
}
