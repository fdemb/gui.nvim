/// Tracks which regions of the grid have been modified since the last render.
///
/// This enables partial updates where only changed rows need to be re-rendered,
/// improving performance for typical editing workloads.
#[derive(Debug, Clone)]
pub struct DirtyTracker {
    /// Bitmap of dirty rows. Each bit represents one row.
    dirty_rows: Vec<u64>,
    /// Number of rows being tracked.
    row_count: usize,
    /// Flag indicating the entire grid is dirty.
    full_dirty: bool,
}

impl DirtyTracker {
    /// Creates a new dirty tracker for the given number of rows.
    /// Initially marks all rows as dirty.
    pub fn new(row_count: usize) -> Self {
        let num_chunks = (row_count + 63) / 64;
        Self {
            dirty_rows: vec![u64::MAX; num_chunks],
            row_count,
            full_dirty: true,
        }
    }

    /// Returns the number of rows being tracked.
    #[allow(dead_code)]
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Marks a single row as dirty.
    pub fn mark_row(&mut self, row: usize) {
        if row < self.row_count {
            let chunk = row / 64;
            let bit = row % 64;
            self.dirty_rows[chunk] |= 1 << bit;
        }
    }

    /// Marks a range of rows as dirty.
    pub fn mark_rows(&mut self, start: usize, end: usize) {
        let start = start.min(self.row_count);
        let end = end.min(self.row_count);
        for row in start..end {
            self.mark_row(row);
        }
    }

    /// Marks the entire grid as dirty.
    pub fn mark_all(&mut self) {
        for chunk in &mut self.dirty_rows {
            *chunk = u64::MAX;
        }
        self.full_dirty = true;
    }

    /// Clears all dirty flags.
    pub fn clear(&mut self) {
        for chunk in &mut self.dirty_rows {
            *chunk = 0;
        }
        self.full_dirty = false;
    }

    /// Returns true if the given row is dirty.
    #[allow(dead_code)]
    pub fn is_row_dirty(&self, row: usize) -> bool {
        if row >= self.row_count {
            return false;
        }
        let chunk = row / 64;
        let bit = row % 64;
        (self.dirty_rows[chunk] & (1 << bit)) != 0
    }

    /// Returns true if any row is dirty.
    #[allow(dead_code)]
    pub fn has_dirty(&self) -> bool {
        self.dirty_rows.iter().any(|&chunk| chunk != 0)
    }

    /// Returns true if the entire grid was marked as dirty.
    #[allow(dead_code)]
    pub fn is_full_dirty(&self) -> bool {
        self.full_dirty
    }

    /// Returns an iterator over dirty row indices.
    #[allow(dead_code)]
    pub fn dirty_rows(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.row_count).filter(|&row| self.is_row_dirty(row))
    }

    /// Resizes the tracker for a new row count.
    /// Marks all rows as dirty after resize.
    pub fn resize(&mut self, new_row_count: usize) {
        let num_chunks = (new_row_count + 63) / 64;
        self.dirty_rows.resize(num_chunks, u64::MAX);
        self.row_count = new_row_count;
        self.mark_all();
    }
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let tracker = DirtyTracker::new(100);
        assert_eq!(tracker.row_count(), 100);
        assert!(tracker.is_full_dirty());
        assert!(tracker.has_dirty());
    }

    #[test]
    fn test_mark_row() {
        let mut tracker = DirtyTracker::new(10);
        tracker.clear();
        assert!(!tracker.has_dirty());

        tracker.mark_row(5);
        assert!(tracker.is_row_dirty(5));
        assert!(!tracker.is_row_dirty(4));
        assert!(tracker.has_dirty());
    }

    #[test]
    fn test_mark_rows() {
        let mut tracker = DirtyTracker::new(100);
        tracker.clear();

        tracker.mark_rows(10, 20);
        for row in 0..10 {
            assert!(!tracker.is_row_dirty(row));
        }
        for row in 10..20 {
            assert!(tracker.is_row_dirty(row));
        }
        for row in 20..100 {
            assert!(!tracker.is_row_dirty(row));
        }
    }

    #[test]
    fn test_mark_all() {
        let mut tracker = DirtyTracker::new(100);
        tracker.clear();
        assert!(!tracker.is_full_dirty());

        tracker.mark_all();
        assert!(tracker.is_full_dirty());
        for row in 0..100 {
            assert!(tracker.is_row_dirty(row));
        }
    }

    #[test]
    fn test_clear() {
        let mut tracker = DirtyTracker::new(100);
        assert!(tracker.has_dirty());

        tracker.clear();
        assert!(!tracker.has_dirty());
        assert!(!tracker.is_full_dirty());
    }

    #[test]
    fn test_dirty_rows_iterator() {
        let mut tracker = DirtyTracker::new(10);
        tracker.clear();

        tracker.mark_row(2);
        tracker.mark_row(5);
        tracker.mark_row(8);

        let dirty: Vec<usize> = tracker.dirty_rows().collect();
        assert_eq!(dirty, vec![2, 5, 8]);
    }

    #[test]
    fn test_resize() {
        let mut tracker = DirtyTracker::new(10);
        tracker.clear();
        assert!(!tracker.has_dirty());

        tracker.resize(200);
        assert_eq!(tracker.row_count(), 200);
        assert!(tracker.is_full_dirty());
        assert!(tracker.has_dirty());
    }

    #[test]
    fn test_large_grid() {
        let mut tracker = DirtyTracker::new(1000);
        tracker.clear();

        tracker.mark_row(500);
        tracker.mark_row(999);

        assert!(tracker.is_row_dirty(500));
        assert!(tracker.is_row_dirty(999));
        assert!(!tracker.is_row_dirty(501));
    }

    #[test]
    fn test_out_of_bounds() {
        let mut tracker = DirtyTracker::new(10);
        tracker.clear();

        tracker.mark_row(100);
        assert!(!tracker.is_row_dirty(100));
        assert!(!tracker.has_dirty());
    }
}
