//! Font loader for Windows using DirectWrite.

use crate::renderer::font::FaceError;

/// Creates a DirectWrite font directly from font data.
///
/// TODO: Implement using DirectWrite in-memory font loader.
pub fn create_font_from_bytes(_data: &[u8], _size_px: f32) -> Option<()> {
    // TODO: Use DirectWrite in-memory font loader
    None
}
