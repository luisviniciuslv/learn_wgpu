// src/menu/explorer.rs
use crate::menu::renderer::{Renderer, Texture, rasterizar_texto};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

struct LabelTexture {
    bind_group: Arc<wgpu::BindGroup>,
    width: f32,
    height: f32,
}

pub struct ElementoArquivo {
    pub nome: String,
    pub eh_diretorio: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct CopyInfo {
    path: String,
    is_crop: bool,
}

pub struct FileExplorer {
    pub caminho_atual: PathBuf,
    pub itens: Vec<ElementoArquivo>,
    pub selecionado: Option<usize>, // Substitui o ListState do Ratatui
    scroll_offset: usize,
    viewport_w: f32,
    viewport_h: f32,
    fonte: ab_glyph::FontArc,
    cache_rotulos: HashMap<String, LabelTexture>,
    clipboard_interno: Option<CopyInfo>,
}

impl FileExplorer {
    pub fn new(caminho_inicial: PathBuf) -> Self {
        let caminho_inicial = std::fs::canonicalize(&caminho_inicial).unwrap_or(caminho_inicial);
        let mut explorer = Self {
            caminho_atual: caminho_inicial,
            itens: Vec::new(),
            selecionado: None,
            scroll_offset: 0,
            viewport_w: 1280.0,
            viewport_h: 720.0,
            fonte: carregar_fonte_explorer(),
            cache_rotulos: HashMap::new(),
            clipboard_interno: None,
        };
        explorer.atualizar_arquivos();
        explorer
    }

    fn item_height() -> f32 {
        40.0
    }

    fn start_y() -> f32 {
        70.0
    }

    fn item_padding() -> f32 {
        6.0
    }

    fn slot_height() -> f32 {
        Self::item_height() + Self::item_padding()
    }

    fn visible_capacity(&self) -> usize {
        let avail = (self.viewport_h - Self::start_y()).max(Self::item_height());
        let rows = (avail / Self::slot_height()).floor() as usize;
        rows.max(1)
    }

    fn normalize_selection(&mut self) {
        if self.itens.is_empty() {
            self.selecionado = None;
            self.scroll_offset = 0;
            return;
        }

        self.selecionado = Some(
            self.selecionado
                .unwrap_or(0)
                .min(self.itens.len().saturating_sub(1)),
        );
        self.ensure_selection_visible();
    }

    fn ensure_selection_visible(&mut self) {
        let Some(sel) = self.selecionado else {
            return;
        };
        let cap = self.visible_capacity();
        if sel < self.scroll_offset {
            self.scroll_offset = sel;
            return;
        }
        if sel >= self.scroll_offset + cap {
            self.scroll_offset = sel + 1 - cap;
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.itens.is_empty() {
            return (0, 0);
        }

        let cap = self.visible_capacity();
        let max_start = self.itens.len().saturating_sub(1);
        let start = self.scroll_offset.min(max_start);
        let end = (start + cap).min(self.itens.len());
        (start, end)
    }

    pub fn atualizar_arquivos(&mut self) {
        self.itens.clear();
        self.cache_rotulos.clear();
        if let Ok(entradas) = fs::read_dir(&self.caminho_atual) {
            for entrada in entradas.flatten() {
                let nome = entrada.file_name().to_string_lossy().into_owned();
                let eh_diretorio = entrada.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                self.itens.push(ElementoArquivo { nome, eh_diretorio });
            }
        }
        // Organiza: pastas primeiro, depois arquivos
        self.itens.sort_by(|a, b| {
            b.eh_diretorio
                .cmp(&a.eh_diretorio)
                .then_with(|| a.nome.cmp(&b.nome))
        });
        self.normalize_selection();
    }

    pub fn proximo(&mut self) {
        if self.itens.is_empty() {
            return;
        }
        self.selecionado = Some(match self.selecionado {
            Some(i) => {
                if i >= self.itens.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        });
        self.ensure_selection_visible();
    }

    pub fn anterior(&mut self) {
        if self.itens.is_empty() {
            return;
        }
        self.selecionado = Some(match self.selecionado {
            Some(i) => {
                if i == 0 {
                    self.itens.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        });
        self.ensure_selection_visible();
    }

    pub fn ao_pressionar_enter(&mut self) {
        if let Some(index) = self.selecionado {
            if index < self.itens.len() {
                let item_selecionado = &self.itens[index];
                if item_selecionado.eh_diretorio {
                    self.caminho_atual.push(&item_selecionado.nome);
                    self.atualizar_arquivos();
                }
            }
        }
    }

    pub fn voltar_diretorio(&mut self) {
        if self.caminho_atual.pop() {
            self.atualizar_arquivos();
        }
    }

    pub fn copiar_caminho_de_arquivo_ou_pasta(&mut self, is_crop: bool) {
        if let Some(index) = self.selecionado {
            if index < self.itens.len() {
                let item_selecionado: &ElementoArquivo = &self.itens[index];
                let caminho_completo = self.caminho_atual.join(&item_selecionado.nome);

                let copy_info = CopyInfo {
                    is_crop,
                    path: caminho_completo.to_str().unwrap().to_owned(),
                };

                println!("Copiado (internamente): {:?}", copy_info.path);
                self.clipboard_interno = Some(copy_info);
            }
        }
    }

    pub fn colar_arquivo_ou_pasta_pelo_caminho(&mut self) {
        println!("Tentando colar (internamente)...");
        if let Some(copy_info) = &self.clipboard_interno {
            let caminho_copiado = std::path::PathBuf::from(&copy_info.path);
            if caminho_copiado.exists() {
                let destino = self
                    .caminho_atual
                    .join(caminho_copiado.file_name().unwrap());
                if copy_info.is_crop {
                    if let Err(e) = std::fs::rename(&caminho_copiado, &destino) {
                        eprintln!("Erro ao recortar arquivo: {}", e);
                    } else {
                        // Opcional: limpar clipboard após recortar
                        self.clipboard_interno = None;
                    }
                } else {
                    if let Err(e) = std::fs::copy(&caminho_copiado, &destino) {
                        eprintln!("Erro ao colar arquivo: {}", e);
                    }
                }
                self.atualizar_arquivos();
            } else {
                eprintln!("O caminho copiado não existe: {:?}", caminho_copiado);
            }
        } else {
            eprintln!("Clipboard interno vazio.");
        }
    }

    pub fn deletar_arquivo_ou_pasta(&mut self) {
        if let Some(index) = self.selecionado {
            if index < self.itens.len() {
                let item_selecionado: &ElementoArquivo = &self.itens[index];
                let caminho_completo = self.caminho_atual.join(&item_selecionado.nome);
                if item_selecionado.eh_diretorio {
                    if let Err(e) = std::fs::remove_dir_all(&caminho_completo) {
                        eprintln!("Erro ao deletar arquivo: {}", e);
                    }
                } else {
                    if let Err(e) = std::fs::remove_file(&caminho_completo) {
                        eprintln!("Erro ao deletar arquivo: {}", e);
                    }
                }
                self.atualizar_arquivos();
            }
        }
    }

    pub fn scroll_by_lines(&mut self, delta_lines: i32) {
        if self.itens.is_empty() {
            return;
        }

        let cap = self.visible_capacity();
        let max_start = self.itens.len().saturating_sub(cap);
        let next = (self.scroll_offset as i32 + delta_lines).clamp(0, max_start as i32) as usize;
        self.scroll_offset = next;

        if self.selecionado.is_none() {
            self.selecionado = Some(self.scroll_offset.min(self.itens.len().saturating_sub(1)));
        }
    }

    pub fn click(&mut self, mx: f32, my: f32) -> bool {
        let Some(index) = self.item_at_position(mx, my) else {
            return false;
        };

        if self.selecionado == Some(index) {
            self.ao_pressionar_enter();
        } else {
            self.selecionado = Some(index);
            self.ensure_selection_visible();
        }
        true
    }

    fn item_at_position(&self, mx: f32, my: f32) -> Option<usize> {
        if mx < 20.0 || mx > self.viewport_w - 20.0 || my < Self::start_y() {
            return None;
        }

        let (start, end) = self.visible_range();
        for (row, idx) in (start..end).enumerate() {
            let y = Self::start_y() + row as f32 * Self::slot_height();
            if my >= y && my <= y + Self::item_height() {
                return Some(idx);
            }
        }
        None
    }

    fn obter_rotulo<'a>(&'a mut self, renderer: &Renderer, nome: &str) -> &'a LabelTexture {
        let entry = self
            .cache_rotulos
            .entry(nome.to_owned())
            .or_insert_with(|| {
                let img = rasterizar_texto(&self.fonte, nome, 22.0, [1.0, 1.0, 1.0, 1.0]);
                let width = img.width() as f32;
                let height = img.height() as f32;
                let tex = Texture::from_image_buffer(
                    &renderer.device,
                    &renderer.queue,
                    &img,
                    &renderer.texture_bind_group_layout,
                    &format!("explorer_label_{}", nome),
                );
                LabelTexture {
                    bind_group: tex.bind_group,
                    width,
                    height,
                }
            });
        &*entry
    }

    /// Renderiza o explorador na tela graficamente com WGPU
    pub fn render(&mut self, renderer: &mut Renderer, vp_w: f32, vp_h: f32) -> anyhow::Result<()> {
        self.viewport_w = vp_w;
        self.viewport_h = vp_h;
        self.normalize_selection();

        // Fundo do Explorador (Cinza bem escuro)
        renderer.draw_rect(0.0, 0.0, vp_w, vp_h, [0.06, 0.06, 0.09, 1.0]);

        // Cabeçalho com o Caminho Atual
        renderer.draw_rect(0.0, 0.0, vp_w, 55.0, [0.12, 0.12, 0.18, 1.0]);

        // Renderização dos Itens da Lista
        let item_height = Self::item_height();
        let start_y = Self::start_y();

        let (start, end) = self.visible_range();
        for (row, idx) in (start..end).enumerate() {
            let item_nome = self.itens[idx].nome.clone();
            let item_eh_diretorio = self.itens[idx].eh_diretorio;
            let item_y = start_y + row as f32 * Self::slot_height();

            let eh_selecionado = self.selecionado == Some(idx);

            // Cor de fundo do painel do item
            let bg_color = if eh_selecionado {
                [0.20, 0.40, 0.85, 1.0] // Azul se selecionado
            } else {
                [0.14, 0.14, 0.20, 1.0] // Cor padrão de cartão
            };

            // Cor do indicador geométrico de tipo (substituindo os ícones de texto 📁 e 📄)
            let type_color = if item_eh_diretorio {
                [0.0, 0.75, 0.75, 1.0] // Ciano para Pastas
            } else {
                [0.70, 0.70, 0.70, 1.0] // Cinza claro para Arquivos
            };

            // Desenha o fundo do item
            renderer.draw_rect(20.0, item_y, vp_w - 40.0, item_height, bg_color);

            // Desenha um quadrado indicativo de tipo à esquerda
            renderer.draw_rect(35.0, item_y + 10.0, 20.0, 20.0, type_color);

            let (label_bind_group, label_w, label_h) = {
                let label = self.obter_rotulo(renderer, &item_nome);
                (label.bind_group.clone(), label.width, label.height)
            };

            let max_label_w = (vp_w - 100.0).max(10.0);
            let scale = (max_label_w / label_w).min(1.0);
            let draw_w = (label_w * scale).max(1.0);
            let draw_h = (label_h * scale).max(1.0);
            let text_y = item_y + (item_height - draw_h) * 0.5;

            renderer.draw_textured_rect(
                65.0,
                text_y,
                draw_w,
                draw_h,
                [0.95, 0.95, 0.98, 1.0],
                [0.0, 0.0, scale, 1.0],
                label_bind_group,
            );
        }

        Ok(())
    }
}

fn carregar_fonte_explorer() -> ab_glyph::FontArc {
    let candidatas = [
        "assets/font.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];

    for caminho in &candidatas {
        if let Ok(dados) = std::fs::read(caminho) {
            if let Ok(font) = ab_glyph::FontArc::try_from_vec(dados) {
                return font;
            }
        }
    }

    panic!(
        "Nenhuma fonte encontrada. Coloque um arquivo .ttf em: assets/font.ttf\n\
         Baixe qualquer fonte gratuita e salve com esse nome."
    )
}
