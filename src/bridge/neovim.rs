use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, Value};
use winit::event_loop::EventLoopProxy;

use super::parser::parse_redraw;
use super::NvimWriter;
use crate::event::{NeovimEvent, UserEvent};

#[derive(Clone)]
pub struct NeovimHandler {
    event_proxy: EventLoopProxy<UserEvent>,
}

impl NeovimHandler {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>) -> Self {
        Self { event_proxy }
    }

    fn send_event(&self, event: NeovimEvent) {
        if let Err(e) = self.event_proxy.send_event(UserEvent::Neovim(event)) {
            log::warn!("Failed to send neovim event: {:?}", e);
        }
    }
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = NvimWriter;

    async fn handle_notify(&self, name: String, args: Vec<Value>, _neovim: Neovim<Self::Writer>) {
        log::trace!("Notification: {} {:?}", name, args);

        match name.as_str() {
            "redraw" => {
                let events = parse_redraw(args);
                if events.is_empty() {
                    return;
                }

                // Send all events (including Flush) in a single batch.
                // The window handler will request a redraw for this event.
                self.send_event(NeovimEvent::Redraw(events));
            }
            _ => {
                log::debug!("Unhandled notification: {}", name);
            }
        }
    }

    async fn handle_request(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Self::Writer>,
    ) -> Result<Value, Value> {
        log::debug!("Request: {} {:?}", name, args);

        Err(Value::from(format!("Unknown request: {}", name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<NeovimHandler>();
    }

    #[test]
    fn test_handler_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NeovimHandler>();
    }
}
