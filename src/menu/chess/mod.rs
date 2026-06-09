// Módulo do jogo de xadrez em WGPU

mod input;
mod moves;
mod piece;
mod render;

pub use piece::{Piece, PieceColor};

#[allow(unused_imports)]
use piece::PieceType::*;

pub struct ChessGame {
    pub board: [[Option<Piece>; 8]; 8],
    pub selected_square: Option<(usize, usize)>,
    pub cursor: Option<(usize, usize)>, // Guarda onde a seleção do teclado está apontando (linha, coluna)
    pub current_turn: PieceColor,
    pub valid_moves: Vec<(usize, usize)>, // Lista de destinos possíveis para a peça selecionada
    pub player_color: PieceColor,
}

impl ChessGame {
    pub fn new() -> Self {
        let b = PieceColor::Black;
        let w = PieceColor::White;
        let p = |piece_type, color| Some(Piece { piece_type, color });

        // PEGA O TEMPO DO SISTEMA
        let tempo_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        // EMBARALHAR OS BITS DO TEMPO (Bit Mixing) ===
        // Isso resolve o problema de o relógio do Windows terminar sempre em número par
        let mut x = tempo_nanos;
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d049bb133111eb);
        x ^= x >> 31;

        let player_color = if x % 2 == 0 {
            PieceColor::White
        } else {
            PieceColor::Black
        };
        Self {
            board: [
                // Linha 0: Peças Pretas de trás
                [
                    p(Rook, b),
                    p(Knight, b),
                    p(Bishop, b),
                    p(Queen, b),
                    p(King, b),
                    p(Bishop, b),
                    p(Knight, b),
                    p(Rook, b),
                ],
                // Linha 1: Peões Pretos
                [p(Pawn, b); 8],
                [None; 8],
                [None; 8],
                [None; 8],
                [None; 8],
                // Linha 6: Peões Brancos
                [p(Pawn, w); 8],
                // Linha 7: Peças Brancas de trás
                [
                    p(Rook, w),
                    p(Knight, w),
                    p(Bishop, w),
                    p(Queen, w),
                    p(King, w),
                    p(Bishop, w),
                    p(Knight, w),
                    p(Rook, w),
                ],
            ],
            selected_square: None,
            cursor: Some((7, 4)), // Inicia com o cursor apontando para o Rei Branco
            current_turn: PieceColor::White,
            valid_moves: Vec::new(),
            player_color,
        }
    }

    pub fn make_ai_move(&mut self) {
        // Lista para guardar todos os movimentos possíveis da IA
        // Formato: ((linha_origem, coluna_origem), (linha_destino, coluna_destino))
        let mut all_moves = Vec::new();

        // Procura todas as peças da IA no tabuleiro
        for r in 0..8 {
            for c in 0..8 {
                if let Some(piece) = self.board[r][c] {
                    if piece.color == self.current_turn {
                        // Calcula temporariamente os movimentos válidos desta peça
                        self.calculate_valid_moves(r, c);
                        for &(tr, tc) in &self.valid_moves {
                            all_moves.push(((r, c), (tr, tc)));
                        }
                    }
                }
            }
        }

        // Limpa os círculos de validação da tela para não confundir o jogador
        self.valid_moves.clear();

        // Se a IA tiver pelo menos um movimento válido disponível
        if !all_moves.is_empty() {
            // Sorteia um movimento usando o relógio do sistema
            let tempo_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let idx = (tempo_nanos % all_moves.len() as u128) as usize;

            let ((from_row, from_col), (to_row, to_col)) = all_moves[idx];

            // Realiza o movimento sorteado no tabuleiro
            let piece = self.board[from_row][from_col];
            self.board[to_row][to_col] = piece;
            self.board[from_row][from_col] = None;
        }

        // Passa o turno de volta para o jogador
        self.current_turn = match self.current_turn {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        };
    }
}
