use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use winit::window::Window;

use crate::config::Config;
use crate::editor::EditorState;
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

pub struct RenderLoop {
    state: RenderState,
}

impl RenderLoop {
    pub fn new() -> Self {
        Self {
            state: RenderState::Uninitialized,
        }
    }

    pub fn initialize(&mut self, window: Arc<Window>, config: Config) {
        self.state = RenderState::Initializing(Box::pin(Renderer::new(window, config)));
    }

    pub fn poll(&mut self, window: &Window) -> Poll<Result<&mut Renderer, ()>> {
        let state = std::mem::replace(&mut self.state, RenderState::Uninitialized);

        self.state = match state {
            RenderState::Initializing(mut future) => {
                struct NoopWaker;
                impl Wake for NoopWaker {
                    fn wake(self: Arc<Self>) {}
                }

                let waker = Waker::from(Arc::new(NoopWaker));
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
            RenderState::Uninitialized => Poll::Pending, // Should probably be handled
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
