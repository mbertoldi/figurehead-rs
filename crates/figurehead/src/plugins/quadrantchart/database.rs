//! Quadrant chart database
//!
//! Stores parsed quadrantChart data: title, axes labels, quadrant labels,
//! and data points with their [x, y] coordinates.

use crate::core::Database;
use anyhow::Result;

/// Placeholder node type — quadrant charts don't use nodes.
#[derive(Debug, Clone)]
pub struct QcNode {
    pub id: String,
    pub label: String,
}

/// Placeholder edge type — quadrant charts don't use edges.
#[derive(Debug, Clone)]
pub struct QcEdge {
    pub from: String,
    pub to: String,
}

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

impl Database for QuadrantChartDatabase {
    type Node = QcNode;
    type Edge = QcEdge;

    fn add_node(&mut self, _node: Self::Node) -> Result<()> {
        Ok(()) // no-op
    }

    fn add_edge(&mut self, _edge: Self::Edge) -> Result<()> {
        Ok(()) // no-op
    }

    fn get_node(&self, _id: &str) -> Option<&Self::Node> {
        None
    }

    fn node_count(&self) -> usize {
        0
    }

    fn edge_count(&self) -> usize {
        0
    }

    fn nodes(&self) -> impl Iterator<Item = &Self::Node> {
        std::iter::empty()
    }

    fn edges(&self) -> impl Iterator<Item = &Self::Edge> {
        std::iter::empty()
    }

    fn clear(&mut self) {
        self.title = None;
        self.x_axis_low.clear();
        self.x_axis_high.clear();
        self.y_axis_low.clear();
        self.y_axis_high.clear();
        self.quadrant_labels = [String::new(), String::new(), String::new(), String::new()];
        self.points.clear();
    }
}
