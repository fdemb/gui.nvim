#[cfg(target_os = "linux")]
pub use super::platform::Face;

#[cfg(target_os = "windows")]
pub use super::platform::Face;

pub use super::types::{FaceError, FontConfig, GlyphBuffer, RasterizedGlyph};
