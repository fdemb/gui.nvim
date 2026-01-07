use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use winit::event_loop::EventLoopProxy;

use crate::bridge::NeovimProcess;
use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS};
use crate::event::{NeovimEvent, UserEvent};

pub enum AppCommand {
    SpawnNeovim(Vec<String>),
    Resize {
        cols: u64,
        rows: u64,
    },
    Input(String),
    MouseInput {
        button: String,
        action: String,
        modifier: String,
        grid: i64,
        row: i64,
        col: i64,
    },
    Quit,
}

#[cfg(test)]
impl PartialEq for AppCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SpawnNeovim(a), Self::SpawnNeovim(b)) => a == b,
            (Self::Resize { cols: c1, rows: r1 }, Self::Resize { cols: c2, rows: r2 }) => {
                c1 == c2 && r1 == r2
            }
            (Self::Input(a), Self::Input(b)) => a == b,
            (
                Self::MouseInput {
                    button: b1,
                    action: a1,
                    modifier: m1,
                    grid: g1,
                    row: r1,
                    col: c1,
                },
                Self::MouseInput {
                    button: b2,
                    action: a2,
                    modifier: m2,
                    grid: g2,
                    row: r2,
                    col: c2,
                },
            ) => b1 == b2 && a1 == a2 && m1 == m2 && g1 == g2 && r1 == r2 && c1 == c2,
            (Self::Quit, Self::Quit) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
impl std::fmt::Debug for AppCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnNeovim(args) => f.debug_tuple("SpawnNeovim").field(args).finish(),
            Self::Resize { cols, rows } => f
                .debug_struct("Resize")
                .field("cols", cols)
                .field("rows", rows)
                .finish(),
            Self::Input(keys) => f.debug_tuple("Input").field(keys).finish(),
            Self::MouseInput {
                button,
                action,
                modifier,
                grid,
                row,
                col,
            } => f
                .debug_struct("MouseInput")
                .field("button", button)
                .field("action", action)
                .field("modifier", modifier)
                .field("grid", grid)
                .field("row", row)
                .field("col", col)
                .finish(),
            Self::Quit => write!(f, "Quit"),
        }
    }
}

pub struct AppBridge {
    command_tx: mpsc::UnboundedSender<AppCommand>,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
}

impl AppBridge {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let rt = runtime.clone();
        std::thread::spawn(move || {
            rt.block_on(async move {
                run_neovim_loop(event_proxy, command_rx).await;
            });
        });

        Self {
            command_tx,
            runtime,
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> (Self, mpsc::UnboundedReceiver<AppCommand>) {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
        (
            Self {
                command_tx,
                runtime,
            },
            command_rx,
        )
    }

    pub fn spawn_neovim(&self, args: Vec<String>) {
        let _ = self.command_tx.send(AppCommand::SpawnNeovim(args));
    }

    pub fn resize(&self, cols: u64, rows: u64) {
        let _ = self.command_tx.send(AppCommand::Resize { cols, rows });
    }

    pub fn input(&self, keys: String) {
        let _ = self.command_tx.send(AppCommand::Input(keys));
    }

    pub fn mouse_input(
        &self,
        button: &str,
        action: &str,
        modifier: &str,
        grid: i64,
        row: i64,
        col: i64,
    ) {
        let _ = self.command_tx.send(AppCommand::MouseInput {
            button: button.to_string(),
            action: action.to_string(),
            modifier: modifier.to_string(),
            grid,
            row,
            col,
        });
    }

    pub fn quit(&self) {
        let _ = self.command_tx.send(AppCommand::Quit);
    }
}

async fn run_neovim_loop(
    event_proxy: EventLoopProxy<UserEvent>,
    mut command_rx: mpsc::UnboundedReceiver<AppCommand>,
) {
    let mut nvim: Option<NeovimProcess> = None;

    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            AppCommand::SpawnNeovim(args) => {
                match NeovimProcess::spawn(event_proxy.clone(), args).await {
                    Ok(mut process) => {
                        if let Err(e) = process.ui_attach(DEFAULT_COLS, DEFAULT_ROWS).await {
                            log::error!("Failed to attach UI: {:?}", e);
                            continue;
                        }
                        log::info!("Neovim UI attached");

                        if let Some(io_handle) = process.io_handle.take() {
                            let proxy = event_proxy.clone();
                            tokio::spawn(async move {
                                let _ = io_handle.await;
                                let _ = proxy.send_event(UserEvent::Neovim(NeovimEvent::Quit));
                            });
                        }

                        nvim = Some(process);
                    }
                    Err(e) => {
                        log::error!("Failed to spawn Neovim: {}", e);
                    }
                }
            }
            AppCommand::Resize { cols, rows } => {
                if let Some(ref nvim) = nvim {
                    let neovim = nvim.neovim.clone();
                    tokio::spawn(async move {
                        if let Err(e) = neovim.ui_try_resize(cols as i64, rows as i64).await {
                            log::warn!("Failed to resize UI: {:?}", e);
                        }
                    });
                }
            }
            AppCommand::Input(keys) => {
                if let Some(ref nvim) = nvim {
                    if let Err(e) = nvim.input(&keys).await {
                        log::warn!("Failed to send input: {:?}", e);
                    }
                }
            }
            AppCommand::MouseInput {
                button,
                action,
                modifier,
                grid,
                row,
                col,
            } => {
                if let Some(ref nvim) = nvim {
                    if let Err(e) = nvim
                        .input_mouse(&button, &action, &modifier, grid, row, col)
                        .await
                    {
                        log::warn!("Failed to send mouse input: {:?}", e);
                    }
                }
            }
            AppCommand::Quit => {
                if let Some(ref nvim) = nvim {
                    let _ = nvim.quit().await;
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_bridge_send_commands() {
        let (bridge, mut rx) = AppBridge::new_for_test();

        // SpawnNeovim
        bridge.spawn_neovim(vec!["--clean".to_string()]);
        match rx.blocking_recv() {
            Some(AppCommand::SpawnNeovim(args)) => {
                assert_eq!(args, vec!["--clean".to_string()]);
            }
            _ => panic!("Expected SpawnNeovim"),
        }

        // Resize
        bridge.resize(100, 50);
        match rx.blocking_recv() {
            Some(AppCommand::Resize { cols, rows }) => {
                assert_eq!(cols, 100);
                assert_eq!(rows, 50);
            }
            _ => panic!("Expected Resize"),
        }

        // Input
        bridge.input("<Esc>".to_string());
        match rx.blocking_recv() {
            Some(AppCommand::Input(keys)) => {
                assert_eq!(keys, "<Esc>");
            }
            _ => panic!("Expected Input"),
        }

        // MouseInput
        bridge.mouse_input("left", "press", "", 0, 10, 20);
        match rx.blocking_recv() {
            Some(AppCommand::MouseInput {
                button,
                action,
                modifier,
                grid,
                row,
                col,
            }) => {
                assert_eq!(button, "left");
                assert_eq!(action, "press");
                assert_eq!(modifier, "");
                assert_eq!(grid, 0);
                assert_eq!(row, 10);
                assert_eq!(col, 20);
            }
            _ => panic!("Expected MouseInput"),
        }

        // Quit
        bridge.quit();
        match rx.blocking_recv() {
            Some(AppCommand::Quit) => {}
            _ => panic!("Expected Quit"),
        }
    }
}
