//! Selected date state for DatePicker.

/// Holds the selected date for a DatePicker.
#[derive(Debug, Clone, Default)]
pub struct DatePickerState {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl DatePickerState {
    pub fn is_set(&self) -> bool {
        self.day > 0
    }

    pub fn format(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
