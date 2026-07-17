//! Table builder struct — a styled data table with headers and rows.

/// A data table: `border rounded-lg text-sm`.
#[must_use]
pub struct Table {
    pub(crate) headers: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) striped: bool,
    pub(crate) col_weights: Option<Vec<f32>>,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            striped: false,
            col_weights: None,
        }
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn striped(mut self) -> Self {
        self.striped = true;
        self
    }

    pub fn col_weights(mut self, weights: Vec<f32>) -> Self {
        self.col_weights = Some(weights);
        self
    }
}
