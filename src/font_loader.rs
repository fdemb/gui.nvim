#[cfg(target_os = "macos")]
mod macos {
    use core_foundation::base::CFTypeRef;
    use core_foundation::error::CFErrorRef;
    use core_graphics::data_provider::CGDataProvider;
    use core_graphics::font::CGFont;
    use foreign_types::ForeignType;
    use std::sync::{Arc, Once};

    static REGISTER_FONTS: Once = Once::new();

    #[link(name = "CoreText", kind = "framework")]
    extern "C" {
        fn CTFontManagerRegisterGraphicsFont(font: CFTypeRef, error: *mut CFErrorRef) -> bool;
    }

    pub fn register_embedded_fonts() {
        REGISTER_FONTS.call_once(|| {
            log::info!("Registering embedded fonts...");
            let font_data = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");

            // Create data provider from the embedded bytes
            // We need to wrap it in Arc for CGDataProvider
            let provider = CGDataProvider::from_buffer(Arc::new(font_data));

            // Create CGFont from the provider
            let font = match CGFont::from_data_provider(provider) {
                Ok(f) => f,
                Err(_) => {
                    log::error!("Failed to create CGFont from embedded data");
                    return;
                }
            };

            // Register the font with the system (for this process)
            let mut error: CFErrorRef = std::ptr::null_mut();
            let success = unsafe {
                // CGFont typically exposes as_ptr() to get the underlying CGFontRef
                CTFontManagerRegisterGraphicsFont(font.as_ptr() as CFTypeRef, &mut error)
            };

            if !success {
                // TODO: Convert CFErrorRef to something readable if needed
                log::error!("Failed to register embedded font 'Symbols Nerd Font'");
            } else {
                log::info!("Successfully registered embedded font: Symbols Nerd Font");
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
mod other {
    pub fn register_embedded_fonts() {
        log::debug!("Embedded fonts registration not implemented for this platform");
    }
}

pub fn register_embedded_fonts() {
    #[cfg(target_os = "macos")]
    macos::register_embedded_fonts();
    #[cfg(not(target_os = "macos"))]
    other::register_embedded_fonts();
}
