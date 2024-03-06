mod game;
mod rpc_server;

use game::tic_tac_toe::{TicTacToe, TurnData, FieldRow, FieldCol};


fn main() {
    let player1 = 1;
    let player2 = 2;
    let mut ttt = TicTacToe::new(player1, player2).unwrap();
    ttt.make_turn(player1, TurnData::new(FieldRow::R2, FieldCol::C2)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player2, TurnData::new(FieldRow::R2, FieldCol::C3)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player1, TurnData::new(FieldRow::R3, FieldCol::C3)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player2, TurnData::new(FieldRow::R1, FieldCol::C1)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player1, TurnData::new(FieldRow::R3, FieldCol::C2)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player2, TurnData::new(FieldRow::R1, FieldCol::C2)).unwrap();
    println!("{:?}", ttt);
    ttt.make_turn(player1, TurnData::new(FieldRow::R3, FieldCol::C1)).unwrap();
    println!("{:?}", ttt);
}
