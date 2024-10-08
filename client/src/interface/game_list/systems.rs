use bevy::prelude::*;
use game_server::core;

use super::{GameList, HORIZONTAL_MARGIN};
use crate::commands::EntityCommandsExt;
use crate::grpc::{Connected, Disconnected};
use crate::interface::common;
use crate::interface::components::JoinGameButtonBundle;

pub fn update(
    game_list: Query<(Entity, &GameList), Changed<GameList>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let style = common::menu_item_style();
    let text_style = common::menu_text_style(&asset_server);
    for (entity, list) in game_list.iter() {
        match list {
            GameList::Message(msg) => {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_child(TextBundle::from_section(msg, text_style.clone()));
            }
            GameList::Games(games) if games.is_empty() => {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_child(TextBundle::from_section(
                        "No games available",
                        text_style.clone(),
                    ));
            }
            GameList::Games(games) => {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_children(|builder| {
                        for game in games {
                            let state_text = match game.state {
                                core::GameState::Turn(id) => {
                                    let Some(user_id) = game.get_user_id(id) else {
                                        println!("skipping corrupted GameInfo");
                                        continue;
                                    };
                                    format!("Next: {}", user_id)
                                }
                                core::GameState::Finished(core::FinishedState::Win(id)) => {
                                    let Some(user_id) = game.get_user_id(id) else {
                                        println!("skipping corrupted GameInfo");
                                        continue;
                                    };
                                    format!("Winner: {}", user_id)
                                }
                                core::GameState::Finished(core::FinishedState::Draw) => {
                                    "Draw".into()
                                }
                            };
                            builder
                                .spawn(common::row_node_bundle())
                                .with_children(|builder| {
                                    for s in [
                                        &format!("ID: {}", game.id),
                                        &state_text,
                                        &format!("Players: {:?}", game.players),
                                    ] {
                                        let mut text =
                                            TextBundle::from_section(s, text_style.clone());
                                        text.style.margin.left = Val::Px(HORIZONTAL_MARGIN);
                                        text.style.margin.right = Val::Px(HORIZONTAL_MARGIN);
                                        builder.spawn(text);
                                    }
                                    let mut join =
                                        JoinGameButtonBundle::new(style.clone(), game.clone());
                                    join.button.style.margin.left = Val::Px(HORIZONTAL_MARGIN);
                                    join.button.style.margin.right = Val::Px(HORIZONTAL_MARGIN);
                                    builder.spawn(join).with_child(TextBundle::from_section(
                                        "Join",
                                        text_style.clone(),
                                    ));
                                });
                        }
                    });
            }
        }
    }
}

pub fn on_connect(mut game_list: Query<&mut GameList>, mut connected: EventReader<Connected>) {
    if connected.read().next().is_some() {
        for mut list in game_list.iter_mut() {
            *list = GameList::Message("Loading".into());
        }
    }
}

pub fn on_disconnect(
    mut game_list: Query<&mut GameList>,
    mut disconnected: EventReader<Disconnected>,
) {
    if disconnected.read().next().is_some() {
        for mut list in game_list.iter_mut() {
            *list = GameList::Message("Server is down".into());
        }
    }
}
