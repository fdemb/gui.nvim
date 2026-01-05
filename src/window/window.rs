use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::event::{GUIEvent, NeovimEvent, UserEvent};

const DEFAULT_COLS: u32 = 80;
const DEFAULT_ROWS: u32 = 24;
const DEFAULT_CELL_WIDTH: u32 = 10;
const DEFAULT_CELL_HEIGHT: u32 = 20;
const PADDING: u32 = 2;

pub struct GuiApp {
    window: Option<Arc<Window>>,
    event_proxy: EventLoopProxy<UserEvent>,
    close_requested: bool,
}

impl GuiApp {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            event_proxy,
            close_requested: false,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let width = DEFAULT_COLS * DEFAULT_CELL_WIDTH + 2 * PADDING;
        let height = DEFAULT_ROWS * DEFAULT_CELL_HEIGHT + 2 * PADDING;

        let window_attrs = WindowAttributes::default()
            .with_title("gui.nvim")
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(200, 100));

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                log::info!("Window created: {:?}", window.id());
                let window = Arc::new(window);
                self.window = Some(window.clone());

                let _ = self
                    .event_proxy
                    .send_event(UserEvent::GUI(GUIEvent::WindowCreated(window)));
            }
            Err(e) => {
                log::error!("Failed to create window: {}", e);
                self.close_requested = true;
            }
        }
    }

    pub fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }
}

impl ApplicationHandler<UserEvent> for GuiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.create_window(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|w| w.id()) != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested");
                self.close_requested = true;
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    log::debug!("Window resized: {}x{}", size.width, size.height);
                    let _ = self
                        .event_proxy
                        .send_event(UserEvent::GUI(GUIEvent::Resized(size)));
                }
            }

            WindowEvent::RedrawRequested => {
                let _ = self
                    .event_proxy
                    .send_event(UserEvent::GUI(GUIEvent::RedrawRequested));
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let _ = self
                        .event_proxy
                        .send_event(UserEvent::GUI(GUIEvent::KeyboardInput(event)));
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                log::debug!("Scale factor changed: {}", scale_factor);
                let _ = self
                    .event_proxy
                    .send_event(UserEvent::GUI(GUIEvent::ScaleFactorChanged(scale_factor)));
            }

            WindowEvent::Focused(focused) => {
                log::debug!("Window focused: {}", focused);
                let _ = self
                    .event_proxy
                    .send_event(UserEvent::GUI(GUIEvent::Focused(focused)));
            }

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Neovim(neovim_event) => {
                log::trace!("Received Neovim event: {:?}", neovim_event);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UserEvent::GUI(_) => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.close_requested {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dimensions() {
        let width = DEFAULT_COLS * DEFAULT_CELL_WIDTH + 2 * PADDING;
        let height = DEFAULT_ROWS * DEFAULT_CELL_HEIGHT + 2 * PADDING;
        assert_eq!(width, 804);
        assert_eq!(height, 484);
    }
}
