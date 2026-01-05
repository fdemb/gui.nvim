use std::collections::HashMap;

use bitflags::bitflags;

/// RGBA color represented as a 32-bit value.
/// Format: 0xRRGGBBAA (alpha is always 0xFF for solid colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(pub u32);

impl Color {
    /// Creates a new color from RGB values.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF)
    }

    /// Creates a new color from a 24-bit RGB value (as sent by Neovim).
    pub const fn from_u24(rgb: u32) -> Self {
        let r = ((rgb >> 16) & 0xFF) as u8;
        let g = ((rgb >> 8) & 0xFF) as u8;
        let b = (rgb & 0xFF) as u8;
        Self::from_rgb(r, g, b)
    }

    /// Returns the red component.
    #[allow(dead_code)]
    pub const fn r(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    /// Returns the green component.
    #[allow(dead_code)]
    pub const fn g(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Returns the blue component.
    #[allow(dead_code)]
    pub const fn b(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Returns the alpha component.
    #[allow(dead_code)]
    pub const fn a(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Returns the color as normalized float array [r, g, b, a] for GPU.
    #[allow(dead_code)]
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            self.r() as f32 / 255.0,
            self.g() as f32 / 255.0,
            self.b() as f32 / 255.0,
            self.a() as f32 / 255.0,
        ]
    }
}

bitflags! {
    /// Style flags for highlight attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StyleFlags: u16 {
        const BOLD = 0b0000_0000_0001;
        const ITALIC = 0b0000_0000_0010;
        const UNDERLINE = 0b0000_0000_0100;
        const UNDERCURL = 0b0000_0000_1000;
        const UNDERDOUBLE = 0b0000_0001_0000;
        const UNDERDOTTED = 0b0000_0010_0000;
        const UNDERDASHED = 0b0000_0100_0000;
        const STRIKETHROUGH = 0b0000_1000_0000;
        const REVERSE = 0b0001_0000_0000;
        const ALTFONT = 0b0010_0000_0000;
    }
}

/// Underline style derived from StyleFlags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curl,
    Dotted,
    Dashed,
}

impl From<StyleFlags> for UnderlineStyle {
    fn from(flags: StyleFlags) -> Self {
        if flags.contains(StyleFlags::UNDERCURL) {
            UnderlineStyle::Curl
        } else if flags.contains(StyleFlags::UNDERDOUBLE) {
            UnderlineStyle::Double
        } else if flags.contains(StyleFlags::UNDERDOTTED) {
            UnderlineStyle::Dotted
        } else if flags.contains(StyleFlags::UNDERDASHED) {
            UnderlineStyle::Dashed
        } else if flags.contains(StyleFlags::UNDERLINE) {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        }
    }
}

/// Highlight attributes as defined by Neovim's `hl_attr_define` event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HighlightAttributes {
    /// Foreground color (None means use default).
    pub foreground: Option<Color>,
    /// Background color (None means use default).
    pub background: Option<Color>,
    /// Special color for underlines (None means use foreground).
    pub special: Option<Color>,
    /// Style flags.
    pub style: StyleFlags,
    /// Blend level (0-100) for transparency.
    pub blend: u8,
    /// URL for clickable hyperlinks.
    pub url: Option<String>,
}

impl HighlightAttributes {
    /// Creates new highlight attributes.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the effective foreground color, applying reverse if set.
    #[allow(dead_code)]
    pub fn effective_fg(&self, defaults: &DefaultColors) -> Color {
        if self.style.contains(StyleFlags::REVERSE) {
            self.background.unwrap_or(defaults.background)
        } else {
            self.foreground.unwrap_or(defaults.foreground)
        }
    }

    /// Returns the effective background color, applying reverse if set.
    #[allow(dead_code)]
    pub fn effective_bg(&self, defaults: &DefaultColors) -> Color {
        if self.style.contains(StyleFlags::REVERSE) {
            self.foreground.unwrap_or(defaults.foreground)
        } else {
            self.background.unwrap_or(defaults.background)
        }
    }

    /// Returns the special color for underlines.
    #[allow(dead_code)]
    pub fn effective_special(&self, defaults: &DefaultColors) -> Color {
        self.special.or(self.foreground).unwrap_or(defaults.special)
    }

    /// Returns the underline style.
    pub fn underline_style(&self) -> UnderlineStyle {
        self.style.into()
    }

    /// Returns true if the text should be bold.
    #[allow(dead_code)]
    pub fn is_bold(&self) -> bool {
        self.style.contains(StyleFlags::BOLD)
    }

    /// Returns true if the text should be italic.
    #[allow(dead_code)]
    pub fn is_italic(&self) -> bool {
        self.style.contains(StyleFlags::ITALIC)
    }

    /// Returns true if the text has strikethrough.
    pub fn has_strikethrough(&self) -> bool {
        self.style.contains(StyleFlags::STRIKETHROUGH)
    }
}

/// Default colors as defined by Neovim's `default_colors_set` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultColors {
    pub foreground: Color,
    pub background: Color,
    pub special: Color,
}

impl Default for DefaultColors {
    fn default() -> Self {
        Self {
            foreground: Color::from_rgb(0xD4, 0xD4, 0xD4), // Light gray
            background: Color::from_rgb(0x1E, 0x1E, 0x1E), // Dark gray
            special: Color::from_rgb(0xAE, 0xAF, 0xAD),    // Gray
        }
    }
}

/// Map of highlight IDs to their attributes.
///
/// Neovim sends `hl_attr_define` events that define highlights by ID.
/// ID 0 always uses default colors with no styles.
#[derive(Debug, Clone, Default)]
pub struct HighlightMap {
    attributes: HashMap<u64, HighlightAttributes>,
    pub defaults: DefaultColors,
}

impl HighlightMap {
    /// Creates a new highlight map with default colors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines highlight attributes for the given ID.
    pub fn define(&mut self, id: u64, attrs: HighlightAttributes) {
        if id == 0 {
            // ID 0 is reserved for default, don't store it
            return;
        }
        self.attributes.insert(id, attrs);
    }

    /// Gets the highlight attributes for the given ID.
    /// Returns default attributes for ID 0 or unknown IDs.
    pub fn get(&self, id: u64) -> &HighlightAttributes {
        static DEFAULT_ATTRS: HighlightAttributes = HighlightAttributes {
            foreground: None,
            background: None,
            special: None,
            style: StyleFlags::empty(),
            blend: 0,
            url: None,
        };

        if id == 0 {
            &DEFAULT_ATTRS
        } else {
            self.attributes.get(&id).unwrap_or(&DEFAULT_ATTRS)
        }
    }

    /// Sets the default colors.
    pub fn set_defaults(&mut self, foreground: Color, background: Color, special: Color) {
        self.defaults = DefaultColors {
            foreground,
            background,
            special,
        };
    }

    /// Clears all defined highlights (but keeps defaults).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.attributes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_rgb() {
        let color = Color::from_rgb(0xFF, 0x80, 0x00);
        assert_eq!(color.r(), 0xFF);
        assert_eq!(color.g(), 0x80);
        assert_eq!(color.b(), 0x00);
        assert_eq!(color.a(), 0xFF);
    }

    #[test]
    fn test_color_from_u24() {
        // Neovim sends colors as 24-bit RGB
        let color = Color::from_u24(0xFF8000);
        assert_eq!(color.r(), 0xFF);
        assert_eq!(color.g(), 0x80);
        assert_eq!(color.b(), 0x00);
    }

    #[test]
    fn test_color_to_f32_array() {
        let color = Color::from_rgb(255, 128, 0);
        let arr = color.to_f32_array();
        assert!((arr[0] - 1.0).abs() < 0.01);
        assert!((arr[1] - 0.5).abs() < 0.01);
        assert!((arr[2] - 0.0).abs() < 0.01);
        assert!((arr[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_underline_style_from_flags() {
        assert_eq!(
            UnderlineStyle::from(StyleFlags::empty()),
            UnderlineStyle::None
        );
        assert_eq!(
            UnderlineStyle::from(StyleFlags::UNDERLINE),
            UnderlineStyle::Single
        );
        assert_eq!(
            UnderlineStyle::from(StyleFlags::UNDERCURL),
            UnderlineStyle::Curl
        );
        assert_eq!(
            UnderlineStyle::from(StyleFlags::UNDERDOUBLE),
            UnderlineStyle::Double
        );
    }

    #[test]
    fn test_highlight_attributes_effective_colors() {
        let defaults = DefaultColors::default();

        // Normal case
        let attrs = HighlightAttributes {
            foreground: Some(Color::from_rgb(255, 0, 0)),
            background: Some(Color::from_rgb(0, 255, 0)),
            ..Default::default()
        };
        assert_eq!(attrs.effective_fg(&defaults), Color::from_rgb(255, 0, 0));
        assert_eq!(attrs.effective_bg(&defaults), Color::from_rgb(0, 255, 0));

        // Reverse case
        let attrs_reverse = HighlightAttributes {
            foreground: Some(Color::from_rgb(255, 0, 0)),
            background: Some(Color::from_rgb(0, 255, 0)),
            style: StyleFlags::REVERSE,
            ..Default::default()
        };
        assert_eq!(
            attrs_reverse.effective_fg(&defaults),
            Color::from_rgb(0, 255, 0)
        );
        assert_eq!(
            attrs_reverse.effective_bg(&defaults),
            Color::from_rgb(255, 0, 0)
        );

        // Default colors
        let attrs_default = HighlightAttributes::default();
        assert_eq!(attrs_default.effective_fg(&defaults), defaults.foreground);
        assert_eq!(attrs_default.effective_bg(&defaults), defaults.background);
    }

    #[test]
    fn test_highlight_map_define_and_get() {
        let mut map = HighlightMap::new();

        let attrs = HighlightAttributes {
            foreground: Some(Color::from_rgb(255, 0, 0)),
            style: StyleFlags::BOLD,
            ..Default::default()
        };
        map.define(1, attrs.clone());

        let retrieved = map.get(1);
        assert_eq!(retrieved.foreground, Some(Color::from_rgb(255, 0, 0)));
        assert!(retrieved.is_bold());
    }

    #[test]
    fn test_highlight_map_id_zero() {
        let mut map = HighlightMap::new();

        // ID 0 should not be stored
        let attrs = HighlightAttributes {
            foreground: Some(Color::from_rgb(255, 0, 0)),
            ..Default::default()
        };
        map.define(0, attrs);

        // ID 0 should always return default
        let retrieved = map.get(0);
        assert_eq!(retrieved.foreground, None);
    }

    #[test]
    fn test_highlight_map_unknown_id() {
        let map = HighlightMap::new();

        // Unknown ID should return default
        let retrieved = map.get(999);
        assert_eq!(retrieved.foreground, None);
        assert!(!retrieved.is_bold());
    }

    #[test]
    fn test_highlight_map_set_defaults() {
        let mut map = HighlightMap::new();
        map.set_defaults(
            Color::from_rgb(255, 255, 255),
            Color::from_rgb(0, 0, 0),
            Color::from_rgb(128, 128, 128),
        );

        assert_eq!(map.defaults.foreground, Color::from_rgb(255, 255, 255));
        assert_eq!(map.defaults.background, Color::from_rgb(0, 0, 0));
        assert_eq!(map.defaults.special, Color::from_rgb(128, 128, 128));
    }

    #[test]
    fn test_style_flags() {
        let mut flags = StyleFlags::empty();
        assert!(!flags.contains(StyleFlags::BOLD));

        flags |= StyleFlags::BOLD | StyleFlags::ITALIC;
        assert!(flags.contains(StyleFlags::BOLD));
        assert!(flags.contains(StyleFlags::ITALIC));
        assert!(!flags.contains(StyleFlags::UNDERLINE));
    }
}
