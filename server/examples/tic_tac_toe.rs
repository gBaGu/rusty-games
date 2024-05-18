extern crate server;

use server::game::game::Game;
use server::game::tic_tac_toe::{FieldCol, FieldRow, TicTacToe};

fn main() {
    let player1 = 1;
    let player2 = 2;
    let mut ttt = TicTacToe::new(&[player1, player2]).unwrap();

    let mut turn_data;
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R2, FieldCol::C2);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R2, FieldCol::C3);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R3, FieldCol::C3);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R1, FieldCol::C1);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R3, FieldCol::C2);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R1, FieldCol::C2);
    ttt.update(player2, turn_data).unwrap();
    println!("{:?}", ttt);
    turn_data = <TicTacToe as Game>::TurnData::new(FieldRow::R3, FieldCol::C1);
    ttt.update(player1, turn_data).unwrap();
    println!("{:?}", ttt);
}