// Módulo do jogo de xadrez em WGPU

mod input;
mod moves;
mod piece;
mod render;

pub use piece::{Piece, PieceColor, PieceType};

#[allow(unused_imports)]
use piece::PieceType::*;

pub struct ChessGame {
    pub board: [[Option<Piece>; 8]; 8],
    pub selected_square: Option<(usize, usize)>,
    pub cursor: Option<(usize, usize)>, // Guarda onde a seleção do teclado está apontando (linha, coluna)
    pub current_turn: PieceColor,
    // ADICIONE ESTES DOIS CAMPOS:
    pub selected_piece: Option<(usize, usize)>, // (linha, coluna) da peça clicada
    pub valid_moves: Vec<(usize, usize)>, // Lista de destinos possíveis para a peça selecionada
}

impl ChessGame {
    pub fn new() -> Self {
        let b = PieceColor::Black;
        let w = PieceColor::White;
        let p = |piece_type, color| Some(Piece { piece_type, color });

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
            selected_piece: None,
            valid_moves: Vec::new(),
        }
    }
}
