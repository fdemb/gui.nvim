pub mod bridge;
pub mod cli;
pub mod config;
pub mod constants;
pub mod editor;
pub mod env;
pub mod event;

pub mod input;
pub mod renderer;
pub mod window;

use log::info;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::event::UserEvent;
use crate::window::GuiApp;

pub fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    renderer::font::loader::register_embedded_fonts();

    let config = config::Config::load();
    let mut app = GuiApp::new(proxy, config, args);

    info!("Starting event loop");
    event_loop.run_app(&mut app)?;

    info!("gui.nvim shutting down");
    Ok(())
}
