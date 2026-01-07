use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{AppBridge, PADDING, PADDING_TOP};
use crate::bridge::ui::RedrawEvent;
use crate::bridge::{DEFAULT_COLS, DEFAULT_ROWS};
use crate::config::Config;
use crate::editor::EditorState;
use crate::event::{GUIEvent, NeovimEvent, UserEvent};
use crate::input::{
    key_event_to_neovim, modifiers_to_string, mouse_button_to_type, pixel_to_grid,
    scroll_delta_to_direction, CellMetrics, Modifiers, MouseAction, MouseState,
};
use crate::renderer::Renderer;

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

enum RenderState {
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

pub struct GuiApp {
    window: Option<Arc<Window>>,
    event_proxy: EventLoopProxy<UserEvent>,
    config: Config,
    app_bridge: Option<AppBridge>,
    close_requested: bool,
    current_cols: u64,
    current_rows: u64,
    modifiers: Modifiers,
    mouse_state: MouseState,
    cell_metrics: CellMetrics,
    editor_state: EditorState,
    render_state: RenderState,
}

impl GuiApp {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>, config: Config) -> Self {
        let mut cell_metrics = CellMetrics::default();
        cell_metrics.padding_x = PADDING as f64;
        cell_metrics.padding_y = PADDING_TOP as f64;

        Self {
            window: None,
            event_proxy,
            config,
            app_bridge: None,
            close_requested: false,
            current_cols: DEFAULT_COLS,
            current_rows: DEFAULT_ROWS,
            modifiers: Modifiers::default(),
            mouse_state: MouseState::new(),
            cell_metrics,
            editor_state: EditorState::new(DEFAULT_COLS as usize, DEFAULT_ROWS as usize),
            render_state: RenderState::Uninitialized,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let (cell_width, cell_height) =
            (self.cell_metrics.cell_width, self.cell_metrics.cell_height);
        let width = DEFAULT_COLS as u32 * cell_width as u32 + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * cell_height as u32 + PADDING + PADDING_TOP;

        let mut window_attrs = WindowAttributes::default()
            .with_title("gui.nvim")
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(200, 100));

        #[cfg(target_os = "macos")]
        {
            window_attrs = window_attrs
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_title_hidden(true);
        }

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                log::info!("Window created: {:?}", window.id());
                let window = Arc::new(window);
                self.window = Some(window.clone());
                self.render_state = RenderState::Initializing(Box::pin(Renderer::new(
                    window.clone(),
                    self.config.clone(),
                )));

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

    fn poll_renderer(&mut self) {
        let render_state = std::mem::replace(&mut self.render_state, RenderState::Uninitialized);

        self.render_state = match render_state {
            RenderState::Initializing(mut future) => {
                use std::sync::Arc;
                use std::task::{Context, Poll, Wake, Waker};

                struct NoopWaker;
                impl Wake for NoopWaker {
                    fn wake(self: Arc<Self>) {}
                }

                let waker = Waker::from(Arc::new(NoopWaker));
                let mut cx = Context::from_waker(&waker);

                match future.as_mut().poll(&mut cx) {
                    Poll::Ready(Ok(renderer)) => {
                        log::info!("GPU renderer initialized");
                        let (cw, ch) = renderer.cell_size();
                        self.cell_metrics.cell_width = cw as f64;
                        self.cell_metrics.cell_height = ch as f64;

                        if let Some(ref bridge) = self.app_bridge {
                            if let Some(ref window) = self.window {
                                let size = window.inner_size();
                                let (cols, rows) =
                                    self.calculate_grid_size(size.width, size.height);
                                if cols != self.current_cols || rows != self.current_rows {
                                    self.current_cols = cols;
                                    self.current_rows = rows;
                                    bridge.resize(cols, rows);
                                }
                            }
                        }

                        RenderState::Ready(renderer)
                    }
                    Poll::Ready(Err(e)) => {
                        log::error!("Failed to initialize renderer: {}", e);
                        RenderState::Failed
                    }
                    Poll::Pending => {
                        if let Some(ref window) = self.window {
                            window.request_redraw();
                        }
                        RenderState::Initializing(future)
                    }
                }
            }
            other => other,
        };
    }

    fn calculate_grid_size(&self, width: u32, height: u32) -> (u64, u64) {
        let cols = (width.saturating_sub(2 * PADDING)) as f64 / self.cell_metrics.cell_width;
        let rows =
            (height.saturating_sub(PADDING + PADDING_TOP)) as f64 / self.cell_metrics.cell_height;
        (cols.max(1.0) as u64, rows.max(1.0) as u64)
    }

    fn apply_redraw_events(&mut self, events: Vec<RedrawEvent>) {
        for event in events {
            match event {
                RedrawEvent::GridResize {
                    grid,
                    width,
                    height,
                } => {
                    self.editor_state.grid_resize(grid, width, height);
                }
                RedrawEvent::GridClear { grid } => {
                    self.editor_state.grid_clear(grid);
                }
                RedrawEvent::GridLine {
                    grid,
                    row,
                    col_start,
                    cells,
                } => {
                    let cells: Vec<(String, Option<u64>, usize)> = cells
                        .into_iter()
                        .map(|c| (c.text, c.hl_id, c.repeat))
                        .collect();
                    self.editor_state.grid_line(grid, row, col_start, &cells);
                }
                RedrawEvent::GridScroll {
                    grid,
                    top,
                    bot,
                    left,
                    right,
                    rows,
                } => {
                    self.editor_state
                        .grid_scroll(grid, top, bot, left, right, rows);
                }
                RedrawEvent::GridCursorGoto { grid, row, col } => {
                    self.editor_state.grid_cursor_goto(grid, row, col);
                }
                RedrawEvent::GridDestroy { .. } => {}
                RedrawEvent::HlAttrDefine { id, attrs } => {
                    self.editor_state.hl_attr_define(id, attrs);
                }
                RedrawEvent::HlGroupSet { .. } => {}
                RedrawEvent::DefaultColorsSet { fg, bg, sp } => {
                    self.editor_state.default_colors_set(fg, bg, sp);
                    if let RenderState::Ready(ref mut renderer) = self.render_state {
                        renderer.update_default_colors(fg, bg);
                    }
                }
                RedrawEvent::ModeInfoSet { modes, .. } => {
                    self.editor_state.mode_info_set(modes);
                }
                RedrawEvent::ModeChange { mode, mode_idx } => {
                    self.editor_state.mode_change(&mode, mode_idx);
                }
                RedrawEvent::SetTitle { title } => {
                    if let Some(ref window) = self.window {
                        window.set_title(&title);
                    }
                }
                RedrawEvent::SetIcon { .. } => {}
                RedrawEvent::OptionSet { .. } => {}
                RedrawEvent::Flush => {
                    self.editor_state.flush();
                }
                RedrawEvent::Busy { .. } => {}
                RedrawEvent::MouseOn => {}
                RedrawEvent::MouseOff => {}
            }
        }
    }

    fn do_render(&mut self) {
        if let RenderState::Ready(ref mut renderer) = self.render_state {
            match renderer.render(&self.editor_state, PADDING as f32, PADDING_TOP as f32) {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    if let Some(ref window) = self.window {
                        renderer.resize(window.inner_size());
                    }
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    log::error!("Out of GPU memory");
                    self.close_requested = true;
                }
                Err(e) => {
                    log::warn!("Render error: {:?}", e);
                }
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

                    if let RenderState::Ready(ref mut renderer) = self.render_state {
                        renderer.resize(size);
                    }

                    let (cols, rows) = self.calculate_grid_size(size.width, size.height);
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
                self.poll_renderer();
                self.do_render();
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
                                0,
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
                match neovim_event {
                    NeovimEvent::Redraw(events) => {
                        self.apply_redraw_events(events);
                    }
                    NeovimEvent::Flush => {}
                    NeovimEvent::Quit => {
                        self.close_requested = true;
                        _event_loop.exit();
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UserEvent::GUI(_) => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.close_requested {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        if self.editor_state.update_blink(now) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        let mode = self.editor_state.current_mode();
        if mode.blink_on > 0 && mode.blink_off > 0 {
            // Schedule next check. Since update_blink uses absolute time,
            // we can just wake up periodically to check.
            // 100ms is a reasonable resolution for cursor blinking.
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + Duration::from_millis(100),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PADDING;

    #[test]
    fn test_default_dimensions() {
        let cell_metrics = CellMetrics::default();
        let width = DEFAULT_COLS as u32 * cell_metrics.cell_width as u32 + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * cell_metrics.cell_height as u32 + 2 * PADDING;
        assert!(width > 0);
        assert!(height > 0);
    }
}
