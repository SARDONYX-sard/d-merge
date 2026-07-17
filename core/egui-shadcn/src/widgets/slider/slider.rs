//! shadcn-styled Slider builder struct.

/// A slider widget styled after shadcn/ui.
#[must_use]
pub struct Slider<'a> {
    pub(crate) value: SliderValue<'a>,
    pub(crate) range: std::ops::RangeInclusive<f64>,
    pub(crate) step: Option<f64>,
    pub(crate) width: Option<f32>,
    pub(crate) suffix: Option<String>,
    pub(crate) prefix: Option<String>,
}

pub(crate) enum SliderValue<'a> {
    F64(&'a mut f64),
    F32(&'a mut f32),
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f64, range: std::ops::RangeInclusive<f64>) -> Self {
        Self {
            value: SliderValue::F64(value),
            range,
            step: None,
            width: None,
            suffix: None,
            prefix: None,
        }
    }

    pub fn f32(value: &'a mut f32, range: std::ops::RangeInclusive<f32>) -> Self {
        Self {
            value: SliderValue::F32(value),
            range: (*range.start() as f64)..=(*range.end() as f64),
            step: None,
            width: None,
            suffix: None,
            prefix: None,
        }
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
