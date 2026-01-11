use objc2_core_foundation::{CFData, CFError, CFRetained};
use objc2_core_graphics::{CGDataProvider, CGFont};
#[allow(deprecated)]
use objc2_core_text::CTFontManagerRegisterGraphicsFont;
use std::ptr::{self, NonNull};
use std::sync::Once;

use super::EMBEDDED_NERD_FONT;

static REGISTER_FONTS: Once = Once::new();

pub fn register_embedded_fonts() {
    REGISTER_FONTS.call_once(|| {
        log::info!("Registering embedded fonts...");

        let cf_data = CFData::from_static_bytes(EMBEDDED_NERD_FONT);

        let Some(provider) = CGDataProvider::with_cf_data(Some(&cf_data)) else {
            log::error!("Failed to create CGDataProvider from embedded data");
            return;
        };

        let Some(font) = CGFont::with_data_provider(&provider) else {
            log::error!("Failed to create CGFont from embedded data");
            return;
        };

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
        }
    });
}
