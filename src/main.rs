use clap::Parser;
use gui_nvim::cli::{Cli, Command};
use gui_nvim::{env, run};
use log::info;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Env) => match env::dump_env() {
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
        None => {
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

            if let Err(e) = run(cli.nvim_args) {
                log::error!("Application error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
