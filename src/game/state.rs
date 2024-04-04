use crate::game::player_pool::PlayerId;

#[derive(Clone, Copy, Debug)]
pub enum FinishedState {
    Win(PlayerId),
    Draw,
}

#[derive(Clone, Copy, Debug)]
pub enum GameState {
    Turn(PlayerId),
    Finished(FinishedState),
}