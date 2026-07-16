// Renderização do tabuleiro e das peças usando a spritesheet texturizada
use super::ChessGame;
use super::piece::{PieceColor, get_piece_column_index};
use crate::menu::renderer::Renderer;
use std::sync::Arc;

impl ChessGame {
    /// Renderização principal utilizando a spritesheet texturizada
    pub fn render(
        &mut self,
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

        // Desenha as casas e as peças estáticas
        for row in 0..8 {
            for col in 0..8 {
                let (visual_row, visual_col) = if self.player_color == PieceColor::Black {
                    (7 - row, 7 - col)
                } else {
                    (row, col)
                };
                let cell_x = start_x + visual_col as f32 * cell_size;
                let cell_y = start_y + visual_row as f32 * cell_size;

                let is_light = (row + col) % 2 == 0;
                let cell_color = if is_light {
                    [0.94, 0.85, 0.71, 1.0] // Bege claro
                } else {
                    [0.48, 0.35, 0.24, 1.0] // Marrom médio
                };

                renderer.draw_rect(cell_x, cell_y, cell_size, cell_size, cell_color);

                // CORREÇÃO PASSO 2: Ocultar a peça estática se ela estiver em movimento
                if let Some(piece) = self.board[row][col] {
                    let esta_em_movimento_para_ca = if let Some(ref mp) = self.moving_piece {
                        mp.to_row == row && mp.to_col == col
                    } else {
                        false
                    };

                    if !esta_em_movimento_para_ca {
                        let col_idx = get_piece_column_index(piece.piece_type);
                        let u_step = 1.0 / 6.0;
                        let min_u = col_idx as f32 * u_step;
                        let max_u = (col_idx + 1) as f32 * u_step;

                        let (min_v, max_v) = match piece.color {
                            PieceColor::White => (0.0, 0.5),
                            PieceColor::Black => (0.5, 1.0),
                        };

                        let uvs = [min_u, min_v, max_u, max_v];

                        renderer.draw_textured_rect(
                            cell_x,
                            cell_y,
                            cell_size,
                            cell_size,
                            [1.0, 1.0, 1.0, 1.0],
                            uvs,
                            pieces_bind_group.clone(),
                        );
                    }
                }

                // Círculo de movimentos válidos
                if (row, col)
                    == self
                        .valid_moves
                        .iter()
                        .cloned()
                        .find(|&m| m == (row, col))
                        .unwrap_or((usize::MAX, usize::MAX))
                    && self.selected_square.is_some()
                {
                    renderer.draw_circle(
                        cell_x + cell_size * 0.5,
                        cell_y + cell_size * 0.5,
                        cell_size * 0.15,
                        [1.0, 0.0, 0.0, 0.4],
                    );
                }
            }
        } // FIM do loop das casas

        // CORREÇÃO PASSO 1: Desenhar a animação da peça movendo UMA única vez (fora do loop)
        if let Some(ref mp) = self.moving_piece {
            let (from_v_row, from_v_col) = if self.player_color == PieceColor::Black {
                (7 - mp.from_row, 7 - mp.from_col)
            } else {
                (mp.from_row, mp.from_col)
            };
            let (to_v_row, to_v_col) = if self.player_color == PieceColor::Black {
                (7 - mp.to_row, 7 - mp.to_col)
            } else {
                (mp.to_row, mp.to_col)
            };

            let from_x = start_x + from_v_col as f32 * cell_size;
            let from_y = start_y + from_v_row as f32 * cell_size;
            let to_x = start_x + to_v_col as f32 * cell_size;
            let to_y = start_y + to_v_row as f32 * cell_size;

            let t_suave = crate::menu::animation::ease_out(mp.t);
            let draw_x = crate::menu::animation::lerp(from_x, to_x, t_suave);
            let draw_y = crate::menu::animation::lerp(from_y, to_y, t_suave);

            let col_idx = get_piece_column_index(mp.piece.piece_type);
            let u_step = 1.0 / 6.0;
            let min_u = col_idx as f32 * u_step;
            let max_u = (col_idx + 1) as f32 * u_step;
            let (min_v, max_v) = match mp.piece.color {
                PieceColor::White => (0.0, 0.5),
                PieceColor::Black => (0.5, 1.0),
            };

            renderer.draw_textured_rect(
                draw_x,
                draw_y,
                cell_size,
                cell_size,
                [1.0, 1.0, 1.0, 1.0],
                [min_u, min_v, max_u, max_v],
                pieces_bind_group.clone(),
            );
        }

        // Desenhar indicador verde de seleção
        if let Some((sel_row, sel_col)) = self.selected_square {
            let (visual_row, visual_col) = if self.player_color == PieceColor::Black {
                (7 - sel_row, 7 - sel_col)
            } else {
                (sel_row, sel_col)
            };

            let sel_x = start_x + visual_col as f32 * cell_size;
            let sel_y = start_y + visual_row as f32 * cell_size;

            renderer.draw_rect(
                sel_x + cell_size * 0.05,
                sel_y + cell_size * 0.05,
                cell_size * 0.9,
                cell_size * 0.9,
                [0.0, 1.0, 0.0, 0.4],
            );
        }

        // Desenhar borda amarela do cursor de teclado
        if let Some((cur_row, cur_col)) = self.cursor {
            let (visual_row, visual_col) = if self.player_color == PieceColor::Black {
                (7 - cur_row, 7 - cur_col)
            } else {
                (cur_row, cur_col)
            };

            let cur_x = start_x + visual_col as f32 * cell_size;
            let cur_y = start_y + visual_row as f32 * cell_size;
            renderer.draw_rect(
                cur_x + cell_size * 0.15,
                cur_y + cell_size * 0.15,
                cell_size * 0.7,
                cell_size * 0.7,
                [1.0, 1.0, 0.0, 0.4],
            );
        }

        // UI da Promoção (Permanece idêntica abaixo...)
        if self.promotion_pending.is_some() {
            let menu_w = 400.0 * scale_factor;
            let menu_h = 120.0 * scale_factor;
            let menu_start_x = (vp_w - menu_w) * 0.5;
            let menu_start_y = (vp_h - menu_h) * 0.5;

            renderer.draw_rect(
                menu_start_x - 5.0 * scale_factor,
                menu_start_y - 5.0 * scale_factor,
                menu_w + 10.0 * scale_factor,
                menu_h + 10.0 * scale_factor,
                [0.8, 0.7, 0.3, 1.0],
            );

            renderer.draw_rect(
                menu_start_x,
                menu_start_y,
                menu_w,
                menu_h,
                [0.1, 0.1, 0.1, 0.95],
            );

            let item_w = menu_w / 4.0;
            let pieces = [
                crate::menu::chess::PieceType::Queen,
                crate::menu::chess::PieceType::Rook,
                crate::menu::chess::PieceType::Bishop,
                crate::menu::chess::PieceType::Knight,
            ];

            for (i, p) in pieces.iter().enumerate() {
                let p_x = menu_start_x + i as f32 * item_w;

                if i == self.promotion_cursor {
                    renderer.draw_rect(p_x, menu_start_y, item_w, menu_h, [0.3, 0.6, 0.3, 0.8]);
                }

                let col_idx = get_piece_column_index(*p);
                let u_step = 1.0 / 6.0;
                let min_u = col_idx as f32 * u_step;
                let max_u = (col_idx + 1) as f32 * u_step;

                let (min_v, max_v) = match self.player_color {
                    PieceColor::White => (0.0, 0.5),
                    PieceColor::Black => (0.5, 1.0),
                };

                let uvs = [min_u, min_v, max_u, max_v];

                let piece_size = item_w * 0.8;
                let p_y = menu_start_y + (menu_h - piece_size) * 0.5;

                renderer.draw_textured_rect(
                    p_x + (item_w - piece_size) * 0.5,
                    p_y,
                    piece_size,
                    piece_size,
                    [1.0, 1.0, 1.0, 1.0],
                    uvs,
                    pieces_bind_group.clone(),
                );
            }
        }

        if let Some(outcome) = self.game_outcome {
            let (accent_color, accent_soft, headline, subline) = match outcome {
                super::GameOutcome::Checkmate { winner } if winner == self.player_color => (
                    [0.20, 0.78, 0.35, 1.0],
                    [0.14, 0.48, 0.24, 0.95],
                    "VITORIA!",
                    "VOCE VENCE",
                ),
                super::GameOutcome::Checkmate { .. } => (
                    [0.88, 0.24, 0.24, 1.0],
                    [0.52, 0.16, 0.16, 0.95],
                    "DERROTA!",
                    "VOCE PERDE",
                ),
                super::GameOutcome::Stalemate => (
                    [0.62, 0.64, 0.67, 1.0],
                    [0.38, 0.40, 0.44, 0.95],
                    "EMPATE!",
                    "SEM VENCEDOR",
                ),
            };

            let overlay_color = [
                accent_soft[0] * 0.30,
                accent_soft[1] * 0.30,
                accent_soft[2] * 0.30,
                0.78,
            ];
            renderer.draw_rect(0.0, 0.0, vp_w, vp_h, overlay_color);

            // Main result panel with soft shadow and accent frame.
            let banner_w = vp_w * 0.82;
            let banner_h = vp_h * 0.52;
            let banner_x = (vp_w - banner_w) * 0.5;
            let banner_y = (vp_h - banner_h) * 0.5;
            renderer.draw_rect(
                banner_x + 10.0,
                banner_y + 12.0,
                banner_w,
                banner_h,
                [0.0, 0.0, 0.0, 0.32],
            );
            renderer.draw_rect(
                banner_x,
                banner_y,
                banner_w,
                banner_h,
                [0.06, 0.07, 0.09, 0.98],
            );
            renderer.draw_rect(banner_x, banner_y, banner_w, 5.0, accent_color);
            renderer.draw_rect(
                banner_x,
                banner_y + banner_h - 5.0,
                banner_w,
                5.0,
                accent_color,
            );

            let side_stripe_w = banner_w * 0.015;
            renderer.draw_rect(banner_x, banner_y, side_stripe_w, banner_h, accent_soft);
            renderer.draw_rect(
                banner_x + banner_w - side_stripe_w,
                banner_y,
                side_stripe_w,
                banner_h,
                accent_soft,
            );

            // Hero area
            let top_bar_h = banner_h * 0.38;
            renderer.draw_rect(
                banner_x + side_stripe_w,
                banner_y + side_stripe_w,
                banner_w - side_stripe_w * 2.0,
                top_bar_h,
                accent_soft,
            );
            renderer.draw_rect(
                banner_x + side_stripe_w,
                banner_y + top_bar_h * 0.55,
                banner_w - side_stripe_w * 2.0,
                top_bar_h * 0.45,
                [0.03, 0.03, 0.04, 0.30],
            );
            self.draw_outcome_text(
                renderer,
                banner_x + side_stripe_w,
                banner_y,
                banner_w - side_stripe_w * 2.0,
                top_bar_h,
                headline,
            );

            // Subline badge
            let badge_w = banner_w * 0.54;
            let badge_h = banner_h * 0.11;
            let badge_x = banner_x + (banner_w - badge_w) * 0.5;
            let badge_y = banner_y + top_bar_h + banner_h * 0.07;
            renderer.draw_rect(badge_x, badge_y, badge_w, badge_h, [0.11, 0.12, 0.15, 0.95]);
            renderer.draw_rect(badge_x, badge_y, badge_w, 2.0, accent_color);
            renderer.draw_rect(badge_x, badge_y + badge_h - 2.0, badge_w, 2.0, accent_color);

            let subline_chars = subline.chars().count().max(1) as f32;
            let subline_px = ((badge_w * 0.90) / (subline_chars * 6.0))
                .min((badge_h * 0.70) / 7.0)
                .max(1.2);
            let subline_w = subline_chars * 6.0 * subline_px;
            let subline_h = 7.0 * subline_px;
            let subline_x = badge_x + (badge_w - subline_w) * 0.5;
            let subline_y = badge_y + (badge_h - subline_h) * 0.5;
            self.draw_bitmap_text_foreground(
                renderer,
                subline,
                subline_x,
                subline_y,
                subline_px,
                [0.96, 0.97, 0.98, 0.96],
                6.0 * subline_px,
            );

            // Bottom instruction bar - "Nova Partida" button
            let instr_h = banner_h * 0.20;
            let instr_y = banner_y + banner_h - instr_h;
            renderer.draw_rect(
                banner_x + side_stripe_w,
                instr_y,
                banner_w - side_stripe_w * 2.0,
                instr_h,
                [0.08, 0.09, 0.11, 0.95],
            );
            renderer.draw_rect(
                banner_x + side_stripe_w,
                instr_y,
                banner_w - side_stripe_w * 2.0,
                2.0,
                accent_soft,
            );

            // Draw "Nova Partida" button in the center of instruction bar
            let btn_w = banner_w * 0.44;
            let btn_h = instr_h * 0.58;
            let btn_x = banner_x + (banner_w - btn_w) * 0.5;
            let btn_y = instr_y + (instr_h - btn_h) * 0.5;

            // Store button rect for click detection (using pub field)
            self.new_game_button_rect = Some((btn_x, btn_y, btn_w, btn_h));

            // Button background
            renderer.draw_rect(btn_x, btn_y, btn_w, btn_h, accent_color);
            renderer.draw_rect(
                btn_x + 2.0,
                btn_y + 2.0,
                btn_w - 4.0,
                btn_h - 4.0,
                [0.10, 0.11, 0.13, 0.96],
            );

            // Draw refresh/restart icon (simplified ♻ symbol using small rectangles)
            let icon_size = btn_h * 0.35;
            let icon_x = btn_x + btn_w * 0.15;
            let icon_y = btn_y + (btn_h - icon_size) * 0.5;

            // Curved arrows (represented as rectangles forming a circular pattern)
            for angle_step in 0..6 {
                let angle = (angle_step as f32) * std::f32::consts::PI / 3.0;
                let cx = icon_x + icon_size * 0.5;
                let cy = icon_y + icon_size * 0.5;
                let r = icon_size * 0.3;
                let px = cx + angle.cos() * r;
                let py = cy + angle.sin() * r;
                renderer.draw_rect(px - 2.0, py - 2.0, 4.0, 4.0, accent_color);
            }

            self.draw_button_text(renderer, btn_x, btn_y, btn_w, btn_h);
        } else {
            self.new_game_button_rect = None;
        }

        Ok(())
    }

    fn glyph_rows(ch: char) -> [u8; 7] {
        match ch {
            'A' => [
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'B' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ],
            'C' => [
                0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
            ],
            'D' => [
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ],
            'E' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ],
            'F' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'G' => [
                0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
            ],
            'H' => [
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'I' => [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
            ],
            'K' => [
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ],
            'L' => [
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ],
            'M' => [
                0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
            ],
            'N' => [
                0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
            ],
            'O' => [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'P' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'Q' => [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
            ],
            'R' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ],
            'S' => [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            'T' => [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
            'U' => [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'V' => [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ],
            'W' => [
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
            ],
            'Y' => [
                0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
            '0' => [
                0b01110, 0b10011, 0b10101, 0b10101, 0b10101, 0b11001, 0b01110,
            ],
            '1' => [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
            '2' => [
                0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
            ],
            '3' => [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            '4' => [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
            '5' => [
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ],
            '6' => [
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
            '7' => [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
            '8' => [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
            '9' => [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
            ],
            '!' => [
                0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
            ],
            ' ' => [0; 7],
            _ => [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b00000, 0b01000,
            ],
        }
    }

    fn draw_bitmap_text(
        &self,
        renderer: &mut Renderer,
        text: &str,
        x: f32,
        y: f32,
        px: f32,
        color: [f32; 4],
    ) {
        let advance = 6.0 * px;
        let glyph_h = 7.0 * px;

        let text_w = text.chars().count() as f32 * advance;
        renderer.draw_rect(
            x - px,
            y - px,
            text_w + px * 2.0,
            glyph_h + px * 2.0,
            [0.0, 0.0, 0.0, 0.18],
        );
        self.draw_bitmap_text_foreground(renderer, text, x, y, px, color, advance);
    }

    fn draw_bitmap_text_foreground(
        &self,
        renderer: &mut Renderer,
        text: &str,
        x: f32,
        y: f32,
        px: f32,
        color: [f32; 4],
        advance: f32,
    ) {
        let mut cx = x;
        for ch in text.chars() {
            let rows = Self::glyph_rows(ch.to_ascii_uppercase());
            for (row_idx, row_bits) in rows.iter().enumerate() {
                for col_idx in 0..5 {
                    if (row_bits >> (4 - col_idx)) & 1 == 1 {
                        renderer.draw_rect(
                            cx + col_idx as f32 * px,
                            y + row_idx as f32 * px,
                            px,
                            px,
                            color,
                        );
                    }
                }
            }
            cx += advance;
        }
    }

    fn draw_outcome_text(
        &self,
        renderer: &mut Renderer,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
    ) {
        let chars = text.chars().count().max(1) as f32;
        let px_by_width = (w * 0.82) / (chars * 6.0);
        let px_by_height = (h * 0.72) / 7.0;
        let px = px_by_width.min(px_by_height).max(1.5);
        let text_w = chars * 6.0 * px;
        let text_h = 7.0 * px;
        let text_x = x + (w - text_w) * 0.5;
        let text_y = y + (h - text_h) * 0.5;
        self.draw_bitmap_text(renderer, text, text_x, text_y, px, [0.97, 0.97, 0.99, 0.95]);
    }

    fn draw_button_text(
        &self,
        renderer: &mut Renderer,
        btn_x: f32,
        btn_y: f32,
        btn_w: f32,
        btn_h: f32,
    ) {
        let text = "NOVA PARTIDA";
        let chars = text.chars().count().max(1) as f32;
        let px_by_width = (btn_w * 0.62) / (chars * 6.0);
        let px_by_height = (btn_h * 0.72) / 7.0;
        let px = px_by_width.min(px_by_height).max(1.2);
        let text_h = 7.0 * px;
        let text_x = btn_x + btn_w * 0.30;
        let text_y = btn_y + (btn_h - text_h) * 0.5;

        self.draw_bitmap_text_foreground(
            renderer,
            text,
            text_x,
            text_y,
            px,
            [0.95, 0.95, 0.98, 0.95],
            6.0 * px,
        );
    }
}
