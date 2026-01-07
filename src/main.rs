mod bridge;
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

use crate::event::UserEvent;
use crate::window::GuiApp;

#[derive(Debug, PartialEq)]
enum CliAction {
    Run(Vec<String>),
    Env,
    Help,
    Version,
}

fn parse_args(args: Vec<String>) -> CliAction {
    if args.len() > 1 {
        match args[1].as_str() {
            "env" => CliAction::Env,
            "--help" | "-h" => CliAction::Help,
            "--version" | "-V" => CliAction::Version,
            _ => CliAction::Run(args[1..].to_vec()),
        }
    } else {
        CliAction::Run(Vec::new())
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();

    match parse_args(args) {
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
            print_help();
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

fn print_help() {
    println!("gui.nvim {}", env!("CARGO_PKG_VERSION"));
    println!("GPU-accelerated Neovim GUI");
    println!();
    println!("USAGE:");
    println!("    gui.nvim [COMMAND] [ARGS...]");
    println!();
    println!("COMMANDS:");
    println!("    env         Capture shell environment variables for GUI launches");
    println!("    (none)      Start the GUI (default)");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Print this help message");
    println!("    -V, --version   Print version information");
    println!();
    println!("ARGS:");
    println!("    Any other arguments are passed directly to Neovim.");
    println!();
    println!("ENVIRONMENT CAPTURE:");
    println!("    Run `gui.nvim env` from your terminal to capture environment variables.");
    println!("    This is useful when launching from Finder/Spotlight on macOS, where");
    println!("    GUI apps don't inherit your shell's PATH and other variables.");
    println!();
    println!("    The captured environment includes PATH modifications from version");
    println!("    managers like nvm, rbenv, pyenv, mise, and asdf.");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_no_args() {
        let args = vec!["gui.nvim".to_string()];
        assert_eq!(parse_args(args), CliAction::Run(vec![]));
    }

    #[test]
    fn test_parse_args_help() {
        let args = vec!["gui.nvim".to_string(), "--help".to_string()];
        assert_eq!(parse_args(args), CliAction::Help);
    }

    #[test]
    fn test_parse_args_version() {
        let args = vec!["gui.nvim".to_string(), "-V".to_string()];
        assert_eq!(parse_args(args), CliAction::Version);
    }

    #[test]
    fn test_parse_args_env() {
        let args = vec!["gui.nvim".to_string(), "env".to_string()];
        assert_eq!(parse_args(args), CliAction::Env);
    }

    #[test]
    fn test_parse_args_nvim_args() {
        let args = vec![
            "gui.nvim".to_string(),
            "file.txt".to_string(),
            "--clean".to_string(),
        ];
        assert_eq!(
            parse_args(args),
            CliAction::Run(vec!["file.txt".to_string(), "--clean".to_string()])
        );
    }
}
