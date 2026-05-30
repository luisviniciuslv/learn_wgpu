use std::sync::Arc;
use wgpu::util::DeviceExt;

// 1. Representação de um vértice simples para renderização 2D
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2], // Posição em pixels lógicos: (x, y)
    pub color: [f32; 4],    // Cor RGBA: (r, g, b, a)
}

impl Vertex {
    // Descreve o layout deste vértice para a GPU saber como interpretar os bytes
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2, // @location(0) position
        1 => Float32x4, // @location(1) color
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// 2. Uniform contendo o tamanho LÓGICO do viewport (não da janela física)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub screen_size: [f32; 2], // Tamanho lógico do viewport em pixels
    pub _pad: [f32; 2],        // Preenchimento obrigatório para alinhamento de 16 bytes na GPU
}

// 3. Viewport físico calculado a cada resize
#[derive(Clone, Copy, Debug, Default)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// 4. O Renderer Simplificado
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub window: Arc<winit::window::Window>,

    // Recursos de renderização
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    // Buffers locais de CPU para acumular os retângulos que desenhamos no frame
    vertices_data: Vec<Vertex>,
    indices_data: Vec<u16>,
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Inicializar a GPU básica
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

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

        // Carregar o Shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // Uniform inicial — o tamanho lógico será atualizado a cada resize
        let uniforms = Uniforms {
            screen_size: [size.width as f32, size.height as f32],
            _pad: [0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline Layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                immediate_size: 0,
            });

        // Criar Pipeline de Renderização
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Criar buffers dinâmicos para vértices e índices da GPU
        let max_vertices = 1000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let max_indices = 1500;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Index Buffer"),
            size: (max_indices * 2) as u64, // u16 tem 2 bytes
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            vertices_data: Vec::new(),
            indices_data: Vec::new(),
        })
    }

    /// Reconfigura a superfície WGPU com as dimensões FÍSICAS da janela.
    /// O uniform screen_size é atualizado separadamente via update_logical_size().
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    /// Atualiza o uniform screen_size com as dimensões LÓGICAS do viewport.
    /// Isso permite que o shader converta corretamente pixels lógicos → clip space,
    /// independentemente do tamanho físico da janela.
    pub fn update_logical_size(&mut self, logical_w: f32, logical_h: f32) {
        self.uniforms.screen_size = [logical_w, logical_h];
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    /// Limpa a tela localmente para iniciar a emissão de geometrias no frame.
    pub fn clear(&mut self) {
        self.vertices_data.clear();
        self.indices_data.clear();
    }

    /// Desenha um retângulo simples passando coordenadas em PIXELS LÓGICOS do viewport.
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let base_index = self.vertices_data.len() as u16;

        // Vértices do retângulo (Sentido anti-horário)
        let v0 = Vertex { position: [x, y], color };         // Canto sup. esquerdo
        let v1 = Vertex { position: [x, y + h], color };     // Canto inf. esquerdo
        let v2 = Vertex { position: [x + w, y + h], color }; // Canto inf. direito
        let v3 = Vertex { position: [x + w, y], color };     // Canto sup. direito

        self.vertices_data.extend_from_slice(&[v0, v1, v2, v3]);

        // Índices para fechar os dois triângulos do retângulo
        self.indices_data.extend_from_slice(&[
            base_index + 0, base_index + 1, base_index + 2,
            base_index + 0, base_index + 2, base_index + 3,
        ]);
    }

    /// Submete a renderização finalizada à GPU.
    /// O parâmetro `viewport` define a região FÍSICA da janela onde o conteúdo será desenhado.
    /// O restante da janela será preenchido com a cor de clear (preto = letterbox/pillarbox).
    pub fn present(&mut self, viewport: Viewport) -> anyhow::Result<()> {
        if !self.is_surface_configured {
            return Ok(());
        }

        if self.vertices_data.is_empty() {
            return Ok(());
        }

        // Escrever os dados acumulados de CPU nos buffers dedicados da GPU
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.vertices_data),
        );
        self.queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(&self.indices_data),
        );

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => anyhow::bail!("Erro ao obter textura: {:?}", e),
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("2D Command Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("2D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Limpa toda a janela com preto — cria o efeito de barra de letterbox/pillarbox
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // Define o viewport físico: apenas esta região da janela receberá pixels desenhados.
            // Tudo fora fica com a cor de clear (preto) — letterbox automático!
            render_pass.set_viewport(
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                0.0,
                1.0,
            );

            // Aplica scissor rect igual ao viewport para não vazar pixels fora da área
            render_pass.set_scissor_rect(
                viewport.x as u32,
                viewport.y as u32,
                viewport.width as u32,
                viewport.height as u32,
            );

            let indices_count = self.indices_data.len() as u32;
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
