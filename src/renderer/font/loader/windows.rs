//! Windows font loader implementation using DirectWrite.
//!
//! TODO: Implement using:
//! - DirectWrite's `IDWriteFontCollection` or in-memory font loading
//! - Or `AddFontMemResourceEx` from GDI for process-wide registration

use std::sync::Once;

static REGISTER_FONTS: Once = Once::new();

pub fn register_embedded_fonts() {
    REGISTER_FONTS.call_once(|| {
        log::info!("Registering embedded fonts (Windows - not yet implemented)...");
        // TODO: Register embedded Nerd Font with DirectWrite or GDI
    });
}
