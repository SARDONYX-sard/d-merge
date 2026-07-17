//! Carousel builder struct -- a content slider with navigation.

/// A carousel: horizontally scrollable content with prev/next.
#[must_use]
pub struct Carousel {
    pub(crate) item_count: usize,
}

impl Carousel {
    pub fn new(item_count: usize) -> Self {
        Self { item_count }
    }
}
