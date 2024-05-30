use bevy::prelude::{
    default, Added, AlignItems, BackgroundColor, BuildChildren, Button, Changed, Color, Commands,
    Display, Entity, EventReader, EventWriter, GridPlacement, Interaction, JustifyContent,
    NodeBundle, Parent, Query, Style, UiRect, Val, With,
};
use bevy::ui::{node_bundles, UiImage};

use super::components::{Board, ButtonBundle, ButtonContentBundle};
use super::events::{ButtonContentArrived, ButtonPressed};
use crate::game::Position;

/// Default style for button image
fn button_image_style() -> Style {
    Style {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        margin: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

/// Default style for a board button.
/// Accepts `col` and `row` parameters which are used as a [`GridPlacement`]
/// for `grid_column` and `grid_row` respectively.
fn board_button_style(col: i16, row: i16) -> Style {
    Style {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        grid_column: GridPlacement::start(col),
        grid_row: GridPlacement::start(row),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// System that fills newly created board with buttons and borders.
/// For every row and column 1, 3, 5 elements are buttons.
/// Places in a grid between them are used for borders.
pub fn create(mut commands: Commands, new_board: Query<Entity, Added<Board>>) {
    for entity in new_board.iter() {
        commands.entity(entity).with_children(|builder| {
            for i in 1i16..=5 {
                for j in 1i16..=5 {
                    let i_is_odd = i % 2 != 0;
                    let j_is_odd = j % 2 != 0;
                    if i_is_odd && j_is_odd {
                        let position = Position::new((i / 2) as u32, (j / 2) as u32);
                        builder.spawn(ButtonBundle {
                            button: node_bundles::ButtonBundle {
                                style: board_button_style(i, j),
                                background_color: BackgroundColor(Color::YELLOW_GREEN),
                                ..default()
                            },
                            position,
                        });
                    }
                    if i_is_odd && !j_is_odd {
                        let background_color = BackgroundColor(Color::BLACK);
                        // vertical borders
                        builder.spawn(NodeBundle {
                            style: Style {
                                display: Display::Grid,
                                grid_column: GridPlacement::start(j),
                                grid_row: GridPlacement::start(i),
                                width: Val::Px(1.0),
                                ..default()
                            },
                            background_color,
                            ..default()
                        });
                        // horizontal borders
                        builder.spawn(NodeBundle {
                            style: Style {
                                display: Display::Grid,
                                grid_column: GridPlacement::start(i),
                                grid_row: GridPlacement::start(j),
                                height: Val::Px(1.0),
                                ..default()
                            },
                            background_color,
                            ..default()
                        });
                    }
                }
            }
        });
    }
}

/// Check if button is pressed and emit [`ButtonPressed`] event.
pub fn button_press(
    button: Query<(&Interaction, &Position, &Parent), (With<Button>, Changed<Interaction>)>,
    mut pressed: EventWriter<ButtonPressed>,
) {
    for (_, pos, parent) in button.iter().filter(|(&i, _, _)| i == Interaction::Pressed) {
        pressed.send(ButtonPressed {
            board: parent.get(),
            pos: *pos,
        });
    }
}

/// Receive [`ButtonContentArrived`] event and make content entity a child of a button entity.
pub fn add_content(
    mut commands: Commands,
    button: Query<(Entity, &Parent, &Position), With<Button>>,
    mut content_arrived: EventReader<ButtonContentArrived>,
) {
    for event in content_arrived.read().cloned() {
        if let Some((entity, _, _)) = button
            .iter()
            .find(|(_, parent, &pos)| parent.get() == event.board && pos == event.pos)
        {
            commands
                .entity(entity)
                .clear_children()
                .with_children(|builder| {
                    builder.spawn(ButtonContentBundle {
                        node: NodeBundle {
                            style: button_image_style(),
                            background_color: BackgroundColor(Color::WHITE),
                            ..default()
                        },
                        image: UiImage::new(event.image),
                    });
                });
        } else {
            println!("unable to get button with position: {:?}", event.pos);
            continue;
        }
    }
}
