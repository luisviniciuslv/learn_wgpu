use std::sync::Arc;

use winit::{
    application::ApplicationHandler, event::*, event_loop::ActiveEventLoop, keyboard::PhysicalKey,
    window::Window,
};

use crate::menu::menu::assets::{carregar_fonte, gerar_icones};
use crate::menu::menu::layout::{BASE_HEIGHT, BASE_WIDTH};
use crate::menu::renderer::Renderer;

use super::state::App;

impl ApplicationHandler for App {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 || self.renderer.is_none() {
                    return;
                }

                if let Some(renderer) = self.renderer.as_ref() {
                    self.is_fullscreen = renderer.window.fullscreen().is_some();
                }

                if !self.is_fullscreen {
                    if self.apply_pending_window_restore() {
                        return;
                    }
                }

                if !self.is_fullscreen {
                    if self.suppress_maximize_to_fullscreen_once
                        || self.restore_windowed_size_after_fullscreen.is_some()
                    {
                        self.suppress_maximize_to_fullscreen_once = false;
                    } else if let Some(renderer) = self.renderer.as_ref() {
                        if renderer.window.is_maximized() {
                            self.toggle_fullscreen();
                            return;
                        }
                    }
                }

                if self.is_fullscreen {
                    if let Some(ref mut renderer) = self.renderer {
                        renderer.resize(size.width, size.height);
                    }
                    self.resize_layout(size.width, size.height);
                    self.last_width = size.width;
                    self.last_height = size.height;
                    return;
                }

                let is_our_own_request = self
                    .pending_size
                    .map(|(pw, ph)| {
                        (size.width as i32 - pw as i32).abs() <= 2
                            && (size.height as i32 - ph as i32).abs() <= 2
                    })
                    .unwrap_or(false);

                if is_our_own_request {
                    self.pending_size = None;
                } else {
                    let dw = (size.width as i32 - self.last_width as i32).abs();
                    let dh = (size.height as i32 - self.last_height as i32).abs();
                    let ratio = self.target_aspect_ratio;

                    let (new_w, new_h) = if dw >= dh {
                        let h = (size.width as f32 / ratio).round() as u32;
                        (size.width, h.max(1))
                    } else {
                        let w = (size.height as f32 * ratio).round() as u32;
                        (w.max(1), size.height)
                    };

                    if new_w != size.width || new_h != size.height {
                        if let Some(ref renderer) = self.renderer {
                            let _ = renderer
                                .window
                                .request_inner_size(winit::dpi::PhysicalSize::new(new_w, new_h));
                        }
                        self.pending_size = Some((new_w, new_h));
                        self.last_width = size.width;
                        self.last_height = size.height;
                    }
                }

                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.resize_layout(size.width, size.height);
                self.last_width = size.width;
                self.last_height = size.height;

                if let Some(renderer) = self.renderer.as_ref() {
                    if !renderer.window.is_maximized()
                        && self.restore_windowed_size_after_fullscreen.is_none()
                    {
                        self.last_windowed_size = (size.width, size.height);
                    }
                }
            }

            WindowEvent::Moved(_) => {
                let update = self.renderer.as_ref().and_then(|renderer| {
                    renderer.window.current_monitor().and_then(|monitor| {
                        let monitor_name = monitor.name();
                        if monitor_name != self.last_monitor_name {
                            let mon_size = monitor.size();
                            let win_size = renderer.window.inner_size();
                            Some((monitor_name, mon_size, win_size))
                        } else {
                            None
                        }
                    })
                });

                if let Some((monitor_name, mon_size, win_size)) = update {
                    self.target_aspect_ratio = mon_size.width as f32 / mon_size.height as f32;
                    self.last_monitor_name = monitor_name;
                    self.resize_layout(win_size.width, win_size.height);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.handle_keyboard(event_loop, key_code),

            WindowEvent::CursorMoved { position, .. } => self.handle_cursor_moved(position),

            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse_input(state, button),

            WindowEvent::RedrawRequested => {
                if self.apply_pending_window_restore() {
                    if let Some(ref renderer) = self.renderer {
                        renderer.window.request_redraw();
                    }
                    return;
                }

                self.update();
                if let Err(e) = self.render() {
                    let msg = format!("{:?}", e);
                    if !msg.contains("Outdated")
                        && !msg.contains("Timeout")
                        && !msg.contains("Lost")
                        && !msg.contains("Other")
                    {
                        eprintln!("Erro Critico de Renderizacao: {:?}", e);
                        event_loop.exit();
                    }
                }
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());

        let mut init_width = BASE_WIDTH as u32;
        let mut init_height = BASE_HEIGHT as u32;

        if let Some(ref m) = monitor {
            let size = m.size();
            init_width = (size.width as f32 * 0.27) as u32;
            init_height = (size.height as f32 * 0.27) as u32;
            self.target_aspect_ratio = size.width as f32 / size.height as f32;
            self.last_monitor_name = m.name();
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("2D UI Engine com WGPU")
                        .with_inner_size(winit::dpi::PhysicalSize::new(init_width, init_height))
                        .with_resizable(true),
                )
                .unwrap(),
        );

        let renderer = pollster::block_on(Renderer::new(window)).unwrap();

        let font = carregar_fonte();
        let root_items = self.menu_stack[0].items.clone();
        self.icones = gerar_icones(&root_items, &renderer, &font);

        let img_chess = crate::menu::renderer::carregar_png_ou_fallback(
            "assets/chess_pieces.png",
            [255, 255, 255, 255],
        );
        let tex_chess = crate::menu::renderer::Texture::from_image_buffer(
            &renderer.device,
            &renderer.queue,
            &img_chess,
            &renderer.texture_bind_group_layout,
            "chess_pieces_spritesheet",
        );
        self.chess_texture = Some(tex_chess.bind_group);

        let size = renderer.window.inner_size();
        self.last_width = size.width;
        self.last_height = size.height;
        self.last_windowed_size = (size.width, size.height);
        self.windowed_size_before_fullscreen = Some((size.width, size.height));
        self.renderer = Some(renderer);
        self.resize_layout(size.width, size.height);
    }
}
