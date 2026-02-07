#![allow(clippy::large_enum_variant)]
#![allow(clippy::result_unit_err)]

use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::config::Config;
use crate::editor::EditorState;
use crate::event::UserEvent;
use crate::renderer::Renderer;

pub enum RenderState {
    Uninitialized,
    Initializing(
        std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<Renderer, crate::renderer::RendererError>>
                    + Send,
            >,
        >,
    ),
    Ready(Renderer),
    Failed,
}

/// A waker that sends a user event to the winit event loop, ensuring the
/// loop wakes up and re-polls the GPU initialization future. This is
/// necessary because wgpu's `request_adapter`/`request_device` can be
/// truly async on some backends (Vulkan, DX12).
struct EventLoopWaker {
    proxy: EventLoopProxy<UserEvent>,
}

impl Wake for EventLoopWaker {
    fn wake(self: Arc<Self>) {
        // Send a redraw-triggering event so the event loop will call
        // RedrawRequested, which in turn calls poll() again.
        let _ = self
            .proxy
            .send_event(UserEvent::GUI(crate::event::GUIEvent::Focused(true)));
    }
}

pub struct RenderLoop {
    state: RenderState,
    event_proxy: Option<EventLoopProxy<UserEvent>>,
}

impl Default for RenderLoop {
    fn default() -> Self {
        Self {
            state: RenderState::Uninitialized,
            event_proxy: None,
        }
    }
}

impl RenderLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_event_proxy(&mut self, proxy: EventLoopProxy<UserEvent>) {
        self.event_proxy = Some(proxy);
    }

    pub fn initialize(&mut self, window: Arc<Window>, config: Config) {
        self.state = RenderState::Initializing(Box::pin(Renderer::new(window, config)));
    }

    pub fn poll(&mut self, window: &Window) -> Poll<Result<&mut Renderer, ()>> {
        let state = std::mem::replace(&mut self.state, RenderState::Uninitialized);

        self.state = match state {
            RenderState::Initializing(mut future) => {
                let waker = if let Some(ref proxy) = self.event_proxy {
                    Waker::from(Arc::new(EventLoopWaker {
                        proxy: proxy.clone(),
                    }))
                } else {
                    // Fallback: noop waker. We compensate by always calling
                    // request_redraw() below when the future is pending.
                    struct NoopWaker;
                    impl Wake for NoopWaker {
                        fn wake(self: Arc<Self>) {}
                    }
                    Waker::from(Arc::new(NoopWaker))
                };
                let mut cx = Context::from_waker(&waker);

                match future.as_mut().poll(&mut cx) {
                    Poll::Ready(Ok(renderer)) => {
                        log::info!("GPU renderer initialized");
                        RenderState::Ready(renderer)
                    }
                    Poll::Ready(Err(e)) => {
                        log::error!("Failed to initialize renderer: {}", e);
                        RenderState::Failed
                    }
                    Poll::Pending => {
                        // Also request a redraw as a safety net so we
                        // re-poll even if the waker is never invoked.
                        window.request_redraw();
                        RenderState::Initializing(future)
                    }
                }
            }
            other => other,
        };

        match &mut self.state {
            RenderState::Ready(renderer) => Poll::Ready(Ok(renderer)),
            RenderState::Failed => Poll::Ready(Err(())),
            RenderState::Initializing(_) => Poll::Pending,
            RenderState::Uninitialized => Poll::Pending,
        }
    }

    pub fn renderer(&mut self) -> Option<&mut Renderer> {
        match &mut self.state {
            RenderState::Ready(renderer) => Some(renderer),
            _ => None,
        }
    }

    pub fn render(
        &mut self,
        state: &EditorState,
        x_offset: f32,
        y_offset: f32,
        window: &Window,
    ) -> Result<(), ()> {
        if let RenderState::Ready(ref mut renderer) = self.state {
            match renderer.render(state, x_offset, y_offset) {
                Ok(()) => Ok(()),
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    renderer.resize(window.inner_size());
                    Ok(())
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    log::error!("Out of GPU memory");
                    Err(())
                }
                Err(e) => {
                    log::warn!("Render error: {:?}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}
