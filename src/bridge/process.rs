use std::env;
use std::io;
use std::process::Stdio;

use nvim_rs::compat::tokio::Compat;
use nvim_rs::create::tokio::new_child_cmd;
use nvim_rs::error::LoopError;
use nvim_rs::Neovim;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use winit::event_loop::EventLoopProxy;

use super::NeovimHandler;
use crate::event::UserEvent;

pub type NvimWriter = Compat<ChildStdin>;

pub struct NeovimProcess {
    pub neovim: Neovim<NvimWriter>,
    pub io_handle: JoinHandle<Result<(), Box<LoopError>>>,
    pub child: Child,
}

impl NeovimProcess {
    pub async fn spawn(event_proxy: EventLoopProxy<UserEvent>) -> io::Result<Self> {
        let nvim_path = find_nvim_path()?;
        let handler = NeovimHandler::new(event_proxy);

        let mut cmd = Command::new(&nvim_path);
        cmd.arg("--embed")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let (neovim, io_handle, child) = new_child_cmd(&mut cmd, handler)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        log::info!("Neovim process spawned: {:?}", nvim_path);

        Ok(Self {
            neovim,
            io_handle,
            child,
        })
    }

    pub async fn quit(&self) -> Result<(), Box<nvim_rs::error::CallError>> {
        self.neovim.command("qa!").await
    }
}

fn find_nvim_path() -> io::Result<String> {
    if let Ok(path) = env::var("NVIM_PATH") {
        return Ok(path);
    }

    let candidates = if cfg!(target_os = "windows") {
        vec![
            "nvim.exe",
            "C:\\Program Files\\Neovim\\bin\\nvim.exe",
            "C:\\Program Files (x86)\\Neovim\\bin\\nvim.exe",
        ]
    } else {
        vec![
            "nvim",
            "/usr/local/bin/nvim",
            "/usr/bin/nvim",
            "/opt/homebrew/bin/nvim",
        ]
    };

    for candidate in &candidates {
        if which_nvim(candidate) {
            return Ok(candidate.to_string());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Neovim executable not found. Set NVIM_PATH or ensure nvim is in PATH.",
    ))
}

fn which_nvim(path: &str) -> bool {
    if path.contains('/') || path.contains('\\') {
        std::path::Path::new(path).exists()
    } else {
        std::process::Command::new("which")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nvim_path_with_env() {
        env::set_var("NVIM_PATH", "/custom/path/nvim");
        let result = find_nvim_path();
        env::remove_var("NVIM_PATH");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/custom/path/nvim");
    }

    #[test]
    fn test_which_nvim_absolute_path_nonexistent() {
        assert!(!which_nvim("/nonexistent/path/to/nvim"));
    }

    #[test]
    fn test_which_nvim_in_path() {
        let result = which_nvim("nvim");
        // Result depends on whether nvim is installed
        // Just verify it doesn't panic
        let _ = result;
    }
}
