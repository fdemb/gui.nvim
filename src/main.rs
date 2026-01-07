mod bridge;
mod cli;
mod config;
mod constants;
mod editor;
mod env;
mod event;
pub mod font_loader;
mod input;
mod renderer;
mod window;

use log::info;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::cli::CliAction;
use crate::event::UserEvent;
use crate::window::GuiApp;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();

    match cli::parse_args(args) {
        CliAction::Env => match env::dump_env() {
            Ok(count) => {
                if let Some(path) = env::env_file_path() {
                    println!("Captured {} environment variables to:", count);
                    println!("  {}", path.display());
                    println!();
                    println!("These will be loaded automatically when gui.nvim starts.");
                    println!("Re-run this command after changing your shell configuration.");
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error capturing environment: {}", e);
                std::process::exit(1);
            }
        },
        CliAction::Help => {
            cli::print_help();
            std::process::exit(0);
        }
        CliAction::Version => {
            println!("gui.nvim {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        CliAction::Run(nvim_args) => {
            match env::load_env() {
                Ok(Some(count)) => {
                    info!("Loaded {} environment variables from config", count);
                }
                Ok(None) => {
                    info!("No environment file found, using system environment");
                }
                Err(e) => {
                    log::warn!("Failed to load environment file: {}", e);
                }
            }

            info!("gui.nvim starting");

            if let Err(e) = run(nvim_args) {
                log::error!("Application error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    // Register embedded fonts
    font_loader::register_embedded_fonts();

    let config = config::Config::load();
    let mut app = GuiApp::new(proxy, config, args);

    info!("Starting event loop");
    event_loop.run_app(&mut app)?;

    info!("gui.nvim shutting down");
    Ok(())
}
