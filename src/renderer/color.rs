/// Color conversion utilities for GPU rendering.
///
/// Colors from Neovim are in sRGB space, but GPU framebuffers with sRGB format
/// expect linear colors as input (they auto-convert linear→sRGB on output).
/// These utilities handle the necessary conversions.

/// Convert sRGB color component to linear space.
/// The GPU will convert back to sRGB when writing to the framebuffer.
#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert a packed RGB u32 color (0xRRGGBB) to linear RGBA.
#[inline]
pub fn u32_to_linear_rgba(color: u32) -> [f32; 4] {
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
    let b = (color & 0xFF) as f32 / 255.0;
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_to_linear_black() {
        assert!((srgb_to_linear(0.0) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_srgb_to_linear_white() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_srgb_to_linear_mid_gray() {
        // sRGB 0.5 should convert to approximately 0.214
        let linear = srgb_to_linear(0.5);
        assert!((linear - 0.214).abs() < 0.01);
    }

    #[test]
    fn test_u32_to_linear_rgba_white() {
        let rgba = u32_to_linear_rgba(0xFFFFFF);
        assert!((rgba[0] - 1.0).abs() < 0.0001);
        assert!((rgba[1] - 1.0).abs() < 0.0001);
        assert!((rgba[2] - 1.0).abs() < 0.0001);
        assert!((rgba[3] - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_u32_to_linear_rgba_black() {
        let rgba = u32_to_linear_rgba(0x000000);
        assert!((rgba[0] - 0.0).abs() < 0.0001);
        assert!((rgba[1] - 0.0).abs() < 0.0001);
        assert!((rgba[2] - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_u32_to_linear_rgba_red() {
        let rgba = u32_to_linear_rgba(0xFF0000);
        assert!((rgba[0] - 1.0).abs() < 0.0001);
        assert!((rgba[1] - 0.0).abs() < 0.0001);
        assert!((rgba[2] - 0.0).abs() < 0.0001);
    }
}

