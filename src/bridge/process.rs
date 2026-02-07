use std::env;
use std::io;
use std::process::Stdio;

use nvim_rs::compat::tokio::Compat;
use nvim_rs::create::tokio::new_child_cmd;
use nvim_rs::error::{CallError, LoopError};
use nvim_rs::{Neovim, UiAttachOptions};
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use winit::event_loop::EventLoopProxy;

use super::NeovimHandler;
use crate::event::UserEvent;

pub type NvimWriter = Compat<ChildStdin>;

pub struct NeovimProcess {
    pub neovim: Neovim<NvimWriter>,
    #[allow(dead_code)]
    pub io_handle: Option<JoinHandle<Result<(), Box<LoopError>>>>,
    #[allow(dead_code)]
    pub child: Child,
}

impl NeovimProcess {
    pub async fn spawn(
        event_proxy: EventLoopProxy<UserEvent>,
        args: Vec<String>,
    ) -> io::Result<Self> {
        let nvim_path = find_nvim_path()?;
        let handler = NeovimHandler::new(event_proxy);

        let current_dir = env::current_dir()?;
        if current_dir.as_os_str() == "/" {
            log::warn!("Current directory is /. This is probably not what you want. Changing to home directory.");
            let home = dirs::home_dir().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
            })?;
            env::set_current_dir(home)?;
        }

        let mut cmd = Command::new(&nvim_path);
        cmd.args(&args)
            .arg("--embed")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let (neovim, io_handle, child) = new_child_cmd(&mut cmd, handler)
            .await
            .map_err(io::Error::other)?;

        log::info!("Neovim process spawned: {:?}", nvim_path);

        Ok(Self {
            neovim,
            io_handle: Some(io_handle),
            child,
        })
    }

    pub async fn quit(&self) -> Result<(), Box<nvim_rs::error::CallError>> {
        self.neovim.command("qa!").await
    }

    pub async fn ui_attach(&self, cols: u64, rows: u64) -> Result<(), Box<CallError>> {
        let mut opts = UiAttachOptions::new();
        opts.set_rgb(true).set_linegrid_external(true);

        log::info!("Attaching UI with dimensions {}x{}", cols, rows);
        self.neovim.ui_attach(cols as i64, rows as i64, &opts).await
    }

    #[allow(dead_code)]
    pub async fn ui_try_resize(&self, cols: u64, rows: u64) -> Result<(), Box<CallError>> {
        self.neovim.ui_try_resize(cols as i64, rows as i64).await
    }

    pub async fn input(&self, keys: &str) -> Result<i64, Box<CallError>> {
        self.neovim.input(keys).await
    }

    pub async fn input_mouse(
        &self,
        button: &str,
        action: &str,
        modifier: &str,
        grid: i64,
        row: i64,
        col: i64,
    ) -> Result<(), Box<CallError>> {
        self.neovim
            .input_mouse(button, action, modifier, grid, row, col)
            .await
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

    #[test]
    fn test_default_dimensions() {
        use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS};
        assert_eq!(DEFAULT_COLS, 80);
        assert_eq!(DEFAULT_ROWS, 24);
    }
}
