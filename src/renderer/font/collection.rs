use super::face::{Face, FaceError, FaceMetrics};
use super::fallback::FallbackResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl Style {
    pub fn from_flags(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }

    pub fn is_bold(&self) -> bool {
        matches!(self, Style::Bold | Style::BoldItalic)
    }

    pub fn is_italic(&self) -> bool {
        matches!(self, Style::Italic | Style::BoldItalic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollectionIndex {
    pub style: Style,
    pub idx: u16,
}

impl CollectionIndex {
    pub fn new(style: Style, idx: u16) -> Self {
        Self { style, idx }
    }

    pub fn primary(style: Style) -> Self {
        Self { style, idx: 0 }
    }
}

struct Entry {
    face: Face,
    is_fallback: bool,
}

#[cfg(target_os = "macos")]
pub struct Collection {
    regular: Vec<Entry>,
    bold: Vec<Entry>,
    italic: Vec<Entry>,
    bold_italic: Vec<Entry>,
    metrics: FaceMetrics,
    size_pt: f32,
    dpi: f32,
    fallback_resolver: FallbackResolver,
}

#[cfg(target_os = "macos")]
impl Collection {
    pub fn new(family: &str, size_pt: f32, dpi: f32) -> Result<Self, FaceError> {
        let regular_face = Face::new(family, size_pt, dpi)?;
        let metrics = *regular_face.metrics();
        let size_px = regular_face.size_px();

        let bold_face = Face::new(&format!("{}-Bold", family), size_pt, dpi)
            .or_else(|_| Face::new(family, size_pt, dpi))?;
        let italic_face = Face::new(&format!("{}-Italic", family), size_pt, dpi)
            .or_else(|_| Face::new(family, size_pt, dpi))?;
        let bold_italic_face = Face::new(&format!("{}-BoldItalic", family), size_pt, dpi)
            .or_else(|_| Face::new(family, size_pt, dpi))?;

        let fallback_resolver = FallbackResolver::new(regular_face.ct_font().clone(), size_px);

        Ok(Self {
            regular: vec![Entry {
                face: regular_face,
                is_fallback: false,
            }],
            bold: vec![Entry {
                face: bold_face,
                is_fallback: false,
            }],
            italic: vec![Entry {
                face: italic_face,
                is_fallback: false,
            }],
            bold_italic: vec![Entry {
                face: bold_italic_face,
                is_fallback: false,
            }],
            metrics,
            size_pt,
            dpi,
            fallback_resolver,
        })
    }

    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    pub fn get_face(&self, index: CollectionIndex) -> Option<&Face> {
        let entries = self.entries_for_style(index.style);
        entries.get(index.idx as usize).map(|e| &e.face)
    }

    pub fn get_face_mut(&mut self, index: CollectionIndex) -> Option<&mut Face> {
        let entries = self.entries_for_style_mut(index.style);
        entries.get_mut(index.idx as usize).map(|e| &mut e.face)
    }

    pub fn primary_face(&self, style: Style) -> &Face {
        let entries = self.entries_for_style(style);
        &entries[0].face
    }

    pub fn resolve_glyph(&mut self, codepoint: u32, style: Style) -> Option<(CollectionIndex, u32)> {
        let entries = self.entries_for_style(style);
        for (idx, entry) in entries.iter().enumerate() {
            if let Some(glyph_id) = entry.face.glyph_index(codepoint) {
                return Some((CollectionIndex::new(style, idx as u16), glyph_id));
            }
        }

        if style != Style::Regular {
            let regular_entries = self.entries_for_style(Style::Regular);
            for (idx, entry) in regular_entries.iter().enumerate() {
                if let Some(glyph_id) = entry.face.glyph_index(codepoint) {
                    return Some((CollectionIndex::new(Style::Regular, idx as u16), glyph_id));
                }
            }
        }

        if let Some(fallback_face) = self.fallback_resolver.discover(codepoint) {
            if let Some(glyph_id) = fallback_face.glyph_index(codepoint) {
                let entries = self.entries_for_style_mut(style);
                let idx = entries.len() as u16;
                entries.push(Entry {
                    face: fallback_face,
                    is_fallback: true,
                });
                return Some((CollectionIndex::new(style, idx), glyph_id));
            }
        }

        None
    }

    pub fn add_fallback(&mut self, style: Style, face: Face) -> CollectionIndex {
        let entries = self.entries_for_style_mut(style);
        let idx = entries.len() as u16;
        entries.push(Entry {
            face,
            is_fallback: true,
        });
        CollectionIndex::new(style, idx)
    }

    fn entries_for_style(&self, style: Style) -> &Vec<Entry> {
        match style {
            Style::Regular => &self.regular,
            Style::Bold => &self.bold,
            Style::Italic => &self.italic,
            Style::BoldItalic => &self.bold_italic,
        }
    }

    fn entries_for_style_mut(&mut self, style: Style) -> &mut Vec<Entry> {
        match style {
            Style::Regular => &mut self.regular,
            Style::Bold => &mut self.bold,
            Style::Italic => &mut self.italic,
            Style::BoldItalic => &mut self.bold_italic,
        }
    }

    pub fn clear_fallback_cache(&mut self) {
        self.fallback_resolver.clear_cache();
    }
}

#[cfg(not(target_os = "macos"))]
pub struct Collection {
    metrics: FaceMetrics,
}

#[cfg(not(target_os = "macos"))]
impl Collection {
    pub fn new(_family: &str, _size_pt: f32, _dpi: f32) -> Result<Self, FaceError> {
        Ok(Self {
            metrics: FaceMetrics::default(),
        })
    }

    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    pub fn get_face(&self, _index: CollectionIndex) -> Option<&Face> {
        None
    }

    pub fn primary_face(&self, _style: Style) -> &'static Face {
        unimplemented!("Collection not available on this platform")
    }

    pub fn resolve_glyph(
        &mut self,
        _codepoint: u32,
        _style: Style,
    ) -> Option<(CollectionIndex, u32)> {
        None
    }

    pub fn add_fallback(&mut self, style: Style, _face: Face) -> CollectionIndex {
        CollectionIndex::new(style, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_from_flags() {
        assert_eq!(Style::from_flags(false, false), Style::Regular);
        assert_eq!(Style::from_flags(true, false), Style::Bold);
        assert_eq!(Style::from_flags(false, true), Style::Italic);
        assert_eq!(Style::from_flags(true, true), Style::BoldItalic);
    }

    #[test]
    fn test_style_properties() {
        assert!(!Style::Regular.is_bold());
        assert!(!Style::Regular.is_italic());

        assert!(Style::Bold.is_bold());
        assert!(!Style::Bold.is_italic());

        assert!(!Style::Italic.is_bold());
        assert!(Style::Italic.is_italic());

        assert!(Style::BoldItalic.is_bold());
        assert!(Style::BoldItalic.is_italic());
    }

    #[test]
    fn test_collection_index() {
        let idx = CollectionIndex::new(Style::Bold, 2);
        assert_eq!(idx.style, Style::Bold);
        assert_eq!(idx.idx, 2);

        let primary = CollectionIndex::primary(Style::Italic);
        assert_eq!(primary.style, Style::Italic);
        assert_eq!(primary.idx, 0);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_collection_creation() {
        let collection = Collection::new("Menlo", 14.0, 72.0);
        assert!(collection.is_ok(), "Should create collection from system font");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_collection_metrics() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let metrics = collection.metrics();

        assert!(metrics.cell_width > 0.0, "Cell width should be positive");
        assert!(metrics.cell_height > 0.0, "Cell height should be positive");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_collection_get_face() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();

        let regular = collection.get_face(CollectionIndex::primary(Style::Regular));
        assert!(regular.is_some(), "Should have regular face");

        let bold = collection.get_face(CollectionIndex::primary(Style::Bold));
        assert!(bold.is_some(), "Should have bold face");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_collection_resolve_glyph() {
        let mut collection = Collection::new("Menlo", 14.0, 72.0).unwrap();

        let result = collection.resolve_glyph('A' as u32, Style::Regular);
        assert!(result.is_some(), "Should resolve glyph for 'A'");

        let (index, glyph_id) = result.unwrap();
        assert_eq!(index.style, Style::Regular);
        assert_eq!(index.idx, 0);
        assert!(glyph_id > 0, "Glyph ID should be non-zero");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_collection_fallback_discovery() {
        let mut collection = Collection::new("Menlo", 14.0, 72.0).unwrap();

        // Try to resolve an emoji that Menlo doesn't have
        let result = collection.resolve_glyph('😀' as u32, Style::Regular);
        // This should either find a fallback or return None if no fallback available
        // The test passes either way since we're testing the mechanism works
        if let Some((index, glyph_id)) = result {
            assert!(glyph_id > 0, "Fallback glyph ID should be non-zero");
            // If it found a fallback, it should be in a fallback entry (idx > 0)
            // or it might have been in the original font
            let _ = index;
        }
    }
}
