use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::BitAnd;
use std::path::Path;

use game_server::core::tic_tac_toe::{self as ttt, TicTacToe};
use game_server::core::{BoardCell, Game, GameState, PlayerPosition};
use rand::distributions::Uniform;
use rand::Rng;

const STATE_SIZE: usize = 9;
const TOTAL_STATE_COUNT: usize = 3usize.pow(STATE_SIZE as u32);

const WIN_REWARD: f32 = 10.0;
const TWO_OUT_OF_THREE_REWARD: f32 = 3.0;
const ONE_OUT_OF_THREE_REWARD: f32 = 1.3;

const DECAY_INTERVAL: usize = 1000;
const EXPLORATION_DECAY_RATE: f32 = 0.00001;
const LEARNING_RATE_DECAY_RATE: f32 = 0.00001;
const MIN_EXPLORATION_RATE: f32 = 0.2;
const MIN_LEARNING_RATE: f32 = 0.1;

type Action = (usize, usize);
type ActionValues = [Option<QValue>; STATE_SIZE];
type QValue = f32;
type Reward = f32;

fn calculate_q(old: QValue, max_q_next: QValue, reward: Reward, lr: f32, gamma: f32) -> QValue {
    old + lr * (reward + gamma * max_q_next - old)
}

fn action_to_index(action: Action) -> usize {
    (action.0 * 3) + action.1
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
        .all_indexed()
        .filter_map(|(index, cell)| {
            if cell.is_none() {
                return Some((index.row(), index.col()));
            }
            None
        })
        .collect()
}

fn get_best_actions(actions: &[Action], action_values: &[Option<QValue>]) -> Vec<Action> {
    let mut best_actions = Vec::with_capacity(STATE_SIZE);
    let mut max_q = None;
    for action in actions.iter() {
        if let Some(q_value) = action_values[action_to_index(*action)] {
            if let Some(max) = max_q {
                match q_value.partial_cmp(&max) {
                    Some(Ordering::Greater) => {
                        best_actions.clear();
                        best_actions.push(*action);
                        let _ = max_q.insert(q_value);
                    }
                    Some(Ordering::Equal) => {
                        best_actions.push(*action);
                    }
                    _ => {}
                }
            } else {
                let _ = max_q.insert(q_value);
                best_actions.push(*action);
            }
        }
    }
    best_actions
}

fn calculate_reward(state: &<TicTacToe as Game>::Board, player: PlayerPosition) -> Reward {
    let mut reward = 0.0;
    for (id1, id2, id3) in ttt::winning_combinations() {
        reward += match (state[id1], state[id2], state[id3]) {
            (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(Some(p3)))
                if p1 == p2 && p2 == p3 =>
            {
                return if p1 == player {
                    WIN_REWARD
                } else {
                    -WIN_REWARD
                };
            }
            (BoardCell(None), BoardCell(Some(p2)), BoardCell(Some(p3))) if p2 == p3 => {
                if p2 == player {
                    TWO_OUT_OF_THREE_REWARD
                } else {
                    -TWO_OUT_OF_THREE_REWARD
                }
            }
            (BoardCell(Some(p1)), BoardCell(None), BoardCell(Some(p3))) if p1 == p3 => {
                if p1 == player {
                    TWO_OUT_OF_THREE_REWARD
                } else {
                    -TWO_OUT_OF_THREE_REWARD
                }
            }
            (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(None)) if p1 == p2 => {
                if p1 == player {
                    TWO_OUT_OF_THREE_REWARD
                } else {
                    -TWO_OUT_OF_THREE_REWARD
                }
            }
            (BoardCell(Some(p1)), BoardCell(None), BoardCell(None)) => {
                if p1 == player {
                    ONE_OUT_OF_THREE_REWARD
                } else {
                    -ONE_OUT_OF_THREE_REWARD
                }
            }
            (BoardCell(None), BoardCell(Some(p2)), BoardCell(None)) => {
                if p2 == player {
                    ONE_OUT_OF_THREE_REWARD
                } else {
                    -ONE_OUT_OF_THREE_REWARD
                }
            }
            (BoardCell(None), BoardCell(None), BoardCell(Some(p3))) => {
                if p3 == player {
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
    pub fn get_max_value(&self, state_index: usize) -> Option<QValue> {
        let mut max_q = None;
        for val in self.get_values(state_index) {
            if let Some(max) = max_q {
                if matches!(val, Some(current) if *current > max) {
                    max_q = *val;
                }
            } else {
                max_q = *val;
            }
        }
        max_q
    }

    pub fn get_value(&self, state_index: usize, action_index: usize) -> Option<QValue> {
        self.0[state_index][action_index]
    }

    pub fn get_values(&self, state_index: usize) -> &[Option<QValue>] {
        self.0[state_index].as_slice()
    }

    pub fn set_value(&mut self, state_index: usize, action_index: usize, new_val: QValue) {
        self.0[state_index][action_index] = Some(new_val);
    }
}

pub struct Agent {
    q_table: QTable,
}

impl Agent {
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut q_table = QTable::default();
        let mut metadata_buf = [0; 2];
        let mut buf = [0; 4];
        for values in q_table.0.iter_mut() {
            file.read(&mut metadata_buf)?;
            let metadata = u16::from_ne_bytes(metadata_buf);
            if metadata == 0 {
                continue;
            }
            for (i, value) in values.iter_mut().enumerate() {
                let mask = 2u16.pow(i as u32);
                if metadata.bitand(mask) == mask {
                    file.read(&mut buf)?; // TODO: handle return value
                    let _ = value.insert(f32::from_ne_bytes(buf));
                }
            }
        }
        Ok(Self { q_table })
    }

    pub fn get_best_action(&self, board: &<TicTacToe as Game>::Board) -> Option<Action> {
        let valid_actions = get_valid_actions(board);
        if valid_actions.is_empty() {
            return None;
        }

        let q_values = self.q_table.get_values(state_to_index(board));
        let best_actions = get_best_actions(&valid_actions, q_values);
        if best_actions.is_empty() {
            None
        } else if best_actions.len() == 1 {
            Some(best_actions[0])
        } else {
            let mut rng = rand::thread_rng();
            Some(best_actions[rng.sample(Uniform::from(0..best_actions.len()))])
        }
    }
}

pub struct Model<R> {
    initial_learning_rate: f32,
    current_learning_rate: f32,
    exploration_level: f32,
    gamma: f32,
    episode: usize,
    q_table: QTable,
    env: TicTacToe,
    rng: R,
}

impl<R: Rng> Model<R> {
    pub fn new(rng: R, gamma: f32, lr: f32) -> Self {
        Self {
            initial_learning_rate: lr,
            current_learning_rate: lr,
            exploration_level: 1.0,
            gamma,
            episode: 0,
            q_table: Default::default(),
            env: Default::default(),
            rng,
        }
    }

    pub fn run_episode(&mut self, verbose: bool) {
        let mut step: usize = 0;
        let mut total_reward = 0.0;
        // for half of games simulate first action to train agent to play both X and O
        if self.episode % 2 == 1 {
            self.simulate_enemy_action();
        }
        while matches!(self.env.state(), GameState::Turn(_)) {
            let (action, greedy) = self.choose_epsilon_greedy_action();
            let prev_state = self.env.board().clone();
            let prev_state_index = state_to_index(&prev_state);
            let prev_state_values = self.q_table.get_values(prev_state_index).to_vec();
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
                    self.q_table.get_values(prev_state_index)
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
        if self.episode % DECAY_INTERVAL == 0 {
            self.decay();
        }
        self.reset();
    }

    pub fn dump_table(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        for values in &self.q_table.0 {
            let mut metadata = 0;
            for (i, value) in values.iter().enumerate() {
                if value.is_some() {
                    metadata += 2u16.pow(i as u32);
                }
            }
            file.write(&metadata.to_ne_bytes())?;
            for value in values {
                if let Some(value) = value {
                    file.write(&value.to_ne_bytes())?; // TODO: handle return value
                }
            }
        }
        Ok(())
    }

    fn decay(&mut self) {
        self.exploration_level = (-EXPLORATION_DECAY_RATE * self.episode as f32)
            .exp()
            .max(MIN_EXPLORATION_RATE);
        self.current_learning_rate = (self.initial_learning_rate
            / (1.0 + LEARNING_RATE_DECAY_RATE * self.episode as f32))
            .max(MIN_LEARNING_RATE);
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
        let prev_state_index = state_to_index(&prev_state);
        let action_index = action_to_index(action);
        let q_prev = self.q_table.get_value(prev_state_index, action_index);
        let max_q_next = self.q_table.get_max_value(state_to_index(self.env.board()));
        let new_q = calculate_q(
            q_prev.unwrap_or_default(),
            max_q_next.unwrap_or_default(),
            reward,
            self.current_learning_rate,
            self.gamma,
        );
        self.q_table
            .set_value(prev_state_index, action_index, new_q);
    }

    /// true - greedy, false - exploration
    fn choose_epsilon_greedy_action(&mut self) -> (Action, bool) {
        let valid_actions = get_valid_actions(self.env.board());
        if self.rng.sample(Uniform::from(0.0..1.0)) < self.exploration_level {
            (
                valid_actions[self.rng.sample(Uniform::from(0..valid_actions.len()))],
                false,
            )
        } else {
            (self.choose_best_action(&valid_actions), true)
        }
    }

    fn choose_best_action(&mut self, actions: &Vec<Action>) -> Action {
        if actions.is_empty() {
            panic!("no available actions");
        }
        let q_values = self.q_table.get_values(state_to_index(self.env.board()));
        let best_actions = get_best_actions(&actions, q_values);
        if best_actions.len() == 1 {
            best_actions[0]
        } else {
            if best_actions.is_empty() {
                actions[self.rng.sample(Uniform::from(0..actions.len()))]
            } else {
                best_actions[self.rng.sample(Uniform::from(0..best_actions.len()))]
            }
        }
    }

    fn perform_action(&mut self, id: PlayerPosition, action: Action) {
        self.env.update(id, action.into()).unwrap();
    }

    fn simulate_enemy_action(&mut self) {
        if let GameState::Turn(enemy) = self.env.state() {
            let (enemy_action, _) = self.choose_epsilon_greedy_action();
            self.perform_action(enemy, enemy_action);
        }
    }
}

#[cfg(test)]
mod test {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;

    type TTTBoard = <TicTacToe as Game>::Board;

    #[test]
    fn test_dump_load() {
        let mut model = Model::new(rand::thread_rng(), 1.0, 1.0);
        // fill with some data
        for i in 0..9u8 {
            model.q_table.set_value(i.into(), i.into(), i.into());
        }

        let tmp_file = "tmp";
        model.dump_table(tmp_file).unwrap();
        let res = std::panic::catch_unwind(|| {
            let agent = Agent::load(tmp_file).unwrap();
            itertools::assert_equal(model.q_table.0, agent.q_table.0);
        });
        std::fs::remove_file(tmp_file).unwrap();
        assert!(res.is_ok());
    }

    #[test]
    fn test_state_to_index() {
        let empty_board = TTTBoard::default();
        {
            let mut board = empty_board.clone();
            board[(2, 1).into()].0 = Some(0);
            assert_eq!(state_to_index(&board), 3);
        }
        {
            let mut board = empty_board.clone();
            board[(1, 1).into()].0 = Some(0);
            board[(2, 0).into()].0 = Some(1);
            board[(2, 2).into()].0 = Some(0);
            assert_eq!(state_to_index(&board), 100);
        }
        {
            let mut board = empty_board.clone();
            board[(2, 2).into()].0 = Some(0);
            board[(0, 1).into()].0 = Some(1);
            board[(1, 1).into()].0 = Some(0);
            assert_eq!(state_to_index(&board), 4456);
        }
        {
            let mut board = empty_board.clone();
            board[(2, 0).into()].0 = Some(0);
            board[(0, 2).into()].0 = Some(1);
            board[(0, 1).into()].0 = Some(0);
            board[(1, 0).into()].0 = Some(1);
            assert_eq!(state_to_index(&board), 4140);
        }
        {
            let mut board = empty_board.clone();
            board[(0, 0).into()].0 = Some(1);
            board[(0, 1).into()].0 = Some(1);
            board[(0, 2).into()].0 = Some(1);
            board[(1, 0).into()].0 = Some(1);
            board[(1, 1).into()].0 = Some(1);
            assert_eq!(state_to_index(&board), 19602);
        }
        {
            let mut board = empty_board.clone();
            board[(0, 0).into()].0 = Some(1);
            board[(0, 1).into()].0 = Some(1);
            board[(0, 2).into()].0 = Some(1);
            board[(1, 0).into()].0 = Some(1);
            board[(1, 1).into()].0 = Some(0);
            board[(1, 2).into()].0 = Some(0);
            board[(2, 0).into()].0 = Some(0);
            board[(2, 1).into()].0 = Some(0);
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
        board[(1, 1).into()].0 = Some(0);
        assert_eq!(calculate_reward(&board, 0), 5.2);
        assert_eq!(calculate_reward(&board, 1), -5.2);

        // - - -
        // - X -
        // - - O
        board[(2, 2).into()].0 = Some(1);
        assert_eq!(calculate_reward(&board, 0), 1.3);
        assert_eq!(calculate_reward(&board, 1), -1.3);

        // - - -
        // - X -
        // X - O
        board[(2, 0).into()].0 = Some(0);
        assert_eq!(calculate_reward(&board, 0), 5.6);
        assert_eq!(calculate_reward(&board, 1), -5.6);

        // O - -
        // - X -
        // X - O
        board[(0, 0).into()].0 = Some(1);
        assert_eq!(calculate_reward(&board, 0), 3.0);
        assert_eq!(calculate_reward(&board, 1), -3.0);

        // O - X
        // - X -
        // X - O
        board[(0, 2).into()].0 = Some(0);
        assert_eq!(calculate_reward(&board, 0), 10.0);
        assert_eq!(calculate_reward(&board, 1), -10.0);
    }

    #[test]
    fn test_choose_best_action() {
        let rng = rand_chacha::ChaCha8Rng::seed_from_u64(0);
        let mut model = Model::new(rng, 1.0, 1.0);
        // starting actions
        let valid_actions = get_valid_actions(model.env.board());

        // uninitialized state -> choose randomly from all actions
        assert_eq!(model.choose_best_action(&valid_actions), (2, 0));

        // add single negative q_value
        model.q_table.set_value(0, 0, -10.0);
        // choose added value because it's the only value that was set
        assert_eq!(model.choose_best_action(&valid_actions), (0, 0));

        // add single zero q_value
        model.q_table.set_value(0, 8, 0.0);
        // choose added value because it's the only value that was set
        assert_eq!(model.choose_best_action(&valid_actions), (2, 2));

        // add second zero q_value
        model.q_table.set_value(0, 4, 0.0);
        // randomly choose between two zeroes
        assert_eq!(model.choose_best_action(&valid_actions), (1, 1));
    }

    /// agent - X
    /// scenario: X O O   order: 2 7 1
    ///           O X X          5 0 4
    ///           X X O          8 6 3
    #[test]
    fn test_update_q_first_episode_best_rewards_for_x() {
        let agent = 0;
        let mut model = Model::new(rand::thread_rng(), 1.0, 1.0);
        // step 0
        let mut prev_state = model.env.board().clone();
        let mut prev_state_index = state_to_index(&prev_state);
        let mut action = (1, 1);
        model.perform_action(agent, action);
        model.perform_action(1, (0, 2));
        let mut reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 1.3);

        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            [None, None, None, None, Some(reward), None, None, None, None],
        );

        // step 1
        prev_state = model.env.board().clone();
        prev_state_index = state_to_index(&prev_state);
        action = (0, 0);
        model.perform_action(agent, action);
        model.perform_action(1, (2, 2));
        reward = calculate_reward(model.env.board(), agent);
        // needed due to floating point arithmetics
        assert!(reward > -0.4 - f32::EPSILON && reward < -0.4 + f32::EPSILON);

        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            [Some(reward), None, None, None, None, None, None, None, None],
        );

        // step 2
        prev_state = model.env.board().clone();
        prev_state_index = state_to_index(&prev_state);
        action = (1, 2);
        model.perform_action(agent, action);
        model.perform_action(1, (1, 0));
        reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 0.0);

        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            [None, None, None, None, None, Some(reward), None, None, None],
        );

        // step 3
        prev_state = model.env.board().clone();
        prev_state_index = state_to_index(&prev_state);
        action = (2, 1);
        model.perform_action(agent, action);
        model.perform_action(1, (0, 1));
        reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 0.0);

        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            [None, None, None, None, None, None, None, Some(reward), None],
        );

        // step 4
        prev_state = model.env.board().clone();
        prev_state_index = state_to_index(&prev_state);
        action = (2, 0);
        model.perform_action(agent, action);
        reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 0.0);

        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            ActionValues::default(),
        );
        model.update_q(&prev_state, action, reward);
        itertools::assert_equal(
            model.q_table.get_values(prev_state_index).to_vec(),
            [None, None, None, None, None, None, Some(reward), None, None],
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
        let mut model = Model::new(rand::thread_rng(), 1.0, 1.0);
        // init board
        model.perform_action(agent, (2, 2));
        model.perform_action(1, (2, 1));
        model.perform_action(agent, (1, 1));
        model.perform_action(1, (0, 1));
        // init q value for starting board and action
        let action = (1, 2);
        model.q_table.set_value(
            state_to_index(model.env.board()),
            action_to_index(action),
            -1.0,
        );

        let prev_state = model.env.board().clone();
        model.perform_action(agent, action);
        model.perform_action(1, (0, 2));
        let reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, 3.0);

        // init q values for next board state
        for (action, value) in [((0, 0), 3.0), ((1, 0), 5.0), ((2, 0), -1.0)] {
            model.q_table.set_value(
                state_to_index(model.env.board()),
                action_to_index(action),
                value,
            );
        }

        model.update_q(&prev_state, action, reward);
        let new_q = 8.0; // reward + max q
        itertools::assert_equal(
            model
                .q_table
                .get_values(state_to_index(&prev_state))
                .to_vec(),
            [None, None, None, None, None, Some(new_q), None, None, None],
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
        let mut model = Model::new(rand::thread_rng(), 0.8, 0.5);
        // init board
        model.perform_action(0, (1, 0));
        model.perform_action(agent, (1, 2));
        model.perform_action(0, (2, 1));
        // init q value for starting board and action
        let action = (2, 0);
        model.q_table.set_value(
            state_to_index(model.env.board()),
            action_to_index(action),
            2.0,
        );

        let prev_state = model.env.board().clone();
        model.perform_action(agent, action);
        model.perform_action(0, (1, 1));
        let reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, -3.0);

        // init q values for next board state
        for (action, value) in [((0, 0), -1.0), ((0, 1), 3.0), ((0, 2), 4.0), ((2, 2), -3.0)] {
            model.q_table.set_value(
                state_to_index(model.env.board()),
                action_to_index(action),
                value,
            );
        }

        model.update_q(&prev_state, action, reward);
        let new_q = 1.1; // 2 + 0.5*(-3 + (0.8*4) - 2)
        itertools::assert_equal(
            model
                .q_table
                .get_values(state_to_index(&prev_state))
                .to_vec(),
            [None, None, None, None, None, None, Some(new_q), None, None],
        );
    }

    /// agent - X
    /// initial state: O X -
    ///                - O X
    ///                ^ * -
    /// * - action that agent takes
    /// ^ - action that enemy takes
    #[test]
    fn test_update_q_not_influenced_by_invalid_actions() {
        let agent = 0;
        let mut model = Model::new(rand::thread_rng(), 1.0, 1.0);
        // init board
        model.perform_action(agent, (0, 1));
        model.perform_action(1, (0, 0));
        model.perform_action(agent, (1, 2));
        model.perform_action(1, (1, 1));
        // init q value for starting board and action
        let action = (2, 1);
        model.q_table.set_value(
            state_to_index(model.env.board()),
            action_to_index(action),
            -5.0,
        );

        let prev_state = model.env.board().clone();
        model.perform_action(agent, action);
        model.perform_action(1, (2, 0));
        let reward = calculate_reward(model.env.board(), agent);
        assert_eq!(reward, -7.7);

        // init q values for next board state
        for (action, value) in [((0, 2), -6.0), ((1, 0), -4.0), ((2, 2), -7.0)] {
            model.q_table.set_value(
                state_to_index(model.env.board()),
                action_to_index(action),
                value,
            );
        }

        model.update_q(&prev_state, action, reward);
        let new_q = -11.7; // -5 + (-7.7 + -4 - -5)
        itertools::assert_equal(
            model
                .q_table
                .get_values(state_to_index(&prev_state))
                .to_vec(),
            [None, None, None, None, None, None, None, Some(new_q), None],
        );
    }
}
