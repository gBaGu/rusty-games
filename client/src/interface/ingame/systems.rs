use crate::interface::ingame::events::PlayerInfoReady;
use crate::interface::plugin::GameStateUpdated;
use bevy::prelude::*;
use game_server::game::game::GameState;

use super::components::{
    EmptyNextPlayerImageBundle, GameStateContainer, GameStateContainerBundle, InGameUI, NextPlayer,
    PlayerImage, PlayerImageBundle, PlayerInfo, PlayerInfoContainerBundle,
};

use super::{ENEMY_TURN_COLOR, FONT_PATH, FONT_SIZE, PLAYER_TURN_COLOR, TEXT_COLOR};

pub fn create(
    mut commands: Commands,
    new_ingame_ui: Query<(Entity, &InGameUI), Added<InGameUI>>,
    asset_server: Res<AssetServer>,
) {
    if !new_ingame_ui.is_empty() {
        let text_style = TextStyle {
            font: asset_server.load(FONT_PATH),
            font_size: FONT_SIZE,
            color: TEXT_COLOR,
        };
        for (entity, data) in new_ingame_ui.iter() {
            commands.entity(entity).with_children(|builder| {
                builder
                    .spawn(PlayerInfoContainerBundle::new(
                        data.player_id,
                        PLAYER_TURN_COLOR,
                    ))
                    .with_children(|builder| {
                        builder.spawn(PlayerImageBundle::default());
                        builder.spawn(TextBundle::from_section(
                            data.player_id.to_string(),
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
                    ))
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            data.enemy_id.to_string(),
                            text_style.clone(),
                        ));
                        builder.spawn(PlayerImageBundle::default());
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
    mut state_updated: EventReader<GameStateUpdated>,
) {
    for event in state_updated.read() {
        if let GameState::Turn(id) = event.0 {
            for (mut border, info) in player_info.iter_mut() {
                if info.id == id {
                    *border = info.color.into();
                    if let (Ok(mut next_player_image), Some(image)) =
                        (next_player.get_single_mut(), info.image.as_ref())
                    {
                        *next_player_image = UiImage::new(image.clone());
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

pub fn update_player_info(
    mut player_info: Query<(Entity, &mut PlayerInfo)>,
    mut player_image: Query<(&mut UiImage, &Parent), With<PlayerImage>>,
    mut info_ready: EventReader<PlayerInfoReady>,
) {
    for event in info_ready.read() {
        if let Some((entity, mut info)) = player_info.iter_mut().find(|(_, i)| i.id == event.id) {
            info.image = Some(event.image.clone());
            if let Some((mut img, _)) = player_image.iter_mut().find(|(_, p)| p.get() == entity) {
                *img = UiImage::new(event.image.clone());
            }
        }
    }
}
