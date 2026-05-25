//! Quadrant chart plugin
//!
//! Implements Mermaid quadrantChart visualization with ASCII art.

mod database;
mod detector;
mod parser;
mod renderer;

pub use database::{QuadrantChartDatabase, QuadrantPoint};
pub use detector::QuadrantChartDetector;
pub use parser::QuadrantChartParser;
pub use renderer::{QuadrantChartConfig, QuadrantChartRenderer};

use crate::core::{Detector, Diagram};
use std::sync::Arc;

pub struct QuadrantChartDiagram;

impl Diagram for QuadrantChartDiagram {
    type Database = QuadrantChartDatabase;
    type Parser = QuadrantChartParser;
    type Renderer = QuadrantChartRenderer;

    fn detector() -> Arc<dyn Detector> {
        Arc::new(QuadrantChartDetector::new())
    }

    fn create_parser() -> Self::Parser {
        QuadrantChartParser::new()
    }

    fn create_database() -> Self::Database {
        QuadrantChartDatabase::new()
    }

    fn create_renderer() -> Self::Renderer {
        QuadrantChartRenderer::new()
    }

    fn name() -> &'static str {
        "quadrantchart"
    }

    fn version() -> &'static str {
        "0.1.0"
    }
}
