use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

// Estado central de GPU e janela para renderizacao.
pub struct State {
    // Surface da janela onde os frames sao apresentados.
    surface: wgpu::Surface<'static>,
    // Device de GPU usado para criar recursos e encoders.
    device: wgpu::Device,
    // Queue usada para submeter command buffers.
    queue: wgpu::Queue,
    // Configuracao de apresentacao da surface.
    config: wgpu::SurfaceConfiguration,
    // True quando a surface esta configurada com tamanho.
    is_surface_configured: bool,
    // Cor usada para limpar o frame.
    clear_color: wgpu::Color,
    // Handle compartilhado da janela da aplicacao.
    window: Arc<Window>,
}

impl State {
    // Monta o estado de GPU ligado a janela informada.
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Instance e o ponto de entrada para os backends graficos.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        });

        // Surface e o alvo nativo de apresentacao da janela.
        let surface = instance.create_surface(window.clone()).unwrap();

        // Escolhe um adapter compativel com a surface.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        // Solicita device logico e queue a partir do adapter.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;
        let surface_caps = surface.get_capabilities(&adapter);
        // Prefere formato sRGB para cores corretas.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        // Configura a surface com o tamanho atual da janela.
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            clear_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            window,
        })
    }

    // Atualiza a configuracao quando o tamanho da janela muda.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    // Renderiza um frame.
    fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        // Nao renderiza sem surface configurada.
        if !self.is_surface_configured {
            return Ok(());
        }
            
        let output = match self.surface.get_current_texture() {
            Ok(surface_texture) => surface_texture,
            Err(wgpu::SurfaceError::Outdated) => {
                // Surface desatualizada, reconfigura e pula o frame.
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Lost) => {
                // Poderia recriar o device e recursos aqui, mas saimos.
                anyhow::bail!("Lost device");
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                anyhow::bail!("Out of memory");
            }
            Err(_) => {
                // Timeout ou erros nao fatais: pula o frame.
                return Ok(());
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            // Limpa o frame com uma cor solida.
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color.r,
                            g: self.clear_color.g,
                            b: self.clear_color.b,
                            a: self.clear_color.a,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        // Envia comandos e apresenta o frame.
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    // Atualiza a cor de limpeza com base na posicao do mouse.
    fn handle_mouse_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }

        let x = (position.x / self.config.width as f64).clamp(0.0, 1.0);
        let y = (position.y / self.config.height as f64).clamp(0.0, 1.0);

        self.clear_color = wgpu::Color {
            r: x,
            g: y,
            b: 0.5,
            a: 1.0,
        };
    }

    // Avanca o estado de simulacao fora do render.
    fn update(&mut self) {
        // Espaco para logica de jogo ou animacoes.
    }
}

// Entrada da aplicacao que possui o State opcional.
pub struct App {
    state: Option<State>,
}

impl App {
    // Cria o app vazio; State e inicializado no resume.
    pub fn new() -> Self {
        Self {
            state: None,
        }
    }
}

impl ApplicationHandler<State> for App {
    // Encaminha eventos de janela para o State.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        // Loga o erro e encerra de forma limpa.
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                println!("Mouse moved to: ({:.2}, {:.2})", position.x, position.y);
                state.handle_mouse_moved(position);
                state.window.request_redraw();
            },
            
            _ => {}
        }
    }

    // Cria a janela e o estado de GPU quando a app volta.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Inicializa o State de forma sincrona no desktop.
        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }
}

// Monta e executa o event loop do winit.
pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

// Ponto de entrada do programa.
fn main() {
    run().unwrap();
}
