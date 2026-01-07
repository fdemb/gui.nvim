use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::event::KeyEvent;
use winit::window::Window;

use crate::bridge::events::RedrawEvent;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Neovim(NeovimEvent),
    #[allow(dead_code)]
    GUI(GUIEvent),
}

#[derive(Debug, Clone)]
pub enum NeovimEvent {
    Redraw(Vec<RedrawEvent>),
    Flush,
    #[allow(dead_code)]
    Quit,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum GUIEvent {
    WindowCreated(Arc<Window>),
    Resized(PhysicalSize<u32>),
    RedrawRequested,
    KeyboardInput(KeyEvent),
    ScaleFactorChanged(f64),
    Focused(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_event_debug() {
        let event = UserEvent::Neovim(NeovimEvent::Redraw(vec![]));
        assert!(format!("{:?}", event).contains("Redraw"));
    }

    #[test]
    fn test_gui_event_debug() {
        let event = GUIEvent::Focused(true);
        assert!(format!("{:?}", event).contains("Focused"));
    }

    #[test]
    fn test_neovim_event_variants() {
        let redraw = NeovimEvent::Redraw(vec![]);
        let flush = NeovimEvent::Flush;
        let quit = NeovimEvent::Quit;

        assert!(matches!(redraw, NeovimEvent::Redraw(_)));
        assert!(matches!(flush, NeovimEvent::Flush));
        assert!(matches!(quit, NeovimEvent::Quit));
    }
}
