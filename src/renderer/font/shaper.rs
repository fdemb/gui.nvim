use super::collection::{Collection, CollectionIndex, Style};
use super::platform::Face;

#[derive(Debug, Clone, Copy)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub x_advance: i32,
    #[allow(dead_code)]
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub font_index: CollectionIndex,
}

pub struct TextRun<'a> {
    pub text: &'a str,
    pub style: Style,
}

struct HbBuffer {
    ptr: *mut harfbuzz_sys::hb_buffer_t,
}

impl HbBuffer {
    fn new() -> Option<Self> {
        let ptr = unsafe { harfbuzz_sys::hb_buffer_create() };
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }

    fn reset(&mut self) {
        unsafe { harfbuzz_sys::hb_buffer_reset(self.ptr) };
    }

    fn add_str(&mut self, text: &str) {
        unsafe {
            harfbuzz_sys::hb_buffer_add_utf8(
                self.ptr,
                text.as_ptr() as *const i8,
                text.len() as i32,
                0,
                text.len() as i32,
            );
        }
    }

    fn set_direction(&mut self, dir: harfbuzz_sys::hb_direction_t) {
        unsafe { harfbuzz_sys::hb_buffer_set_direction(self.ptr, dir) };
    }

    fn guess_segment_properties(&mut self) {
        unsafe { harfbuzz_sys::hb_buffer_guess_segment_properties(self.ptr) };
    }

    #[allow(dead_code)]
    fn get_length(&self) -> u32 {
        unsafe { harfbuzz_sys::hb_buffer_get_length(self.ptr) }
    }

    fn get_glyph_infos(&self) -> &[harfbuzz_sys::hb_glyph_info_t] {
        let mut len = 0u32;
        let ptr = unsafe { harfbuzz_sys::hb_buffer_get_glyph_infos(self.ptr, &mut len) };
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len as usize) }
        }
    }

    fn get_glyph_positions(&self) -> &[harfbuzz_sys::hb_glyph_position_t] {
        let mut len = 0u32;
        let ptr = unsafe { harfbuzz_sys::hb_buffer_get_glyph_positions(self.ptr, &mut len) };
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len as usize) }
        }
    }
}

impl Drop for HbBuffer {
    fn drop(&mut self) {
        unsafe { harfbuzz_sys::hb_buffer_destroy(self.ptr) };
    }
}

fn make_tag(a: u8, b: u8, c: u8, d: u8) -> harfbuzz_sys::hb_tag_t {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

// HarfBuzz feature range constants (not exported by harfbuzz-sys)
const HB_FEATURE_GLOBAL_START: u32 = 0;
const HB_FEATURE_GLOBAL_END: u32 = u32::MAX;

pub struct Shaper {
    buffer: HbBuffer,
    features: Vec<harfbuzz_sys::hb_feature_t>,
}

impl Shaper {
    pub fn new() -> Self {
        let features = Self::default_features();
        Self {
            buffer: HbBuffer::new().expect("Failed to create HarfBuzz buffer"),
            features,
        }
    }

    #[allow(dead_code)]
    pub fn with_features(feature_strings: &[&str]) -> Self {
        let mut features = Self::default_features();
        for feature_str in feature_strings {
            if let Some(feature) = Self::parse_feature(feature_str) {
                features.push(feature);
            }
        }

        Self {
            buffer: HbBuffer::new().expect("Failed to create HarfBuzz buffer"),
            features,
        }
    }

    #[allow(dead_code)]
    fn parse_feature(s: &str) -> Option<harfbuzz_sys::hb_feature_t> {
        let bytes = s.as_bytes();
        if bytes.len() < 4 {
            return None;
        }
        Some(harfbuzz_sys::hb_feature_t {
            tag: make_tag(bytes[0], bytes[1], bytes[2], bytes[3]),
            value: 1,
            start: HB_FEATURE_GLOBAL_START,
            end: HB_FEATURE_GLOBAL_END,
        })
    }

    fn default_features() -> Vec<harfbuzz_sys::hb_feature_t> {
        vec![
            // calt - contextual alternates
            harfbuzz_sys::hb_feature_t {
                tag: make_tag(b'c', b'a', b'l', b't'),
                value: 1,
                start: HB_FEATURE_GLOBAL_START,
                end: HB_FEATURE_GLOBAL_END,
            },
            // liga - standard ligatures
            harfbuzz_sys::hb_feature_t {
                tag: make_tag(b'l', b'i', b'g', b'a'),
                value: 1,
                start: HB_FEATURE_GLOBAL_START,
                end: HB_FEATURE_GLOBAL_END,
            },
        ]
    }

    #[allow(dead_code)]
    pub fn shape(&mut self, run: &TextRun, collection: &Collection) -> Vec<ShapedGlyph> {
        let face = collection.primary_face(run.style);
        self.shape_with_face(run, face, CollectionIndex::primary(run.style))
    }

    pub fn shape_with_collection(
        &mut self,
        run: &TextRun,
        collection: &mut Collection,
    ) -> Vec<ShapedGlyph> {
        let mut results = Vec::new();
        let mut remaining_text = run.text;
        let mut cluster_offset = 0u32;

        while !remaining_text.is_empty() {
            let (run_text, next_remaining, font_index) =
                self.find_font_run(remaining_text, run.style, collection);

            if let Some(face) = collection.get_face(font_index) {
                let sub_run = TextRun {
                    text: run_text,
                    style: run.style,
                };

                let shaped = self.shape_with_face(&sub_run, face, font_index);

                for mut glyph in shaped {
                    glyph.cluster += cluster_offset;
                    results.push(glyph);
                }
            }

            cluster_offset += run_text.chars().count() as u32;
            remaining_text = next_remaining;
        }

        results
    }

    fn find_font_run<'a>(
        &self,
        text: &'a str,
        style: Style,
        collection: &mut Collection,
    ) -> (&'a str, &'a str, CollectionIndex) {
        let mut chars = text.char_indices();

        if let Some((_, first_char)) = chars.next() {
            let first_codepoint = first_char as u32;
            let (first_index, _) = collection
                .resolve_glyph(first_codepoint, style)
                .unwrap_or((CollectionIndex::primary(style), 0));

            let mut end_byte = text.len();

            for (byte_idx, ch) in chars {
                let codepoint = ch as u32;
                if let Some((index, _)) = collection.resolve_glyph(codepoint, style) {
                    if index != first_index {
                        end_byte = byte_idx;
                        break;
                    }
                }
            }

            let (run, remaining) = text.split_at(end_byte);
            (run, remaining, first_index)
        } else {
            (text, "", CollectionIndex::primary(style))
        }
    }

    fn shape_with_face(
        &mut self,
        run: &TextRun,
        face: &Face,
        font_index: CollectionIndex,
    ) -> Vec<ShapedGlyph> {
        let hb_font = face.hb_font();

        // Reset and configure buffer
        self.buffer.reset();
        self.buffer.add_str(run.text);
        self.buffer.set_direction(harfbuzz_sys::HB_DIRECTION_LTR);
        self.buffer.guess_segment_properties();

        unsafe {
            harfbuzz_sys::hb_shape(
                hb_font.as_ptr(),
                self.buffer.ptr,
                self.features.as_ptr(),
                self.features.len() as u32,
            );
        }

        let infos = self.buffer.get_glyph_infos();
        let positions = self.buffer.get_glyph_positions();

        let mut results = Vec::with_capacity(infos.len());
        for (info, pos) in infos.iter().zip(positions.iter()) {
            results.push(ShapedGlyph {
                glyph_id: info.codepoint,
                cluster: info.cluster,
                x_advance: pos.x_advance,
                y_advance: pos.y_advance,
                x_offset: pos.x_offset,
                y_offset: pos.y_offset,
                font_index,
            });
        }

        results
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shaper_creation() {
        let shaper = Shaper::new();
        assert!(!shaper.features.is_empty());
    }

    #[test]
    fn test_shaper_with_features() {
        let shaper = Shaper::with_features(&["dlig", "ss01"]);
        assert!(shaper.features.len() >= 2);
    }

    #[test]
    fn test_text_run() {
        let run = TextRun {
            text: "Hello",
            style: Style::Regular,
        };
        assert_eq!(run.text, "Hello");
        assert_eq!(run.style, Style::Regular);
    }

    #[test]
    fn test_shaped_glyph() {
        let glyph = ShapedGlyph {
            glyph_id: 42,
            cluster: 0,
            x_advance: 512,
            y_advance: 0,
            x_offset: 0,
            y_offset: 0,
            font_index: CollectionIndex::primary(Style::Regular),
        };
        assert_eq!(glyph.glyph_id, 42);
        assert_eq!(glyph.x_advance, 512);
    }

    #[test]
    fn test_make_tag() {
        let tag = make_tag(b'c', b'a', b'l', b't');
        assert_eq!(tag, 0x63616c74); // "calt" in hex
    }

    #[test]
    fn test_hb_buffer() {
        let mut buffer = HbBuffer::new().expect("Failed to create buffer");
        buffer.add_str("Hello");
        buffer.set_direction(harfbuzz_sys::HB_DIRECTION_LTR);
        buffer.guess_segment_properties();
        // Buffer length should match codepoints before shaping
        assert!(buffer.get_length() > 0);
    }

    #[test]
    fn test_shape_simple_text() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let mut shaper = Shaper::new();
        let run = TextRun {
            text: "ABC",
            style: Style::Regular,
        };

        let shaped = shaper.shape(&run, &collection);

        assert_eq!(shaped.len(), 3, "Should produce 3 glyphs for 'ABC'");
        for glyph in &shaped {
            assert!(glyph.glyph_id > 0, "Glyph ID should be positive");
        }
    }

    #[test]
    fn test_shape_with_collection() {
        let mut collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let mut shaper = Shaper::new();
        let run = TextRun {
            text: "Hello World",
            style: Style::Regular,
        };

        let shaped = shaper.shape_with_collection(&run, &mut collection);

        assert!(shaped.len() >= 11, "Should produce at least 11 glyphs");
    }

    #[test]
    fn test_shape_ligature_potential() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let mut shaper = Shaper::new();

        let sequences = ["->", "=>", ">=", "<=", "!=", "=="];

        for seq in sequences {
            let run = TextRun {
                text: seq,
                style: Style::Regular,
            };
            let shaped = shaper.shape(&run, &collection);
            assert!(!shaped.is_empty(), "Should produce glyphs for '{}'", seq);
        }
    }

    #[test]
    fn test_shape_cluster_indices() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let mut shaper = Shaper::new();
        let run = TextRun {
            text: "ABC",
            style: Style::Regular,
        };

        let shaped = shaper.shape(&run, &collection);

        let mut prev_cluster = 0u32;
        for (i, glyph) in shaped.iter().enumerate() {
            if i > 0 {
                assert!(
                    glyph.cluster >= prev_cluster,
                    "Clusters should be non-decreasing"
                );
            }
            prev_cluster = glyph.cluster;
        }
    }

    #[test]
    fn test_shape_advances() {
        let collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
        let mut shaper = Shaper::new();
        let run = TextRun {
            text: "M",
            style: Style::Regular,
        };

        let shaped = shaper.shape(&run, &collection);

        assert_eq!(shaped.len(), 1);
        // The x_advance should be positive (in 26.6 fixed point)
        assert!(shaped[0].x_advance > 0, "x_advance should be positive");
    }
}

#[test]
fn test_shape_nerd_font_icons() {
    let mut collection = Collection::new("Menlo", 14.0, 72.0).unwrap();
    let mut shaper = Shaper::new();

    // Test a string with nerd font icons
    let nerd_icon = "\u{E62B}"; // Seti-UI icon
    let run = TextRun {
        text: nerd_icon,
        style: Style::Regular,
    };

    let shaped = shaper.shape_with_collection(&run, &mut collection);

    println!("Shaped {} glyphs for nerd icon", shaped.len());
    for glyph in &shaped {
        println!(
            "  glyph_id={}, font_index={:?}, advance={}",
            glyph.glyph_id, glyph.font_index, glyph.x_advance
        );
    }

    assert!(!shaped.is_empty(), "Should produce glyphs for nerd icon");
    // The glyph_id should NOT be 0 (.notdef)
    assert!(
        shaped[0].glyph_id != 0,
        "Glyph ID should not be 0 (.notdef), got {}",
        shaped[0].glyph_id
    );
}
