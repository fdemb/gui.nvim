//! Font loader for Linux using FreeType.

use crate::renderer::font::FaceError;

/// Creates a FreeType font directly from font data.
///
/// TODO: Implement using `FT_New_Memory_Face` from FreeType.
pub fn create_font_from_bytes(_data: &[u8], _size_px: f32) -> Option<()> {
    // TODO: Use FT_New_Memory_Face to load font from memory
    None
}
