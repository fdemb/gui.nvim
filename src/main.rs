use gui_nvim::cli::CliAction;
use gui_nvim::{cli, env, run};
use log::info;

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
