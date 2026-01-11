#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

use crate::config::FontSettings;

/// Rasterized glyph with positioning data.
#[derive(Clone)]
pub struct RasterizedGlyph {
    #[allow(dead_code)]
    pub character: char,
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub buffer: GlyphBuffer,
}

#[derive(Clone)]
pub enum GlyphBuffer {
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

impl GlyphBuffer {
    pub fn is_colored(&self) -> bool {
        matches!(self, GlyphBuffer::Rgba(_))
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            GlyphBuffer::Rgb(b) | GlyphBuffer::Rgba(b) => b,
        }
    }
}

/// Font configuration with fallback chain.
pub struct FontConfig {
    pub family: String,
    pub size_pt: f32,
    pub scale_factor: f32,
}

impl FontConfig {
    pub fn new(settings: &FontSettings, scale_factor: f64) -> Self {
        Self {
            family: settings.family.clone().unwrap_or_else(default_font_family),
            size_pt: settings.size.unwrap_or(14.0),
            scale_factor: scale_factor as f32,
        }
    }

    #[allow(dead_code)]
    pub fn scaled_size(&self) -> f32 {
        self.size_pt * self.scale_factor
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size_pt: 14.0,
            scale_factor: 1.0,
        }
    }
}

#[cfg(target_os = "macos")]
fn default_font_family() -> String {
    "Menlo".to_string()
}

#[cfg(target_os = "linux")]
fn default_font_family() -> String {
    "monospace".to_string()
}

#[cfg(target_os = "windows")]
fn default_font_family() -> String {
    "Consolas".to_string()
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FaceMetrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikeout_position: f32,
    pub strikeout_thickness: f32,
}

impl Default for FaceMetrics {
    fn default() -> Self {
        Self {
            cell_width: 8.0,
            cell_height: 16.0,
            ascent: 12.0,
            descent: 4.0,
            line_gap: 0.0,
            underline_position: 2.0,
            underline_thickness: 1.0,
            strikeout_position: 6.0,
            strikeout_thickness: 1.0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FaceError {
    #[error("Failed to create font with name: {0}")]
    FontNotFound(String),
    #[error("Failed to create graphics context")]
    ContextCreationFailed,
    #[error("Failed to create HarfBuzz face")]
    HarfBuzzFaceCreation,
    #[error("Failed to copy font table")]
    TableCopyFailed,
    #[error("Platform not implemented")]
    NotImplemented,
}
