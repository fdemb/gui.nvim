mod cache;
mod collection;
mod face;
mod fallback;
mod legacy;
mod run;
mod shaper;

#[allow(unused_imports)]
pub use cache::{CachedGlyph as ShapedCachedGlyph, GlyphCacheKey, ShapedGlyphCache};
#[allow(unused_imports)]
pub use collection::{Collection, CollectionIndex, Style};
#[allow(unused_imports)]
pub use face::FaceError;
#[allow(unused_imports)]
pub use run::RunIterator;
#[allow(unused_imports)]
pub use shaper::{ShapedGlyph, Shaper, TextRun};

pub use legacy::{
    CachedGlyph, FontConfig, FontError, FontSystem, GlyphBuffer, GlyphCache, RasterizedGlyph,
};
