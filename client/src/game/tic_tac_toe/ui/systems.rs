use bevy::prelude::*;
use game_server::core;

use super::{
    GameStateBox, GameStateInfoBundle, NextPlayer, NextPlayerImageBundle, PlayerActionApplied,
    PlayerImageBundle, PlayerInfoBundle, ENEMY_COLOR, FONT_SIZE, FRIENDLY_COLOR,
};
use crate::commands::CommandsExt;
use crate::game::components::Winner;
use crate::game::tic_tac_toe::Images;
use crate::game::{
    BotAuthority, CurrentPlayer, CurrentUser, Draw, GameLink, PlayerPosition, PlayerWon, TurnStart,
    UserAuthority,
};
use crate::interface;

fn create_player_info_bundle(
    game: Entity,
    player_id: core::PlayerPosition,
    color: Color,
    is_current_player: bool,
) -> PlayerInfoBundle {
    if is_current_player {
        PlayerInfoBundle::new_active(game, player_id, color)
    } else {
        PlayerInfoBundle::new_inactive(game, player_id, color)
    }
}

/// Create in-game ui after [`Playground`] component had been added to a game entity.
pub fn create(
    mut commands: Commands,
    playground: Query<(Entity, &GameLink), Added<interface::Playground>>,
    player: Query<(
        &Parent,
        &PlayerPosition,
        Option<&UserAuthority>,
        Option<&BotAuthority>,
        Option<&CurrentUser>,
        Option<&CurrentPlayer>,
        Option<&Winner>,
    )>,
    images: Res<Images>,
    asset_server: Res<AssetServer>,
) {
    if playground.is_empty() {
        return;
    }
    let text_font = TextFont {
        font: asset_server.load(interface::common::FONT_PATH),
        font_size: FONT_SIZE,
        ..default()
    };
    for (playground_entity, game_link) in playground.iter() {
        let mut player_iter = player
            .iter()
            .filter(|(parent, ..)| parent.get() == game_link.get());
        let (user, enemy) = match (player_iter.next(), player_iter.next(), player_iter.next()) {
            (Some(p1), Some(p2), None) if p2.4.is_some() => (p2, p1),
            (Some(p1), Some(p2), None) => (p1, p2),
            _ => {
                error!("invalid number of players found for a game");
                continue;
            }
        };
        let user_color = if user.4.is_some() {
            FRIENDLY_COLOR
        } else {
            ENEMY_COLOR
        };
        let enemy_color = if enemy.4.is_some() {
            FRIENDLY_COLOR
        } else {
            ENEMY_COLOR
        };
        let player1_image = images.get(**user.1).cloned().unwrap_or_default();
        let player2_image = images.get(**enemy.1).cloned().unwrap_or_default();
        let player1_info =
            create_player_info_bundle(game_link.get(), **user.1, user_color, user.5.is_some());
        let player2_info =
            create_player_info_bundle(game_link.get(), **enemy.1, enemy_color, enemy.5.is_some());
        commands.entity(playground_entity).with_children(|builder| {
            let mut row_node = interface::common::row_node();
            row_node.margin = UiRect::bottom(Val::Auto);
            builder.spawn(row_node).with_children(|builder| {
                builder.spawn(player1_info).with_children(|builder| {
                    let text = match (user.2, user.3) {
                        (Some(v), None) => format!("{:?}", v),
                        (None, Some(v)) => format!("{:?}", v),
                        _ => "-".into(),
                    };
                    builder.spawn(PlayerImageBundle::new(player1_image.clone()));
                    builder.spawn(interface::TextBundle::new(text, text_font.clone()));
                });
                builder
                    .spawn(GameStateInfoBundle::new(game_link.get()))
                    .with_children(|builder| {
                        let text = if user.5.is_some() || enemy.5.is_some() {
                            "Next:"
                        } else if user.6.is_some() || enemy.6.is_some() {
                            "Winner:"
                        } else {
                            "Draw"
                        };
                        builder.spawn(interface::TextBundle::new(text, text_font.clone()));
                        if user.5.is_some() {
                            builder
                                .spawn(NextPlayerImageBundle::new(game_link.get(), player1_image));
                        } else if enemy.5.is_some() {
                            builder.spawn(NextPlayerImageBundle::new(
                                game_link.get(),
                                player2_image.clone(),
                            ));
                        } else if user.6.is_some() {
                            builder.spawn(PlayerImageBundle::new(player1_image));
                        } else if enemy.6.is_some() {
                            builder.spawn(PlayerImageBundle::new(player2_image.clone()));
                        }
                    });
                builder.spawn(player2_info).with_children(|builder| {
                    let text = match (enemy.2, enemy.3) {
                        (Some(v), None) => format!("{:?}", v),
                        (None, Some(v)) => format!("{:?}", v),
                        _ => "-".into(),
                    };
                    builder.spawn(interface::TextBundle::new(text, text_font.clone()));
                    builder.spawn(PlayerImageBundle::new(player2_image));
                });
            });
        });
    }
}

/// Receive [`TurnStart`] event and set current player info border color to [`PlayerColor`],
/// reset border color for others.
pub fn update_player_info_border(
    mut player_info: Query<(
        &mut BorderColor,
        &PlayerPosition,
        &interface::PlayerColor,
        &GameLink,
    )>,
    mut turn_start: EventReader<TurnStart>,
) {
    for event in turn_start.read() {
        for (mut border, &position, &color, _) in player_info
            .iter_mut()
            .filter(|(.., game)| game.get() == event.game())
        {
            if *position == event.player() {
                *border = (*color).into();
            } else {
                *border = Color::NONE.into();
            }
        }
    }
}

/// Receive [`TurnStart`] event and update next player image to an image (X/O) of a current player.
pub fn update_next_player(
    mut next_player: Query<(&mut ImageNode, &GameLink), With<NextPlayer>>,
    mut turn_start: EventReader<TurnStart>,
    images: Res<Images>,
) {
    for event in turn_start.read() {
        if let Some((mut next_player_image, _)) = next_player
            .iter_mut()
            .find(|(_, game)| game.get() == event.game())
        {
            *next_player_image = images
                .get(event.player())
                .cloned()
                .map(ImageNode::new)
                .unwrap_or_default();
        }
    }
}

/// Receive [`PlayerWon`] event and game state container to show the winner.
pub fn set_winner(
    mut commands: Commands,
    game_state_info: Query<(Entity, &GameLink), With<GameStateBox>>,
    mut player_won: EventReader<PlayerWon>,
    images: Res<Images>,
    asset_server: Res<AssetServer>,
) {
    for event in player_won.read() {
        let Some((game_state_entity, _)) = game_state_info
            .iter()
            .find(|(_, game)| game.get() == event.game())
        else {
            continue;
        };

        let text_font = TextFont {
            font: asset_server.load(interface::common::FONT_PATH),
            font_size: FONT_SIZE,
            ..default()
        };
        commands
            .entity(game_state_entity)
            .despawn_descendants()
            .with_children(|builder| {
                builder.spawn(interface::TextBundle::new("Winner:", text_font));
                let player_image = images
                    .get(event.player())
                    .cloned()
                    .map(PlayerImageBundle::new)
                    .unwrap_or_default();
                builder.spawn(player_image);
            });
    }
}

/// Receive [`Draw`] event and game state container to show the draw.
pub fn set_draw(
    mut commands: Commands,
    game_state_info: Query<(Entity, &GameLink), With<GameStateBox>>,
    mut draw: EventReader<Draw>,
    asset_server: Res<AssetServer>,
) {
    for event in draw.read() {
        let Some((game_state_entity, _)) = game_state_info
            .iter()
            .find(|(_, game)| game.get() == event.game())
        else {
            continue;
        };

        let text_font = TextFont {
            font: asset_server.load(interface::common::FONT_PATH),
            font_size: FONT_SIZE,
            ..default()
        };
        commands
            .entity(game_state_entity)
            .despawn_descendants()
            .with_child(interface::TextBundle::new("Draw", text_font));
    }
}

/// Receive [`PlayerActionApplied`] and play turn sound.
pub fn action_sound(
    mut commands: Commands,
    mut action_applied: EventReader<PlayerActionApplied>,
    asset_server: Res<AssetServer>,
) {
    for _ in action_applied.read() {
        commands.play_sound(&asset_server, interface::common::TURN_SOUND_PATH);
    }
}
