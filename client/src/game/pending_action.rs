use std::fmt;

use game_server::core;

/// Confirmation status of a [`PendingAction`].
/// In case of a bot game pending actions are created as `Confirmed`.
/// In case of a network game pending actions are created as `NotConfirmed` and need to undergo
/// confirmation process by executing them on the server side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConfirmationStatus {
    NotConfirmed,
    WaitingConfirmation,
    Confirmed,
}

impl fmt::Display for ConfirmationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Player action that is waiting to be applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingAction<T> {
    player: core::PlayerPosition,
    action: T,
    status: ConfirmationStatus,
}

impl<T> PendingAction<T> {
    pub fn new(player: core::PlayerPosition, action: T, status: ConfirmationStatus) -> Self {
        Self {
            player,
            action,
            status,
        }
    }

    pub fn new_confirmed(player: core::PlayerPosition, action: T) -> Self {
        Self::new(player, action, ConfirmationStatus::Confirmed)
    }

    pub fn new_unconfirmed(player: core::PlayerPosition, action: T) -> Self {
        Self::new(player, action, ConfirmationStatus::NotConfirmed)
    }

    pub fn is_confirmed(&self) -> bool {
        self.status == ConfirmationStatus::Confirmed
    }

    pub fn is_not_confirmed(&self) -> bool {
        self.status == ConfirmationStatus::NotConfirmed
    }

    pub fn is_waiting_confirmation(&self) -> bool {
        self.status == ConfirmationStatus::WaitingConfirmation
    }

    pub fn action(&self) -> &T {
        &self.action
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn status(&self) -> ConfirmationStatus {
        self.status
    }

    pub fn set_status(&mut self, status: ConfirmationStatus) {
        self.status = status
    }
}
