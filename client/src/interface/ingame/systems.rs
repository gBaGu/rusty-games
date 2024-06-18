use bevy::prelude::*;
use game_server::game::GameState;

use super::components::{
    EmptyNextPlayerImageBundle, GameStateContainer, GameStateContainerBundle, InGameUI, NextPlayer,
    PlayerImageBundle, PlayerInfo, PlayerInfoContainerBundle,
};
use super::{ENEMY_TURN_COLOR, FONT_SIZE, PLAYER_TURN_COLOR};
use crate::game::StateUpdated;
use crate::interface::common::{FONT_PATH, SECONDARY_COLOR};

pub fn create(
    mut commands: Commands,
    new_ingame_ui: Query<(Entity, &InGameUI), Added<InGameUI>>,
    asset_server: Res<AssetServer>,
) {
    if !new_ingame_ui.is_empty() {
        let text_style = TextStyle {
            font: asset_server.load(FONT_PATH),
            font_size: FONT_SIZE,
            color: SECONDARY_COLOR,
        };
        for (entity, data) in new_ingame_ui.iter() {
            commands.entity(entity).with_children(|builder| {
                builder
                    .spawn(PlayerInfoContainerBundle::new(
                        data.player_id,
                        PLAYER_TURN_COLOR,
                        data.player_image.clone(),
                    ))
                    .with_children(|builder| {
                        builder.spawn(PlayerImageBundle::new(data.player_image.clone()));
                        builder.spawn(TextBundle::from_section(
                            format!("{:?}", data.player),
                            text_style.clone(),
                        ));
                    });
                builder
                    .spawn(GameStateContainerBundle::default())
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section("Next:", text_style.clone()));
                        builder.spawn(EmptyNextPlayerImageBundle::default());
                    });
                builder
                    .spawn(PlayerInfoContainerBundle::new(
                        data.enemy_id,
                        ENEMY_TURN_COLOR,
                        data.enemy_image.clone(),
                    ))
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            format!("{:?}", data.enemy),
                            text_style.clone(),
                        ));
                        builder.spawn(PlayerImageBundle::new(data.enemy_image.clone()));
                    });
            });
        }
    }
}

pub fn handle_state_update(
    mut commands: Commands,
    mut player_info: Query<(&mut BorderColor, &PlayerInfo)>,
    mut next_player: Query<&mut UiImage, With<NextPlayer>>,
    state_container: Query<Entity, With<GameStateContainer>>,
    mut state_updated: EventReader<StateUpdated>,
) {
    for event in state_updated.read() {
        if let GameState::Turn(id) = event.0 {
            for (mut border, info) in player_info.iter_mut() {
                if info.id == id {
                    *border = info.color.into();
                    if let Ok(mut next_player_image) = next_player.get_single_mut() {
                        *next_player_image = UiImage::new(info.image.clone());
                    }
                } else {
                    *border = Color::NONE.into();
                }
            }
        } else {
            let Ok(state_container) = state_container.get_single() else {
                continue;
            };
            commands.entity(state_container).despawn_descendants();
        }
    }
}
