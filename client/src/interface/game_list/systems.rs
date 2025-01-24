use bevy::prelude::*;
use game_server::core;

use super::GameList;
use crate::grpc::{Connected, Disconnected};
use crate::interface;

pub fn update(
    game_list: Query<(Entity, &GameList), Changed<GameList>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let text_font = interface::common::load_text_font(&asset_server);
    for (entity, list) in game_list.iter() {
        match list {
            GameList::Message(msg) => {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_child(interface::TextBundle::new(msg, text_font.clone()));
            }
            GameList::Games(games) if games.is_empty() => {
                commands.entity(entity).despawn_descendants().with_child(
                    interface::TextBundle::new("No games available", text_font.clone()),
                );
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
                                        error!("unable to show game: corrupted GameInfo");
                                        continue;
                                    };
                                    format!("Next: {}", user_id)
                                }
                                core::GameState::Finished(core::FinishedState::Win(id)) => {
                                    let Some(user_id) = game.get_user_id(id) else {
                                        error!("unable to show game: corrupted GameInfo");
                                        continue;
                                    };
                                    format!("Winner: {}", user_id)
                                }
                                core::GameState::Finished(core::FinishedState::Draw) => {
                                    "Draw".into()
                                }
                            };
                            builder
                                .spawn(interface::common::row_node())
                                .with_children(|builder| {
                                    let mut item_node = interface::common::menu_item_node();
                                    item_node.width = Val::Auto;
                                    for s in [
                                        &format!("ID: {} ", game.id),
                                        &format!("Players: {:?}", game.players),
                                        &state_text,
                                    ] {
                                        builder.spawn(item_node.clone()).with_child(
                                            interface::TextBundle::new(s, text_font.clone()),
                                        );
                                    }
                                    let mut flex_row = interface::common::flex_row();
                                    flex_row.flex_grow = 1.;
                                    flex_row.justify_content = JustifyContent::End;
                                    let join = interface::JoinGameButtonBundle::new(
                                        interface::common::menu_item_node(),
                                        game.clone(),
                                    );
                                    builder.spawn(flex_row).with_children(|builder| {
                                        builder.spawn(join).with_child(interface::TextBundle::new(
                                            "Join",
                                            text_font.clone(),
                                        ));
                                    });
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
