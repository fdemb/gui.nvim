#[derive(Debug, PartialEq)]
pub enum CliAction {
    Run(Vec<String>),
    Env,
    Help,
    Version,
}

pub fn parse_args(args: Vec<String>) -> CliAction {
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

pub fn print_help() {
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
