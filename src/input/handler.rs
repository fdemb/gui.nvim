use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};

use crate::bridge::AppBridge;
use crate::input::{
    key_event_to_neovim, modifiers_to_string, mouse_button_to_type, pixel_to_grid,
    scroll_delta_to_direction, CellMetrics, Modifiers, MouseAction, MouseState,
};

pub struct InputHandler {
    modifiers: Modifiers,
    mouse_state: MouseState,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            modifiers: Modifiers::default(),
            mouse_state: MouseState::new(),
        }
    }

    pub fn handle_modifiers_changed(&mut self, state: winit::event::Modifiers) {
        self.modifiers = Modifiers::from(state.state());
    }

    pub fn handle_keyboard_input(&self, event: &KeyEvent, bridge: &AppBridge) {
        if let Some(keys) = key_event_to_neovim(event, &self.modifiers) {
            log::trace!("Keyboard input: {}", keys);
            bridge.input(keys);
        }
    }

    pub fn handle_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
        bridge: &AppBridge,
    ) {
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
                bridge.mouse_input(
                    button_type.as_str(),
                    action.as_str(),
                    &modifier_str,
                    0,
                    grid_pos.row,
                    grid_pos.col,
                );
            }
        }
    }

    pub fn handle_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        cell_metrics: &CellMetrics,
        bridge: &AppBridge,
    ) {
        let grid_pos = pixel_to_grid(position, cell_metrics);
        let old_pos = self.mouse_state.last_position;
        self.mouse_state.update_position(grid_pos);

        if self.mouse_state.is_dragging() {
            if old_pos
                .map(|p| p.row != grid_pos.row || p.col != grid_pos.col)
                .unwrap_or(true)
            {
                if let Some(button_type) = self.mouse_state.pressed_button {
                    let modifier_str = modifiers_to_string(&self.modifiers);
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

    pub fn handle_mouse_wheel(&self, delta: MouseScrollDelta, bridge: &AppBridge) {
        if let Some(grid_pos) = self.mouse_state.last_position {
            if let Some((direction, count)) = scroll_delta_to_direction(delta) {
                let modifier_str = modifiers_to_string(&self.modifiers);
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
