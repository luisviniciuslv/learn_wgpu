use std::sync::Arc;
use wgpu::util::DeviceExt;

// =============================================================================
//  1. Vértice 2D com posição, cor e coordenadas de textura (UV)
// =============================================================================
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],   // Pixels lógicos: (x, y)
    pub color: [f32; 4],      // RGBA: (r, g, b, a)
    pub tex_coords: [f32; 2], // Coordenadas UV da textura: (u, v)
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2, // @location(0) position
        1 => Float32x4, // @location(1) color
        2 => Float32x2, // @location(2) tex_coords
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// =============================================================================
//  2. Uniforms: tamanho LÓGICO do viewport enviado à GPU
// =============================================================================
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub screen_size: [f32; 2], // Tamanho lógico do viewport em pixels
    pub _pad: [f32; 2],        // Padding obrigatório para alinhamento de 16 bytes na GPU
}

// =============================================================================
//  3. Viewport físico — região da janela onde desenhamos (calculado no resize)
// =============================================================================
#[derive(Clone, Copy, Debug, Default)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// =============================================================================
//  4. Abstração de Textura na GPU
//
//  Encapsula: textura, view, sampler e o bind_group que os une.
//  O bind_group fica em Arc para poder ser clonado e compartilhado entre frames.
// =============================================================================
// Os campos são mantidos em memória para preservar a ownership dos recursos da GPU.
// Sem eles, a textura seria liberada da GPU antes do uso.
#[allow(dead_code)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Arc<wgpu::BindGroup>,
}

impl Texture {
    /// Cria uma textura a partir de um buffer de pixels RGBA já carregado em memória.
    pub fn from_image_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::RgbaImage,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> Self {
        let (w, h) = img.dimensions();
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{}_bind_group", label)),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group: Arc::new(bind_group),
        }
    }

    /// Cria um pixel branco 1×1 — usado quando não queremos textura real (cor sólida).
    /// `cor_do_vértice * branco = cor_do_vértice`, portanto o comportamento é idêntico ao shader sem textura.
    pub fn create_1x1_white(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        Self::from_image_buffer(device, queue, &img, layout, "white_1x1")
    }
}

// =============================================================================
//  5. Lote de renderização (Batch)
//
//  Agrupa retângulos que compartilham o mesmo bind_group (mesma textura).
//  Isso evita trocar de textura a cada draw call — mais eficiente na GPU.
// =============================================================================
pub struct Batch {
    pub bind_group: Arc<wgpu::BindGroup>,
    pub index_start: u32,
    pub index_count: u32,
}

// =============================================================================
//  6. O Motor 2D
// =============================================================================
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub window: Arc<winit::window::Window>,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    // Layout usado para criar bind_groups de texturas (Grupo 1 do shader)
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    // Textura 1×1 branca — usada por draw_rect (cor sólida sem textura)
    pub default_texture: Texture,

    // Acumuladores de geometria do frame corrente (CPU → GPU no present())
    vertices_data: Vec<Vertex>,
    indices_data: Vec<u16>,

    // Lotes do frame: cada lote = uma textura diferente
    batches: Vec<Batch>,
    active_bind_group: Option<Arc<wgpu::BindGroup>>,
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

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

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // --- Grupo 0: Uniforms (Vertex shader) ---
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

        // --- Grupo 1: Texturas (Fragment shader) ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // --- Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                // Grupo 0 = uniforms (vertex), Grupo 1 = textura (fragment)
                bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
                immediate_size: 0,
            });

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

        // Buffers grandes o suficiente para UI complexa (ícones + texto + botões)
        let max_vertices = 5000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let max_indices = 8000;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Index Buffer"),
            size: (max_indices * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Textura branca padrão para draw_rect (cor sólida)
        let default_texture =
            Texture::create_1x1_white(&device, &queue, &texture_bind_group_layout);

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
            texture_bind_group_layout,
            default_texture,
            vertices_data: Vec::new(),
            indices_data: Vec::new(),
            batches: Vec::new(),
            active_bind_group: None,
        })
    }

    /// Reconfigura a superfície WGPU com as dimensões FÍSICAS da janela.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    /// Atualiza o uniform screen_size com as dimensões LÓGICAS do viewport.
    /// O shader usa isso para converter pixels lógicos → clip space corretamente.
    pub fn update_logical_size(&mut self, logical_w: f32, logical_h: f32) {
        self.uniforms.screen_size = [logical_w, logical_h];
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    /// Descarta toda a geometria acumulada e prepara para um novo frame.
    pub fn clear(&mut self) {
        self.vertices_data.clear();
        self.indices_data.clear();
        self.batches.clear();
        self.active_bind_group = None;
    }

    // -------------------------------------------------------------------------
    //  API de Desenho
    // -------------------------------------------------------------------------

    /// Desenha um retângulo de cor sólida (sem textura).
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let bg = self.default_texture.bind_group.clone();
        // UV [0,0,1,1] cobre toda a textura 1×1 branca → resultado = cor pura
        self.draw_rect_impl(x, y, w, h, color, [0.0, 0.0, 1.0, 1.0], bg);
    }

    /// Desenha um triângulo sólido com três vértices em pixels LÓGICOS.
    ///
    /// Os pontos devem estar em sentido anti-horário para ficarem visíveis
    /// (o pipeline tem cull_mode = None então ambos os sentidos funcionam).
    pub fn draw_triangle(&mut self, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], color: [f32; 4]) {
        let bg = self.default_texture.bind_group.clone();

        // Mesmo mecanismo de batch que draw_rect: usa a textura branca 1×1
        let need_new_batch = match &self.active_bind_group {
            Some(active) => !Arc::ptr_eq(active, &bg),
            None => true,
        };
        if need_new_batch {
            if let Some(active) = self.active_bind_group.take() {
                let start = self
                    .batches
                    .last()
                    .map(|b| b.index_start + b.index_count)
                    .unwrap_or(0);
                let count = self.indices_data.len() as u32 - start;
                if count > 0 {
                    self.batches.push(Batch {
                        bind_group: active,
                        index_start: start,
                        index_count: count,
                    });
                }
            }
            self.active_bind_group = Some(bg);
        }

        let base = self.vertices_data.len() as u16;
        let uv = [0.5, 0.5]; // ponto central do pixel branco — cor pura

        self.vertices_data.extend_from_slice(&[
            Vertex {
                position: p0,
                color,
                tex_coords: uv,
            },
            Vertex {
                position: p1,
                color,
                tex_coords: uv,
            },
            Vertex {
                position: p2,
                color,
                tex_coords: uv,
            },
        ]);
        self.indices_data
            .extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Desenha um retângulo texturizado com UVs customizadas.
    ///
    /// `uvs` = [min_u, min_v, max_u, max_v] — região da textura a amostrar.
    /// `color` funciona como tinte: [1,1,1,1] exibe a textura sem modificação.
    pub fn draw_textured_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        uvs: [f32; 4],
        bind_group: Arc<wgpu::BindGroup>,
    ) {
        self.draw_rect_impl(x, y, w, h, color, uvs, bind_group);
    }

    /// Implementação interna: acumula geometria e detecta mudanças de textura.
    ///
    /// Quando a textura muda, fecha o lote atual e abre um novo.
    /// Isso agrupa draw calls por textura — minimizando trocas de estado na GPU.
    fn draw_rect_impl(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        uvs: [f32; 4],
        bind_group: Arc<wgpu::BindGroup>,
    ) {
        // Verifica se a textura mudou desde o último retângulo
        let need_new_batch = match &self.active_bind_group {
            Some(active) => !Arc::ptr_eq(active, &bind_group),
            None => true, // Primeiro retângulo do frame: cria o primeiro lote
        };

        if need_new_batch {
            // Fecha o lote anterior (se existir) antes de abrir um novo
            if let Some(active) = self.active_bind_group.take() {
                let start = self
                    .batches
                    .last()
                    .map(|b| b.index_start + b.index_count)
                    .unwrap_or(0);
                let count = self.indices_data.len() as u32 - start;
                if count > 0 {
                    self.batches.push(Batch {
                        bind_group: active,
                        index_start: start,
                        index_count: count,
                    });
                }
            }
            self.active_bind_group = Some(bind_group);
        }

        // Acumula a geometria do retângulo (2 triângulos = 4 vértices + 6 índices)
        let base_index = self.vertices_data.len() as u16;

        let v0 = Vertex {
            position: [x, y],
            color,
            tex_coords: [uvs[0], uvs[1]],
        }; // sup. esq.
        let v1 = Vertex {
            position: [x, y + h],
            color,
            tex_coords: [uvs[0], uvs[3]],
        }; // inf. esq.
        let v2 = Vertex {
            position: [x + w, y + h],
            color,
            tex_coords: [uvs[2], uvs[3]],
        }; // inf. dir.
        let v3 = Vertex {
            position: [x + w, y],
            color,
            tex_coords: [uvs[2], uvs[1]],
        }; // sup. dir.

        self.vertices_data.extend_from_slice(&[v0, v1, v2, v3]);
        self.indices_data.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    /// Submete o frame acumulado à GPU e exibe na tela.
    ///
    /// O `viewport` define a região FÍSICA da janela onde desenhamos.
    /// O que estiver fora do viewport aparece com a cor de clear (preto = letterbox/pillarbox).
    pub fn present(&mut self, viewport: Viewport) -> anyhow::Result<()> {
        if !self.is_surface_configured || self.vertices_data.is_empty() {
            return Ok(());
        }

        // Fecha o último lote ativo antes de renderizar
        if let Some(active) = self.active_bind_group.take() {
            let start = self
                .batches
                .last()
                .map(|b| b.index_start + b.index_count)
                .unwrap_or(0);
            let count = self.indices_data.len() as u32 - start;
            if count > 0 {
                self.batches.push(Batch {
                    bind_group: active,
                    index_start: start,
                    index_count: count,
                });
            }
        }

        // Envia os buffers de CPU para a GPU.
        // WGPU exige que o tamanho do write_buffer seja múltiplo de 4 bytes (COPY_BUFFER_ALIGNMENT).
        // Cada vértice tem tamanho fixo (múltiplo de 4), então vertex_buffer é sempre seguro.
        // Os índices são u16 (2 bytes), então um número ímpar de índices = tamanho ímpar de u16s
        // que pode gerar tamanhos não alinhados. Padding com um índice extra se necessário.
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.vertices_data),
        );
        if self.indices_data.len() % 2 == 0 {
            // Número par de u16 → tamanho em bytes múltiplo de 4 → seguro
            self.queue.write_buffer(
                &self.index_buffer,
                0,
                bytemuck::cast_slice(&self.indices_data),
            );
        } else {
            // Número ímpar de u16 → adiciona um índice de padding (não usado em nenhum batch)
            let mut padded = self.indices_data.clone();
            padded.push(0);
            self.queue
                .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&padded));
        }

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => anyhow::bail!("Erro ao obter textura: {:?}", e),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("2D Command Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("2D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Limpa com preto — cria o efeito letterbox/pillarbox fora do viewport
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
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Define a região física da janela onde pixels serão escritos
            render_pass.set_viewport(
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                0.0,
                1.0,
            );
            // Scissor garante que nenhum pixel vaze para fora do viewport
            render_pass.set_scissor_rect(
                viewport.x as u32,
                viewport.y as u32,
                viewport.width as u32,
                viewport.height as u32,
            );

            // Um draw_indexed por lote — troca a textura antes de cada um
            for batch in &self.batches {
                render_pass.set_bind_group(1, &*batch.bind_group, &[]);
                render_pass.draw_indexed(
                    batch.index_start..(batch.index_start + batch.index_count),
                    0,
                    0..1,
                );
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// =============================================================================
//  Funções auxiliares para geração de imagens (ícones e texto)
// =============================================================================

/// Carrega um PNG do disco. Se não encontrar, gera um ícone fallback colorido.
///
/// Assim o programa nunca trava por um asset faltando — útil durante desenvolvimento.
pub fn carregar_png_ou_fallback(caminho: &str, cor_fallback: [u8; 4]) -> image::RgbaImage {
    match image::open(caminho) {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            println!(
                "⚠️ Erro ao carregar '{}': {:?}\n   Procurando em: {:?}",
                caminho,
                e,
                std::env::current_dir()
            );
            // Ícone geométrico 64×64: borda + "×" diagonal centrado
            let mut img = image::RgbaImage::new(64, 64);
            for (x, y, pixel) in img.enumerate_pixels_mut() {
                let borda = x < 4 || x > 59 || y < 4 || y > 59;
                let cruz = (x as i32 - y as i32).abs() < 2 || (x as i32 + y as i32 - 63).abs() < 2;
                if borda || cruz {
                    *pixel = image::Rgba(cor_fallback);
                } else {
                    *pixel = image::Rgba([40, 40, 46, 255]);
                }
            }
            img
        }
    }
}

/// Rasteriza texto vetorial usando `ab_glyph` e retorna uma imagem RGBA com antialiasing.
///
/// A imagem pode ser carregada como textura via `Texture::from_image_buffer`.
pub fn rasterizar_texto(
    font: &ab_glyph::FontArc,
    text: &str,
    scale_px: f32,
    color: [f32; 4],
) -> image::RgbaImage {
    use ab_glyph::{Font, ScaleFont};

    let scaled = font.as_scaled(scale_px);

    // Calcula a largura total somando os avanços horizontais de cada glyph
    let mut largura_total = 0.0f32;
    let mut ultimo_glyph_id = None;
    for c in text.chars() {
        if c.is_control() {
            continue;
        }
        let gid = font.glyph_id(c);
        if let Some(_ult) = ultimo_glyph_id {
            // kerning não está disponível diretamente em FontArc; usamos apenas h_advance
        }
        largura_total += scaled.h_advance(gid);
        ultimo_glyph_id = Some(gid);
    }

    let altura = (scaled.ascent() - scaled.descent()).ceil() as u32;
    let largura = largura_total.ceil() as u32;
    let mut img = image::RgbaImage::new(largura.max(1), altura.max(1));

    let cor_u8 = [
        (color[0] * 255.0).clamp(0.0, 255.0) as u8,
        (color[1] * 255.0).clamp(0.0, 255.0) as u8,
        (color[2] * 255.0).clamp(0.0, 255.0) as u8,
        (color[3] * 255.0).clamp(0.0, 255.0) as u8,
    ];

    // Rasteriza cada glyph posicionando a baseline corretamente
    let mut caret_x = 0.0f32;
    for c in text.chars() {
        if c.is_control() {
            continue;
        }
        let gid = font.glyph_id(c);
        let pos = ab_glyph::point(caret_x, scaled.ascent());
        let glyph = gid.with_scale_and_position(scale_px, pos);
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let px = (bounds.min.x as i32 + x as i32).max(0) as u32;
                let py = (bounds.min.y as i32 + y as i32).max(0) as u32;
                if px < largura && py < altura {
                    let pixel = img.get_pixel_mut(px, py);
                    let alpha = (coverage * cor_u8[3] as f32) as u8;
                    if alpha > pixel[3] {
                        *pixel = image::Rgba([cor_u8[0], cor_u8[1], cor_u8[2], alpha]);
                    }
                }
            });
        }
        caret_x += scaled.h_advance(gid);
    }

    img
}
