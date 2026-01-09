#[cfg(target_os = "macos")]
mod cache;
mod collection;
mod face;
#[cfg(target_os = "macos")]
mod fallback;
#[cfg(not(target_os = "macos"))]
mod legacy;
#[cfg(target_os = "macos")]
mod run;
#[cfg(target_os = "macos")]
mod shaper;

// Shared types (defined in face.rs for macOS, legacy.rs for others)
#[cfg(target_os = "macos")]
pub use face::{FaceError, FontConfig, GlyphBuffer, RasterizedGlyph};
#[cfg(not(target_os = "macos"))]
pub use legacy::{
    CachedGlyph, FontConfig, FontError, FontSystem, GlyphBuffer, GlyphCache, RasterizedGlyph,
};

// macOS-only types (HarfBuzz shaping system)
#[cfg(target_os = "macos")]
pub use cache::{CachedGlyph as ShapedCachedGlyph, GlyphCacheKey, ShapedGlyphCache};
#[cfg(target_os = "macos")]
pub use run::RunIterator;
#[cfg(target_os = "macos")]
pub use shaper::{ShapedGlyph, Shaper, TextRun};

// Collection types are available on all platforms (stub on non-macOS)
pub use collection::{Collection, Style};
#[allow(unused_imports)]
pub use collection::CollectionIndex;
