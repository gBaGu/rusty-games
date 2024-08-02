use std::time::Duration;

use bevy::input::{mouse::MouseButtonInput, ButtonState};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_server::game::grid::GridIndex;
use game_server::game::tic_tac_toe::winning_combinations;
use game_server::game::{BoardCell, Game, GameState};

use super::{
    calculate_tile_center, calculate_tile_size, LocalGame, PlayerActionApplied, TileBundle,
    TilePressed, WinAnimation, WinAnimationBundle, BORDER_WIDTH, WIN_ANIMATION_PATH,
    WIN_ANIMATION_SPRITE_COUNT, WIN_ANIMATION_SPRITE_SIZE, WIN_ANIMATION_TRANSITION_INTERVAL,
};
use crate::commands::EntityCommandsExt;
use crate::game::components::{Board, BoardBundle};
use crate::game::tic_tac_toe::board::components::{BorderBundle, Tile};
use crate::game::tic_tac_toe::resources::Images;
use crate::game::tic_tac_toe::PlayerActionInitialized;
use crate::game::{ActiveGame, CurrentPlayer, CurrentUser, GameLink, PlayerPosition, PlayerWon};
use crate::interface::{GameReadyToExit, Playground};

pub fn create(
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    playground: Query<(&GameLink, &Node, &GlobalTransform), Added<Playground>>,
    game: Query<&LocalGame>,
    images: Res<Images>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };
    for (game_link, node, transform) in playground.iter() {
        let Ok(game) = game.get(game_link.get()) else {
            continue;
        };
        println!("create board for game: {:?}", game_link.get());
        let ui_node_center =
            transform.compute_transform().translation.truncate() + (node.size() / 2.0);
        let window_center = Vec2::new(window.width(), window.height()) / 2.0;
        let board_size = Vec2::splat(node.size().min_element() * 0.7);
        let board_translation = ui_node_center - window_center;
        let tile_size = calculate_tile_size(board_size);
        let v_border_length = tile_size.y * 0.8;
        let h_border_length = tile_size.x * 0.8;
        commands
            .spawn(BoardBundle::new(
                game_link.get(),
                board_size,
                board_translation.extend(0.0),
            ))
            .with_children(|builder| {
                for row in 0..3 {
                    for col in 0..3 {
                        let pos = GridIndex::new(row, col);
                        let tile_translation =
                            calculate_tile_center(board_size, tile_size, pos).extend(1.0);
                        let tile = match game.board()[(row, col).into()] {
                            BoardCell(Some(player)) => {
                                if let Some(img) = images.get(player) {
                                    TileBundle::new(tile_size, tile_translation, pos, img.clone())
                                } else {
                                    println!("unable to get image for {}", player);
                                    TileBundle::new_empty(tile_size, tile_translation, pos)
                                }
                            }
                            BoardCell(None) => {
                                TileBundle::new_empty(tile_size, tile_translation, pos)
                            }
                        };
                        builder.spawn(tile);
                    }
                }
                // draw borders
                for i in 0..3 {
                    for j in 0..2 {
                        // vertical
                        let v_border_x = tile_size.x * (j + 1) as f32
                            + BORDER_WIDTH * j as f32
                            + BORDER_WIDTH / 2.0
                            - board_size.x / 2.0;
                        let v_border_y =
                            tile_size.y * i as f32 + BORDER_WIDTH * i as f32 + tile_size.y / 2.0
                                - board_size.y / 2.0;
                        builder.spawn(BorderBundle::new(
                            Color::BLACK,
                            Vec2::new(BORDER_WIDTH, v_border_length),
                            Vec3::new(v_border_x, v_border_y, 1.0),
                        ));
                        // horizontal
                        let h_border_x =
                            tile_size.x * i as f32 + BORDER_WIDTH * i as f32 + tile_size.x / 2.0
                                - board_size.x / 2.0;
                        let h_border_y = tile_size.y * (j + 1) as f32
                            + BORDER_WIDTH * j as f32
                            + BORDER_WIDTH / 2.0
                            - board_size.y / 2.0;
                        builder.spawn(BorderBundle::new(
                            Color::BLACK,
                            Vec2::new(h_border_length, BORDER_WIDTH),
                            Vec3::new(h_border_x, h_border_y, 1.0),
                        ));
                    }
                }
            });
    }
}

pub fn handle_input(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    tiles: Query<(&GlobalTransform, &Sprite, &Tile, &Parent)>,
    mut button_evr: EventReader<MouseButtonInput>,
    mut pressed: EventWriter<TilePressed>,
) {
    let Ok(window) = window.get_single() else {
        println!("failed to get single window");
        return;
    };
    let Ok((camera, camera_transform)) = camera.get_single() else {
        println!("failed to get single camera");
        return;
    };
    for event in button_evr.read() {
        if let ButtonState::Pressed = event.state {
            let cursor_position = window.cursor_position();
            if let Some(world_position) = cursor_position
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
                if let Some((_, _, &tile, parent)) = tile {
                    println!("tile pressed: {:?}", tile);
                    pressed.send(TilePressed::new(parent.get(), tile.into()));
                }
            }
        }
    }
}

/// Receive Tile pressed event, find game, check if action is legit and
/// send PlayerActionInitialized event
pub fn initialize_action(
    game: Query<&LocalGame>,
    board: Query<&GameLink, With<Board>>,
    player: Query<(&PlayerPosition, &Parent), (With<CurrentPlayer>, With<CurrentUser>)>,
    mut tile_pressed: EventReader<TilePressed>,
    mut action_initialized: EventWriter<PlayerActionInitialized>,
) {
    for event in tile_pressed.read() {
        let Ok(game_link) = board.get(event.board()) else {
            continue;
        };
        let Ok(game) = game.get(game_link.get()) else {
            continue;
        };
        if matches!(game.state(), GameState::Finished(_))
            || game.board()[event.pos().into()].is_some()
        {
            // TODO: send ui invalid action event
            continue;
        }
        if let Some((&player, _)) = player.iter().find(|(_, p)| p.get() == game_link.get()) {
            println!("action initialized by a tile press: {}", event.pos());
            action_initialized.send(PlayerActionInitialized::new(
                game_link.get(),
                *player,
                event.pos(),
            ));
        }
    }
}

pub fn set_tile_image(
    mut tile: Query<(&mut Sprite, &mut Handle<Image>, &Tile, &Parent)>,
    board: Query<(Entity, &GameLink), With<Board>>,
    mut action_applied: EventReader<PlayerActionApplied>,
    images: Res<Images>,
) {
    for event in action_applied.read() {
        let Some((board_entity, _)) = board.iter().find(|(_, g)| g.get() == event.game()) else {
            continue;
        };
        let Some((mut sprite, mut texture, ..)) = tile
            .iter_mut()
            .find(|(.., &tile, parent)| parent.get() == board_entity && *tile == event.action())
        else {
            continue;
        };
        if let Some(img) = images.get(event.player()) {
            sprite.color = Color::default();
            *texture = img.clone();
        }
    }
}

pub fn create_win_animation(
    mut commands: Commands,
    game: Query<&LocalGame, With<ActiveGame>>,
    board: Query<(Entity, &Sprite, &GameLink), With<Board>>,
    mut player_won: EventReader<PlayerWon>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
    for event in player_won.read() {
        let Ok(game) = game.get(event.game()) else {
            continue;
        };
        let Some((board_entity, sprite, _)) = board.iter().find(|(.., g)| g.get() == event.game())
        else {
            continue;
        };
        let Some(board_size) = sprite.custom_size else {
            println!("unable to get board size from sprite");
            continue;
        };
        let Some((index1, _, index3)) =
            winning_combinations().into_iter().find(|(id1, id2, id3)| {
                let cell1 = game.board()[*id1];
                let cell2 = game.board()[*id2];
                let cell3 = game.board()[*id3];
                if cell1 == cell2 && cell2 == cell3 {
                    return cell1.is_some();
                }
                false
            })
        else {
            continue;
        };

        println!("create win animation from {} to {}", index1, index3);
        let texture = asset_server.load(WIN_ANIMATION_PATH);
        let layout = TextureAtlasLayout::from_grid(
            WIN_ANIMATION_SPRITE_SIZE,
            WIN_ANIMATION_SPRITE_COUNT,
            1,
            None,
            None,
        );
        let texture_atlas_layout = texture_atlas_layouts.add(layout);
        let tile_size = calculate_tile_size(board_size);
        let from_center = calculate_tile_center(board_size, tile_size, index1);
        let to_center = calculate_tile_center(board_size, tile_size, index3);
        let center = (from_center + to_center) / 2.0;
        let mut transform = Transform::from_translation(center.extend(1.));
        let line_vector = (from_center - center).normalize();
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, line_vector.extend(0.));
        let target_length = from_center.distance(to_center) + tile_size.x.min(tile_size.y);
        transform.scale = Vec2::splat(target_length / WIN_ANIMATION_SPRITE_SIZE.y).extend(1.);
        commands
            .entity(board_entity)
            .with_child(WinAnimationBundle::new(
                WIN_ANIMATION_SPRITE_COUNT - 1,
                Duration::from_millis(WIN_ANIMATION_TRANSITION_INTERVAL),
                texture,
                texture_atlas_layout,
                transform,
            ));
    }
}

pub fn update_win_animation(
    mut commands: Commands,
    board: Query<&GameLink, With<Board>>,
    mut animation: Query<(Entity, &mut WinAnimation, &mut TextureAtlas, &Parent)>,
    mut ready_to_exit: EventWriter<GameReadyToExit>,
    time: Res<Time>,
) {
    for (animation_entity, mut animation, mut atlas, parent) in animation.iter_mut() {
        if animation.tick(time.delta()).just_finished() {
            if atlas.index < animation.last_sprite_index() {
                atlas.index += 1;
                continue;
            }
            commands
                .entity(parent.get())
                .remove_children(&[animation_entity]);
            commands.entity(animation_entity).despawn();
            if let Ok(game_link) = board.get(parent.get()) {
                println!("game is ready to exit: {:?}", game_link.get());
                ready_to_exit.send(GameReadyToExit::new(game_link.get()));
            }
        }
    }
}
