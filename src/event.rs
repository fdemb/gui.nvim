use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::event::KeyEvent;
use winit::window::Window;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Neovim(NeovimEvent),
    GUI(GUIEvent),
}

#[derive(Debug, Clone)]
pub enum NeovimEvent {
    Redraw,
    Flush,
    Quit,
}

#[derive(Debug, Clone)]
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
        let event = UserEvent::Neovim(NeovimEvent::Redraw);
        assert!(format!("{:?}", event).contains("Redraw"));
    }

    #[test]
    fn test_gui_event_debug() {
        let event = GUIEvent::Focused(true);
        assert!(format!("{:?}", event).contains("Focused"));
    }

    #[test]
    fn test_neovim_event_variants() {
        let redraw = NeovimEvent::Redraw;
        let flush = NeovimEvent::Flush;
        let quit = NeovimEvent::Quit;

        assert!(matches!(redraw, NeovimEvent::Redraw));
        assert!(matches!(flush, NeovimEvent::Flush));
        assert!(matches!(quit, NeovimEvent::Quit));
    }
}
