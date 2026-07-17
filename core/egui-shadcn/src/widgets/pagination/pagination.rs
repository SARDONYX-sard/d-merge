//! Pagination builder struct — page navigation controls.

/// A pagination bar: prev/next buttons with page numbers.
#[must_use]
pub struct Pagination {
    pub(crate) total_pages: usize,
    pub(crate) max_visible: usize,
}

impl Pagination {
    pub fn new(total_pages: usize) -> Self {
        Self {
            total_pages,
            max_visible: 5,
        }
    }

    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }
}
