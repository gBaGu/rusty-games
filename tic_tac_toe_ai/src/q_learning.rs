use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Write};

use game_server::game::tic_tac_toe::{winning_combinations, FieldCol, FieldRow, TicTacToe};
use game_server::game::{BoardCell, Game, GameState, PlayerId};
use rand::distributions::Uniform;
use rand::Rng;

const STATE_SIZE: usize = 9;
const TOTAL_STATE_COUNT: usize = 3usize.pow(STATE_SIZE as u32); // TODO: set to 19561 because last states are all invalid

const WIN_REWARD: f32 = 10.0;
const TWO_OUT_OF_THREE_REWARD: f32 = 3.0;
const ONE_OUT_OF_THREE_REWARD: f32 = 1.4;

const EXPLORATION_DECAY_RATE: f32 = 0.0001;
const LEARNING_RATE_DECAY_RATE: f32 = 0.001;
const MIN_EXPLORATION_RATE: f32 = 0.2;

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
            index += 3usize.pow(exp) * (player_id + 1) as usize;
        }
    }
    index
}

// TODO: try SmallVec here
fn get_valid_actions(board: &<TicTacToe as Game>::Board) -> Vec<Action> {
    board
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

fn calculate_reward(state: &<TicTacToe as Game>::Board, player: PlayerId) -> Reward {
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

#[derive(Clone)]
pub struct QTable(Vec<ActionValues>);

impl Default for QTable {
    fn default() -> Self {
        Self(vec![ActionValues::default(); TOTAL_STATE_COUNT])
    }
}

impl QTable {
    pub fn get_max_value(&self, state: &<TicTacToe as Game>::Board) -> QValue {
        let mut max_q = f32::NEG_INFINITY;
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

pub struct Agent {
    q_table: QTable,
}

impl Agent {
    pub fn load(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut q_table = QTable::default();
        let mut buf = [0; 4];
        for values in q_table.0.iter_mut() {
            for value in values {
                file.read(&mut buf)?; // TODO: handle return value
                *value = f32::from_ne_bytes(buf);
            }
        }
        Ok(Self { q_table })
    }

    pub fn get_action(&self, board: &<TicTacToe as Game>::Board) -> Option<Action> {
        let valid_actions = get_valid_actions(board);
        if valid_actions.is_empty() {
            return None;
        }

        let q_values = self.q_table.get_values(board);
        let mut best_actions = Vec::with_capacity(STATE_SIZE);
        let mut max_q = q_values[action_to_index(valid_actions[0])];
        for action in valid_actions.iter() {
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
        let mut rng = rand::thread_rng();
        Some(*best_actions[rng.sample(Uniform::from(0..best_actions.len()))])
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
        if self.episode % 2 == 1 {
            self.simulate_enemy_action();
        }
        while matches!(self.env.state(), GameState::Turn(_)) {
            let (action, greedy) = self.choose_epsilon_greedy_action();
            let prev_state = self.env.board().clone();
            let prev_state_values = self.q_table.get_values(&prev_state).to_vec();
            let reward = self.step(action);
            self.update_q(&prev_state, action, reward);

            if verbose {
                println!(
                    "state after step {}{}: {}",
                    step,
                    if greedy { "" } else { "*" },
                    self.env.board()
                );
                println!(
                    "rewards before: {:?}\nrewards after: {:?}",
                    prev_state_values,
                    self.q_table.get_values(&prev_state)
                );
            }
            total_reward += reward;
            step += 1;
        }
        if verbose {
            println!(
                "episode {} summary:\nlearning_rate: {}\nexploration level: {}\ntotal steps: {}\ntotal reward: {}",
                self.episode, self.current_learning_rate, self.exploration_level, step, total_reward
            );
        }
        self.episode += 1;
        if self.episode % 100 == 0 {
            self.decay();
        }
        self.reset();
    }

    pub fn dump_table(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        for values in &self.q_table.0 {
            for value in values {
                file.write(&value.to_ne_bytes())?; // TODO: handle return value
            }
        }
        Ok(())
    }

    fn decay(&mut self) {
        self.exploration_level = (-EXPLORATION_DECAY_RATE * self.episode as f32)
            .exp()
            .max(MIN_EXPLORATION_RATE);
        self.current_learning_rate =
            self.initial_learning_rate / (1.0 + LEARNING_RATE_DECAY_RATE * self.episode as f32)
    }

    fn reset(&mut self) {
        self.env = Default::default();
    }

    fn step(&mut self, action: Action) -> Reward {
        if let GameState::Turn(id) = self.env.state() {
            self.perform_action(id, action);
            self.simulate_enemy_action();

            return calculate_reward(self.env.board(), id);
        }
        0.0
    }

    fn update_q(
        &mut self,
        prev_state: &<TicTacToe as Game>::Board,
        action: Action,
        reward: Reward,
    ) {
        let q_prev = self.q_table.get_value(&prev_state, action);
        let max_q_next = self.q_table.get_max_value(self.env.board());
        let new_q = calculate_q(
            q_prev,
            max_q_next,
            reward,
            self.current_learning_rate,
            self.gamma,
        );
        self.q_table.set_value(&prev_state, action, new_q);
    }

    /// true - greedy, false - exploration
    fn choose_epsilon_greedy_action(&self) -> (Action, bool) {
        let valid_actions = get_valid_actions(self.env.board());
        let mut rng = rand::thread_rng();
        if rng.sample(Uniform::from(0.0..1.0)) < self.exploration_level {
            (
                valid_actions[rng.sample(Uniform::from(0..valid_actions.len()))],
                false,
            )
        } else {
            (self.choose_best_action(&valid_actions), true)
        }
    }

    fn choose_best_action(&self, actions: &Vec<Action>) -> Action {
        if actions.is_empty() {
            panic!("no available actions");
        }
        let q_values = self.q_table.get_values(self.env.board());
        let mut best_actions = Vec::with_capacity(STATE_SIZE);
        let mut max_q = q_values[action_to_index(actions[0])];
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
            unreachable!()
        }
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

    fn simulate_enemy_action(&mut self) {
        if let GameState::Turn(enemy) = self.env.state() {
            let enemy_action = self.choose_best_action(&get_valid_actions(self.env.board()));
            self.perform_action(enemy, enemy_action);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use game_server::game::tic_tac_toe::{FieldCol, FieldRow, TicTacToe};
    use game_server::game::Game;

    type TTTBoard = <TicTacToe as Game>::Board;
    type TTTIndex = <TicTacToe as Game>::TurnData;

    fn set_board_cell(board: &mut TTTBoard, index: (usize, usize), value: PlayerId) {
        board
            .get_mut_ref(TTTIndex::new(
                FieldRow::try_from(index.0).unwrap(),
                FieldCol::try_from(index.1).unwrap(),
            ))
            .0 = Some(value);
    }

    #[test]
    fn test_state_to_index() {
        let empty_board = TTTBoard::default();
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (2, 1), 0);
            assert_eq!(state_to_index(&board), 3);
        }
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (1, 1), 0);
            set_board_cell(&mut board, (2, 0), 1);
            set_board_cell(&mut board, (2, 2), 0);
            assert_eq!(state_to_index(&board), 100);
        }
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (2, 2), 0);
            set_board_cell(&mut board, (0, 1), 1);
            set_board_cell(&mut board, (1, 1), 0);
            assert_eq!(state_to_index(&board), 4456);
        }
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (2, 0), 0);
            set_board_cell(&mut board, (0, 2), 1);
            set_board_cell(&mut board, (0, 1), 0);
            set_board_cell(&mut board, (1, 0), 1);
            assert_eq!(state_to_index(&board), 4140);
        }
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (0, 0), 1);
            set_board_cell(&mut board, (0, 1), 1);
            set_board_cell(&mut board, (0, 2), 1);
            set_board_cell(&mut board, (1, 0), 1);
            set_board_cell(&mut board, (1, 1), 1);
            assert_eq!(state_to_index(&board), 19602);
        }
        {
            let mut board = empty_board.clone();
            set_board_cell(&mut board, (0, 0), 1);
            set_board_cell(&mut board, (0, 1), 1);
            set_board_cell(&mut board, (0, 2), 1);
            set_board_cell(&mut board, (1, 0), 1);
            set_board_cell(&mut board, (1, 1), 0);
            set_board_cell(&mut board, (1, 2), 0);
            set_board_cell(&mut board, (2, 0), 0);
            set_board_cell(&mut board, (2, 1), 0);
            assert_eq!(state_to_index(&board), 19560);
        }
    }

    #[test]
    fn test_calculate_reward() {
        let mut board = TTTBoard::default();
        assert_eq!(calculate_reward(&board, 0), 0.0);
        assert_eq!(calculate_reward(&board, 1), 0.0);

        // - - -
        // - X -
        // - - -
        set_board_cell(&mut board, (1, 1), 0);
        assert_eq!(calculate_reward(&board, 0), 5.6);
        assert_eq!(calculate_reward(&board, 1), -5.6);

        // - - -
        // - X -
        // - - O
        set_board_cell(&mut board, (2, 2), 1);
        assert_eq!(calculate_reward(&board, 0), 1.4);
        assert_eq!(calculate_reward(&board, 1), -1.4);

        // - - -
        // - X -
        // X - O
        set_board_cell(&mut board, (2, 0), 0);
        // TODO: fix this
        // assert_eq!(calculate_reward(&board, 0), 5.8);
        // assert_eq!(calculate_reward(&board, 1), -5.8);
        assert!(
            calculate_reward(&board, 0) > 5.8 - 0.000001
                && calculate_reward(&board, 0) < 5.8 + 0.000001
        );
        assert!(
            calculate_reward(&board, 1) > -5.8 - 0.000001
                && calculate_reward(&board, 1) < -5.8 + 0.000001
        );

        // O - -
        // - X -
        // X - O
        set_board_cell(&mut board, (0, 0), 1);
        assert_eq!(calculate_reward(&board, 0), 3.0);
        assert_eq!(calculate_reward(&board, 1), -3.0);

        // O - X
        // - X -
        // X - O
        set_board_cell(&mut board, (0, 2), 0);
        assert_eq!(calculate_reward(&board, 0), 12.8);
        assert_eq!(calculate_reward(&board, 1), -12.8);
    }

    #[test]
    fn test_update_q_first_episode_best_rewards_for_x() {
        let mut model = Model::new(1.0, 1.0);
        // step 0
        let mut prev_state = model.env.board().clone();
        let mut action = (1, 1);
        model.perform_action(0, action);
        model.perform_action(1, (0, 2));
        let mut reward = calculate_reward(model.env.board(), 0);
        assert_eq!(reward, 1.4);

        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [0.0, 0.0, 0.0, 0.0, reward, 0.0, 0.0, 0.0, 0.0],
        );

        // step 1
        prev_state = model.env.board().clone();
        action = (0, 0);
        model.perform_action(0, action);
        model.perform_action(1, (2, 2));
        reward = calculate_reward(model.env.board(), 0);
        // TODO: fix this
        assert!(reward > -0.2 - f32::EPSILON && reward < -0.2 + f32::EPSILON);

        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [reward, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        );

        // step 2
        prev_state = model.env.board().clone();
        action = (1, 2);
        model.perform_action(0, action);
        model.perform_action(1, (1, 0));
        reward = calculate_reward(model.env.board(), 0);
        assert_eq!(reward, 0.0);

        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [0.0, 0.0, 0.0, 0.0, 0.0, reward, 0.0, 0.0, 0.0],
        );

        // step 3
        prev_state = model.env.board().clone();
        action = (2, 1);
        model.perform_action(0, action);
        model.perform_action(1, (0, 1));
        reward = calculate_reward(model.env.board(), 0);
        assert_eq!(reward, 0.0);

        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, reward, 0.0],
        );
    }

    /// agent - X
    /// initial state: - O ^
    ///                - X *
    ///                - O X
    /// * - action that agent takes
    /// ^ - action that enemy takes
    #[test]
    fn test_update_q_with_filled_table_lr_1_gamma_1() {
        let agent = 0;
        let mut model = Model::new(1.0, 1.0);
        // init board
        model.perform_action(agent, (2, 2));
        model.perform_action(1, (2, 1));
        model.perform_action(agent, (1, 1));
        model.perform_action(1, (0, 1));
        // init q value for starting board and action
        let action = (1, 2);
        model.q_table.set_value(model.env.board(), action, -1.0);

        let prev_state = model.env.board().clone();
        model.perform_action(agent, action);
        model.perform_action(1, (0, 2));
        let reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 3.0);

        // init q values for next board state
        for (action, value) in [
            ((0, 0), 3.0),
            ((0, 1), 0.0),
            ((0, 2), -4.0),
            ((1, 0), 5.0),
            ((1, 1), 0.0),
            ((1, 2), -4.0),
            ((2, 0), -1.0),
            ((2, 1), 0.0),
            ((2, 2), 0.0),
        ] {
            model.q_table.set_value(model.env.board(), action, value);
        }

        model.update_q(&prev_state, action, reward);
        let updated_q = 8.0; // reward + max q
        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [0.0, 0.0, 0.0, 0.0, 0.0, updated_q, 0.0, 0.0, 0.0],
        );
    }

    /// agent - O
    /// initial state: - - -
    ///                X ^ O
    ///                * X -
    /// * - action that agent takes
    /// ^ - action that enemy takes
    #[test]
    fn test_update_q_with_filled_table_lr_05_gamma_08() {
        let agent = 1;
        let mut model = Model::new(0.8, 0.5);
        // init board
        model.perform_action(0, (1, 0));
        model.perform_action(agent, (1, 2));
        model.perform_action(0, (2, 1));
        // init q value for starting board and action
        let action = (2, 0);
        model.q_table.set_value(model.env.board(), action, 2.0);

        let prev_state = model.env.board().clone();
        model.perform_action(agent, action);
        model.perform_action(0, (1, 1));
        let reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, -3.0);

        // init q values for next board state
        for (action, value) in [
            ((0, 0), -1.0),
            ((0, 1), 3.0),
            ((0, 2), 4.0),
            ((1, 0), 0.0),
            ((1, 1), 0.0),
            ((1, 2), 0.0),
            ((2, 0), 0.0),
            ((2, 1), 0.0),
            ((2, 2), -3.0),
        ] {
            model.q_table.set_value(model.env.board(), action, value);
        }

        model.update_q(&prev_state, action, reward);
        let updated_q = 1.1; // 2 + 0.5*(-3 + (0.8*4) - 2)

        itertools::assert_equal(
            model.q_table.get_values(&prev_state).to_vec(),
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, updated_q, 0.0, 0.0],
        );
    }
}
