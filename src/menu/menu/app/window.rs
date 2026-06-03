use crate::menu::menu::layout::BASE_WIDTH;
use crate::menu::renderer::Viewport;
use crate::menu::types::{ArrowButton, ArrowId};
use winit::dpi::PhysicalSize;

use super::state::App;

impl App {
    pub(super) fn apply_pending_window_restore(&mut self) -> bool {
        if self.is_fullscreen {
            return false;
        }

        let (rw, rh) = match self.restore_windowed_size_after_fullscreen {
            Some(v) => v,
            None => return false,
        };

        let renderer = match self.renderer.as_ref() {
            Some(r) => r,
            None => return false,
        };

        let current = renderer.window.inner_size();
        let reached = (current.width as i32 - rw as i32).abs() <= 2
            && (current.height as i32 - rh as i32).abs() <= 2;

        if reached {
            self.restore_windowed_size_after_fullscreen = None;
            self.last_windowed_size = (current.width, current.height);
            return false;
        }

        renderer.window.set_maximized(false);
        self.pending_size = Some((rw, rh));
        let _ = renderer
            .window
            .request_inner_size(PhysicalSize::new(rw, rh));
        true
    }

    pub(super) fn toggle_fullscreen(&mut self) {
        let renderer = match self.renderer.as_ref() {
            Some(r) => r,
            None => return,
        };

        let currently_fullscreen = renderer.window.fullscreen().is_some();

        if currently_fullscreen {
            renderer.window.set_fullscreen(None);

            let restore_size = self
                .windowed_size_before_fullscreen
                .unwrap_or(self.last_windowed_size);
            self.restore_windowed_size_after_fullscreen = Some(restore_size);
            renderer.window.set_maximized(false);

            self.suppress_maximize_to_fullscreen_once = true;
            self.is_fullscreen = false;
        } else {
            let size = renderer.window.inner_size();
            let from_maximized = renderer.window.is_maximized();

            if from_maximized {
                self.windowed_size_before_fullscreen = Some(self.last_windowed_size);
            } else if size.width > 0 && size.height > 0 {
                self.windowed_size_before_fullscreen = Some((size.width, size.height));
                self.last_windowed_size = (size.width, size.height);
            } else {
                self.windowed_size_before_fullscreen = Some(self.last_windowed_size);
            }

            let monitor = renderer.window.current_monitor();
            renderer
                .window
                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
            self.is_fullscreen = true;
        }
    }

    pub(super) fn fullscreen_button_rect(&self) -> (f32, f32, f32, f32) {
        let w = 120.0;
        let h = 36.0;
        let margin = 12.0;
        (self.viewport.width - w - margin, margin, w, h)
    }

    pub(super) fn should_show_fullscreen_button(&self) -> bool {
        let is_fullscreen = self
            .renderer
            .as_ref()
            .map(|r| r.window.fullscreen().is_some())
            .unwrap_or(self.is_fullscreen);
        is_fullscreen && self.mouse_pos.1 <= 72.0
    }

    pub(super) fn update_fullscreen_button_hover(&mut self) {
        if !self.should_show_fullscreen_button() {
            self.hovered_fullscreen_button = false;
            return;
        }

        let (x, y, w, h) = self.fullscreen_button_rect();
        self.hovered_fullscreen_button = self.mouse_pos.0 >= x
            && self.mouse_pos.0 <= x + w
            && self.mouse_pos.1 >= y
            && self.mouse_pos.1 <= y + h;
    }

    pub(super) fn resize_layout(&mut self, window_w: u32, window_h: u32) {
        let win_w = window_w as f32;
        let win_h = window_h as f32;

        let (vp_w, vp_h, vp_x, vp_y) = if self.chess_game.is_some() || self.is_fullscreen {
            (win_w, win_h, 0.0, 0.0)
        } else {
            let ratio = self.target_aspect_ratio;
            let (w, h) = if win_w / win_h > ratio {
                (win_h * ratio, win_h)
            } else {
                (win_w, win_w / ratio)
            };
            (w, h, (win_w - w) * 0.5, (win_h - h) * 0.5)
        };

        self.viewport = Viewport {
            x: vp_x,
            y: vp_y,
            width: vp_w,
            height: vp_h,
        };

        let scale_factor = vp_w / BASE_WIDTH;

        if let Some(ref mut renderer) = self.renderer {
            renderer.update_logical_size(vp_w, vp_h);
        }

        let left_w = 50.0 * scale_factor;
        let left_h = 50.0 * scale_factor;
        let left_x = 80.0 * scale_factor;
        let left_y = (vp_h * 0.5) - (left_h * 0.5);

        let right_w = 50.0 * scale_factor;
        let right_h = 50.0 * scale_factor;
        let right_x = vp_w - (80.0 * scale_factor) - right_w;
        let right_y = (vp_h * 0.5) - (right_h * 0.5);

        let (back_x, back_y, back_w, back_h) = if self.chess_game.is_some() {
            let scale_factor = vp_w.min(vp_h * 0.8) / BASE_WIDTH;
            let header_h = 75.0 * scale_factor;
            let footer_h = 65.0 * scale_factor;
            let available_h = (vp_h - header_h - footer_h).max(10.0);

            let board_size = vp_w.min(available_h) * 0.92;
            let chess_start_y = header_h + (available_h - board_size) * 0.5;

            let b_y = (chess_start_y * 0.25).max(4.0);
            let b_h = (chess_start_y * 0.50).max(16.0).min(40.0 * scale_factor);
            (b_y, b_y, b_h, b_h)
        } else {
            (
                20.0 * scale_factor,
                20.0 * scale_factor,
                40.0 * scale_factor,
                40.0 * scale_factor,
            )
        };

        self.arrows = vec![
            ArrowButton {
                id: ArrowId::Left,
                x: left_x,
                y: left_y,
                w: left_w,
                h: left_h,
            },
            ArrowButton {
                id: ArrowId::Right,
                x: right_x,
                y: right_y,
                w: right_w,
                h: right_h,
            },
            ArrowButton {
                id: ArrowId::Back,
                x: back_x,
                y: back_y,
                w: back_w,
                h: back_h,
            },
        ];
    }

    pub(super) fn physical_to_logical(&self, px: f32, py: f32) -> (f32, f32) {
        (px - self.viewport.x, py - self.viewport.y)
    }
}
