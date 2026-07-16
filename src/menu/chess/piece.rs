// Tipos de peças e cores do xadrez

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceColor {
    White,
    Black,
}

#[derive(Clone, Copy)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: PieceColor,
}

/// Retorna o índice de coluna da spritesheet para um dado tipo de peça
pub fn get_piece_column_index(piece_type: PieceType) -> usize {
    match piece_type {
        PieceType::King => 0,
        PieceType::Queen => 1,
        PieceType::Bishop => 2,
        PieceType::Knight => 3,
        PieceType::Rook => 4,
        PieceType::Pawn => 5,
    }
}
