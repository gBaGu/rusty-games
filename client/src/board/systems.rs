use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::components::{Board, TileBundle};
use super::events::{TileFilled, TilePressed};
use super::BORDER_WIDTH;
use crate::game::Position;

fn border_bundle(color: Color, size: Vec2, x: f32, y: f32) -> SpriteBundle {
    SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(x, y, 1.0)),
        ..default()
    }
}

/// System that fills newly created board with tiles and borders.
pub fn create(mut commands: Commands, new_board: Query<(Entity, &Sprite), Added<Board>>) {
    for (entity, sprite) in new_board.iter() {
        let Some(board_size) = sprite.custom_size else {
            continue;
        };
        let tile_width = (board_size.x - BORDER_WIDTH * 2.0) / 3.0;
        let tile_height = (board_size.y - BORDER_WIDTH * 2.0) / 3.0;
        let tile_size = Vec2::new(tile_width, tile_height);
        commands.entity(entity).with_children(|builder| {
            for row in 0..3 {
                for col in 0..3 {
                    let tile_x = (tile_width + BORDER_WIDTH) * col as f32 + tile_width / 2.0
                        - board_size.x / 2.0;
                    let tile_y = (tile_height + BORDER_WIDTH) * row as f32 + tile_height / 2.0
                        - board_size.y / 2.0;
                    // invert y because server expects top left tile to be (0, 0)
                    let pos = Position::new(2 - row, col);
                    builder.spawn(TileBundle::new(
                        tile_size,
                        Vec3::new(tile_x, tile_y, 1.0),
                        pos,
                    ));
                }
            }
            // draw borders
            let v_border_length = tile_height * 0.8;
            let h_border_length = tile_width * 0.8;
            for i in 0..3 {
                for j in 0..2 {
                    // vertical
                    let v_border_x =
                        tile_width * (j + 1) as f32 + BORDER_WIDTH * j as f32 + BORDER_WIDTH / 2.0
                            - board_size.x / 2.0;
                    let v_border_y =
                        tile_height * i as f32 + BORDER_WIDTH * i as f32 + tile_height / 2.0
                            - board_size.y / 2.0;
                    builder.spawn(border_bundle(
                        Color::BLACK,
                        Vec2::new(BORDER_WIDTH, v_border_length),
                        v_border_x,
                        v_border_y,
                    ));
                    // horizontal
                    let h_border_x =
                        tile_width * i as f32 + BORDER_WIDTH * i as f32 + tile_width / 2.0
                            - board_size.x / 2.0;
                    let h_border_y =
                        tile_height * (j + 1) as f32 + BORDER_WIDTH * j as f32 + BORDER_WIDTH / 2.0
                            - board_size.y / 2.0;
                    builder.spawn(border_bundle(
                        Color::BLACK,
                        Vec2::new(h_border_length, BORDER_WIDTH),
                        h_border_x,
                        h_border_y,
                    ));
                }
            }
        });
    }
}

pub fn handle_input(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    tiles: Query<(&GlobalTransform, &Sprite, &Position, &Parent)>,
    mut button_evr: EventReader<MouseButtonInput>,
    mut pressed: EventWriter<TilePressed>,
) {
    let Ok(window) = window.get_single() else {
        println!("failed to get single window");
        return;
    };
    let Ok((camera, camera_transform)) = camera.get_single() else {
        println!("multiple cameras detected");
        return;
    };
    for event in button_evr.read() {
        if let ButtonState::Pressed = event.state {
            if let Some(world_position) = window
                .cursor_position()
                .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
                .map(|ray| ray.origin.truncate())
            {
                let tile = tiles.iter().find(|(gt, sprite, _, _)| {
                    let Some(size) = sprite.custom_size else {
                        return false;
                    };
                    let bounds = Rect::from_center_size(gt.translation().truncate(), size);
                    bounds.contains(world_position)
                });
                if let Some((_, _, pos, parent)) = tile {
                    println!("pressed: {:?}", pos);
                    pressed.send(TilePressed::new(parent.get(), *pos));
                }
            }
        }
    }
}

/// Receive [`TileFilled`] event and make content entity a child of a button entity.
pub fn set_tile_image(
    mut tiles: Query<(&mut Sprite, &mut Handle<Image>, &Parent, &Position)>,
    mut content_arrived: EventReader<TileFilled>,
) {
    for event in content_arrived.read() {
        if let Some((mut sprite, mut texture, _, _)) = tiles
            .iter_mut()
            .find(|(_, _, parent, &pos)| parent.get() == event.board() && pos == event.pos())
        {
            sprite.color = Color::default();
            *texture = event.image().clone();
        } else {
            println!("unable to get button with position: {:?}", event.pos());
            continue;
        }
    }
}
