use crate::menu::menu::draw::desenhar_seta_botao;
use crate::menu::menu::layout::{BASE_WIDTH, carousel_layout};
use crate::menu::types::ArrowId;

use super::state::App;

impl App {
    pub(super) fn update(&mut self) {
        if let Some(menu) = self.menu_stack.last_mut() {
            if menu.animating {
                menu.anim_t += 0.08;
                if menu.anim_t >= 1.0 {
                    menu.anim_t = 1.0;
                    menu.animating = false;
                    menu.selected = menu.anim_to;
                }
            }
        }

        if let Some(ref mut game) = self.chess_game {
            // Se o turno atual não for igual à cor escolhida para o jogador, significa que é o turno da IA
            if game.current_turn != game.player_color {
                game.make_ai_move();
            }
        }
    }

    pub(super) fn render(&mut self) -> anyhow::Result<()> {
        let show_fullscreen_button = self.should_show_fullscreen_button();
        let fullscreen_button_rect = if show_fullscreen_button {
            Some(self.fullscreen_button_rect())
        } else {
            None
        };
        let fullscreen_button_hovered = self.hovered_fullscreen_button;
        let fullscreen_button_pressed = self.pressed_fullscreen_button;

        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return Ok(()),
        };

        renderer.clear();

        let vp_w = renderer.uniforms.screen_size[0];
        let vp_h = renderer.uniforms.screen_size[1];

        let mut hovered_arrow = None;
        for arrow in &self.arrows {
            if arrow.contains(self.mouse_pos.0, self.mouse_pos.1) {
                hovered_arrow = Some(arrow.id);
            }
        }
        self.hovered_arrow = hovered_arrow;

        let scale_factor = vp_w / BASE_WIDTH;

        renderer.draw_rect(0.0, 0.0, vp_w, vp_h, [0.08, 0.08, 0.12, 1.0]);
        renderer.draw_rect(0.0, vp_h * 0.5, vp_w, vp_h * 0.5, [0.10, 0.10, 0.16, 1.0]);

        let top_bar_h = 60.0 * scale_factor;
        renderer.draw_rect(0.0, 0.0, vp_w, top_bar_h, [0.13, 0.13, 0.18, 1.0]);
        renderer.draw_rect(
            0.0,
            top_bar_h - 2.0 * scale_factor,
            vp_w,
            2.0 * scale_factor,
            [0.3, 0.5, 0.9, 0.9],
        );

        if let Some((bx, by, bw, bh)) = fullscreen_button_rect {
            let bg = if fullscreen_button_pressed {
                [0.20, 0.60, 1.00, 1.0]
            } else if fullscreen_button_hovered {
                [0.30, 0.55, 0.90, 1.0]
            } else {
                [0.16, 0.20, 0.30, 0.95]
            };

            renderer.draw_rect(bx, by, bw, bh, bg);
            renderer.draw_rect(bx, by + bh - 2.0, bw, 2.0, [0.8, 0.9, 1.0, 0.9]);

            let pad = 9.0;
            let l = 8.0;
            let t = 2.0;
            let c = [0.95, 0.98, 1.0, 1.0];

            renderer.draw_rect(bx + pad, by + pad, l, t, c);
            renderer.draw_rect(bx + pad, by + pad, t, l, c);
            renderer.draw_rect(bx + bw - pad - l, by + pad, l, t, c);
            renderer.draw_rect(bx + bw - pad - t, by + pad, t, l, c);
            renderer.draw_rect(bx + pad, by + bh - pad - t, l, t, c);
            renderer.draw_rect(bx + pad, by + bh - pad - l, t, l, c);
            renderer.draw_rect(bx + bw - pad - l, by + bh - pad - t, l, t, c);
            renderer.draw_rect(bx + bw - pad - t, by + bh - pad - l, t, l, c);
        }

        if let Some(ref game) = self.chess_game {
            let scale_factor = vp_w.min(vp_h * 0.8) / BASE_WIDTH;

            let header_h = 75.0 * scale_factor;
            let footer_h = 65.0 * scale_factor;
            let available_h = (vp_h - header_h - footer_h).max(10.0);

            let board_size = vp_w.min(available_h) * 0.92;
            let chess_start_y = header_h + (available_h - board_size) * 0.5;

            renderer.draw_rect(0.0, 0.0, vp_w, vp_h, [0.08, 0.08, 0.12, 1.0]);

            // Altere a chamada do game.render para incluir a textura se ela existir:
            if let Some(ref chess_bg) = self.chess_texture {
                game.render(renderer, vp_w, vp_h, chess_bg)?;
            }

            renderer.draw_rect(0.0, 0.0, vp_w, chess_start_y, [0.13, 0.13, 0.18, 1.0]);
            renderer.draw_rect(
                0.0,
                chess_start_y - 2.0 * scale_factor,
                vp_w,
                2.0 * scale_factor,
                [0.3, 0.5, 0.9, 0.9],
            );

            let footer_y = chess_start_y + board_size;
            let footer_render_h = vp_h - footer_y;
            renderer.draw_rect(
                0.0,
                footer_y,
                vp_w,
                footer_render_h,
                [0.13, 0.13, 0.18, 1.0],
            );
            renderer.draw_rect(
                0.0,
                footer_y,
                vp_w,
                2.0 * scale_factor,
                [0.3, 0.5, 0.9, 0.9],
            );

            for arrow in &self.arrows {
                if arrow.id == ArrowId::Back {
                    let is_hovered = self.hovered_arrow == Some(arrow.id);
                    let is_pressed = self.pressed_arrow == Some(arrow.id);

                    let color = if is_pressed {
                        [0.2, 0.6, 1.0, 1.0]
                    } else if is_hovered {
                        [0.4, 0.7, 1.0, 1.0]
                    } else {
                        [0.2, 0.3, 0.5, 1.0]
                    };

                    desenhar_seta_botao(
                        renderer, arrow.id, arrow.x, arrow.y, arrow.w, arrow.h, color,
                    );
                }
            }

            let cursor_size = 8.0 * scale_factor;
            renderer.draw_rect(
                self.mouse_pos.0 - cursor_size * 0.5,
                self.mouse_pos.1 - cursor_size * 0.5,
                cursor_size,
                cursor_size,
                [1.0, 0.9, 0.0, 0.9],
            );

            renderer.present(self.viewport)?;
            return Ok(());
        }

        let (cx, cy, base_size, gap) = carousel_layout(vp_w, vp_h);

        let profundidade = self.menu_stack.len();
        let icones_ativos = if profundidade == 1 {
            &self.icones
        } else {
            &self.icones_sub
        };

        if let Some(menu) = self.menu_stack.last() {
            let selected_f = menu.selected_float();

            for (i, item) in menu.items.iter().enumerate() {
                let d = i as f32 - selected_f;
                if d.abs() > 3.0 {
                    continue;
                }

                let escala = (1.0 - d.abs() * 0.18).clamp(0.6, 1.0);
                let size = base_size * escala;
                let x = cx + d * gap - size * 0.5;
                let y = cy - size * 0.5;

                let is_center = d.abs() < 0.5;

                renderer.draw_rect(x + 4.0, y + 4.0, size, size, [0.0, 0.0, 0.0, 0.4]);

                let cor_fundo = if is_center {
                    [0.18, 0.18, 0.25, 1.0]
                } else {
                    [0.13, 0.13, 0.18, 1.0]
                };
                renderer.draw_rect(x, y, size, size, cor_fundo);

                if let Some(icone) = icones_ativos.get(i) {
                    let padding = size * 0.1;
                    let tint = if is_center {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        [0.6, 0.6, 0.7, 1.0]
                    };
                    renderer.draw_textured_rect(
                        x + padding,
                        y + padding,
                        size - padding * 2.0,
                        size - padding * 2.0,
                        tint,
                        [0.0, 0.0, 1.0, 1.0],
                        icone.bind_group.clone(),
                    );

                    let label_scale = escala * scale_factor;
                    let lw = icone.label_w * label_scale;
                    let lh = icone.label_h * label_scale;
                    let label_x = x + size * 0.5 - lw * 0.5;
                    let label_y = y + size + 6.0 * scale_factor;
                    let label_alpha = if is_center { 1.0 } else { 0.5 };
                    renderer.draw_textured_rect(
                        label_x,
                        label_y,
                        lw,
                        lh,
                        [1.0, 1.0, 1.0, label_alpha],
                        [0.0, 0.0, 1.0, 1.0],
                        icone.label_bind_group.clone(),
                    );
                }

                if item.children.is_some() {
                    let ind = 10.0 * scale_factor;
                    let pad = 5.0 * scale_factor;
                    renderer.draw_rect(
                        x + size - ind - pad,
                        y + pad,
                        ind,
                        ind,
                        [1.0, 1.0, 1.0, 0.9],
                    );
                }

                if is_center {
                    let borda = 2.0 * scale_factor;
                    renderer.draw_rect(x, y, size, borda, [0.3, 0.6, 1.0, 0.8]);
                    renderer.draw_rect(x, y + size - borda, size, borda, [0.3, 0.6, 1.0, 0.8]);
                    renderer.draw_rect(x, y, borda, size, [0.3, 0.6, 1.0, 0.8]);
                    renderer.draw_rect(x + size - borda, y, borda, size, [0.3, 0.6, 1.0, 0.8]);
                }
            }
        }

        for arrow in &self.arrows {
            let is_hovered = self.hovered_arrow == Some(arrow.id);
            let is_pressed = self.pressed_arrow == Some(arrow.id);

            let color = if is_pressed {
                [0.2, 0.6, 1.0, 1.0]
            } else if is_hovered {
                [0.4, 0.7, 1.0, 1.0]
            } else {
                [0.2, 0.3, 0.5, 1.0]
            };

            desenhar_seta_botao(
                renderer, arrow.id, arrow.x, arrow.y, arrow.w, arrow.h, color,
            );
        }

        let cursor_size = 8.0 * scale_factor;
        renderer.draw_rect(
            self.mouse_pos.0 - cursor_size * 0.5,
            self.mouse_pos.1 - cursor_size * 0.5,
            cursor_size,
            cursor_size,
            [1.0, 0.9, 0.0, 0.9],
        );

        renderer.present(self.viewport)?;
        Ok(())
    }
}
