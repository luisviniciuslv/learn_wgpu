use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

use crate::menu::types::ArrowId;

use super::state::App;

impl App {
    pub(super) fn handle_keyboard(&mut self, event_loop: &ActiveEventLoop, key_code: KeyCode) {
        match key_code {
            KeyCode::F11 => self.toggle_fullscreen(),
            KeyCode::Escape => self.back_menu(event_loop),
            KeyCode::ArrowLeft => self.move_selection(-1),
            KeyCode::ArrowRight => self.move_selection(1),
            KeyCode::Enter | KeyCode::Space => self.try_enter_submenu(),
            _ => {}
        }
    }

    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_pos = self.physical_to_logical(position.x as f32, position.y as f32);
        self.update_fullscreen_button_hover();
        if let Some(ref renderer) = self.renderer {
            renderer.window.request_redraw();
        }
    }

    pub(super) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                self.update_fullscreen_button_hover();
                if self.should_show_fullscreen_button() && self.hovered_fullscreen_button {
                    self.pressed_fullscreen_button = true;
                    self.toggle_fullscreen();
                    if let Some(ref renderer) = self.renderer {
                        renderer.window.request_redraw();
                    }
                    return;
                }

                if let Some(arrow) = self.hovered_arrow {
                    self.pressed_arrow = Some(arrow);
                    match arrow {
                        ArrowId::Left => self.move_selection(-1),
                        ArrowId::Right => self.move_selection(1),
                        ArrowId::Back => self.pop_menu(),
                    }
                } else {
                    let mx = self.mouse_pos.0;
                    let my = self.mouse_pos.1;
                    if let Some(idx) = self.card_hit_test(mx, my) {
                        let selected = self.menu_stack.last().map(|m| m.selected).unwrap_or(0);
                        if idx as i32 == selected {
                            self.try_enter_submenu();
                        } else {
                            let delta = idx as i32 - selected;
                            self.move_selection(delta);
                        }
                    }
                }
            } else {
                self.pressed_arrow = None;
                self.pressed_fullscreen_button = false;
                if self.chess_game.is_some() {
                    if let Some((pw, ph)) = self.pending_size {
                        if let Some(ref renderer) = self.renderer {
                            let _ = renderer
                                .window
                                .request_inner_size(winit::dpi::PhysicalSize::new(pw, ph));
                        }
                    }
                }
            }
        }
        if let Some(ref renderer) = self.renderer {
            renderer.window.request_redraw();
        }
    }
}
