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
