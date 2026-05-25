//! Quadrant chart detector implementation
//!
//! Detects Mermaid quadrantChart syntax patterns.

use crate::core::Detector;

/// Quadrant chart detector implementation
pub struct QuadrantChartDetector;

impl Default for QuadrantChartDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl QuadrantChartDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for QuadrantChartDetector {
    fn detect(&self, input: &str) -> bool {
        let input = input.trim();
        if input.is_empty() {
            return false;
        }
        let input_lower = input.to_lowercase();
        input_lower.contains("quadrantchart") || input_lower.contains("quadrant chart")
    }

    fn confidence(&self, input: &str) -> f64 {
        let input = input.trim();
        if input.is_empty() {
            return 0.0;
        }
        let input_lower = input.to_lowercase();
        if input_lower.starts_with("quadrantchart")
            || input_lower.starts_with("quadrant chart")
        {
            return 0.95;
        }
        if input_lower.contains("quadrantchart") || input_lower.contains("quadrant chart") {
            return 0.8;
        }
        let has_axes = input_lower.contains("x-axis") && input_lower.contains("y-axis");
        let has_quadrants = input_lower.contains("quadrant-");
        if has_axes && has_quadrants {
            return 0.6;
        }
        0.0
    }

    fn diagram_type(&self) -> &'static str {
        "quadrantchart"
    }

    fn patterns(&self) -> Vec<&'static str> {
        vec![
            "quadrantChart",
            "quadrant chart",
            "x-axis",
            "y-axis",
            "quadrant-1",
            "quadrant-2",
            "quadrant-3",
            "quadrant-4",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_quadrant_chart_keyword() {
        let detector = QuadrantChartDetector::new();
        assert!(detector.detect("quadrantChart"));
        assert!(detector.detect("quadrant chart"));
    }

    #[test]
    fn test_confidence_scoring() {
        let detector = QuadrantChartDetector::new();
        assert!(detector.confidence("quadrantChart\n    title Test") > 0.9);
    }

    #[test]
    fn test_rejects_other_diagrams() {
        let detector = QuadrantChartDetector::new();
        assert!(!detector.detect("graph TD\n    A --> B"));
        assert!(!detector.detect("sequenceDiagram"));
        assert!(!detector.detect("gitGraph"));
    }
}
