// src/main.rs
// UCI interface for ZeroGravity

mod board;
mod movegen;
mod perft;
mod evaluation;
mod search;

use std::io::{self, BufRead};
use board::{Board, WHITE};
use movegen::generate_legal_moves;
use search::{search, TranspositionTable};

fn parse_position(parts: &[&str], board: &mut Board) {
    if parts.len() < 2 {
        return;
    }
    let moves_idx;
    if parts[1] == "startpos" {
        board.load_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        moves_idx = parts.iter().position(|&x| x == "moves");
    } else if parts[1] == "fen" {
        let moves_pos = parts.iter().position(|&x| x == "moves");
        let fen_str = match moves_pos {
            Some(pos) => parts[2..pos].join(" "),
            None => parts[2..].join(" "),
        };
        board.load_fen(&fen_str);
        moves_idx = moves_pos;
    } else {
        return;
    }

    if let Some(idx) = moves_idx {
        if idx + 1 < parts.len() {
            for &move_str in &parts[idx+1..] {
                let legal = generate_legal_moves(board);
                let mut matched = None;
                for m in legal {
                    if m.to_uci() == move_str {
                        matched = Some(m);
                        break;
                    }
                }
                if let Some(m) = matched {
                    board.make_move(m);
                }
            }
        }
    }
}

fn parse_go(parts: &[&str], board: &Board) -> (i32, f64) {
    let mut wtime = None;
    let mut btime = None;
    let mut winc = 0;
    let mut binc = 0;
    let mut depth = None;
    let mut movetime = None;
    let mut infinite = false;
    let mut movestogo = None;

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "wtime" if i + 1 < parts.len() => {
                wtime = parts[i+1].parse::<i64>().ok();
                i += 2;
            }
            "btime" if i + 1 < parts.len() => {
                btime = parts[i+1].parse::<i64>().ok();
                i += 2;
            }
            "winc" if i + 1 < parts.len() => {
                winc = parts[i+1].parse::<i64>().unwrap_or(0);
                i += 2;
            }
            "binc" if i + 1 < parts.len() => {
                binc = parts[i+1].parse::<i64>().unwrap_or(0);
                i += 2;
            }
            "depth" if i + 1 < parts.len() => {
                depth = parts[i+1].parse::<i32>().ok();
                i += 2;
            }
            "movetime" if i + 1 < parts.len() => {
                movetime = parts[i+1].parse::<i64>().ok();
                i += 2;
            }
            "movestogo" if i + 1 < parts.len() => {
                movestogo = parts[i+1].parse::<i64>().ok();
                i += 2;
            }
            "infinite" => {
                infinite = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    let mut search_time = 0.0;
    if let Some(mt) = movetime {
        search_time = ((mt - 50) as f64 / 1000.0).max(0.01);
    } else if wtime.is_some() || btime.is_some() {
        let us = board.side_to_move;
        let time_left = if us == WHITE { wtime } else { btime };
        let inc = if us == WHITE { winc } else { binc };
        if let Some(tl) = time_left {
            let moves_to_search = movestogo.unwrap_or(40).max(2);
            let allocated = (tl as f64 / moves_to_search as f64) + (inc as f64 / 2.0);
            let allocated = allocated.min((tl - 100) as f64); // buffer
            search_time = (allocated / 1000.0).max(0.05);
        }
    }

    let mut final_depth = if let Some(d) = depth {
        d
    } else if movetime.is_some() || wtime.is_some() || btime.is_some() || infinite {
        64
    } else {
        6
    };

    if infinite {
        search_time = 0.0;
        final_depth = 64;
    }

    (final_depth, search_time)
}

fn main() {
    let mut board = Board::new();
    let mut tt = TranspositionTable::new(16); // 16MB table by default
    let mut history_table = [[0_i32; 64]; 64];

    eprintln!("ZeroGravity Chess Engine v1.16.0 (Rust) ready.");

    // Spawn stdin reader thread to communicate via channel
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line_res in stdin.lock().lines() {
            if let Ok(line) = line_res {
                if tx.send(line).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    // Main UCI command processing loop
    while let Ok(line) = rx.recv() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line_trimmed.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let cmd = parts[0];

        match cmd {
            "uci" => {
                println!("id name ZeroGravity v1.16.0");
                println!("id author Antigravity");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "ucinewgame" => {
                board.clear();
                board.load_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
                tt.clear();
                history_table = [[0_i32; 64]; 64];
            }
            "position" => {
                parse_position(&parts, &mut board);
            }
            "go" => {
                let (depth, search_time) = parse_go(&parts, &board);
                // Clear any leftover messages from the channel (e.g. stop from previous searches)
                while rx.try_recv().is_ok() {}

                let best = search(&mut board, depth, search_time, &mut tt, &mut history_table, &rx);
                if let Some(m) = best {
                    println!("bestmove {}", m.to_uci());
                }
            }
            "quit" => {
                break;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::board::Board;
    use super::perft::perft;
    use super::search::{search, TranspositionTable};

    #[test]
    fn test_perft_initial() {
        let mut b = Board::new();
        let nodes3 = perft(&mut b, 3);
        assert_eq!(nodes3, 8902);
        let nodes4 = perft(&mut b, 4);
        assert_eq!(nodes4, 197281);
    }

    #[test]
    fn test_perft_kiwipete() {
        let mut b = Board::new();
        b.load_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        let nodes2 = perft(&mut b, 2);
        assert_eq!(nodes2, 2039);
        let nodes3 = perft(&mut b, 3);
        assert_eq!(nodes3, 97862);
    }

    #[test]
    fn test_mate_in_1() {
        let mut b = Board::new();
        b.load_fen("rnbqk1nr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2");
        let mut tt = TranspositionTable::new(1);
        let mut history = [[0; 64]; 64];
        let (_tx, rx) = std::sync::mpsc::channel();
        let best = search(&mut b, 3, 0.0, &mut tt, &mut history, &rx);
        assert!(best.is_some());
        assert_eq!(best.unwrap().to_uci(), "d8h4");
    }

    #[test]
    fn test_scholars_mate() {
        let mut b = Board::new();
        b.load_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 4");
        let mut tt = TranspositionTable::new(1);
        let mut history = [[0; 64]; 64];
        let (_tx, rx) = std::sync::mpsc::channel();
        let best = search(&mut b, 3, 0.0, &mut tt, &mut history, &rx);
        assert!(best.is_some());
        assert_eq!(best.unwrap().to_uci(), "f3f7");
    }
}
