//! shadcn-styled NumberInput builder struct.

/// A numeric input wrapping egui's `DragValue` with shadcn theming.
///
/// ```no_run
/// # egui::__run_test_ui(|ui| {
/// let mut value = 42.0_f64;
/// egui_shadcn::NumberInput::new(&mut value)
///     .range(0.0..=100.0)
///     .speed(0.5)
///     .suffix("px")
///     .show(ui);
/// # });
/// ```
#[must_use]
pub struct NumberInput<'a> {
    pub(crate) value: ValueRef<'a>,
    pub(crate) range: Option<std::ops::RangeInclusive<f64>>,
    pub(crate) speed: f64,
    pub(crate) suffix: Option<String>,
    pub(crate) prefix: Option<String>,
    pub(crate) decimals: Option<usize>,
    pub(crate) width: Option<f32>,
}

pub(crate) enum ValueRef<'a> {
    F64(&'a mut f64),
    F32(&'a mut f32),
    I32(&'a mut i32),
}

impl<'a> NumberInput<'a> {
    pub fn new(value: &'a mut f64) -> Self {
        Self {
            value: ValueRef::F64(value),
            range: None,
            speed: 1.0,
            suffix: None,
            prefix: None,
            decimals: None,
            width: None,
        }
    }

    pub fn f32(value: &'a mut f32) -> Self {
        Self {
            value: ValueRef::F32(value),
            range: None,
            speed: 1.0,
            suffix: None,
            prefix: None,
            decimals: None,
            width: None,
        }
    }

    pub fn i32(value: &'a mut i32) -> Self {
        Self {
            value: ValueRef::I32(value),
            range: None,
            speed: 1.0,
            suffix: None,
            prefix: None,
            decimals: Some(0),
            width: None,
        }
    }

    pub fn range(mut self, range: std::ops::RangeInclusive<f64>) -> Self {
        self.range = Some(range);
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
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

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
