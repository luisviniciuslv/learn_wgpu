// Módulo do jogo de xadrez em WGPU
use crate::menu::renderer::Renderer;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceColor {
    White,
    Black,
}

#[derive(Clone, Copy)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: PieceColor,
}

pub struct ChessGame {
    pub board: [[Option<Piece>; 8]; 8],
    pub selected_square: Option<(usize, usize)>,
}

/// Retorna o índice correspondente da peça na sua spritesheet (de 0 a 11).
/// IMPORTANTE: Ajuste a ordem deste match de acordo com a disposição exata da sua imagem!
fn get_piece_index(piece_type: PieceType, color: PieceColor) -> usize {
    // Exemplo baseado em uma ordem comum: Rei, Rainha, Bispo, Cavalo, Torre, Peão
    let base_idx = match piece_type {
        PieceType::King => 4,
        PieceType::Queen => 5,
        PieceType::Bishop => 1,
        PieceType::Knight => 3,
        PieceType::Rook => 2,
        PieceType::Pawn => 0,
    };

    // Se for preta, pula os 6 primeiros índices ocupados pelas brancas
    match color {
        PieceColor::White => base_idx,
        PieceColor::Black => base_idx + 6,
    }
}

fn get_piece_column_index(piece_type: PieceType) -> usize {
    match piece_type {
        PieceType::King => 0,
        PieceType::Queen => 1,
        PieceType::Bishop => 2,
        PieceType::Knight => 3,
        PieceType::Rook => 4,
        PieceType::Pawn => 5,
    }
}
impl ChessGame {
    pub fn new() -> Self {
        Self {
            board: [
                // Linha 0: Peças Pretas de trás
                [
                    Some(Piece {
                        piece_type: PieceType::Rook,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Knight,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Bishop,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Queen,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::King,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Bishop,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Knight,
                        color: PieceColor::Black,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Rook,
                        color: PieceColor::Black,
                    }),
                ],
                // Linha 1: Peões Pretos
                [Some(Piece {
                    piece_type: PieceType::Pawn,
                    color: PieceColor::Black,
                }); 8],
                [None; 8],
                [None; 8],
                [None; 8],
                [None; 8],
                // Linha 6: Peões Brancos
                [Some(Piece {
                    piece_type: PieceType::Pawn,
                    color: PieceColor::White,
                }); 8],
                // Linha 7: Peças Brancas de trás
                [
                    Some(Piece {
                        piece_type: PieceType::Rook,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Knight,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Bishop,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Queen,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::King,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Bishop,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Knight,
                        color: PieceColor::White,
                    }),
                    Some(Piece {
                        piece_type: PieceType::Rook,
                        color: PieceColor::White,
                    }),
                ],
            ],
            selected_square: None,
        }
    }

    /// Renderização principal utilizando a spritesheet texturizada
    pub fn render(
        &self,
        renderer: &mut Renderer,
        vp_w: f32,
        vp_h: f32,
        pieces_bind_group: &Arc<wgpu::BindGroup>,
    ) -> anyhow::Result<()> {
        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;
        let header_h = 75.0 * scale_factor;
        let footer_h = 65.0 * scale_factor;
        let available_h = (vp_h - header_h - footer_h).max(10.0);

        let board_size = vp_w.min(available_h) * 0.92;
        let cell_size = board_size / 8.0;

        let start_x = (vp_w - board_size) * 0.5;
        let start_y = header_h + (available_h - board_size) * 0.5;

        // Borda de madeira do tabuleiro
        let border = cell_size * 0.15;
        renderer.draw_rect(
            start_x - border,
            start_y - border,
            board_size + border * 2.0,
            board_size + border * 2.0,
            [0.25, 0.15, 0.08, 1.0],
        );

        // Desenha as casas e as peças texturizadas
        for row in 0..8 {
            for col in 0..8 {
                let cell_x = start_x + col as f32 * cell_size;
                let cell_y = start_y + row as f32 * cell_size;

                let is_light = (row + col) % 2 == 0;
                let cell_color = if is_light {
                    [0.94, 0.85, 0.71, 1.0] // Bege claro
                } else {
                    [0.48, 0.35, 0.24, 1.0] // Marrom médio
                };

                renderer.draw_rect(cell_x, cell_y, cell_size, cell_size, cell_color);

                if let Some(piece) = self.board[row][col] {
                    // 1. Descobre a coluna da peça (X)
                    let col_idx = get_piece_column_index(piece.piece_type);

                    // 2. Calcula os limites horizontais (U) - agora dividido por 6
                    let u_step = 1.0 / 6.0;
                    let min_u = col_idx as f32 * u_step;
                    let max_u = (col_idx + 1) as f32 * u_step;

                    // 3. Calcula os limites verticais (V) baseado na linha (Cor)
                    // Como são 2 linhas, cada uma ocupa 0.5 (50%) da altura da textura
                    let (min_v, max_v) = match piece.color {
                        PieceColor::White => (0.0, 0.5), // Linha de cima (0% a 50%)
                        PieceColor::Black => (0.5, 1.0), // Linha de baixo (50% a 100%)
                    };

                    // Monta o quadrante de recorte [min_u, min_v, max_u, max_v]
                    let uvs = [min_u, min_v, max_u, max_v];

                    // 4. Desenha a peça na tela
                    renderer.draw_textured_rect(
                        cell_x,
                        cell_y,
                        cell_size,
                        cell_size,
                        [1.0, 1.0, 1.0, 1.0], // Mantém as cores originais
                        uvs,
                        pieces_bind_group.clone(),
                    );
                }
            }
        }

        // Destaque visual da casa E2 (Mantido do seu código)
        let e2_x = start_x + 4.0 * cell_size;
        let e2_y = start_y + 6.0 * cell_size;
        renderer.draw_rect(
            e2_x + cell_size * 0.1,
            e2_y + cell_size * 0.1,
            cell_size * 0.8,
            cell_size * 0.8,
            [1.0, 0.0, 0.0, 0.5],
        );

        Ok(())
    }

    /// Método fallback caso ocorra algum problema no carregamento da textura
    pub fn render_placeholder(
        &self,
        renderer: &mut Renderer,
        vp_w: f32,
        vp_h: f32,
    ) -> anyhow::Result<()> {
        // ... Você pode mover a lógica do seu círculo ou peão antigo para cá como segurança
        Ok(())
    }
}
