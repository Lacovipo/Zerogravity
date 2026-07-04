// src/perft.rs
// Perft verification for ZeroGravity

#![allow(dead_code)]
use crate::board::Board;
use crate::movegen::generate_legal_moves;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = generate_legal_moves(board);
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut nodes = 0_u64;
    for m in moves {
        board.make_move(m);
        nodes += perft(board, depth - 1);
        board.unmake_move(m);
    }
    nodes
}
