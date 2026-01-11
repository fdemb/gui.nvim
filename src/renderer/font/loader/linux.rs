//! Linux font loader implementation using fontconfig.
//!
//! TODO: Implement using:
//! - `fontconfig` crate to register the embedded Nerd Font
//! - Or write the font to a temp file and add to fontconfig search path

use std::sync::Once;

static REGISTER_FONTS: Once = Once::new();

pub fn register_embedded_fonts() {
    REGISTER_FONTS.call_once(|| {
        log::info!("Registering embedded fonts (Linux - not yet implemented)...");
        // TODO: Register embedded Nerd Font with fontconfig
    });
}
