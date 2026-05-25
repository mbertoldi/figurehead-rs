//! Quadrant chart database
//!
//! Stores parsed quadrantChart data: title, axes labels, quadrant labels,
//! and data points with their [x, y] coordinates.

/// A single data point with a label and normalized [x, y] coordinates (0..=1).
#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f64,
    pub y: f64,
}

/// Quadrant chart database storing all parsed elements.
#[derive(Debug, Clone, Default)]
pub struct QuadrantChartDatabase {
    pub title: Option<String>,
    pub x_axis_low: String,
    pub x_axis_high: String,
    pub y_axis_low: String,
    pub y_axis_high: String,
    pub quadrant_labels: [String; 4],
    pub points: Vec<QuadrantPoint>,
}

impl QuadrantChartDatabase {
    pub fn new() -> Self {
        Self {
            title: None,
            x_axis_low: String::new(),
            x_axis_high: String::new(),
            y_axis_low: String::new(),
            y_axis_high: String::new(),
            quadrant_labels: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            points: Vec::new(),
        }
    }
}
