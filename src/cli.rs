use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gui.nvim")]
#[command(version)]
#[command(about = "GPU-accelerated Neovim GUI", long_about = None)]
#[command(after_help = "\
ARGS:
    Any other arguments are passed directly to Neovim.

ENVIRONMENT CAPTURE:
    Run `gui.nvim env` from your terminal to capture environment variables.
    This is useful when launching from Finder/Spotlight on macOS, where
    GUI apps don't inherit your shell's PATH and other variables.

    The captured environment includes PATH modifications from version
    managers like nvm, rbenv, pyenv, mise, and asdf.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Arguments passed directly to Neovim
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub nvim_args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Capture shell environment variables for GUI launches
    Env,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_args() {
        let cli = Cli::parse_from(["gui.nvim"]);
        assert!(cli.command.is_none());
        assert!(cli.nvim_args.is_empty());
    }

    #[test]
    fn test_parse_env() {
        let cli = Cli::parse_from(["gui.nvim", "env"]);
        assert!(matches!(cli.command, Some(Command::Env)));
    }

    #[test]
    fn test_parse_nvim_args() {
        let cli = Cli::parse_from(["gui.nvim", "file.txt", "--clean"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.nvim_args, vec!["file.txt", "--clean"]);
    }

    #[test]
    fn test_parse_nvim_args_with_dash() {
        let cli = Cli::parse_from(["gui.nvim", "-c", "echo 'hello'"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.nvim_args, vec!["-c", "echo 'hello'"]);
    }
}
