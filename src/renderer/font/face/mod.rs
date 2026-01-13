#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

pub use super::platform::Face;
pub use super::types::{FaceError, FaceMetrics, FontConfig, GlyphBuffer, RasterizedGlyph};
