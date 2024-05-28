use bevy::prelude::{BackgroundColor, Bundle, Color, Component, default, Display, GridTrack, JustifyContent, NodeBundle, Style, UiImage, UiRect, Val};
use bevy::ui::node_bundles;

/// Empty component to indicate that an entity is a board.
#[derive(Debug, Component)]
pub struct Board;

/// Component that stores a position inside the board.
#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub struct Position {
    row: u32,
    col: u32,
}

impl Position {
    /// Creates [`Position`] from row and col
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Getter for `self.row`
    pub fn row(&self) -> u32 {
        self.row
    }

    /// Getter for `self.col`
    pub fn col(&self) -> u32 {
        self.col
    }
}

// Bundles

/// Bundle for a board.
/// Contains [`NodeBundle`] and a [`Board`].
/// `self.node` must have a [`Style`] component with`display` set to [`Display::Grid`]
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

/// Bundle for board button.
/// Contains [`node_bundles::ButtonBundle`] and [`Position`].
#[derive(Debug, Bundle)]
pub struct ButtonBundle {
    pub button: node_bundles::ButtonBundle,
    pub position: Position,
}

/// Bundle for image inside a button.
/// Contains [`NodeBundle`] and [`UiImage`].
#[derive(Debug, Bundle)]
pub struct ButtonContentBundle {
    pub node: NodeBundle,
    pub image: UiImage,
}
