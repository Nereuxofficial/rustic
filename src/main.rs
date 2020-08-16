mod board;
mod defs;
mod evaluation;
mod extra;
// mod interface;
mod engine;
mod misc;
mod movegen;

use board::{defs::ERR_FEN_PARTS, Board};
use extra::perft;
// use interface::console;
use movegen::MoveGenerator;
use std::sync::Arc;

fn main() {
    let test_pos = Some("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let move_generator = MoveGenerator::new();
    let mut board: Board = Board::new(Arc::new(move_generator));
    let setup_result = board.fen_read(test_pos);

    let engine = engine::Engine::new();

    engine.about();

    match setup_result {
        Ok(()) => perft::run(&board, 6), //while console::get_input(&mut board) != 0 {},
        Err(e) => println!("Error in FEN-part: {}", ERR_FEN_PARTS[e as usize]),
    }
}
