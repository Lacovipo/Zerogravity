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
use search::{search, TranspositionTable, ThreadSafeReceiver};

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
    let mut num_threads = 1;

    eprintln!("ZeroGravity Chess Engine v1.18.0 (Rust) ready.");

    // Spawn stdin reader thread to communicate via channel
    let (tx, rx) = std::sync::mpsc::channel();
    let rx = ThreadSafeReceiver(rx);
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
    while let Ok(line) = rx.0.recv() {
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
                println!("id name ZeroGravity v1.18.0");
                println!("id author Antigravity");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Threads type spin default 1 min 1 max 128");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "setoption" => {
                if parts.len() >= 5 && parts[1] == "name" && parts[3] == "value" {
                    let opt_name = parts[2].to_lowercase();
                    let opt_val = parts[4];
                    match opt_name.as_str() {
                        "hash" => {
                            if let Ok(mb) = opt_val.parse::<usize>() {
                                tt = TranspositionTable::new(mb);
                                eprintln!("info string Hash set to {} MB", mb);
                            }
                        }
                        "threads" => {
                            if let Ok(t) = opt_val.parse::<usize>() {
                                num_threads = t.max(1).min(128);
                                eprintln!("info string Threads set to {}", num_threads);
                            }
                        }
                        _ => {}
                    }
                }
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
                while rx.0.try_recv().is_ok() {}

                let best = search(&mut board, depth, search_time, &tt, &mut history_table, &rx, num_threads);
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
    use super::search::{search, TranspositionTable, ThreadSafeReceiver};

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
        let tt = TranspositionTable::new(1);
        let mut history = [[0; 64]; 64];
        let (_tx, rx) = std::sync::mpsc::channel();
        let rx = ThreadSafeReceiver(rx);
        let best = search(&mut b, 3, 0.0, &tt, &mut history, &rx, 1);
        assert!(best.is_some());
        assert_eq!(best.unwrap().to_uci(), "d8h4");
    }

    #[test]
    fn test_scholars_mate() {
        let mut b = Board::new();
        b.load_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 4");
        let tt = TranspositionTable::new(1);
        let mut history = [[0; 64]; 64];
        let (_tx, rx) = std::sync::mpsc::channel();
        let rx = ThreadSafeReceiver(rx);
        let best = search(&mut b, 3, 0.0, &tt, &mut history, &rx, 1);
        assert!(best.is_some());
        assert_eq!(best.unwrap().to_uci(), "f3f7");
    }

    #[test]
    fn test_pawn_ending_draw() {
        let mut b = Board::new();
        // King vs King: draw evaluation should be exactly 0
        b.load_fen("k7/8/8/8/8/8/8/K7 w - - 0 1");
        let eval = super::evaluation::evaluate(&b, false);
        assert_eq!(eval, 0);
    }

    #[test]
    fn test_unstoppable_passed_pawn() {
        let mut b = Board::new();
        // Pawn at a2, White king at a1, Black king at h8. White to move.
        // The black king is too far (Chebyshev dist 7 > 4) so the pawn is unstoppable.
        b.load_fen("7k/8/8/8/8/8/P7/K7 w - - 0 1");
        let eval = super::evaluation::evaluate(&b, false);
        // Should have a huge score (over 1000) due to unstoppable bonus (+800)
        assert!(eval > 1000);
    }

    #[test]
    fn test_protected_passed_pawn() {
        let mut b1 = Board::new();
        let mut b2 = Board::new();
        
        // b1: White passed pawn at d5, supported by c4.
        b1.load_fen("k7/7p/6p1/3P4/2P5/8/8/K7 w - - 0 1");
        let eval_protected = super::evaluation::evaluate(&b1, false);

        // b2: White passed pawn at d5, unsupported by c3 (c3 does not defend d5).
        b2.load_fen("k7/7p/6p1/3P4/8/2P5/8/K7 w - - 0 1");
        let eval_unprotected = super::evaluation::evaluate(&b2, false);

        // The protected passed pawn position should evaluate significantly higher
        assert!(eval_protected > eval_unprotected);
    }

    #[test]
    fn test_insufficient_material() {
        let mut b = Board::new();

        // 1. King vs King
        b.load_fen("k7/8/8/8/8/8/8/K7 w - - 0 1");
        assert!(b.is_insufficient_material());

        // 2. King + Knight vs King
        b.load_fen("k7/8/8/8/8/8/8/K1N5 w - - 0 1");
        assert!(b.is_insufficient_material());
        b.load_fen("k1n5/8/8/8/8/8/8/K7 w - - 0 1");
        assert!(b.is_insufficient_material());

        // 3. King + Bishop vs King
        b.load_fen("k7/8/8/8/8/8/8/K1B5 w - - 0 1");
        assert!(b.is_insufficient_material());
        b.load_fen("k1b5/8/8/8/8/8/8/K7 w - - 0 1");
        assert!(b.is_insufficient_material());

        // 4. King + Bishop vs King + Bishop (same color squares)
        // c1 (dark) and f8 (dark)
        b.load_fen("k4b2/8/8/8/8/8/8/K1B5 w - - 0 1");
        assert!(b.is_insufficient_material());

        // 5. King + Bishop vs King + Bishop (different color squares)
        // c1 (dark) and c8 (light)
        b.load_fen("k1b5/8/8/8/8/8/8/K1B5 w - - 0 1");
        assert!(!b.is_insufficient_material());

        // 6. King + Pawn vs King (not draw by insufficient material)
        b.load_fen("k7/8/8/8/8/8/P7/K7 w - - 0 1");
        assert!(!b.is_insufficient_material());
    }
}
