//! Gantt chart ASCII renderer

use anyhow::Result;
use tracing::debug;

use crate::core::Renderer;
use super::database::{GanttDatabase, TaskStatus};

const BAR_WIDTH: usize = 60;
const REF_YEAR: i32 = 2020;

pub struct GanttRenderer;

impl Default for GanttRenderer {
    fn default() -> Self { Self::new() }
}

impl GanttRenderer {
    pub fn new() -> Self { Self }

    pub fn render(&self, db: &GanttDatabase) -> Result<String> {
        debug!("Rendering Gantt chart");

        // ── Dimensions ──────────────────────────────────────────
        let label_w = db.sections.iter()
            .flat_map(|s| s.tasks.iter().map(|t| t.label.len()))
            .max().unwrap_or(8)
            .max(db.sections.iter().map(|s| s.name.len()).max().unwrap_or(6))
            + 2;
        let label_w = label_w.min(28); // cap label width

        let total_days = (db.max_day - db.min_day).max(1) as f64;
        let chart_w = BAR_WIDTH;
        let total_w = label_w + chart_w + 3; // borders + separator

        let mut rows = Vec::new();

        // ── Top border ─────────────────────────────────────────
        let tw = total_w.saturating_sub(2);
        rows.push(format!("┌{}┐", "─".repeat(tw)));

        // ── Title ──────────────────────────────────────────────
        if let Some(title) = &db.title {
            let t = format!(" {} ", title);
            let t = clip(&t, tw);
            let pad = tw.saturating_sub(t.len());
            rows.push(format!("│{}{}{}│", " ".repeat(pad/2), t, " ".repeat(pad-pad/2)));
            rows.push(format!("│{}│", " ".repeat(tw)));
        }

        // ── Sections and tasks ─────────────────────────────────
        for section in &db.sections {
            // Section header
            if !section.name.is_empty() {
                let hdr = format!(" ▸ {}", section.name);
                rows.push(format!("│{}│", pad_right(&hdr, tw)));
            }

            // Tasks
            for task in &section.tasks {
                let bar_fill = match task.status {
                    TaskStatus::Done => '▓',
                    TaskStatus::Active => '█',
                    TaskStatus::Crit => '▒',
                    TaskStatus::Normal => '░',
                    TaskStatus::Milestone => '◆',
                };

                // Label (right-aligned in label column)
                let label = clip(&task.label, label_w);
                let label = format!("{:>width$}", label, width = label_w);

                // Bar
                let bar_start = ((task.start_day - db.min_day) as f64 / total_days * chart_w as f64).round() as usize;
                let bar_len = ((task.duration_days as f64 / total_days * chart_w as f64).round() as usize).max(1);
                let bar_start = bar_start.min(chart_w.saturating_sub(1));
                let bar_len = bar_len.min(chart_w.saturating_sub(bar_start));

                let mut bar_line = String::with_capacity(chart_w);
                for i in 0..chart_w {
                    if i >= bar_start && i < bar_start + bar_len {
                        bar_line.push(bar_fill);
                    } else {
                        bar_line.push(' ');
                    }
                }

                // Date range annotation
                let start_date = day_to_str(task.start_day);
                let end_date = day_to_str(task.start_day + task.duration_days);
                let date_info = format!("{start_date}—{end_date}");
                // Append date info after bar, within remaining width
                let bar_with_date = format!("{bar_line} {date_info}");
                let cell = clip(&bar_with_date, tw.saturating_sub(label_w + 1));

                rows.push(format!("│{} {}│", label, pad_right(&cell, tw.saturating_sub(label_w + 1))));
            }

            // Gap after section
            rows.push(format!("│{}│", " ".repeat(tw)));
        }

        // ── Timeline axis ──────────────────────────────────────
        let num_ticks = 6;
        let tick_interval = chart_w.max(1) / num_ticks;

        let mut axis_line = String::with_capacity(chart_w + 1);
        axis_line.push('├');
        for i in 0..chart_w {
            if i > 0 && i % tick_interval == 0 {
                axis_line.push('┼');
            } else {
                axis_line.push('─');
            }
        }

        let indent = " ".repeat(label_w);
        rows.push(format!("│{} {}│", indent, axis_line));

        // ── Date labels below axis ─────────────────────────────
        // 4 evenly-spaced date labels
        let num_dates = 4usize;
        let mut date_line = String::new();
        for ti in 0..num_dates {
            let frac = (ti as f64 + 0.5) / num_dates as f64;
            let day = db.min_day + (frac * total_days) as i64;
            let ds = day_to_str(day);
            let col = ti * chart_w / num_dates + (chart_w / num_dates).saturating_sub(ds.len()) / 2;
            // Ensure column is within bounds
            let col = col.min(chart_w.saturating_sub(ds.len()));
            while date_line.len() < col { date_line.push(' '); }
            if date_line.len() == col {
                date_line.push_str(&ds);
            }
        }
        while date_line.len() < chart_w { date_line.push(' '); }

        rows.push(format!("│{} {}│", indent, date_line));

        // ── Bottom border ──────────────────────────────────────
        rows.push(format!("└{}┘", "─".repeat(tw)));

        Ok(rows.join("\n"))
    }
}

/// Convert day offset (from REF_DATE 2020-01-01) to YYYY-MM-DD string.
fn day_to_str(day_offset: i64) -> String {
    let total_days = day_offset;
    let mut year = REF_YEAR;
    let mut remaining = total_days;

    // Advance years
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining >= days_in_year {
            remaining -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let days_in_month: [i64; 12] = [
        31, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30,
        31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1;
    for md in days_in_month {
        if remaining >= md {
            remaining -= md;
            month += 1;
        } else {
            break;
        }
    }
    let day = remaining + 1;
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { s.chars().take(max).collect() }
}

fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width { s.chars().take(width).collect() }
    else { format!("{}{}", s, " ".repeat(width - s.len())) }
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
        assert!(o.contains("2024-"));
    }

    #[test]
    fn test_day_to_str() {
        // 2020-01-01 = offset 0
        assert_eq!(day_to_str(0), "2020-01-01");
        // 2020-01-31 = offset 30
        assert_eq!(day_to_str(30), "2020-01-31");
        // 2020-02-01 = offset 31
        assert_eq!(day_to_str(31), "2020-02-01");
        // 2024-01-01 = from 2020-01-01: 4 years = 365*3 + 366 = 1461 days
        assert_eq!(day_to_str(1461), "2024-01-01");
    }
}
