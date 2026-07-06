// src/search.rs
// Alpha-Beta Search Engine for ZeroGravity

#![allow(dead_code)]
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::board::{
    Board, Move, WHITE, WHITE_KING, BLACK_KING,
    bit_scan_forward
};
use crate::movegen::{generate_legal_moves, is_square_attacked};
use crate::evaluation::evaluate;

// Constants
pub const INF: i32 = 1000000;
pub const MATE_SCORE: i32 = 100000;
pub const MAX_PLY: usize = 64;

// Transposition Table flags
pub const EXACT: u8 = 0;
pub const ALPHA: u8 = 1;
pub const BETA: u8 = 2;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: i32,
    pub flag: u8,
    pub val: i32,
    pub best_move: Option<Move>,
}

pub struct TranspositionTable {
    pub entries: Vec<std::cell::UnsafeCell<Option<TTEntry>>>,
}

unsafe impl Sync for TranspositionTable {}
unsafe impl Send for TranspositionTable {}

impl TranspositionTable {
    pub fn new(megabytes: usize) -> Self {
        let size = (megabytes * 1024 * 1024) / std::mem::size_of::<std::cell::UnsafeCell<Option<TTEntry>>>();
        let size = size.next_power_of_two();
        let mut entries = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(std::cell::UnsafeCell::new(None));
        }
        TranspositionTable { entries }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) & (self.entries.len() - 1);
        unsafe {
            if let Some(entry) = *self.entries[index].get() {
                if entry.hash == hash {
                    return Some(entry);
                }
            }
        }
        None
    }

    pub fn store(&self, hash: u64, depth: i32, flag: u8, val: i32, best_move: Option<Move>) {
        let index = (hash as usize) & (self.entries.len() - 1);
        unsafe {
            let entry_ptr = self.entries[index].get();
            let replace = match *entry_ptr {
                None => true,
                Some(entry) => entry.depth <= depth,
            };
            if replace {
                *entry_ptr = Some(TTEntry {
                    hash,
                    depth,
                    flag,
                    val,
                    best_move,
                });
            }
        }
    }

    pub fn clear(&self) {
        for entry in self.entries.iter() {
            unsafe {
                *entry.get() = None;
            }
        }
    }
}

pub struct ThreadSafeReceiver(pub std::sync::mpsc::Receiver<String>);
unsafe impl Sync for ThreadSafeReceiver {}
unsafe impl Send for ThreadSafeReceiver {}

pub struct SearchState<'a> {
    pub start_time: Instant,
    pub time_limit: f64, // in seconds, 0.0 means no limit
    pub stop_search: Arc<AtomicBool>,
    pub is_main: bool,
    pub nodes_searched: u64,
    pub killer_moves: [[Option<Move>; 2]; MAX_PLY],
    pub history_table: &'a mut [[i32; 64]; 64],
    pub lmr_table: [[i32; 64]; 64],
    pub eval_history: [i32; MAX_PLY],
    pub tt: &'a TranspositionTable,
    pub rx: &'a ThreadSafeReceiver,
}

impl<'a> SearchState<'a> {
    pub fn time_up(&mut self) -> bool {
        if self.stop_search.load(Ordering::Relaxed) {
            return true;
        }
        if (self.nodes_searched & 2047) == 0 {
            if self.is_main {
                while let Ok(cmd) = self.rx.0.try_recv() {
                    let trimmed = cmd.trim();
                    if trimmed == "stop" || trimmed == "quit" {
                        self.stop_search.store(true, Ordering::Relaxed);
                        return true;
                    } else if trimmed == "isready" {
                        println!("readyok");
                    }
                }
            }
            if self.time_limit > 0.0 {
                let elapsed = self.start_time.elapsed().as_secs_f64();
                if elapsed >= self.time_limit {
                    self.stop_search.store(true, Ordering::Relaxed);
                    return true;
                }
            }
        }
        false
    }
}

fn has_non_pawns(board: &Board, color: usize) -> bool {
    use crate::board::{WHITE_KNIGHT, WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN, BLACK_KNIGHT, BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN};
    if color == WHITE {
        (board.pieces[WHITE_KNIGHT] | board.pieces[WHITE_BISHOP] | board.pieces[WHITE_ROOK] | board.pieces[WHITE_QUEEN]) != 0
    } else {
        (board.pieces[BLACK_KNIGHT] | board.pieces[BLACK_BISHOP] | board.pieces[BLACK_ROOK] | board.pieces[BLACK_QUEEN]) != 0
    }
}

pub fn order_moves(board: &Board, moves: &mut [Move], tt_move: Option<Move>, killer_1: Option<Move>, killer_2: Option<Move>, history_table: &[[i32; 64]; 64]) {
    let score_move = |m: &Move| -> i32 {
        if let Some(tt_m) = tt_move {
            if m.from_sq == tt_m.from_sq && m.to_sq == tt_m.to_sq && m.promotion == tt_m.promotion {
                return 100000;
            }
        }
        if m.is_capture {
            let victim = board.get_piece_at(m.to_sq);
            let victim_type = match victim {
                Some(v) => (v % 6) as i32,
                None => 0, // EP pawn
            };
            let aggressor = board.get_piece_at(m.from_sq);
            let aggressor_type = match aggressor {
                Some(a) => (a % 6) as i32,
                None => 0,
            };
            return 90000 + 10 * victim_type - aggressor_type;
        }
        if let Some(kill_m) = killer_1 {
            if m.from_sq == kill_m.from_sq && m.to_sq == kill_m.to_sq && m.promotion == kill_m.promotion {
                return 85000;
            }
        }
        if let Some(kill_m) = killer_2 {
            if m.from_sq == kill_m.from_sq && m.to_sq == kill_m.to_sq && m.promotion == kill_m.promotion {
                return 80000;
            }
        }
        if let Some(promo) = m.promotion {
            return 70000 + promo as i32;
        }
        (history_table[m.from_sq as usize][m.to_sq as usize]).min(60000)
    };

    moves.sort_unstable_by_key(|m| -score_move(m));
}

pub fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ply: i32, state: &mut SearchState) -> i32 {
    state.nodes_searched += 1;

    if state.time_up() {
        return 0;
    }

    if board.is_insufficient_material() || board.halfmove_clock >= 100 {
        return 0;
    }

    // Check if we are in check
    let us = board.side_to_move;
    let them = 1 - us;
    let king_p = if us == WHITE { WHITE_KING } else { BLACK_KING };
    let king_bb = board.pieces[king_p];
    let in_check = if king_bb != 0 {
        let king_sq = bit_scan_forward(king_bb) as u8;
        is_square_attacked(board, king_sq, them)
    } else {
        false
    };

    if !in_check {
        let stand_pat = evaluate(board, false);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    let moves = generate_legal_moves(board);
    let mut quiesce_moves: Vec<Move> = if in_check {
        moves
    } else {
        moves.into_iter().filter(|m| m.is_capture).collect()
    };

    if in_check && quiesce_moves.is_empty() {
        return -MATE_SCORE + ply;
    }

    order_moves(board, &mut quiesce_moves, None, None, None, &state.history_table);

    for m in quiesce_moves {
        board.make_move(m);
        let val = -quiescence(board, -beta, -alpha, ply + 1, state);
        board.unmake_move(m);

        if state.time_up() {
            return 0;
        }

        if val >= beta {
            return beta;
        }
        if val > alpha {
            alpha = val;
        }
    }

    alpha
}

pub fn negamax(board: &mut Board, depth: i32, mut alpha: i32, beta: i32, ply: usize, state: &mut SearchState) -> i32 {
    if ply >= MAX_PLY - 1 {
        return evaluate(board, false);
    }

    state.nodes_searched += 1;

    if state.time_up() {
        return 0;
    }

    let static_eval = evaluate(board, false);
    if ply < MAX_PLY {
        state.eval_history[ply] = static_eval;
    }
    let improving = if ply >= 2 && ply < MAX_PLY {
        static_eval >= state.eval_history[ply - 2]
    } else {
        false
    };

    if board.halfmove_clock >= 100 || board.is_insufficient_material() {
        return 0;
    }

    if ply > 0 {
        let limit = board.history.len().saturating_sub(board.halfmove_clock as usize);
        if board.history.len() >= 2 {
            let mut i = board.history.len() - 2;
            while i >= limit {
                if board.history[i].hash == board.hash {
                    return 0;
                }
                if i < 2 {
                    break;
                }
                i -= 2;
            }
        }
    }

    // Check if we are in check
    let us = board.side_to_move;
    let them = 1 - us;
    let king_p = if us == WHITE { WHITE_KING } else { BLACK_KING };
    let king_bb = board.pieces[king_p];
    let in_check = if king_bb != 0 {
        let king_sq = bit_scan_forward(king_bb) as u8;
        is_square_attacked(board, king_sq, them)
    } else {
        false
    };
    let extension = if in_check && ply < MAX_PLY - 2 { 1 } else { 0 };

    // 2. Transposition Table Lookup
    let alpha_orig = alpha;
    let tt_entry = state.tt.get(board.hash);
    if let Some(entry) = tt_entry {
        if entry.depth >= depth {
            let mut val = entry.val;
            if val > MATE_SCORE - 100 {
                val -= ply as i32;
            } else if val < -MATE_SCORE + 100 {
                val += ply as i32;
            }

            match entry.flag {
                EXACT => return val,
                ALPHA if val <= alpha => return val,
                BETA if val >= beta => return val,
                _ => {}
            }
        }
    }

    // 2.5 Static Null Move Pruning / Reverse Futility Pruning (RFP)
    if depth <= 3 && !in_check && ply > 0 {
        let margin = if improving { depth * 80 } else { depth * 120 };
        if static_eval - margin >= beta {
            return beta; // RFP returns beta for clean cutoff
        }
    }

    // 3. Null Move Pruning (NMP)
    if depth >= 3 && !in_check && static_eval >= beta && has_non_pawns(board, us) && ply > 0 {
        board.make_null_move();
        let reduction = 3 + depth / 6;
        let val = -negamax(board, depth - 1 - reduction, -beta, -beta + 1, ply + 1, state);
        board.unmake_null_move();

        if val >= beta {
            return beta;
        }
    }

    // 4. Quiescence Search at depth 0
    if depth <= 0 {
        return quiescence(board, alpha, beta, ply as i32, state);
    }

    // 5. Move generation
    let mut moves = generate_legal_moves(board);

    if moves.is_empty() {
        if in_check {
            return -MATE_SCORE + ply as i32;
        }
        return 0;
    }

    let tt_best_move = tt_entry.and_then(|e| e.best_move);
    let (killer_1, killer_2) = if ply < MAX_PLY {
        (state.killer_moves[ply][0], state.killer_moves[ply][1])
    } else {
        (None, None)
    };
    order_moves(board, &mut moves, tt_best_move, killer_1, killer_2, &state.history_table);

    let mut best_val = -INF;
    let mut best_move = None;
    let mut moves_searched = 0;
    let mut quiet_moves_searched = 0;

    // Futility Pruning check (reusing static_eval from start of node)
    let mut futility_pruning = false;
    if depth <= 2 && !in_check && ply > 0 {
        if static_eval + depth * 150 < alpha {
            futility_pruning = true;
        }
    }

    for &m in &moves {
        let is_quiet = !m.is_capture && m.promotion.is_none();
        if is_quiet {
            quiet_moves_searched += 1;
            // Late Move Pruning (LMP)
            if depth <= 3 && !in_check && ply > 0 {
                let limit = 3 + depth * depth;
                if quiet_moves_searched > limit {
                    continue;
                }
            }
        }

        if futility_pruning && is_quiet {
            continue;
        }

        board.make_move(m);
        moves_searched += 1;

        // Check if move gives check (opponent is in check now)
        let opp = board.side_to_move;
        let opp_king_p = if opp == WHITE { WHITE_KING } else { BLACK_KING };
        let opp_king_bb = board.pieces[opp_king_p];
        let is_check = if opp_king_bb != 0 {
            let opp_king_sq = bit_scan_forward(opp_king_bb) as u8;
            is_square_attacked(board, opp_king_sq, 1 - opp)
        } else {
            false
        };

        let mut val;
        if moves_searched == 1 {
            val = -negamax(board, depth - 1 + extension, -beta, -alpha, ply + 1, state);
        } else {
            let mut lmr = false;
            if depth >= 3 && moves_searched >= 4 && !m.is_capture && m.promotion.is_none() && !in_check && !is_check {
                let d_idx = (depth as usize).min(63);
                let m_idx = moves_searched.min(63);
                let base_reduction = state.lmr_table[d_idx][m_idx];

                // History adjustment
                let hist = state.history_table[m.from_sq as usize][m.to_sq as usize];
                let hist_adjustment = hist / 10000;

                // Improving adjustment
                let improving_adjustment = if improving { 1 } else { 0 };

                let reduction = base_reduction - hist_adjustment - improving_adjustment;
                if reduction >= 1 {
                    lmr = true;
                    let reduced_depth = (depth - 1 - reduction).max(1);
                    val = -negamax(board, reduced_depth, -alpha - 1, -alpha, ply + 1, state);
                } else {
                    val = -INF;
                }
            } else {
                val = -INF; // dummy to trigger PVS search below
            }

            let mut pvs_val = val;
            if !lmr || pvs_val > alpha {
                pvs_val = -negamax(board, depth - 1 + extension, -alpha - 1, -alpha, ply + 1, state);
            }

            if pvs_val > alpha && pvs_val < beta {
                val = -negamax(board, depth - 1 + extension, -beta, -alpha, ply + 1, state);
            } else {
                val = pvs_val;
            }
        }

        board.unmake_move(m);

        if state.time_up() {
            return 0;
        }

        if val > best_val {
            best_val = val;
            best_move = Some(m);
        }

        if val > alpha {
            alpha = val;
        }

        if alpha >= beta {
            // Beta cutoff
            if !m.is_capture && ply < MAX_PLY {
                if state.killer_moves[ply][0] != Some(m) {
                    state.killer_moves[ply][1] = state.killer_moves[ply][0];
                    state.killer_moves[ply][0] = Some(m);
                }
                state.history_table[m.from_sq as usize][m.to_sq as usize] += depth * depth;

                // Penalize previously searched quiet moves
                for &prev_m in &moves {
                    if prev_m.from_sq == m.from_sq && prev_m.to_sq == m.to_sq && prev_m.promotion == m.promotion {
                        break;
                    }
                    if !prev_m.is_capture && prev_m.promotion.is_none() {
                        state.history_table[prev_m.from_sq as usize][prev_m.to_sq as usize] -= depth;
                    }
                }
            }
            break;
        }
    }

    // 6. Store in Transposition Table (if search was not aborted)
    if !state.stop_search.load(Ordering::Relaxed) {
        let tt_flag = if best_val <= alpha_orig {
            ALPHA
        } else if best_val >= beta {
            BETA
        } else {
            EXACT
        };

        let mut tt_val = best_val;
        if best_val > MATE_SCORE - 100 {
            tt_val += ply as i32;
        } else if best_val < -MATE_SCORE + 100 {
            tt_val -= ply as i32;
        }

        state.tt.store(board.hash, depth, tt_flag, tt_val, best_move);
    }

    best_val
}

pub fn get_pv_line(board: &mut Board, depth: i32, tt: &TranspositionTable) -> Vec<Move> {
    let mut pv = Vec::new();
    let mut temp = board.clone();
    for _ in 0..depth {
        if let Some(entry) = tt.get(temp.hash) {
            if let Some(m) = entry.best_move {
                pv.push(m);
                temp.make_move(m);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    pv
}

pub fn search(
    board: &mut Board,
    max_depth: i32,
    search_time: f64,
    tt: &TranspositionTable,
    history_table: &mut [[i32; 64]; 64],
    rx: &ThreadSafeReceiver,
    num_threads: usize,
) -> Option<Move> {
    // Decay history table by dividing by 2 to prevent overflow and age old moves
    for r in 0..64 {
        for c in 0..64 {
            history_table[r][c] /= 2;
        }
    }

    let mut lmr_table = [[0; 64]; 64];
    for d in 1..64 {
        for m in 1..64 {
            lmr_table[d][m] = ((d as f64).ln() * (m as f64).ln() / 2.0) as i32;
        }
    }

    let stop_search = Arc::new(AtomicBool::new(false));
    let mut best_move = None;

    std::thread::scope(|s| {
        // Spawn helper threads
        for thread_id in 1..num_threads {
            let mut local_board = board.clone();
            let mut local_history = history_table.clone();
            let stop_search_clone = Arc::clone(&stop_search);
            s.spawn(move || {
                let mut local_state = SearchState {
                    start_time: Instant::now(),
                    time_limit: search_time,
                    stop_search: stop_search_clone,
                    is_main: false,
                    nodes_searched: 0,
                    killer_moves: [[None; 2]; MAX_PLY],
                    history_table: &mut local_history,
                    lmr_table,
                    eval_history: [0; MAX_PLY],
                    tt,
                    rx,
                };
                for depth in 1..=max_depth {
                    if local_state.stop_search.load(Ordering::Relaxed) {
                        break;
                    }
                    let d = depth + (thread_id as i32 % 2);
                    negamax(&mut local_board, d, -INF, INF, 0, &mut local_state);
                }
            });
        }

        // Main thread search
        let mut state = SearchState {
            start_time: Instant::now(),
            time_limit: search_time,
            stop_search: Arc::clone(&stop_search),
            is_main: true,
            nodes_searched: 0,
            killer_moves: [[None; 2]; MAX_PLY],
            history_table,
            lmr_table,
            eval_history: [0; MAX_PLY],
            tt,
            rx,
        };

        let mut previous_score = 0;

        for depth in 1..=max_depth {
            let mut score;
            if depth >= 5 {
                let mut margin = 35;
                let mut alpha = previous_score - margin;
                let mut beta = previous_score + margin;

                loop {
                    score = negamax(board, depth, alpha, beta, 0, &mut state);
                    if state.stop_search.load(Ordering::Relaxed) {
                        break;
                    }

                    if score <= alpha {
                        alpha = (alpha - margin).max(-INF);
                        margin *= 2;
                    } else if score >= beta {
                        beta = (beta + margin).min(INF);
                        margin *= 2;
                    } else {
                        break;
                    }
                }
            } else {
                score = negamax(board, depth, -INF, INF, 0, &mut state);
            }

            if state.stop_search.load(Ordering::Relaxed) {
                break;
            }

            previous_score = score;

            let elapsed = state.start_time.elapsed().as_secs_f64();
            let nps = if elapsed > 0.0 { state.nodes_searched as f64 / elapsed } else { 0.0 };

            let pv = get_pv_line(board, depth, &state.tt);
            let pv_str = pv.iter().map(|m| m.to_uci()).collect::<Vec<String>>().join(" ");

            let score_str = if score > MATE_SCORE - 100 {
                let mate_in_ply = MATE_SCORE - score;
                let mate_in_moves = (mate_in_ply + 1) / 2;
                format!("mate {}", mate_in_moves)
            } else if score < -MATE_SCORE + 100 {
                let mate_in_ply = MATE_SCORE + score;
                let mate_in_moves = (mate_in_ply + 1) / 2;
                format!("mate -{}", mate_in_moves)
            } else {
                format!("cp {}", score)
            };

            println!(
                "info depth {} score {} time {} nodes {} nps {} pv {}",
                depth, score_str, (elapsed * 1000.0) as u64, state.nodes_searched, nps as u64, pv_str
            );

            if !pv.is_empty() {
                best_move = Some(pv[0]);
            }
        }
    });

    // Make sure we stop helper threads when main thread returns
    stop_search.store(true, Ordering::Relaxed);

    if best_move.is_none() {
        let moves = generate_legal_moves(board);
        if !moves.is_empty() {
            best_move = Some(moves[0]);
        }
    }

    best_move
}
