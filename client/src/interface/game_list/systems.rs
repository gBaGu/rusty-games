use bevy::prelude::*;

use super::components::GameList;
use crate::commands::EntityCommandsExt;
use crate::interface::common::{menu_item_style, menu_text_style, row_node_bundle};
use crate::interface::components::JoinGameButtonBundle;

pub fn update(
    game_list: Query<(Entity, &GameList), Changed<GameList>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let style = menu_item_style();
    let text_style = menu_text_style(&asset_server);
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
                commands.entity(entity).with_children(|builder| {
                    for game in games {
                        builder.spawn(row_node_bundle()).with_children(|builder| {
                            for s in [
                                &format!("ID: {}", game.id),
                                &format!("{:?}", game.state),
                                &format!("{:?}", game.players),
                            ] {
                                builder.spawn(TextBundle::from_section(s, text_style.clone()));
                            }
                            builder
                                .spawn(JoinGameButtonBundle::new(style.clone(), game.clone()))
                                .with_child(TextBundle::from_section("Join", text_style.clone()));
                        });
                    }
                });
            }
        }
    }
}
