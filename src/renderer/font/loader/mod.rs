#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

pub const EMBEDDED_NERD_FONT: &[u8] =
    include_bytes!("../../../../assets/fonts/SymbolsNerdFont-Regular.ttf");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_exists() {
        assert!(
            !EMBEDDED_NERD_FONT.is_empty(),
            "Embedded font data should not be empty"
        );
    }

    #[test]
    fn test_register_embedded_fonts_no_panic() {
        register_embedded_fonts();
    }
}
