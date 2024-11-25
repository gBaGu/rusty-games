use std::ops::Deref;

use bevy::prelude::*;
use game_server::{core, proto};

use super::{
    ActiveGame, CurrentPlayer, CurrentUser, Draw, NetworkGame, PlayerPosition, PlayerWon,
    StateUpdated, TurnStart, UserAuthority, Winner,
};
use crate::game::components::{FinishedGame, Game, PendingActionStatus};
use crate::grpc::RpcResultReady;
use crate::interface::{GameLeft, GameReady, GameReadyToExit};
use crate::UserIdChanged;

/// Watch local game entity creation and send [`GameReady`] event.
pub fn handle_local_game_creation(
    game: Query<Entity, (Added<Game>, Without<NetworkGame>)>,
    mut game_ready: EventWriter<GameReady>,
) {
    for game_entity in game.iter() {
        game_ready.send(GameReady::new(game_entity));
    }
}

/// Receive reply for MakeTurn rpc and if it's successful update [`PendingActionStatus`] component.
/// Does not depend on specific game type.
pub fn confirm_pending_action(
    mut game: Query<&mut PendingActionStatus, (With<NetworkGame>, With<ActiveGame>)>,
    mut make_turn_reply: EventReader<RpcResultReady<proto::MakeTurnReply>>,
) {
    let Ok(mut status) = game.get_single_mut() else {
        return;
    };
    for event in make_turn_reply.read() {
        if status.is_confirmed() {
            return;
        }
        *status = PendingActionStatus::NotConfirmed;
        let _ = match event.deref() {
            Ok(response) => {
                if let Some(new_state) = &response.get_ref().game_state {
                    match core::GameState::try_from(new_state.clone()) {
                        Ok(state) => state,
                        Err(err) => {
                            println!("failed to convert game state: {}", err);
                            continue;
                        }
                    }
                } else {
                    println!("MakeTurn returned empty response");
                    continue;
                }
            }
            Err(err) => {
                println!("MakeTurn request failed: {}", err);
                continue;
            }
        };
        *status = PendingActionStatus::Confirmed;
    }
}

/// Receive [`StateUpdated`] event and send [`TurnStart`], [`PlayerWon`] or [`Draw`]
/// depending on a new state.
pub fn handle_state_updated(
    mut state_updated: EventReader<StateUpdated>,
    mut turn_start: EventWriter<TurnStart>,
    mut player_won: EventWriter<PlayerWon>,
    mut draw: EventWriter<Draw>,
) {
    for event in state_updated.read() {
        match event.state() {
            core::GameState::Turn(next_player) => {
                println!("turn start: {:?}", event.game());
                turn_start.send(TurnStart::new(event.game(), next_player));
            }
            core::GameState::Finished(core::FinishedState::Win(winner)) => {
                println!("win: {:?}", event.game());
                player_won.send(PlayerWon::new(event.game(), winner));
            }
            core::GameState::Finished(core::FinishedState::Draw) => {
                println!("draw: {:?}", event.game());
                draw.send(Draw::new(event.game()));
            }
        }
    }
}

/// Receives [`TurnStart`] event and updates [`CurrentPlayer`] accordingly.
pub fn update_current_player(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut turn_start: EventReader<TurnStart>,
) {
    for event in turn_start.read() {
        player
            .iter()
            .filter(|(.., p)| event.game() == p.get())
            .for_each(|(player_entity, &position, _)| {
                let mut player_cmds = commands.entity(player_entity);
                if *position == event.player() {
                    println!("set current player: {}", event.player());
                    player_cmds.insert(CurrentPlayer);
                } else {
                    player_cmds.remove::<CurrentPlayer>();
                }
            });
    }
}

/// Receives [`Draw`] event, clears [`CurrentPlayer`] tag for players
/// and sends [`GameReadyToExit`] event because currently
/// there is nothing to do after the game is ended with a draw.
pub fn handle_draw(
    mut commands: Commands,
    player: Query<(Entity, &Parent), With<CurrentPlayer>>,
    mut draw: EventReader<Draw>,
    mut ready_to_exit: EventWriter<GameReadyToExit>,
) {
    for event in draw.read() {
        for (player_entity, _) in player.iter().filter(|(.., p)| p.get() == event.game()) {
            let mut player_cmds = commands.entity(player_entity);
            player_cmds.remove::<CurrentPlayer>();
        }
        ready_to_exit.send(GameReadyToExit::new(event.game()));
    }
}

/// Receives [`PlayerWon`] event, clears [`CurrentPlayer`] tag for players
/// and inserts [`Winner`] tag into the entity of a player that won the game.
pub fn handle_win(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut player_won: EventReader<PlayerWon>,
) {
    for event in player_won.read() {
        player
            .iter()
            .filter(|(.., p)| p.get() == event.game())
            .for_each(|(player, &pos, _)| {
                let mut player_cmds = commands.entity(player);
                player_cmds.remove::<CurrentPlayer>();
                if event.player() == *pos {
                    player_cmds.insert(Winner);
                }
            })
    }
}

/// Listen for [`Draw`] and [`PlayerWon`] events and insert [`FinishedGame`] component
/// into game entity.
pub fn set_game_finished(
    mut commands: Commands,
    mut draw: EventReader<Draw>,
    mut player_won: EventReader<PlayerWon>,
) {
    for event in draw.read() {
        commands.entity(event.game()).insert(FinishedGame);
    }
    for event in player_won.read() {
        commands.entity(event.game()).insert(FinishedGame);
    }
}

/// Whenever user id is changed in settings insert/remove [`CurrentUser`] for every player
/// according to his id.
pub fn update_current_user(
    mut commands: Commands,
    player: Query<(Entity, &UserAuthority)>,
    mut user_id_changed: EventReader<UserIdChanged>,
) {
    if let Some(event) = user_id_changed.read().last() {
        for (player_entity, &user_authority) in player.iter() {
            let mut player_cmds = commands.entity(player_entity);
            if *user_authority == event.new_user_id() {
                player_cmds.insert(CurrentUser);
            } else {
                player_cmds.remove::<CurrentUser>();
            }
        }
    }
}

/// Listen for [`UserIdChanged`] event and if none of players ids matches new user id
/// then despawn game entity and its descendants.
pub fn clear_foreign_network_games(
    mut commands: Commands,
    game: Query<Entity, With<NetworkGame>>,
    player: Query<(&UserAuthority, &Parent)>,
    mut user_id_changed: EventReader<UserIdChanged>,
) {
    if let Some(event) = user_id_changed.read().last() {
        for game_entity in game.iter() {
            if player
                .iter()
                .filter(|(_, p)| p.get() == game_entity)
                .all(|(&user, _)| *user != event.new_user_id())
            {
                commands.entity(game_entity).despawn_recursive();
            }
        }
    }
}

/// Listen to [`GameLeft`] event and despawn game entity and its descendants
/// if one of the next conditions is met for a game:
/// - it is local (bot game);
/// - it is a network game and is finished.
pub fn clear_game_on_exit(
    mut commands: Commands,
    game: Query<(Option<&FinishedGame>, Option<&NetworkGame>), With<Game>>,
    mut game_left: EventReader<GameLeft>,
) {
    for event in game_left.read() {
        let Ok((finished_game, network_game)) = game.get(event.get()) else {
            continue;
        };
        let is_finished_game = finished_game.is_some();
        let is_network_game = network_game.is_some();
        if !is_network_game || (is_network_game && is_finished_game) {
            commands.entity(event.get()).despawn_recursive();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::components::{
        CurrentUserPlayerBundle, LocalGameBundle, NetworkGameBundle, NetworkPlayerBundle,
    };
    use crate::Settings;
    use game_server::core::tic_tac_toe::TicTacToe;

    type TTTLocalGameBundle = LocalGameBundle<TicTacToe>;
    type TTTNetworkGameBundle = NetworkGameBundle<TicTacToe>;

    #[test]
    fn game_entity_updated_on_finish() {
        let mut app = App::new();
        app.add_event::<Draw>();
        app.add_event::<PlayerWon>();
        app.add_systems(Update, set_game_finished);

        let game1 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game2 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();

        // neither of games has FinishedGame component
        assert!(!app.world().entity(game1).contains::<FinishedGame>());
        assert!(!app.world().entity(game2).contains::<FinishedGame>());

        // first game is finished with draw
        app.world_mut()
            .resource_mut::<Events<Draw>>()
            .send(Draw::new(game1));
        app.update();

        // only first game has FinishedGame component
        assert!(app.world().entity(game1).contains::<FinishedGame>());
        assert!(!app.world().entity(game2).contains::<FinishedGame>());

        // second game is finished with win
        app.world_mut()
            .resource_mut::<Events<PlayerWon>>()
            .send(PlayerWon::new(game2, 0));
        app.update();

        // both games now have FinishedGame component
        assert!(app.world().entity(game1).contains::<FinishedGame>());
        assert!(app.world().entity(game2).contains::<FinishedGame>());
    }

    #[test]
    fn user_id_change_clears_foreign_games() {
        let mut app = App::new();
        app.insert_resource(Settings::builder().user_id(1).build());
        app.add_event::<UserIdChanged>();
        app.add_systems(Update, clear_foreign_network_games);

        let make_spawner = |my_id, enemy_id| -> _ {
            move |builder: &mut WorldChildBuilder| {
                builder.spawn(CurrentUserPlayerBundle::new(my_id, 0));
                builder.spawn(NetworkPlayerBundle::new(enemy_id, 1));
            }
        };

        let game1_user1 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(1, TicTacToe::default()))
            .with_children(make_spawner(1, 2))
            .id();
        let game2_user1 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(2, TicTacToe::default()))
            .with_children(make_spawner(1, 3))
            .id();
        let game3_user2 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(3, TicTacToe::default()))
            .with_children(make_spawner(2, 4))
            .id();

        // simulate user id change
        app.world_mut()
            .resource_mut::<Events<UserIdChanged>>()
            .send(UserIdChanged::new(1));
        app.update();

        // game that doesn't have user 1 is despawned
        assert!(app.world().get::<Game>(game3_user2).is_none());

        // change user id
        app.world_mut().resource_mut::<Settings>().set_user_id(4);
        app.world_mut()
            .resource_mut::<Events<UserIdChanged>>()
            .send(UserIdChanged::new(4));
        app.update();

        // all games are cleared
        assert!(app.world().get::<Game>(game1_user1).is_none());
        assert!(app.world().get::<Game>(game2_user1).is_none());
        assert!(app.world().get::<Game>(game3_user2).is_none());
    }

    // GameLeft clears every local game and every finished network game
    #[test]
    fn game_left_event_clears_games() {
        let mut app = App::new();
        app.add_event::<GameLeft>();
        app.add_systems(Update, clear_game_on_exit);

        let game1 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game2 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game3 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(1, TicTacToe::default()))
            .insert(FinishedGame)
            .id();
        let game4 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(2, TicTacToe::default()))
            .id();

        // emit GameLeft events for games 1, 3, 4
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game1));
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game3));
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game4));
        app.update();

        // games 1, 3 are despawned, 2 and 4 remain in the world
        // 2 because no event was fired for it
        // 4 because it is not finished
        assert!(app.world().get::<Game>(game1).is_none());
        assert!(app.world().get::<Game>(game2).is_some());
        assert!(app.world().get::<Game>(game3).is_none());
        assert!(app.world().get::<Game>(game4).is_some());
    }
}
