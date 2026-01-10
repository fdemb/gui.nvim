use winit::dpi::PhysicalPosition;
use winit::event::{MouseButton, MouseScrollDelta};

use super::keyboard::Modifiers;
use crate::constants::{DEFAULT_CELL_HEIGHT, DEFAULT_CELL_WIDTH, PADDING};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
    Drag,
    #[allow(dead_code)]
    Move,
}

impl MouseAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            MouseAction::Press => "press",
            MouseAction::Release => "release",
            MouseAction::Drag => "drag",
            MouseAction::Move => "move",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButtonType {
    Left,
    Right,
    Middle,
}

impl MouseButtonType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MouseButtonType::Left => "left",
            MouseButtonType::Right => "right",
            MouseButtonType::Middle => "middle",
        }
    }
}

pub fn mouse_button_to_type(button: MouseButton) -> Option<MouseButtonType> {
    match button {
        MouseButton::Left => Some(MouseButtonType::Left),
        MouseButton::Right => Some(MouseButtonType::Right),
        MouseButton::Middle => Some(MouseButtonType::Middle),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GridPosition {
    pub row: i64,
    pub col: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct CellMetrics {
    pub cell_width: f64,
    pub cell_height: f64,
    pub padding_x: f64,
    pub padding_y: f64,
}

impl Default for CellMetrics {
    fn default() -> Self {
        Self {
            cell_width: DEFAULT_CELL_WIDTH as f64,
            cell_height: DEFAULT_CELL_HEIGHT as f64,
            padding_x: PADDING as f64,
            padding_y: PADDING as f64,
        }
    }
}

pub fn pixel_to_grid(position: PhysicalPosition<f64>, metrics: &CellMetrics) -> GridPosition {
    let x = (position.x - metrics.padding_x).max(0.0);
    let y = (position.y - metrics.padding_y).max(0.0);

    GridPosition {
        col: (x / metrics.cell_width).floor() as i64,
        row: (y / metrics.cell_height).floor() as i64,
    }
}

pub fn modifiers_to_string(modifiers: &Modifiers) -> String {
    let mut result = String::new();
    if modifiers.shift {
        result.push('S');
    }
    if modifiers.ctrl {
        result.push('C');
    }
    if modifiers.alt {
        result.push('A');
    }
    if modifiers.logo {
        result.push('D');
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScrollDirection::Up => "up",
            ScrollDirection::Down => "down",
            ScrollDirection::Left => "left",
            ScrollDirection::Right => "right",
        }
    }
}

pub fn scroll_delta_to_direction(delta: MouseScrollDelta) -> Option<(ScrollDirection, u32)> {
    const PIXELS_PER_LINE: f64 = 20.0;

    let (x, y, threshold) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64, 0.0),
        MouseScrollDelta::PixelDelta(d) => (d.x / PIXELS_PER_LINE, d.y / PIXELS_PER_LINE, 0.5),
    };

    let (value, dir_pos, dir_neg) = if y.abs() > x.abs() {
        (y, ScrollDirection::Up, ScrollDirection::Down)
    } else {
        (x, ScrollDirection::Left, ScrollDirection::Right)
    };

    match value {
        v if v > threshold => Some((dir_pos, v.abs().ceil() as u32)),
        v if v < -threshold => Some((dir_neg, v.abs().ceil() as u32)),
        _ => None,
    }
}

pub struct MouseState {
    pub last_position: Option<GridPosition>,
    pub pressed_button: Option<MouseButtonType>,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            last_position: None,
            pressed_button: None,
        }
    }
}

impl MouseState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_position(&mut self, position: GridPosition) {
        self.last_position = Some(position);
    }

    pub fn button_pressed(&mut self, button: MouseButtonType) {
        self.pressed_button = Some(button);
    }

    pub fn button_released(&mut self) {
        self.pressed_button = None;
    }

    pub fn is_dragging(&self) -> bool {
        self.pressed_button.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_action_str() {
        assert_eq!(MouseAction::Press.as_str(), "press");
        assert_eq!(MouseAction::Release.as_str(), "release");
        assert_eq!(MouseAction::Drag.as_str(), "drag");
        assert_eq!(MouseAction::Move.as_str(), "move");
    }

    #[test]
    fn test_mouse_button_str() {
        assert_eq!(MouseButtonType::Left.as_str(), "left");
        assert_eq!(MouseButtonType::Right.as_str(), "right");
        assert_eq!(MouseButtonType::Middle.as_str(), "middle");
    }

    #[test]
    fn test_mouse_button_conversion() {
        assert_eq!(
            mouse_button_to_type(MouseButton::Left),
            Some(MouseButtonType::Left)
        );
        assert_eq!(
            mouse_button_to_type(MouseButton::Right),
            Some(MouseButtonType::Right)
        );
        assert_eq!(
            mouse_button_to_type(MouseButton::Middle),
            Some(MouseButtonType::Middle)
        );
        assert_eq!(mouse_button_to_type(MouseButton::Back), None);
        assert_eq!(mouse_button_to_type(MouseButton::Forward), None);
    }

    #[test]
    fn test_pixel_to_grid_basic() {
        let metrics = CellMetrics {
            cell_width: 10.0,
            cell_height: 20.0,
            padding_x: 2.0,
            padding_y: 2.0,
        };

        let pos = PhysicalPosition::new(12.0, 22.0); // First cell
        let grid = pixel_to_grid(pos, &metrics);
        assert_eq!(grid.col, 1);
        assert_eq!(grid.row, 1);
    }

    #[test]
    fn test_pixel_to_grid_origin() {
        let metrics = CellMetrics::default();
        let pos = PhysicalPosition::new(2.0, 2.0); // At padding
        let grid = pixel_to_grid(pos, &metrics);
        assert_eq!(grid.col, 0);
        assert_eq!(grid.row, 0);
    }

    #[test]
    fn test_pixel_to_grid_negative_clamps() {
        let metrics = CellMetrics::default();
        let pos = PhysicalPosition::new(-10.0, -10.0);
        let grid = pixel_to_grid(pos, &metrics);
        assert_eq!(grid.col, 0);
        assert_eq!(grid.row, 0);
    }

    #[test]
    fn test_pixel_to_grid_large_coords() {
        let metrics = CellMetrics {
            cell_width: 10.0,
            cell_height: 20.0,
            padding_x: 0.0,
            padding_y: 0.0,
        };

        let pos = PhysicalPosition::new(85.0, 105.0);
        let grid = pixel_to_grid(pos, &metrics);
        assert_eq!(grid.col, 8);
        assert_eq!(grid.row, 5);
    }

    #[test]
    fn test_modifiers_to_string_empty() {
        let mods = Modifiers::default();
        assert_eq!(modifiers_to_string(&mods), "");
    }

    #[test]
    fn test_modifiers_to_string_shift() {
        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(modifiers_to_string(&mods), "S");
    }

    #[test]
    fn test_modifiers_to_string_ctrl() {
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(modifiers_to_string(&mods), "C");
    }

    #[test]
    fn test_modifiers_to_string_all() {
        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: true,
            logo: true,
        };
        assert_eq!(modifiers_to_string(&mods), "SCAD");
    }

    #[test]
    fn test_scroll_direction_str() {
        assert_eq!(ScrollDirection::Up.as_str(), "up");
        assert_eq!(ScrollDirection::Down.as_str(), "down");
        assert_eq!(ScrollDirection::Left.as_str(), "left");
        assert_eq!(ScrollDirection::Right.as_str(), "right");
    }

    #[test]
    fn test_scroll_line_delta_up() {
        let delta = MouseScrollDelta::LineDelta(0.0, 3.0);
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Up, 3)));
    }

    #[test]
    fn test_scroll_line_delta_down() {
        let delta = MouseScrollDelta::LineDelta(0.0, -2.0);
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Down, 2)));
    }

    #[test]
    fn test_scroll_line_delta_left() {
        let delta = MouseScrollDelta::LineDelta(-1.0, 0.0);
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Left, 1)));
    }

    #[test]
    fn test_scroll_line_delta_right() {
        let delta = MouseScrollDelta::LineDelta(1.0, 0.0);
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Right, 1)));
    }

    #[test]
    fn test_scroll_line_delta_zero() {
        let delta = MouseScrollDelta::LineDelta(0.0, 0.0);
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, None);
    }

    #[test]
    fn test_scroll_pixel_delta_down() {
        let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -40.0));
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Down, 2)));
    }

    #[test]
    fn test_scroll_pixel_delta_up() {
        let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 40.0));
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, Some((ScrollDirection::Up, 2)));
    }

    #[test]
    fn test_scroll_pixel_delta_small() {
        let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 5.0));
        let result = scroll_delta_to_direction(delta);
        assert_eq!(result, None);
    }

    #[test]
    fn test_mouse_state_new() {
        let state = MouseState::new();
        assert!(state.last_position.is_none());
        assert!(state.pressed_button.is_none());
        assert!(!state.is_dragging());
    }

    #[test]
    fn test_mouse_state_update_position() {
        let mut state = MouseState::new();
        state.update_position(GridPosition { row: 5, col: 10 });
        assert!(state.last_position.is_some());
        let pos = state.last_position.unwrap();
        assert_eq!(pos.row, 5);
        assert_eq!(pos.col, 10);
    }

    #[test]
    fn test_mouse_state_button_press_release() {
        let mut state = MouseState::new();
        assert!(!state.is_dragging());

        state.button_pressed(MouseButtonType::Left);
        assert!(state.is_dragging());
        assert_eq!(state.pressed_button, Some(MouseButtonType::Left));

        state.button_released();
        assert!(!state.is_dragging());
        assert!(state.pressed_button.is_none());
    }

    #[test]
    fn test_cell_metrics_default() {
        let metrics = CellMetrics::default();
        assert_eq!(metrics.cell_width, 10.0);
        assert_eq!(metrics.cell_height, 20.0);
        assert_eq!(metrics.padding_x, 2.0);
        assert_eq!(metrics.padding_y, 2.0);
    }
}
