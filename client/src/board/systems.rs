use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_server::game::tic_tac_toe::winning_combinations;
use game_server::game::FinishedState;

use super::components::{Board, OneTimeAnimation, TileBundle, WinAnimation};
use super::events::{TileFilled, TilePressed};
use super::{calculate_tile_center, calculate_tile_size, create_win_animation, BORDER_WIDTH};
use crate::game::{CurrentGame, GameExit, GameOver, Position};

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
        let tile_size = calculate_tile_size(board_size);
        commands.entity(entity).with_children(|builder| {
            for row in 0..3 {
                for col in 0..3 {
                    let tile_center = calculate_tile_center(board_size, tile_size, (row, col));
                    let pos = Position::new(row, col);
                    builder.spawn(TileBundle::new(tile_size, tile_center.extend(1.0), pos));
                }
            }
            // draw borders
            let v_border_length = tile_size.y * 0.8;
            let h_border_length = tile_size.x * 0.8;
            for i in 0..3 {
                for j in 0..2 {
                    // vertical
                    let v_border_x =
                        tile_size.x * (j + 1) as f32 + BORDER_WIDTH * j as f32 + BORDER_WIDTH / 2.0
                            - board_size.x / 2.0;
                    let v_border_y =
                        tile_size.y * i as f32 + BORDER_WIDTH * i as f32 + tile_size.y / 2.0
                            - board_size.y / 2.0;
                    builder.spawn(border_bundle(
                        Color::BLACK,
                        Vec2::new(BORDER_WIDTH, v_border_length),
                        v_border_x,
                        v_border_y,
                    ));
                    // horizontal
                    let h_border_x =
                        tile_size.x * i as f32 + BORDER_WIDTH * i as f32 + tile_size.x / 2.0
                            - board_size.x / 2.0;
                    let h_border_y =
                        tile_size.y * (j + 1) as f32 + BORDER_WIDTH * j as f32 + BORDER_WIDTH / 2.0
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

pub fn handle_game_over(
    mut commands: Commands,
    board: Query<&Sprite, With<Board>>,
    mut game_over: EventReader<GameOver>,
    mut game_exit: EventWriter<GameExit>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
) {
    if let Some(event) = game_over.read().last() {
        if matches!(event, GameOver(FinishedState::Win(_))) {
            let Ok(Some(board_size)) = board.get_single().map(|sprite| sprite.custom_size) else {
                return;
            };
            let Some(board) = game.board_entity() else {
                return;
            };
            let Some((index1, _, index3)) =
                winning_combinations().into_iter().find(|(id1, id2, id3)| {
                    let cell1 = game.get_cell((id1.row().into(), id1.col().into()));
                    let cell2 = game.get_cell((id2.row().into(), id2.col().into()));
                    let cell3 = game.get_cell((id3.row().into(), id3.col().into()));
                    if cell1 == cell2 && cell2 == cell3 {
                        return cell1.is_some();
                    }
                    false
                })
            else {
                return;
            };
            let animation = commands
                .spawn(create_win_animation(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    board_size,
                    (
                        usize::from(index1.row()) as u32,
                        usize::from(index1.col()) as u32,
                    ),
                    (
                        usize::from(index3.row()) as u32,
                        usize::from(index3.col()) as u32,
                    ),
                ))
                .id();
            commands.entity(*board).add_child(animation);
        } else {
            game_exit.send(GameExit);
        }
    }
}

/// Receive [`TileFilled`] event and set tile sprite image.
pub fn set_tile_image(
    mut tiles: Query<(&mut Sprite, &mut Handle<Image>, &Parent, &Position)>,
    mut tile_filled: EventReader<TileFilled>,
) {
    for event in tile_filled.read() {
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

pub fn win_animation(
    mut commands: Commands,
    mut animation: Query<(Entity, &mut OneTimeAnimation, &mut TextureAtlas), With<WinAnimation>>,
    mut game_exit: EventWriter<GameExit>,
    game: Res<CurrentGame>,
    time: Res<Time>,
) {
    for (entity, mut animation, mut atlas) in animation.iter_mut() {
        if animation.tick(time.delta()).just_finished() {
            if atlas.index < animation.last_sprite_index() {
                atlas.index += 1;
            } else {
                if let Some(board) = game.board_entity() {
                    commands.entity(*board).remove_children(&[entity]);
                    commands.entity(entity).despawn();
                }
                game_exit.send(GameExit);
            }
        }
    }
}
