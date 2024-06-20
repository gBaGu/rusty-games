use std::cmp::Ordering;
use std::fs::File;

use game_server::game::tic_tac_toe::{winning_combinations, FieldCol, FieldRow, TicTacToe};
use game_server::game::{BoardCell, Game, GameState, PlayerId};
use rand::distributions::Uniform;
use rand::Rng;

const STATE_SIZE: usize = 9;
const TOTAL_STATE_COUNT: usize = 3usize.pow(STATE_SIZE as u32);

const WIN_REWARD: f32 = 10.0;
const TWO_OUT_OF_THREE_REWARD: f32 = 3.0;
const ONE_OUT_OF_THREE_REWARD: f32 = 2.0;

const EXPLORATION_DECAY_RATE: f32 = 0.0001;
const LEARNING_RATE_DECAY_RATE: f32 = 0.001;

type Action = (usize, usize);
type ActionValues = [QValue; STATE_SIZE];
type QValue = f32;
type Reward = f32;

fn calculate_q(old: QValue, max_q_next: QValue, reward: Reward, lr: f32, gamma: f32) -> QValue {
    old + lr * (reward + gamma * max_q_next - old)
}

fn action_to_index(action: Action) -> usize {
    (action.0 * 3) + action.1
}

fn index_to_action(index: usize) -> Action {
    (index / 3, index % 3)
}

fn state_to_index(state: &<TicTacToe as Game>::Board) -> usize {
    let board_iter = state.iter().flatten();
    if board_iter.clone().all(|cell| cell.is_none()) {
        return 0;
    }
    let mut index: usize = 0;
    for (cell, exp) in board_iter.zip((0..STATE_SIZE as u32).rev()) {
        if let BoardCell(Some(player_id)) = cell {
            index += player_id.pow(exp) as usize;
        }
    }
    index
}

pub struct QTable(Vec<ActionValues>);

impl Default for QTable {
    fn default() -> Self {
        Self(vec![ActionValues::default(); TOTAL_STATE_COUNT])
    }
}

impl QTable {
    pub fn get_max_value(&self, state: &<TicTacToe as Game>::Board) -> QValue {
        let mut max_q = 0.0;
        for val in self.get_values(state) {
            if val > &max_q {
                max_q = *val;
            }
        }
        max_q
    }

    pub fn get_value(&self, state: &<TicTacToe as Game>::Board, action: Action) -> QValue {
        self.0[state_to_index(state)][action_to_index(action)]
    }

    pub fn get_values(&self, state: &<TicTacToe as Game>::Board) -> &[QValue] {
        self.0[state_to_index(state)].as_slice()
    }

    pub fn set_value(
        &mut self,
        state: &<TicTacToe as Game>::Board,
        action: Action,
        new_val: QValue,
    ) {
        // TODO: move state_to_index and action_to_index to Model level
        self.0[state_to_index(state)][action_to_index(action)] = new_val;
    }
}

pub struct Model {
    initial_learning_rate: f32,
    current_learning_rate: f32,
    exploration_level: f32,
    gamma: f32,
    episode: usize,
    q_table: QTable,
    env: TicTacToe,
}

impl Model {
    pub fn new(gamma: f32, lr: f32) -> Self {
        Self {
            initial_learning_rate: lr,
            current_learning_rate: lr,
            exploration_level: 1.0,
            gamma,
            episode: 0,
            q_table: Default::default(),
            env: Default::default(),
        }
    }

    pub fn run_episode(&mut self, verbose: bool) {
        let mut step: usize = 0;
        let mut total_reward = 0.0;
        while matches!(self.env.state(), GameState::Turn(_)) {
            let action = self.choose_epsilon_greedy_action();
            let prev_state = self.env.board().clone();
            let reward = self.step(action);
            self.update_q(&prev_state, action, reward);

            if verbose {
                println!("state after step {}: {}", step, self.env.board());
            }
            total_reward += reward;
            step += 1;
        }
        if verbose {
            println!(
                "episode {} summary:\nexploration level: {}\ntotal steps: {}\ntotal reward: {}",
                self.episode, self.exploration_level, step, total_reward
            );
        }
        self.episode += 1;
        if self.episode % 100 == 0 {
            self.decay();
            if verbose {
                println!(
                    "decay: exploration_level {}, learning_rate {}",
                    self.exploration_level, self.current_learning_rate
                );
            }
        }
        self.reset();
    }

    pub fn dump_table(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        todo!()
    }

    fn decay(&mut self) {
        self.exploration_level = (-EXPLORATION_DECAY_RATE * self.episode as f32).exp();
        self.current_learning_rate =
            self.initial_learning_rate / (1.0 + LEARNING_RATE_DECAY_RATE * self.episode as f32)
    }

    fn reset(&mut self) {
        self.env = Default::default();
    }

    fn step(&mut self, action: Action) -> Reward {
        if let GameState::Turn(id) = self.env.state() {
            self.perform_action(id, action);
            return self.calculate_reward(self.env.board(), id);
        }
        0.0
    }

    fn update_q(
        &mut self,
        prev_state: &<TicTacToe as Game>::Board,
        action: Action,
        reward: Reward,
    ) {
        let max_q_next = if let GameState::Turn(enemy) = self.env.state() {
            // simulate enemy action
            let action = self.choose_best_action(&self.get_valid_actions());
            let mut next_state = self.env.board().clone();
            next_state
                .get_mut_ref(<TicTacToe as Game>::TurnData::new(
                    FieldRow::try_from(action.0).unwrap(),
                    FieldCol::try_from(action.1).unwrap(),
                ))
                .0 = Some(enemy);
            self.q_table.get_max_value(self.env.board())
        } else {
            0.0
        };

        let q_prev = self.q_table.get_value(&prev_state, action);
        let new_q = calculate_q(
            q_prev,
            max_q_next,
            reward,
            self.current_learning_rate,
            self.gamma,
        );
        self.q_table.set_value(&prev_state, action, new_q);
    }

    // TODO: try SmallVec here
    fn get_valid_actions(&self) -> Vec<Action> {
        self.env
            .board()
            .iter()
            .enumerate()
            .map(|(i, row)| row.iter().enumerate().map(move |(j, cell)| (i, j, cell)))
            .flatten()
            .filter_map(|(row, col, cell)| {
                if cell.is_none() {
                    return Some((row, col));
                }
                None
            })
            .collect()
    }

    fn choose_epsilon_greedy_action(&self) -> Action {
        let valid_actions = self.get_valid_actions();
        let mut rng = rand::thread_rng();
        if rng.sample(Uniform::from(0.0..1.0)) < self.exploration_level {
            valid_actions[rng.sample(Uniform::from(0..valid_actions.len()))]
        } else {
            self.choose_best_action(&valid_actions)
        }
    }

    fn choose_best_action(&self, actions: &Vec<Action>) -> Action {
        let q_values = self.q_table.get_values(self.env.board());
        let mut max_q = 0.0;
        let mut best_actions = Vec::with_capacity(STATE_SIZE);
        for action in actions {
            let q_value = q_values[action_to_index(*action)];
            match q_value.partial_cmp(&max_q) {
                Some(Ordering::Greater) => {
                    best_actions.clear();
                    best_actions.push(action);
                    max_q = q_value;
                }
                Some(Ordering::Equal) => {
                    best_actions.push(action);
                }
                _ => {}
            }
        }
        if best_actions.len() == 1 {
            *best_actions[0]
        } else if best_actions.len() > 1 {
            let mut rng = rand::thread_rng();
            *best_actions[rng.sample(Uniform::from(0..best_actions.len()))]
        } else {
            panic!("no available actions");
        }
    }

    fn calculate_reward(&self, state: &<TicTacToe as Game>::Board, player: PlayerId) -> Reward {
        let mut reward = 0.0;
        for (id1, id2, id3) in winning_combinations() {
            reward += match (state.get_ref(id1), state.get_ref(id2), state.get_ref(id3)) {
                (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(Some(p3)))
                    if p1 == p2 && p2 == p3 =>
                {
                    if *p1 == player {
                        WIN_REWARD
                    } else {
                        -WIN_REWARD
                    }
                }
                (BoardCell(None), BoardCell(Some(p2)), BoardCell(Some(p3))) if p2 == p3 => {
                    if *p2 == player {
                        TWO_OUT_OF_THREE_REWARD
                    } else {
                        -TWO_OUT_OF_THREE_REWARD
                    }
                }
                (BoardCell(Some(p1)), BoardCell(None), BoardCell(Some(p3))) if p1 == p3 => {
                    if *p1 == player {
                        TWO_OUT_OF_THREE_REWARD
                    } else {
                        -TWO_OUT_OF_THREE_REWARD
                    }
                }
                (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(None)) if p1 == p2 => {
                    if *p1 == player {
                        TWO_OUT_OF_THREE_REWARD
                    } else {
                        -TWO_OUT_OF_THREE_REWARD
                    }
                }
                (BoardCell(Some(p1)), BoardCell(None), BoardCell(None)) => {
                    if *p1 == player {
                        ONE_OUT_OF_THREE_REWARD
                    } else {
                        -ONE_OUT_OF_THREE_REWARD
                    }
                }
                (BoardCell(None), BoardCell(Some(p2)), BoardCell(None)) => {
                    if *p2 == player {
                        ONE_OUT_OF_THREE_REWARD
                    } else {
                        -ONE_OUT_OF_THREE_REWARD
                    }
                }
                (BoardCell(None), BoardCell(None), BoardCell(Some(p3))) => {
                    if *p3 == player {
                        ONE_OUT_OF_THREE_REWARD
                    } else {
                        -ONE_OUT_OF_THREE_REWARD
                    }
                }
                _ => 0.0,
            }
        }
        reward
    }

    fn perform_action(&mut self, id: PlayerId, action: Action) {
        self.env
            .update(
                id,
                <TicTacToe as Game>::TurnData::new(
                    FieldRow::try_from(action.0).unwrap(),
                    FieldCol::try_from(action.1).unwrap(),
                ),
            )
            .unwrap();
    }
}
