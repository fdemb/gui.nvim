use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{
    calculate_grid_size, AppBridge, DEFAULT_CELL_HEIGHT, DEFAULT_CELL_WIDTH, PADDING,
};
use crate::bridge::{DEFAULT_COLS, DEFAULT_ROWS};
use crate::event::{GUIEvent, UserEvent};
use crate::input::{
    key_event_to_neovim, modifiers_to_string, mouse_button_to_type, pixel_to_grid,
    scroll_delta_to_direction, CellMetrics, Modifiers, MouseAction, MouseState,
};

pub struct GuiApp {
    window: Option<Arc<Window>>,
    event_proxy: EventLoopProxy<UserEvent>,
    app_bridge: Option<AppBridge>,
    close_requested: bool,
    current_cols: u64,
    current_rows: u64,
    modifiers: Modifiers,
    mouse_state: MouseState,
    cell_metrics: CellMetrics,
}

impl GuiApp {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            event_proxy,
            app_bridge: None,
            close_requested: false,
            current_cols: DEFAULT_COLS,
            current_rows: DEFAULT_ROWS,
            modifiers: Modifiers::default(),
            mouse_state: MouseState::new(),
            cell_metrics: CellMetrics {
                cell_width: DEFAULT_CELL_WIDTH as f64,
                cell_height: DEFAULT_CELL_HEIGHT as f64,
                padding_x: PADDING as f64,
                padding_y: PADDING as f64,
            },
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let width = DEFAULT_COLS as u32 * DEFAULT_CELL_WIDTH + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * DEFAULT_CELL_HEIGHT + 2 * PADDING;

        let window_attrs = WindowAttributes::default()
            .with_title("gui.nvim")
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(200, 100));

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                log::info!("Window created: {:?}", window.id());
                let window = Arc::new(window);
                self.window = Some(window.clone());

                let bridge = AppBridge::new(self.event_proxy.clone());
                bridge.spawn_neovim();
                self.app_bridge = Some(bridge);

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
                if let Some(ref bridge) = self.app_bridge {
                    bridge.quit();
                }
                self.close_requested = true;
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    log::debug!("Window resized: {}x{}", size.width, size.height);

                    let (cols, rows) = calculate_grid_size(size);
                    if cols != self.current_cols || rows != self.current_rows {
                        self.current_cols = cols;
                        self.current_rows = rows;
                        if let Some(ref bridge) = self.app_bridge {
                            bridge.resize(cols, rows);
                        }
                    }

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

            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = Modifiers::from(modifiers.state());
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(keys) = key_event_to_neovim(&event, &self.modifiers) {
                    log::trace!("Keyboard input: {}", keys);
                    if let Some(ref bridge) = self.app_bridge {
                        bridge.input(keys);
                    }
                }

                // Also forward to event system for other handlers
                if event.state == ElementState::Pressed {
                    let _ = self
                        .event_proxy
                        .send_event(UserEvent::GUI(GUIEvent::KeyboardInput(event)));
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button_type) = mouse_button_to_type(button) {
                    if let Some(grid_pos) = self.mouse_state.last_position {
                        let action = match state {
                            ElementState::Pressed => {
                                self.mouse_state.button_pressed(button_type);
                                MouseAction::Press
                            }
                            ElementState::Released => {
                                self.mouse_state.button_released();
                                MouseAction::Release
                            }
                        };

                        let modifier_str = modifiers_to_string(&self.modifiers);
                        if let Some(ref bridge) = self.app_bridge {
                            bridge.mouse_input(
                                button_type.as_str(),
                                action.as_str(),
                                &modifier_str,
                                0, // grid 0 for single-grid mode
                                grid_pos.row,
                                grid_pos.col,
                            );
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let grid_pos = pixel_to_grid(position, &self.cell_metrics);
                let old_pos = self.mouse_state.last_position;
                self.mouse_state.update_position(grid_pos);

                // Send drag event if button is pressed and position changed
                if self.mouse_state.is_dragging() {
                    if old_pos
                        .map(|p| p.row != grid_pos.row || p.col != grid_pos.col)
                        .unwrap_or(true)
                    {
                        if let Some(button_type) = self.mouse_state.pressed_button {
                            let modifier_str = modifiers_to_string(&self.modifiers);
                            if let Some(ref bridge) = self.app_bridge {
                                bridge.mouse_input(
                                    button_type.as_str(),
                                    MouseAction::Drag.as_str(),
                                    &modifier_str,
                                    0,
                                    grid_pos.row,
                                    grid_pos.col,
                                );
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(grid_pos) = self.mouse_state.last_position {
                    if let Some((direction, count)) = scroll_delta_to_direction(delta) {
                        let modifier_str = modifiers_to_string(&self.modifiers);
                        if let Some(ref bridge) = self.app_bridge {
                            for _ in 0..count {
                                bridge.mouse_input(
                                    "wheel",
                                    direction.as_str(),
                                    &modifier_str,
                                    0,
                                    grid_pos.row,
                                    grid_pos.col,
                                );
                            }
                        }
                    }
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
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
        let width = DEFAULT_COLS as u32 * DEFAULT_CELL_WIDTH + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * DEFAULT_CELL_HEIGHT + 2 * PADDING;
        assert_eq!(width, 804);
        assert_eq!(height, 484);
    }
}
