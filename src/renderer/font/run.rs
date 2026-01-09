use crate::editor::{Cell, HighlightMap, StyleFlags};

use super::collection::Style;

/// A text run is a sequence of consecutive cells with the same styling.
///
/// Runs are the unit of text shaping - HarfBuzz shapes entire runs at once
/// to correctly handle ligatures and complex scripts.
#[derive(Debug, Clone)]
pub struct Run {
    /// Starting column index in the row.
    pub start_col: usize,
    /// The accumulated text content of the run.
    pub text: String,
    /// Font style derived from highlight attributes.
    pub style: Style,
    /// Highlight ID for color resolution.
    pub highlight_id: u64,
}

impl Run {
    /// Returns true if this run has no text content.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Iterator that groups consecutive cells into text runs.
///
/// A new run starts when:
/// - The highlight ID changes (different colors or styles)
/// - A wide spacer is encountered (skip it)
///
/// Wide spacers are skipped entirely as they are placeholders for
/// the right half of wide characters.
pub struct RunIterator<'a> {
    cells: &'a [Cell],
    highlights: &'a HighlightMap,
    current_pos: usize,
}

impl<'a> RunIterator<'a> {
    /// Creates a new run iterator for a row of cells.
    pub fn new(cells: &'a [Cell], highlights: &'a HighlightMap) -> Self {
        Self {
            cells,
            highlights,
            current_pos: 0,
        }
    }

    /// Determines the font style from highlight attributes.
    fn style_for_highlight(&self, highlight_id: u64) -> Style {
        let attrs = self.highlights.get(highlight_id);
        Style::from_flags(
            attrs.style.contains(StyleFlags::BOLD),
            attrs.style.contains(StyleFlags::ITALIC),
        )
    }
}

impl<'a> Iterator for RunIterator<'a> {
    type Item = Run;

    fn next(&mut self) -> Option<Run> {
        // Skip past any wide spacers at the start
        while self.current_pos < self.cells.len() {
            if !self.cells[self.current_pos].is_wide_spacer() {
                break;
            }
            self.current_pos += 1;
        }

        if self.current_pos >= self.cells.len() {
            return None;
        }

        let start_col = self.current_pos;
        let first_cell = &self.cells[self.current_pos];
        let highlight_id = first_cell.highlight_id;
        let style = self.style_for_highlight(highlight_id);

        let mut text = String::new();
        text.push_str(&first_cell.text);
        self.current_pos += 1;

        // Continue accumulating while highlight_id matches
        while self.current_pos < self.cells.len() {
            let cell = &self.cells[self.current_pos];

            // Skip wide spacers within the run
            if cell.is_wide_spacer() {
                self.current_pos += 1;
                continue;
            }

            // Break if highlight changes
            if cell.highlight_id != highlight_id {
                break;
            }

            text.push_str(&cell.text);
            self.current_pos += 1;
        }

        Some(Run {
            start_col,
            text,
            style,
            highlight_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{CellFlags, HighlightAttributes};

    fn make_cell(text: &str, highlight_id: u64) -> Cell {
        Cell {
            text: text.into(),
            highlight_id,
            flags: CellFlags::empty(),
        }
    }

    fn make_wide_spacer(highlight_id: u64) -> Cell {
        Cell {
            text: Default::default(),
            highlight_id,
            flags: CellFlags::WIDE_CHAR_SPACER,
        }
    }

    #[test]
    fn test_single_run() {
        let cells = vec![
            make_cell("H", 0),
            make_cell("e", 0),
            make_cell("l", 0),
            make_cell("l", 0),
            make_cell("o", 0),
        ];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "Hello");
        assert_eq!(runs[0].start_col, 0);
        assert_eq!(runs[0].style, Style::Regular);
    }

    #[test]
    fn test_multiple_runs_by_highlight() {
        let cells = vec![
            make_cell("a", 0),
            make_cell("b", 0),
            make_cell("c", 1),
            make_cell("d", 1),
            make_cell("e", 2),
        ];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[0].start_col, 0);
        assert_eq!(runs[0].highlight_id, 0);

        assert_eq!(runs[1].text, "cd");
        assert_eq!(runs[1].start_col, 2);
        assert_eq!(runs[1].highlight_id, 1);

        assert_eq!(runs[2].text, "e");
        assert_eq!(runs[2].start_col, 4);
        assert_eq!(runs[2].highlight_id, 2);
    }

    #[test]
    fn test_wide_char_spacers_skipped() {
        let cells = vec![
            make_cell("あ", 0),
            make_wide_spacer(0),
            make_cell("い", 0),
            make_wide_spacer(0),
        ];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "あい");
        assert_eq!(runs[0].start_col, 0);
    }

    #[test]
    fn test_wide_spacer_at_start() {
        let cells = vec![make_wide_spacer(0), make_cell("a", 0), make_cell("b", 0)];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[0].start_col, 1);
    }

    #[test]
    fn test_empty_cells() {
        let cells: Vec<Cell> = vec![];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert!(runs.is_empty());
    }

    #[test]
    fn test_only_wide_spacers() {
        let cells = vec![make_wide_spacer(0), make_wide_spacer(0)];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert!(runs.is_empty());
    }

    #[test]
    fn test_bold_style_detection() {
        let cells = vec![make_cell("a", 0), make_cell("b", 1), make_cell("c", 1)];

        let mut highlights = HighlightMap::new();
        highlights.define(
            1,
            HighlightAttributes {
                style: StyleFlags::BOLD,
                ..Default::default()
            },
        );

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].style, Style::Regular);
        assert_eq!(runs[1].style, Style::Bold);
    }

    #[test]
    fn test_italic_style_detection() {
        let cells = vec![make_cell("x", 1)];

        let mut highlights = HighlightMap::new();
        highlights.define(
            1,
            HighlightAttributes {
                style: StyleFlags::ITALIC,
                ..Default::default()
            },
        );

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].style, Style::Italic);
    }

    #[test]
    fn test_bold_italic_style_detection() {
        let cells = vec![make_cell("y", 1)];

        let mut highlights = HighlightMap::new();
        highlights.define(
            1,
            HighlightAttributes {
                style: StyleFlags::BOLD | StyleFlags::ITALIC,
                ..Default::default()
            },
        );

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].style, Style::BoldItalic);
    }

    #[test]
    fn test_run_is_empty() {
        let run = Run {
            start_col: 0,
            text: String::new(),
            style: Style::Regular,
            highlight_id: 0,
        };
        assert!(run.is_empty());

        let run = Run {
            start_col: 0,
            text: "a".to_string(),
            style: Style::Regular,
            highlight_id: 0,
        };
        assert!(!run.is_empty());
    }

    #[test]
    fn test_ligature_potential_run() {
        // Test that ligature sequences stay together
        let cells = vec![
            make_cell("-", 0),
            make_cell(">", 0),
            make_cell(" ", 0),
            make_cell("=", 0),
            make_cell("=", 0),
        ];
        let highlights = HighlightMap::new();

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "-> ==");
    }

    #[test]
    fn test_mixed_content() {
        // Realistic example: keyword (bold) + space + identifier (regular) + operator
        let cells = vec![
            make_cell("l", 1),
            make_cell("e", 1),
            make_cell("t", 1),
            make_cell(" ", 0),
            make_cell("x", 0),
            make_cell(" ", 0),
            make_cell("=", 2),
            make_cell(" ", 0),
            make_cell("1", 3),
        ];

        let mut highlights = HighlightMap::new();
        highlights.define(
            1,
            HighlightAttributes {
                style: StyleFlags::BOLD,
                ..Default::default()
            },
        );
        // 2 and 3 have different colors but default style

        let runs: Vec<_> = RunIterator::new(&cells, &highlights).collect();

        assert_eq!(runs.len(), 5);
        assert_eq!(runs[0].text, "let");
        assert_eq!(runs[0].style, Style::Bold);
        assert_eq!(runs[1].text, " x ");
        assert_eq!(runs[2].text, "=");
        assert_eq!(runs[3].text, " ");
        assert_eq!(runs[4].text, "1");
    }
}
