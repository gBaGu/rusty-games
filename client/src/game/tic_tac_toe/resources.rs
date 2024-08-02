use bevy::prelude::*;
use game_server::game::PlayerId;

#[derive(Resource)]
pub struct Images {
    x_img: Handle<Image>,
    o_img: Handle<Image>,
}

impl Images {
    pub fn new(x_img: Handle<Image>, o_img: Handle<Image>) -> Self {
        Self {
            x_img,
            o_img,
        }
    }

    pub fn get(&self, player: PlayerId) -> Option<&Handle<Image>> {
        if player == 0 {
            Some(&self.x_img)
        } else if player == 1 {
            Some(&self.o_img)
        } else {
            None
        }
    }
}
