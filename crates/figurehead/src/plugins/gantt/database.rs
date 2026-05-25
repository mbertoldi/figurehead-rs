//! Gantt chart database

use crate::core::Database;
use anyhow::Result;

/// Placeholder node/edge types.
#[derive(Debug, Clone)]
pub struct GtNode { pub id: String, pub label: String }
#[derive(Debug, Clone)]
pub struct GtEdge { pub from: String, pub to: String }

/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Active,
    Done,
    Crit,
    Normal,
    Milestone,
}

/// A single Gantt task.
#[derive(Debug, Clone)]
pub struct GanttTask {
    pub label: String,
    pub id: String,
    pub status: TaskStatus,
    /// Start day offset from the overall minimum date.
    pub start_day: i64,
    /// Duration in days.
    pub duration_days: i64,
}

/// A section containing tasks.
#[derive(Debug, Clone)]
pub struct GanttSection {
    pub name: String,
    pub tasks: Vec<GanttTask>,
}

/// Gantt chart database.
#[derive(Debug, Clone, Default)]
pub struct GanttDatabase {
    pub title: Option<String>,
    pub date_format: String,
    pub sections: Vec<GanttSection>,
    /// Overall time range: min/max day offsets.
    pub min_day: i64,
    pub max_day: i64,
}

impl GanttDatabase {
    pub fn new() -> Self {
        Self {
            title: None,
            date_format: String::new(),
            sections: Vec::new(),
            min_day: 0,
            max_day: 0,
        }
    }

    /// Recompute min/max day from all tasks.
    pub fn recompute_range(&mut self) {
        self.min_day = i64::MAX;
        self.max_day = i64::MIN;
        for sec in &self.sections {
            for t in &sec.tasks {
                self.min_day = self.min_day.min(t.start_day);
                self.max_day = self.max_day.max(t.start_day + t.duration_days);
            }
        }
        if self.min_day == i64::MAX {
            self.min_day = 0;
            self.max_day = 30;
        }
    }
}

impl Database for GanttDatabase {
    type Node = GtNode;
    type Edge = GtEdge;
    fn add_node(&mut self, _n: Self::Node) -> Result<()> { Ok(()) }
    fn add_edge(&mut self, _e: Self::Edge) -> Result<()> { Ok(()) }
    fn get_node(&self, _id: &str) -> Option<&Self::Node> { None }
    fn node_count(&self) -> usize { 0 }
    fn edge_count(&self) -> usize { 0 }
    fn nodes(&self) -> impl Iterator<Item = &Self::Node> { std::iter::empty() }
    fn edges(&self) -> impl Iterator<Item = &Self::Edge> { std::iter::empty() }
    fn clear(&mut self) {
        self.title = None;
        self.sections.clear();
        self.min_day = 0;
        self.max_day = 0;
    }
}
