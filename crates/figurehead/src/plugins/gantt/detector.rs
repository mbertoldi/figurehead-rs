//! Gantt chart detector

use crate::core::Detector;

pub struct GanttDetector;

impl Default for GanttDetector {
    fn default() -> Self { Self::new() }
}

impl GanttDetector {
    pub fn new() -> Self { Self }
}

impl Detector for GanttDetector {
    fn detect(&self, input: &str) -> bool {
        let input = input.trim();
        if input.is_empty() { return false; }
        let lower = input.to_lowercase();
        lower.starts_with("gantt") || lower.starts_with("ganttchart")
    }

    fn confidence(&self, input: &str) -> f64 {
        let input = input.trim();
        if input.is_empty() { return 0.0; }
        let lower = input.to_lowercase();
        if lower.starts_with("gantt") || lower.starts_with("ganttchart") {
            if lower.contains("section") && lower.contains(":") { return 0.95; }
            return 0.8;
        }
        0.0
    }

    fn diagram_type(&self) -> &'static str { "gantt" }
    fn patterns(&self) -> Vec<&'static str> {
        vec!["gantt", "ganttChart", "section", "dateFormat"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_gantt() {
        let d = GanttDetector::new();
        assert!(d.detect("gantt"));
        assert!(d.detect("ganttChart"));
        assert!(d.detect("gantt\n    title Test"));
    }

    #[test]
    fn test_rejects_other() {
        let d = GanttDetector::new();
        assert!(!d.detect("graph TD"));
        assert!(!d.detect("sequenceDiagram"));
    }
}
