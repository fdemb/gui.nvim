//! Windows font loader implementation.
//!
//! TODO: Implement using DirectWrite to load fonts from memory directly.
//! The embedded Nerd Font data is available via `super::EMBEDDED_NERD_FONT`.

/// Creates a font directly from embedded data.
///
/// TODO: Implement using DirectWrite. Returns the platform's font type.
/// For now returns `()` as a placeholder since we don't have the Windows
/// font infrastructure yet.
pub fn create_embedded_nerd_font(_size_px: f32) -> Option<()> {
    log::warn!("create_embedded_nerd_font not yet implemented for Windows");
    None
}
