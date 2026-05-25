//! Gantt chart plugin

mod database;
mod detector;
mod parser;
mod renderer;

pub use database::GanttDatabase;
pub use detector::GanttDetector;
pub use parser::GanttParser;
pub use renderer::GanttRenderer;

use crate::core::{Detector, Diagram};
use std::sync::Arc;

pub struct GanttDiagram;

impl Diagram for GanttDiagram {
    type Database = GanttDatabase;
    type Parser = GanttParser;
    type Renderer = GanttRenderer;

    fn detector() -> Arc<dyn Detector> { Arc::new(GanttDetector::new()) }
    fn create_parser() -> Self::Parser { GanttParser::new() }
    fn create_database() -> Self::Database { GanttDatabase::new() }
    fn create_renderer() -> Self::Renderer { GanttRenderer::new() }
    fn name() -> &'static str { "gantt" }
    fn version() -> &'static str { "0.1.0" }
}
