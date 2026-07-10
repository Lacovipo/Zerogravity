// src/board.rs
// Core board representation and state definitions for ZeroGravity

#![allow(dead_code)]
use std::sync::OnceLock;

// Colors
pub const WHITE: usize = 0;
pub const BLACK: usize = 1;

// Piece Types
pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;

// Piece identifiers (0 to 11)
pub const WHITE_PAWN: usize = 0;
pub const WHITE_KNIGHT: usize = 1;
pub const WHITE_BISHOP: usize = 2;
pub const WHITE_ROOK: usize = 3;
pub const WHITE_QUEEN: usize = 4;
pub const WHITE_KING: usize = 5;
pub const BLACK_PAWN: usize = 6;
pub const BLACK_KNIGHT: usize = 7;
pub const BLACK_BISHOP: usize = 8;
pub const BLACK_ROOK: usize = 9;
pub const BLACK_QUEEN: usize = 10;
pub const BLACK_KING: usize = 11;

// Castling Rights (4-bit mask)
pub const CASTLE_WHITE_OO: u8 = 1;   // King-side (K)
pub const CASTLE_WHITE_OOO: u8 = 2;  // Queen-side (Q)
pub const CASTLE_BLACK_OO: u8 = 4;   // King-side (k)
pub const CASTLE_BLACK_OOO: u8 = 8;  // Queen-side (q)

// Castling update mask: for each square, which castling rights remain.
pub static CASTLE_MASK: [u8; 64] = {
    let mut masks = [15; 64];
    masks[4] = 12;   // E1 clears WHITE_OO (1) & WHITE_OOO (2) -> 15 - 3 = 12
    masks[0] = 13;   // A1 clears WHITE_OOO (2) -> 15 - 2 = 13
    masks[7] = 14;   // H1 clears WHITE_OO (1) -> 15 - 1 = 14
    masks[60] = 3;   // E8 clears BLACK_OO (4) & BLACK_OOO (8) -> 15 - 12 = 3
    masks[56] = 7;   // A8 clears BLACK_OOO (8) -> 15 - 8 = 7
    masks[63] = 11;  // H8 clears BLACK_OO (4) -> 15 - 4 = 11
    masks
};

// Zobrist Keys
pub struct ZobristKeys {
    pub pieces: [[u64; 64]; 12],
    pub side: u64,
    pub castling: [u64; 16],
    pub ep: [u64; 8],
}

pub static ZOBRIST: OnceLock<ZobristKeys> = OnceLock::new();

pub fn get_zobrist() -> &'static ZobristKeys {
    ZOBRIST.get_or_init(|| {
        let mut state = 1337_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut pieces = [[0; 64]; 12];
        for p in 0..12 {
            for sq in 0..64 {
                pieces[p][sq] = next();
            }
        }
        let side = next();
        let mut castling = [0; 16];
        for i in 0..16 {
            castling[i] = next();
        }
        let mut ep = [0; 8];
        for i in 0..8 {
            ep[i] = next();
        }
        ZobristKeys { pieces, side, castling, ep }
    })
}

// Bitboard helpers
#[inline(always)]
pub fn set_bit(bb: u64, sq: u8) -> u64 {
    bb | (1 << sq)
}

#[inline(always)]
pub fn clear_bit(bb: u64, sq: u8) -> u64 {
    bb & !(1 << sq)
}

#[inline(always)]
pub fn get_bit(bb: u64, sq: u8) -> bool {
    ((bb >> sq) & 1) != 0
}

#[inline(always)]
pub fn bit_scan_forward(bb: u64) -> i32 {
    if bb == 0 {
        -1
    } else {
        bb.trailing_zeros() as i32
    }
}

pub fn parse_square(sq_str: &str) -> Option<u8> {
    if sq_str.len() != 2 {
        return None;
    }
    let chars: Vec<char> = sq_str.chars().collect();
    let file_char = chars[0];
    let rank_char = chars[1];
    if !('a'..='h').contains(&file_char) || !('1'..='8').contains(&rank_char) {
        return None;
    }
    let file_idx = (file_char as u8) - b'a';
    let rank_idx = (rank_char as u8) - b'1';
    Some(rank_idx * 8 + file_idx)
}

pub fn square_to_str(sq: Option<u8>) -> String {
    match sq {
        Some(s) if s <= 63 => {
            let file_idx = s % 8;
            let rank_idx = s / 8;
            let file_char = (b'a' + file_idx) as char;
            let rank_char = (b'1' + rank_idx) as char;
            format!("{}{}", file_char, rank_char)
        }
        _ => "-".to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from_sq: u8,
    pub to_sq: u8,
    pub promotion: Option<u8>, // None, KNIGHT (1), BISHOP (2), ROOK (3), QUEEN (4)
    pub is_capture: bool,
    pub is_en_passant: bool,
    pub is_double_push: bool,
    pub is_castling: bool,
}

impl Move {
    pub fn to_uci(&self) -> String {
        let p_char = match self.promotion {
            Some(1) => "n",
            Some(2) => "b",
            Some(3) => "r",
            Some(4) => "q",
            _ => "",
        };
        format!("{}{}{}", square_to_str(Some(self.from_sq)), square_to_str(Some(self.to_sq)), p_char)
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HistoryEntry {
    pub castling_rights: u8,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u32,
    pub captured_piece: Option<usize>,
    pub moving_piece: usize,
    pub hash: u64,
    pub last_move: Option<Move>,
}

#[derive(Clone)]
pub struct Board {
    pub pieces: [u64; 12],
    pub side_to_move: usize,
    pub castling_rights: u8,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
    pub hash: u64,
    pub history: Vec<HistoryEntry>,
}

impl Board {
    pub fn new() -> Self {
        let mut b = Board {
            pieces: [0; 12],
            side_to_move: WHITE,
            castling_rights: 0,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            hash: 0,
            history: Vec::with_capacity(256),
        };
        b.load_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        b
    }

    pub fn is_insufficient_material(&self) -> bool {
        // If there are pawns, rooks, or queens, it is not insufficient material
        if (self.pieces[WHITE_PAWN] | self.pieces[BLACK_PAWN] |
            self.pieces[WHITE_ROOK] | self.pieces[BLACK_ROOK] |
            self.pieces[WHITE_QUEEN] | self.pieces[BLACK_QUEEN]) != 0 {
            return false;
        }

        let w_knights = self.pieces[WHITE_KNIGHT].count_ones();
        let b_knights = self.pieces[BLACK_KNIGHT].count_ones();
        let w_bishops = self.pieces[WHITE_BISHOP].count_ones();
        let b_bishops = self.pieces[BLACK_BISHOP].count_ones();

        // 1. King vs King
        if w_knights == 0 && b_knights == 0 && w_bishops == 0 && b_bishops == 0 {
            return true;
        }

        // 2. King + Knight vs King
        if (w_knights == 1 && b_knights == 0 && w_bishops == 0 && b_bishops == 0) ||
           (w_knights == 0 && b_knights == 1 && w_bishops == 0 && b_bishops == 0) {
            return true;
        }

        // 3. King + Bishop vs King
        if (w_knights == 0 && b_knights == 0 && w_bishops == 1 && b_bishops == 0) ||
           (w_knights == 0 && b_knights == 0 && w_bishops == 0 && b_bishops == 1) {
            return true;
        }

        // 4. King + Bishop vs King + Bishop (same color squares)
        if w_knights == 0 && b_knights == 0 && w_bishops == 1 && b_bishops == 1 {
            let w_sq = self.pieces[WHITE_BISHOP].trailing_zeros() as u8;
            let b_sq = self.pieces[BLACK_BISHOP].trailing_zeros() as u8;
            let w_color = (w_sq + (w_sq / 8)) % 2;
            let b_color = (b_sq + (b_sq / 8)) % 2;
            if w_color == b_color {
                return true;
            }
        }

        false
    }

    #[inline(always)]
    pub fn occupied_white(&self) -> u64 {
        self.pieces[WHITE_PAWN] | self.pieces[WHITE_KNIGHT] |
        self.pieces[WHITE_BISHOP] | self.pieces[WHITE_ROOK] |
        self.pieces[WHITE_QUEEN] | self.pieces[WHITE_KING]
    }

    #[inline(always)]
    pub fn occupied_black(&self) -> u64 {
        self.pieces[BLACK_PAWN] | self.pieces[BLACK_KNIGHT] |
        self.pieces[BLACK_BISHOP] | self.pieces[BLACK_ROOK] |
        self.pieces[BLACK_QUEEN] | self.pieces[BLACK_KING]
    }

    #[inline(always)]
    pub fn occupied(&self) -> u64 {
        self.occupied_white() | self.occupied_black()
    }

    pub fn compute_hash(&self) -> u64 {
        let zobrist = get_zobrist();
        let mut h = 0_u64;
        for p in 0..12 {
            let mut bb = self.pieces[p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                h ^= zobrist.pieces[p][sq];
                bb &= bb - 1;
            }
        }
        if self.side_to_move == BLACK {
            h ^= zobrist.side;
        }
        h ^= zobrist.castling[self.castling_rights as usize];
        if let Some(ep) = self.en_passant {
            h ^= zobrist.ep[(ep % 8) as usize];
        }
        h
    }

    pub fn clear(&mut self) {
        self.pieces = [0; 12];
        self.side_to_move = WHITE;
        self.castling_rights = 0;
        self.en_passant = None;
        self.halfmove_clock = 0;
        self.fullmove_number = 1;
        self.history.clear();
        self.hash = 0;
    }

    pub fn get_piece_at(&self, sq: u8) -> Option<usize> {
        let mask = 1_u64 << sq;
        for p in 0..12 {
            if (self.pieces[p] & mask) != 0 {
                return Some(p);
            }
        }
        None
    }

    pub fn load_fen(&mut self, fen: &str) {
        self.clear();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        // 1. Piece Placement
        let rows: Vec<&str> = parts[0].split('/').collect();
        for (rank_idx, row) in rows.iter().enumerate() {
            let actual_rank = 7 - rank_idx as u8;
            let mut file_idx = 0_u8;
            for char in row.chars() {
                if let Some(digit) = char.to_digit(10) {
                    file_idx += digit as u8;
                } else {
                    let piece = match char {
                        'P' => WHITE_PAWN, 'N' => WHITE_KNIGHT, 'B' => WHITE_BISHOP, 'R' => WHITE_ROOK, 'Q' => WHITE_QUEEN, 'K' => WHITE_KING,
                        'p' => BLACK_PAWN, 'n' => BLACK_KNIGHT, 'b' => BLACK_BISHOP, 'r' => BLACK_ROOK, 'q' => BLACK_QUEEN, 'k' => BLACK_KING,
                        _ => 99,
                    };
                    if piece != 99 {
                        let sq = actual_rank * 8 + file_idx;
                        self.pieces[piece] = set_bit(self.pieces[piece], sq);
                    }
                    file_idx += 1;
                }
            }
        }

        // 2. Side to move
        if parts.len() > 1 {
            self.side_to_move = if parts[1] == "w" { WHITE } else { BLACK };
        }

        // 3. Castling rights
        if parts.len() > 2 {
            let c_str = parts[2];
            if c_str != "-" {
                if c_str.contains('K') { self.castling_rights |= CASTLE_WHITE_OO; }
                if c_str.contains('Q') { self.castling_rights |= CASTLE_WHITE_OOO; }
                if c_str.contains('k') { self.castling_rights |= CASTLE_BLACK_OO; }
                if c_str.contains('q') { self.castling_rights |= CASTLE_BLACK_OOO; }
            }
        }

        // 4. En passant
        if parts.len() > 3 {
            let ep_str = parts[3];
            if ep_str != "-" {
                self.en_passant = parse_square(ep_str);
            }
        }

        // 5. Halfmove clock
        if parts.len() > 4 {
            if let Ok(hm) = parts[4].parse::<u32>() {
                self.halfmove_clock = hm;
            }
        }

        // 6. Fullmove number
        if parts.len() > 5 {
            if let Ok(fm) = parts[5].parse::<u32>() {
                self.fullmove_number = fm;
            }
        }

        self.hash = self.compute_hash();
    }

    pub fn to_fen(&self) -> String {
        let mut fen_parts = Vec::new();

        // 1. Piece Placement
        let mut rows = Vec::new();
        for rank in (0..=7).rev() {
            let mut row_str = String::new();
            let mut empty_count = 0;
            for file in 0..8 {
                let sq = rank * 8 + file;
                let piece = self.get_piece_at(sq);
                match piece {
                    None => empty_count += 1,
                    Some(p) => {
                        if empty_count > 0 {
                            row_str.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        let c = match p {
                            WHITE_PAWN => 'P', WHITE_KNIGHT => 'N', WHITE_BISHOP => 'B', WHITE_ROOK => 'R', WHITE_QUEEN => 'Q', WHITE_KING => 'K',
                            BLACK_PAWN => 'p', BLACK_KNIGHT => 'n', BLACK_BISHOP => 'b', BLACK_ROOK => 'r', BLACK_QUEEN => 'q', BLACK_KING => 'k',
                            _ => '.',
                        };
                        row_str.push(c);
                    }
                }
            }
            if empty_count > 0 {
                row_str.push_str(&empty_count.to_string());
            }
            rows.push(row_str);
        }
        fen_parts.push(rows.join("/"));

        // 2. Side to move
        fen_parts.push(if self.side_to_move == WHITE { "w" } else { "b" }.to_string());

        // 3. Castling rights
        let mut c_str = String::new();
        if (self.castling_rights & CASTLE_WHITE_OO) != 0 { c_str.push('K'); }
        if (self.castling_rights & CASTLE_WHITE_OOO) != 0 { c_str.push('Q'); }
        if (self.castling_rights & CASTLE_BLACK_OO) != 0 { c_str.push('k'); }
        if (self.castling_rights & CASTLE_BLACK_OOO) != 0 { c_str.push('q'); }
        fen_parts.push(if c_str.is_empty() { "-".to_string() } else { c_str });

        // 4. En passant
        fen_parts.push(square_to_str(self.en_passant));

        // 5. Halfmove clock
        fen_parts.push(self.halfmove_clock.to_string());

        // 6. Fullmove number
        fen_parts.push(self.fullmove_number.to_string());

        fen_parts.join(" ")
    }

    pub fn make_move(&mut self, m: Move) {
        let moving_piece = self.get_piece_at(m.from_sq).unwrap();
        let us = self.side_to_move;
        let them = 1 - us;

        let captured_piece = if m.is_capture {
            if m.is_en_passant {
                Some(if us == WHITE { BLACK_PAWN } else { WHITE_PAWN })
            } else {
                self.get_piece_at(m.to_sq)
            }
        } else {
            None
        };

        // Push history
        self.history.push(HistoryEntry {
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            captured_piece,
            moving_piece,
            hash: self.hash,
            last_move: Some(m),
        });

        let zobrist = get_zobrist();

        // --- Incremental Hash Update ---
        // 1. XOR out moving piece from from_sq
        self.hash ^= zobrist.pieces[moving_piece][m.from_sq as usize];

        // 2. XOR out captured piece if any
        if m.is_capture {
            let cap_piece = captured_piece.unwrap();
            let cap_sq = if m.is_en_passant {
                if us == WHITE { m.to_sq - 8 } else { m.to_sq + 8 }
            } else {
                m.to_sq
            };
            self.hash ^= zobrist.pieces[cap_piece][cap_sq as usize];
        }

        // 3. XOR in moving/promoted piece to to_sq
        if let Some(promo) = m.promotion {
            let promoted_piece = us * 6 + promo as usize;
            self.hash ^= zobrist.pieces[promoted_piece][m.to_sq as usize];
        } else {
            self.hash ^= zobrist.pieces[moving_piece][m.to_sq as usize];
        }

        // 4. Handle rook in castling
        if m.is_castling {
            match m.to_sq {
                6 => { // White OO (G1): White Rook from 7 to 5
                    self.hash ^= zobrist.pieces[WHITE_ROOK][7] ^ zobrist.pieces[WHITE_ROOK][5];
                }
                2 => { // White OOO (C1): White Rook from 0 to 3
                    self.hash ^= zobrist.pieces[WHITE_ROOK][0] ^ zobrist.pieces[WHITE_ROOK][3];
                }
                62 => { // Black OO (G8): Black Rook from 63 to 61
                    self.hash ^= zobrist.pieces[BLACK_ROOK][63] ^ zobrist.pieces[BLACK_ROOK][61];
                }
                58 => { // Black OOO (C8): Black Rook from 56 to 59
                    self.hash ^= zobrist.pieces[BLACK_ROOK][56] ^ zobrist.pieces[BLACK_ROOK][59];
                }
                _ => {}
            }
        }

        // 5. XOR out old castling rights and XOR in new castling rights
        let new_castling = self.castling_rights & CASTLE_MASK[m.from_sq as usize] & CASTLE_MASK[m.to_sq as usize];
        self.hash ^= zobrist.castling[self.castling_rights as usize] ^ zobrist.castling[new_castling as usize];

        // 6. XOR out old en passant file
        if let Some(ep) = self.en_passant {
            self.hash ^= zobrist.ep[(ep % 8) as usize];
        }

        // 7. XOR in new en passant file
        let new_ep = if m.is_double_push {
            Some(if us == WHITE { m.from_sq + 8 } else { m.from_sq - 8 })
        } else {
            None
        };
        if let Some(ep) = new_ep {
            self.hash ^= zobrist.ep[(ep % 8) as usize];
        }

        // 8. XOR side to move
        self.hash ^= zobrist.side;
        // -------------------------------

        // Clear moving piece from its original square
        self.pieces[moving_piece] = clear_bit(self.pieces[moving_piece], m.from_sq);

        // Remove captured piece
        if m.is_capture {
            let cap_piece = captured_piece.unwrap();
            if m.is_en_passant {
                let cap_sq = if us == WHITE { m.to_sq - 8 } else { m.to_sq + 8 };
                self.pieces[cap_piece] = clear_bit(self.pieces[cap_piece], cap_sq);
            } else {
                self.pieces[cap_piece] = clear_bit(self.pieces[cap_piece], m.to_sq);
            }
        }

        // Place piece on target square
        if let Some(promo) = m.promotion {
            let promoted_piece = us * 6 + promo as usize;
            self.pieces[promoted_piece] = set_bit(self.pieces[promoted_piece], m.to_sq);
        } else if m.is_castling {
            self.pieces[moving_piece] = set_bit(self.pieces[moving_piece], m.to_sq);
            // Move rook
            match m.to_sq {
                6 => { // White OO (G1)
                    self.pieces[WHITE_ROOK] = clear_bit(self.pieces[WHITE_ROOK], 7);
                    self.pieces[WHITE_ROOK] = set_bit(self.pieces[WHITE_ROOK], 5);
                }
                2 => { // White OOO (C1)
                    self.pieces[WHITE_ROOK] = clear_bit(self.pieces[WHITE_ROOK], 0);
                    self.pieces[WHITE_ROOK] = set_bit(self.pieces[WHITE_ROOK], 3);
                }
                62 => { // Black OO (G8)
                    self.pieces[BLACK_ROOK] = clear_bit(self.pieces[BLACK_ROOK], 63);
                    self.pieces[BLACK_ROOK] = set_bit(self.pieces[BLACK_ROOK], 61);
                }
                58 => { // Black OOO (C8)
                    self.pieces[BLACK_ROOK] = clear_bit(self.pieces[BLACK_ROOK], 56);
                    self.pieces[BLACK_ROOK] = set_bit(self.pieces[BLACK_ROOK], 59);
                }
                _ => {}
            }
        } else {
            self.pieces[moving_piece] = set_bit(self.pieces[moving_piece], m.to_sq);
        }

        // Update castling rights
        self.castling_rights = new_castling;

        // Update en passant
        self.en_passant = new_ep;

        // Update clocks
        if (moving_piece % 6 == PAWN) || m.is_capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if us == BLACK {
            self.fullmove_number += 1;
        }

        self.side_to_move = them;

        #[cfg(debug_assertions)]
        {
            let computed = self.compute_hash();
            debug_assert_eq!(self.hash, computed, "Hash mismatch in make_move for move {}: incremental = {:X}, computed = {:X}", m.to_uci(), self.hash, computed);
        }
    }

    pub fn unmake_move(&mut self, m: Move) {
        let us = 1 - self.side_to_move;
        self.side_to_move = us;

        let hist = self.history.pop().unwrap();
        self.castling_rights = hist.castling_rights;
        self.en_passant = hist.en_passant;
        self.halfmove_clock = hist.halfmove_clock;

        if us == BLACK {
            self.fullmove_number -= 1;
        }

        // Clear moved piece from to_sq
        let moved_piece = if let Some(promo) = m.promotion {
            us * 6 + promo as usize
        } else {
            hist.moving_piece
        };
        self.pieces[moved_piece] = clear_bit(self.pieces[moved_piece], m.to_sq);

        // Restore moving piece to from_sq
        self.pieces[hist.moving_piece] = set_bit(self.pieces[hist.moving_piece], m.from_sq);

        // Undo rook castling move
        if m.is_castling {
            match m.to_sq {
                6 => {
                    self.pieces[WHITE_ROOK] = clear_bit(self.pieces[WHITE_ROOK], 5);
                    self.pieces[WHITE_ROOK] = set_bit(self.pieces[WHITE_ROOK], 7);
                }
                2 => {
                    self.pieces[WHITE_ROOK] = clear_bit(self.pieces[WHITE_ROOK], 3);
                    self.pieces[WHITE_ROOK] = set_bit(self.pieces[WHITE_ROOK], 0);
                }
                62 => {
                    self.pieces[BLACK_ROOK] = clear_bit(self.pieces[BLACK_ROOK], 61);
                    self.pieces[BLACK_ROOK] = set_bit(self.pieces[BLACK_ROOK], 63);
                }
                58 => {
                    self.pieces[BLACK_ROOK] = clear_bit(self.pieces[BLACK_ROOK], 59);
                    self.pieces[BLACK_ROOK] = set_bit(self.pieces[BLACK_ROOK], 56);
                }
                _ => {}
            }
        }

        // Restore captured piece
        if m.is_capture {
            let cap_piece = hist.captured_piece.unwrap();
            if m.is_en_passant {
                let cap_sq = if us == WHITE { m.to_sq - 8 } else { m.to_sq + 8 };
                self.pieces[cap_piece] = set_bit(self.pieces[cap_piece], cap_sq);
            } else {
                self.pieces[cap_piece] = set_bit(self.pieces[cap_piece], m.to_sq);
            }
        }

        self.hash = hist.hash;
    }

    pub fn make_null_move(&mut self) {
        self.history.push(HistoryEntry {
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            captured_piece: None,
            moving_piece: 99, // dummy
            hash: self.hash,
            last_move: None,
        });

        let zobrist = get_zobrist();

        if let Some(ep) = self.en_passant {
            self.hash ^= zobrist.ep[(ep % 8) as usize];
        }

        self.hash ^= zobrist.side;

        self.en_passant = None;
        self.halfmove_clock += 1;
        self.side_to_move = 1 - self.side_to_move;

        #[cfg(debug_assertions)]
        {
            let computed = self.compute_hash();
            debug_assert_eq!(self.hash, computed, "Hash mismatch in make_null_move");
        }
    }

    pub fn unmake_null_move(&mut self) {
        self.side_to_move = 1 - self.side_to_move;

        let hist = self.history.pop().unwrap();
        self.castling_rights = hist.castling_rights;
        self.en_passant = hist.en_passant;
        self.halfmove_clock = hist.halfmove_clock;
        self.hash = hist.hash;
    }
}

