use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::bridge::events::RedrawEvent;
use crate::bridge::AppBridge;
use crate::config::Config;
use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, PADDING, PADDING_TOP};
use crate::editor::EditorState;
use crate::event::{GUIEvent, NeovimEvent, UserEvent};
use crate::input::InputHandler;
use crate::window::render_loop::RenderLoop;
use crate::window::settings::WindowSettings;

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

pub struct GuiApp {
    window: Option<Arc<Window>>,
    event_proxy: EventLoopProxy<UserEvent>,
    config: Config,
    args: Vec<String>,
    app_bridge: Option<AppBridge>,
    close_requested: bool,
    input_handler: InputHandler,
    editor_state: EditorState,
    render_loop: RenderLoop,
    settings: WindowSettings,
    current_scale_factor: f64,
}

impl GuiApp {
    pub fn new(event_proxy: EventLoopProxy<UserEvent>, config: Config, args: Vec<String>) -> Self {
        Self {
            window: None,
            event_proxy,
            config,
            args,
            app_bridge: None,
            close_requested: false,
            input_handler: InputHandler::new(),
            editor_state: EditorState::new(DEFAULT_COLS as usize, DEFAULT_ROWS as usize),
            render_loop: RenderLoop::new(),
            settings: WindowSettings::new(),
            current_scale_factor: 1.0,
        }
    }

    fn update_padding(&mut self, scale_factor: f64) {
        self.settings.update_padding(scale_factor);
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let (cell_width, cell_height) = (
            self.settings.cell_metrics.cell_width,
            self.settings.cell_metrics.cell_height,
        );
        let width = DEFAULT_COLS as u32 * cell_width as u32 + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * cell_height as u32 + PADDING + PADDING_TOP;

        let window_attrs = WindowAttributes::default()
            .with_title("gui.nvim")
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(200, 100));

        #[cfg(target_os = "macos")]
        let window_attrs = window_attrs
            .with_titlebar_transparent(true)
            .with_fullsize_content_view(true)
            .with_title_hidden(true);

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                log::info!("Window created: {:?}", window.id());
                self.current_scale_factor = window.scale_factor();
                self.update_padding(self.current_scale_factor);
                let window = Arc::new(window);
                self.window = Some(window.clone());

                self.render_loop
                    .initialize(window.clone(), self.config.clone());

                let bridge = AppBridge::new(self.event_proxy.clone());
                bridge.spawn_neovim(self.args.clone());
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

    fn update_metrics_and_resize(&mut self, cw: f32, ch: f32) {
        self.settings.cell_metrics.cell_width = cw as f64;
        self.settings.cell_metrics.cell_height = ch as f64;

        if let Some(ref bridge) = self.app_bridge {
            if let Some(ref window) = self.window {
                let size = window.inner_size();
                let (cols, rows) = self.settings.calculate_grid_size(size.width, size.height);
                if cols != self.settings.cols || rows != self.settings.rows {
                    self.settings.cols = cols;
                    self.settings.rows = rows;
                    bridge.resize(cols, rows);
                }
            }
        }
    }

    fn poll_renderer(&mut self) {
        if let Some(ref window) = self.window {
            use std::task::Poll;
            if let Poll::Ready(Ok(renderer)) = self.render_loop.poll(window) {
                let (cw, ch) = renderer.cell_size();
                if self.settings.cell_metrics.cell_width != cw as f64
                    || self.settings.cell_metrics.cell_height != ch as f64
                {
                    self.update_metrics_and_resize(cw, ch);
                }
            }
        }
    }

    fn update_layout(&mut self, scale_factor: f64) {
        if let Some(renderer) = self.render_loop.renderer() {
            if let Err(e) = renderer.update_font(&self.config, scale_factor) {
                log::error!("Failed to update font: {}", e);
            } else {
                let (cw, ch) = renderer.cell_size();
                self.update_metrics_and_resize(cw, ch);

                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn handle_option_set(&mut self, name: &str, value: &nvim_rs::Value) {
        if name == "guifont" {
            if let Some(s) = value.as_str() {
                if let Some(font_settings) = crate::config::FontSettings::from_guifont(s) {
                    log::info!("Updating font: {:?}", font_settings);

                    if let Some(f) = font_settings.family {
                        self.config.font.family = Some(f);
                    }
                    if let Some(s) = font_settings.size {
                        self.config.font.size = Some(s);
                    }

                    if let Some(window) = &self.window {
                        let scale_factor = window.scale_factor();
                        self.update_layout(scale_factor);
                    }
                }
            }
        }
    }

    fn apply_redraw_events(&mut self, events: Vec<RedrawEvent>) {
        for event in events {
            self.editor_state.handle_redraw_event(&event);

            match event {
                RedrawEvent::DefaultColorsSet { fg, bg, .. } => {
                    if let Some(renderer) = self.render_loop.renderer() {
                        renderer.update_default_colors(fg, bg);
                    }
                }
                RedrawEvent::SetTitle { title } => {
                    if let Some(ref window) = self.window {
                        window.set_title(&title);
                    }
                }
                RedrawEvent::OptionSet { name, value } => {
                    self.handle_option_set(&name, &value);
                }
                _ => {}
            }
        }
    }

    fn do_render(&mut self) {
        if let Some(window) = &self.window {
            if let Err(_) = self.render_loop.render(
                &self.editor_state,
                self.settings.cell_metrics.padding_x as f32,
                self.settings.cell_metrics.padding_y as f32,
                window,
            ) {
                if self.render_loop.renderer().is_none() {
                    // Failed or not ready, nothing to do
                } else {
                    // Out of memory was logged in render_loop
                    self.close_requested = true;
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

                    if let Some(renderer) = self.render_loop.renderer() {
                        renderer.resize(size);
                    }

                    let (cols, rows) = self.settings.calculate_grid_size(size.width, size.height);
                    if cols != self.settings.cols || rows != self.settings.rows {
                        self.settings.cols = cols;
                        self.settings.rows = rows;
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
                self.input_handler.handle_modifiers_changed(modifiers);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(ref bridge) = self.app_bridge {
                    self.input_handler.handle_keyboard_input(&event, bridge);
                }

                if event.state == ElementState::Pressed {
                    let _ = self
                        .event_proxy
                        .send_event(UserEvent::GUI(GUIEvent::KeyboardInput(event)));
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(ref bridge) = self.app_bridge {
                    self.input_handler.handle_mouse_input(state, button, bridge);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref bridge) = self.app_bridge {
                    self.input_handler.handle_cursor_moved(
                        position,
                        &self.settings.cell_metrics,
                        bridge,
                    );
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(ref bridge) = self.app_bridge {
                    self.input_handler.handle_mouse_wheel(delta, bridge);
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if (self.current_scale_factor - scale_factor).abs() >= f64::EPSILON {
                    log::debug!("Scale factor changed: {}", scale_factor);
                    self.current_scale_factor = scale_factor;
                    self.update_padding(scale_factor);
                    let _ = self
                        .event_proxy
                        .send_event(UserEvent::GUI(GUIEvent::ScaleFactorChanged(scale_factor)));
                }
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
            UserEvent::GUI(event) => match event {
                GUIEvent::ScaleFactorChanged(scale_factor) => {
                    self.update_layout(scale_factor);
                }
                _ => {}
            },
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
    use crate::constants::PADDING;

    #[test]
    fn test_default_dimensions() {
        let settings = WindowSettings::new();
        let width = DEFAULT_COLS as u32 * settings.cell_metrics.cell_width as u32 + 2 * PADDING;
        let height = DEFAULT_ROWS as u32 * settings.cell_metrics.cell_height as u32 + 2 * PADDING;
        assert!(width > 0);
        assert!(height > 0);
    }
}
