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
    pub cursor: Option<(usize, usize)>, // Guarda onde a seleção do teclado está apontando (linha, coluna)
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
    
    /// Converte a coordenada (X, Y) do mouse para uma casa (linha, coluna) do tabuleiro
    pub fn mouse_to_square(&mut self, mx: f32, my: f32, vp_w: f32, vp_h: f32) -> Option<(usize, usize)> {
        self.cursor = None; // Reseta o cursor do teclado quando o mouse é usado
        // Mesma matemática usada no seu método render() para achar o tabuleiro na tela
        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;
        let header_h = 75.0 * scale_factor;
        let footer_h = 65.0 * scale_factor;
        let available_h = (vp_h - header_h - footer_h).max(10.0);
        let board_size = vp_w.min(available_h) * 0.92;
        let cell_size = board_size / 8.0;
        let start_x = (vp_w - board_size) * 0.5;
        let start_y = header_h + (available_h - board_size) * 0.5;

        // Verifica se o mouse está dentro dos limites do tabuleiro
        if mx >= start_x && mx < start_x + board_size && my >= start_y && my < start_y + board_size {
            let col = ((mx - start_x) / cell_size) as usize;
            let row = ((my - start_y) / cell_size) as usize;
            
            // Garante que fique estritamente entre 0 e 7
            if row < 8 && col < 8 {
                return Some((row, col));
            }
        }
        None // Mouse clicou fora do tabuleiro
    }

    pub fn select_or_move(&mut self, row: usize, col: usize) {
        // Se o jogador já tinha selecionado uma casa anteriormente
        if let Some((from_row, from_col)) = self.selected_square {
            
            // 1. Se ele clicou na mesma casa, ele quer desfazer a seleção
            if from_row == row && from_col == col {
                self.selected_square = None;
                return;
            }

            // 2. Se ele clicou em OUTRA casa, vamos mover a peça
            // Pegamos a peça que estava na casa de origem
            if let Some(piece) = self.board[from_row][from_col] {
                // Move a peça para o destino
                self.board[row][col] = Some(piece);
                // Apaga a peça da posição antiga
                self.board[from_row][from_col] = None;
            }

            // Movimento concluído, limpamos a seleção para o próximo lance
            self.selected_square = None;

        } else {
            // Se nenhuma casa estava selecionada, tentamos selecionar a atual
            // Mas só selecionamos se houver de fato uma peça ali!
            if self.board[row][col].is_some() {
                self.selected_square = Some((row, col));
            }
        }
    }

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
            cursor: Some((7, 4)), // Inicia com o cursor apontando para o Rei Branco
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

        if let Some((sel_row, sel_col)) = self.selected_square {
            let sel_x = start_x + sel_col as f32 * cell_size;
            let sel_y = start_y + sel_row as f32 * cell_size;
            
            // Uma borda translúcida verde indicando seleção
            renderer.draw_rect(
                sel_x + cell_size * 0.05,
                sel_y + cell_size * 0.05,
                cell_size * 0.9,
                cell_size * 0.9,
                [0.0, 1.0, 0.0, 0.4], // Verde com 40% de opacidade
            );
        }

        // NOVO: Desenha uma borda amarela onde o CURSOR DO TECLADO está apontando
        if let Some((cur_row, cur_col)) = self.cursor {
            let cur_x = start_x + cur_col as f32 * cell_size;
            let cur_y = start_y + cur_row as f32 * cell_size;
            renderer.draw_rect(
                cur_x + cell_size * 0.15,
                cur_y + cell_size * 0.15,
                cell_size * 0.7,
                cell_size * 0.7,
                [1.0, 1.0, 0.0, 0.4], // Amarelo com 40% de opacidade
            );
        }

        Ok(())
    }
}
