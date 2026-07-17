//! AreaChart builder struct — a stacked area chart with smooth curves.

/// A single data series for the area chart.
pub struct AreaSeries {
    pub values: Vec<f32>,
    pub color: egui::Color32,
}

/// A stacked area chart: smooth Catmull-Rom curves with filled regions.
#[must_use]
pub struct AreaChart {
    pub(crate) labels: Vec<String>,
    pub(crate) series: Vec<AreaSeries>,
    pub(crate) height: f32,
    pub(crate) stacked: bool,
}

impl AreaChart {
    pub fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            series: Vec::new(),
            height: 250.0,
            stacked: false,
        }
    }

    pub fn series(mut self, s: AreaSeries) -> Self {
        self.series.push(s);
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }
}
