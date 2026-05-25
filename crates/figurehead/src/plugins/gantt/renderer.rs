//! Gantt chart ASCII renderer

use anyhow::Result;
use tracing::debug;

use crate::core::Renderer;
use super::database::{GanttDatabase, TaskStatus};

const BAR_WIDTH: usize = 50;

pub struct GanttRenderer;

impl Default for GanttRenderer {
    fn default() -> Self { Self::new() }
}

impl GanttRenderer {
    pub fn new() -> Self { Self }

    pub fn render(&self, db: &GanttDatabase) -> Result<String> {
        debug!("Rendering Gantt chart");

        // ── Compute dimensions ──────────────────────────────────
        let label_w = db.sections.iter()
            .flat_map(|s| s.tasks.iter().map(|t| t.label.len()))
            .max().unwrap_or(8)
            .max(db.sections.iter().map(|s| s.name.len()).max().unwrap_or(6))
            + 2;

        let total_days = (db.max_day - db.min_day).max(1) as f64;
        let chart_w = BAR_WIDTH.max(20);
        let total_w = label_w + chart_w + 4; // borders + padding

        let mut rows = Vec::new();

        // ── Top border ─────────────────────────────────────────
        rows.push(format!("┌{}┐", "─".repeat(total_w.saturating_sub(2))));

        // ── Title ──────────────────────────────────────────────
        if let Some(title) = &db.title {
            let t = format!(" {} ", title);
            let t = clip(&t, total_w.saturating_sub(2));
            let pad = total_w.saturating_sub(2).saturating_sub(t.len());
            rows.push(format!("│{}{}{}│", " ".repeat(pad/2), t, " ".repeat(pad-pad/2)));
        }

        // ── Sections and tasks ─────────────────────────────────
        for section in &db.sections {
            if !section.name.is_empty() {
                let hdr = format!(" {}{}", section.name, "─".repeat(total_w.saturating_sub(section.name.len()+3)));
                rows.push(format!("│{}│", clip(&hdr, total_w.saturating_sub(2))));
                rows.push(format!("│{}│", " ".repeat(total_w.saturating_sub(2))));
            }

            for task in &section.tasks {
                let bar_fill = match task.status {
                    TaskStatus::Done => '▓',
                    TaskStatus::Active => '█',
                    TaskStatus::Crit => '▒',
                    TaskStatus::Normal => '░',
                    TaskStatus::Milestone => '◆',
                };

                let label = clip(&task.label, label_w.saturating_sub(1));
                let label_padded = format!("{:>width$}", label, width = label_w.saturating_sub(1));

                // Bar position and width
                let bar_start = ((task.start_day - db.min_day) as f64 / total_days * chart_w as f64).round() as usize;
                let bar_len = ((task.duration_days as f64 / total_days * chart_w as f64).round() as usize).max(1);

                let mut bar_line = String::with_capacity(chart_w);
                for i in 0..chart_w {
                    if i >= bar_start && i < bar_start + bar_len {
                        bar_line.push(bar_fill);
                    } else {
                        bar_line.push(' ');
                    }
                }

                rows.push(format!("│{} {}│", label_padded, bar_line));
            }

            // Gap after section
            rows.push(format!("│{}│", " ".repeat(total_w.saturating_sub(2))));
        }

        // ── Timeline axis ──────────────────────────────────────
        let num_ticks = 6usize;
        let mut axis_line = String::with_capacity(chart_w + 1);
        axis_line.push('├');
        for i in 0..chart_w {
            if i > 0 && i % (chart_w.max(1) / num_ticks) == 0 {
                axis_line.push('┼');
            } else {
                axis_line.push('─');
            }
        }

        let axis_line_full = format!("│{} {}│",
            " ".repeat(label_w.saturating_sub(1)), axis_line);
        rows.push(axis_line_full);

        // ── Bottom border ──────────────────────────────────────
        rows.push(format!("└{}┘", "─".repeat(total_w.saturating_sub(2))));

        Ok(rows.join("\n"))
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { s.chars().take(max).collect() }
}

impl Renderer<GanttDatabase> for GanttRenderer {
    type Output = String;
    fn render(&self, db: &GanttDatabase) -> Result<Self::Output> {
        GanttRenderer::render(self, db)
    }
    fn name(&self) -> &'static str { "gantt-ascii" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn format(&self) -> &'static str { "ascii" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::gantt::parser::GanttParser;

    #[test]
    fn test_render_basic() {
        let input = "gantt\n    title Test\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task A :a1, 2024-01-01, 30d\n    Task B :after a1, 20d\n";
        let p = GanttParser::new();
        let mut db = GanttDatabase::new();
        p.parse(input, &mut db).unwrap();
        let r = GanttRenderer::new();
        let o = r.render(&db).unwrap();
        assert!(o.contains("Test"));
        assert!(o.contains("Task A"));
        assert!(o.contains("Task B"));
    }
}
