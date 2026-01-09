#[cfg(target_os = "macos")]
mod macos {
    use objc2_core_foundation::{CFData, CFError, CFRetained};
    use objc2_core_graphics::{CGDataProvider, CGFont};
    #[allow(deprecated)]
    use objc2_core_text::CTFontManagerRegisterGraphicsFont;
    use std::ptr::{self, NonNull};
    use std::sync::Once;

    static REGISTER_FONTS: Once = Once::new();

    pub fn register_embedded_fonts() {
        REGISTER_FONTS.call_once(|| {
            log::info!("Registering embedded fonts...");

            static FONT_DATA: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");

            let cf_data = CFData::from_static_bytes(FONT_DATA);

            let provider = CGDataProvider::with_cf_data(Some(&cf_data));

            let provider = match provider {
                Some(p) => p,
                None => {
                    log::error!("Failed to create CGDataProvider from embedded data");
                    return;
                }
            };

            let font = CGFont::with_data_provider(&provider);

            let font = match font {
                Some(f) => f,
                None => {
                    log::error!("Failed to create CGFont from embedded data");
                    return;
                }
            };

            // CTFontManagerRegisterGraphicsFont is deprecated but still functional.
            // The recommended alternatives (CTFontManagerCreateFontDescriptorsFromData,
            // CTFontManagerRegisterFontsForURL) require more setup for embedded fonts.
            #[allow(deprecated)]
            let success = unsafe {
                let mut error: *mut CFError = ptr::null_mut();
                let result = CTFontManagerRegisterGraphicsFont(&font, &mut error);
                if !result && !error.is_null() {
                    let error_retained: CFRetained<CFError> =
                        CFRetained::from_raw(NonNull::new_unchecked(error));
                    log::error!(
                        "Failed to register embedded font 'Symbols Nerd Font': {}",
                        error_retained
                    );
                }
                result
            };

            if success {
                log::info!("Successfully registered embedded font: Symbols Nerd Font");
            } else {
                log::error!("Failed to register embedded font 'Symbols Nerd Font'");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_exists() {
        let font_data = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
        assert!(
            !font_data.is_empty(),
            "Embedded font data should not be empty"
        );
    }

    #[test]
    fn test_register_embedded_fonts_no_panic() {
        register_embedded_fonts();
    }
}
