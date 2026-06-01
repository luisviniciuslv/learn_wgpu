// Demo de menu em carrossel interativo construido com WGPU.
//
// Este modulo concentra apenas o ponto de entrada e organiza os submodulos.

mod app;
mod assets;
mod draw;
mod layout;

use winit::event_loop::EventLoop;

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = app::App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
