use crate::menu::chess::ChessGame;
use crate::menu::menu::assets::IconeItem;
use crate::menu::menu::layout::{BASE_HEIGHT, BASE_WIDTH};
use crate::menu::renderer::Viewport;
use crate::menu::types::{ArrowButton, MenuItem, MenuState};

// =============================================================================
//  Nucleo do Aplicativo (estado)
// =============================================================================

pub struct App {
    pub(super) renderer: Option<crate::menu::renderer::Renderer>,
    pub(super) menu_stack: Vec<MenuState>,
    pub(super) arrows: Vec<ArrowButton>,

    pub(super) mouse_pos: (f32, f32),
    pub(super) hovered_arrow: Option<crate::menu::types::ArrowId>,
    pub(super) pressed_arrow: Option<crate::menu::types::ArrowId>,
    pub(super) hovered_fullscreen_button: bool,
    pub(super) pressed_fullscreen_button: bool,

    pub(super) viewport: Viewport,
    pub(super) target_aspect_ratio: f32,
    pub(super) last_monitor_name: Option<String>,

    pub(super) last_width: u32,
    pub(super) last_height: u32,
    pub(super) pending_size: Option<(u32, u32)>,

    pub(super) icones: Vec<IconeItem>,
    pub(super) icones_sub: Vec<IconeItem>,

    pub(super) chess_game: Option<ChessGame>,
    pub(super) chess_texture: Option<std::sync::Arc<wgpu::BindGroup>>,

    pub(super) saved_target_aspect_ratio: f32,
    pub(super) saved_width: u32,
    pub(super) saved_height: u32,
    pub(super) last_windowed_size: (u32, u32),
    pub(super) windowed_size_before_fullscreen: Option<(u32, u32)>,
    pub(super) restore_windowed_size_after_fullscreen: Option<(u32, u32)>,
    pub(super) suppress_maximize_to_fullscreen_once: bool,
    pub(super) is_fullscreen: bool,
}

impl App {
    pub fn new() -> Self {
        let root_items = vec![
            MenuItem {
                id: "apps",
                label: "Apps",
                children: Some(vec![
                    MenuItem {
                        id: "calc",
                        label: "Calc",
                        children: None,
                    },
                    MenuItem {
                        id: "paint",
                        label: "Paint",
                        children: None,
                    },
                    MenuItem {
                        id: "notes",
                        label: "Notes",
                        children: None,
                    },
                ]),
            },
            MenuItem {
                id: "games",
                label: "Games",
                children: Some(vec![MenuItem {
                    id: "chess",
                    label: "Chess",
                    children: None,
                }]),
            },
            MenuItem {
                id: "tools",
                label: "Tools",
                children: Some(vec![
                    MenuItem {
                        id: "terminal",
                        label: "Terminal",
                        children: None,
                    },
                    MenuItem {
                        id: "editor",
                        label: "Editor",
                        children: None,
                    },
                ]),
            },
            MenuItem {
                id: "about",
                label: "About",
                children: None,
            },
        ];

        Self {
            renderer: None,
            menu_stack: vec![MenuState::new(root_items)],
            arrows: Vec::new(),
            mouse_pos: (0.0, 0.0),
            hovered_arrow: None,
            pressed_arrow: None,
            hovered_fullscreen_button: false,
            pressed_fullscreen_button: false,
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: BASE_WIDTH,
                height: BASE_HEIGHT,
            },
            target_aspect_ratio: BASE_WIDTH / BASE_HEIGHT,
            last_monitor_name: None,
            last_width: BASE_WIDTH as u32,
            last_height: BASE_HEIGHT as u32,
            pending_size: None,
            icones: Vec::new(),
            icones_sub: Vec::new(),
            chess_game: None,
            chess_texture: None,
            saved_target_aspect_ratio: BASE_WIDTH / BASE_HEIGHT,
            saved_width: BASE_WIDTH as u32,
            saved_height: BASE_HEIGHT as u32,
            last_windowed_size: (BASE_WIDTH as u32, BASE_HEIGHT as u32),
            windowed_size_before_fullscreen: None,
            restore_windowed_size_after_fullscreen: None,
            suppress_maximize_to_fullscreen_once: false,
            is_fullscreen: false,
        }
    }
}
