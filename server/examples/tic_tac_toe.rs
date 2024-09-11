extern crate server;

use server::core::tic_tac_toe::TicTacToe;
use server::core::Game;

fn main() {
    let player1 = 1;
    let player2 = 2;
    let mut ttt = TicTacToe::new();

    let mut turn_data;
    turn_data = <TicTacToe as Game>::TurnData::new(1, 1);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(1, 2);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(2, 2);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(0, 0);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(2, 1);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(0, 1);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(2, 0);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
}
