mod bridge;
mod editor;
mod event;
mod input;
mod renderer;
mod window;

use log::info;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::event::UserEvent;
use crate::window::GuiApp;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("gui.nvim starting");

    if let Err(e) = run() {
        log::error!("Application error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = GuiApp::new(proxy);

    info!("Starting event loop");
    event_loop.run_app(&mut app)?;

    info!("gui.nvim shutting down");
    Ok(())
}
