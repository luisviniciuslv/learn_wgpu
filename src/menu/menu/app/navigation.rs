use crate::menu::chess::ChessGame;
use crate::menu::menu::assets::{carregar_fonte, gerar_icones};
use crate::menu::menu::layout::{CHESS_TARGET_ASPECT_RATIO, carousel_layout};
use crate::menu::types::MenuState;

use super::state::App;

impl App {
    pub(super) fn move_selection(&mut self, delta: i32) {
        if let Some(menu) = self.menu_stack.last_mut() {
            if menu.animating {
                return;
            }
            let len = menu.items.len() as i32;
            if len == 0 {
                return;
            }
            let next = (menu.selected + delta).clamp(0, len - 1);
            if next == menu.selected {
                return;
            }
            menu.anim_from = menu.selected;
            menu.anim_to = next;
            menu.anim_t = 0.0;
            menu.animating = true;
        }
    }

    pub(super) fn try_enter_submenu(&mut self) {
        let (children, items_clone, item_id) = {
            let menu = match self.menu_stack.last() {
                Some(m) => m,
                None => return,
            };
            let idx = menu.selected as usize;
            let item = match menu.items.get(idx) {
                Some(i) => i,
                None => return,
            };
            match &item.children {
                Some(ch) => (true, ch.clone(), item.id),
                None => (false, vec![], item.id),
            }
        };

        if item_id == "chess" {
            self.chess_game = Some(ChessGame::new());

            self.saved_target_aspect_ratio = self.target_aspect_ratio;
            self.saved_width = self.last_width;
            self.saved_height = self.last_height;

            self.target_aspect_ratio = CHESS_TARGET_ASPECT_RATIO;

            if let Some(ref renderer) = self.renderer {
                let size = renderer.window.inner_size();
                let new_height = size.height;
                let new_width = (new_height as f32 * self.target_aspect_ratio).round() as u32;

                self.pending_size = Some((new_width, new_height));
                let _ = renderer
                    .window
                    .request_inner_size(winit::dpi::PhysicalSize::new(new_width, new_height));

                self.resize_layout(size.width, size.height);
            }
            return;
        }

        if children {
            if let Some(renderer) = &self.renderer {
                let font = carregar_fonte();
                self.icones_sub = gerar_icones(&items_clone, renderer, &font);
            }
            self.menu_stack.push(MenuState::new(items_clone));
        }
    }

    pub(super) fn pop_menu(&mut self) {
        if self.chess_game.is_some() {
            self.chess_game = None;

            self.target_aspect_ratio = self.saved_target_aspect_ratio;
            self.last_width = self.saved_width;
            self.last_height = self.saved_height;
            if let Some(ref renderer) = self.renderer {
                self.pending_size = Some((self.saved_width, self.saved_height));
                let _ = renderer
                    .window
                    .request_inner_size(winit::dpi::PhysicalSize::new(
                        self.saved_width,
                        self.saved_height,
                    ));
            }
            self.resize_layout(self.saved_width, self.saved_height);
            return;
        }

        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
            self.icones_sub.clear();
        }
    }

    pub(super) fn back_menu(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.chess_game.is_some() {
            self.pop_menu();
            return;
        }

        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
            self.icones_sub.clear();
        } else {
            event_loop.exit();
        }
    }

    pub(super) fn card_hit_test(&self, px: f32, py: f32) -> Option<usize> {
        let renderer = self.renderer.as_ref()?;
        let vp_w = renderer.uniforms.screen_size[0];
        let vp_h = renderer.uniforms.screen_size[1];

        let (cx, cy, base_size, gap) = carousel_layout(vp_w, vp_h);

        let menu = self.menu_stack.last()?;

        if menu.animating {
            return None;
        }

        let selected_f = menu.selected as f32;

        let mut melhor_idx: Option<usize> = None;
        let mut melhor_d_abs = f32::MAX;

        for (i, _) in menu.items.iter().enumerate() {
            let d = i as f32 - selected_f;
            if d.abs() > 3.0 {
                continue;
            }

            let scale = (1.0 - d.abs() * 0.18).clamp(0.6, 1.0);
            let size = base_size * scale;
            let x = cx + d * gap - size * 0.5;
            let y = cy - size * 0.5;

            if px >= x && px <= x + size && py >= y && py <= y + size {
                if d.abs() < melhor_d_abs {
                    melhor_d_abs = d.abs();
                    melhor_idx = Some(i);
                }
            }
        }

        melhor_idx
    }
}
