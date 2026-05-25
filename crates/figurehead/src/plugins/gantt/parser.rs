//! Gantt chart parser

use anyhow::{Context, Result, bail};
use tracing::debug;

use super::database::{GanttDatabase, GanttSection, GanttTask, TaskStatus};

/// Simple date: year, month, day.
#[derive(Debug, Clone, Copy)]
struct Date { year: i32, month: u32, day: u32 }

impl Date {
    fn parse_ymd(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('-').collect();
        if parts.len() != 3 { return None; }
        let year: i32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let day: u32 = parts[2].parse().ok()?;
        if month < 1 || month > 12 || day < 1 || day > 31 { return None; }
        Some(Self { year, month, day })
    }

    /// Days since a reference date.
    fn days_since(&self, ref_date: Date) -> i64 {
        let days = (self.year as i64 - ref_date.year as i64) * 365
            + (self.month as i64 - ref_date.month as i64) * 30
            + (self.day as i64 - ref_date.day as i64);
        days
    }
}

/// Reference date for day-offset computation.
const REF_DATE: Date = Date { year: 2020, month: 1, day: 1 };

/// Parse duration string like "30d", "2w", "1M", "7d"
fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.ends_with('d') || s.ends_with('D') {
        s[..s.len()-1].parse::<i64>().ok()
    } else if s.ends_with('w') || s.ends_with('W') {
        s[..s.len()-1].parse::<i64>().ok().map(|d| d * 7)
    } else if s.ends_with('M') {
        s[..s.len()-1].parse::<i64>().ok().map(|d| d * 30)
    } else {
        // Try as day number
        s.parse::<i64>().ok()
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s.trim().to_lowercase().as_str() {
        "done" => TaskStatus::Done,
        "active" => TaskStatus::Active,
        "crit" => TaskStatus::Crit,
        "milestone" => TaskStatus::Milestone,
        _ => TaskStatus::Normal,
    }
}

pub struct GanttParser;

impl Default for GanttParser {
    fn default() -> Self { Self::new() }
}

impl GanttParser {
    pub fn new() -> Self { Self }

    pub fn parse(&self, input: &str, db: &mut GanttDatabase) -> Result<()> {
        let lines: Vec<&str> = input.lines().collect();
        let first = lines.first().map(|l| l.trim()).unwrap_or("");
        if !first.to_lowercase().starts_with("gantt") {
            bail!("Input does not start with 'gantt'");
        }

        let mut current_section: Option<GanttSection> = None;
        // Map from task id → end day for "after" references
        let mut task_end_days: Vec<(String, i64)> = Vec::new();

        for line in &lines[1..] {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            let lower = trimmed.to_lowercase();

            if lower.starts_with("title ") {
                db.title = Some(trimmed["title ".len()..].trim().to_string());
                continue;
            }

            if lower.starts_with("dateformat ") {
                db.date_format = trimmed["dateformat ".len()..].trim().to_string();
                continue;
            }

            if lower.starts_with("section ") {
                // Flush previous section
                if let Some(sec) = current_section.take() {
                    db.sections.push(sec);
                }
                let name = trimmed["section ".len()..].trim().to_string();
                current_section = Some(GanttSection { name, tasks: Vec::new() });
                continue;
            }

            // Try to parse as a task line: <label> :<status>, <id>, <start>, <duration>
            if let Some(task) = parse_task(trimmed, &task_end_days) {
                if let Some(ref mut sec) = current_section {
                    task_end_days.push((task.id.clone(), task.start_day + task.duration_days));
                    sec.tasks.push(task);
                } else {
                    // Implicit default section
                    task_end_days.push((task.id.clone(), task.start_day + task.duration_days));
                    let mut sec = GanttSection { name: String::new(), tasks: Vec::new() };
                    sec.tasks.push(task);
                    current_section = Some(sec);
                }
            }
        }

        if let Some(sec) = current_section {
            db.sections.push(sec);
        }

        db.recompute_range();
        debug!(sections = db.sections.len(), "Gantt parsing complete");
        Ok(())
    }
}

/// Parse a task line like `Task name :done, id1, 2024-01-15, 30d`
fn parse_task(line: &str, after_map: &[(String, i64)]) -> Option<GanttTask> {
    // Split at first colon to get label and rest
    let (label, rest) = if let Some(idx) = line.find(':') {
        let (l, r) = line.split_at(idx);
        (l.trim().to_string(), r[1..].trim().to_string())
    } else {
        return None;
    };

    // Parse comma-separated parts: status?, id?, start, duration
    let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() { return None; }

    let mut pos = 0;
    let mut status = TaskStatus::Normal;
    let mut id = String::new();
    let mut start_day: Option<i64> = None;
    let mut duration: Option<i64> = None;

    // First part could be status or id or start
    let first = parts[pos];
    let first_lower = first.to_lowercase();
    if matches!(first_lower.as_str(), "done" | "active" | "crit" | "milestone") {
        status = parse_status(first);
        pos += 1;
    }

    // Next could be id (alphanumeric, not date-like)
    if pos < parts.len() {
        let part = parts[pos];
        if !part.contains('-') && !part.starts_with("after ") && part.chars().all(|c| c.is_alphanumeric() || c == '_') {
            id = part.to_string();
            pos += 1;
        }
    }

    // Parse start
    if pos < parts.len() {
        let part = parts[pos];
        if part.to_lowercase().starts_with("after ") {
            let ref_id = part["after ".len()..].trim();
            if let Some((_, end)) = after_map.iter().find(|(tid, _)| tid == ref_id) {
                start_day = Some(*end);
            } else {
                start_day = Some(0); // unresolved — default to 0
            }
        } else if let Some(date) = Date::parse_ymd(part) {
            start_day = Some(date.days_since(REF_DATE));
        } else if let Some(d) = parse_duration(part) {
            duration = Some(d);
        }
        if start_day.is_some() { pos += 1; }
    }

    // Parse duration
    if duration.is_none() && pos < parts.len() {
        let part = parts[pos];
        // Could be an end date "YYYY-MM-DD"
        if let Some(end_date) = Date::parse_ymd(part) {
            let start = start_day.unwrap_or(0);
            duration = Some(end_date.days_since(REF_DATE) - start);
        } else if let Some(d) = parse_duration(part) {
            duration = Some(d);
        }
    }

    let start_day = start_day.unwrap_or(0);
    let duration_days = duration.unwrap_or(7).max(1);

    Some(GanttTask {
        label,
        id,
        status,
        start_day,
        duration_days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_gantt() {
        let input = "gantt\n    title Test\n    dateFormat YYYY-MM-DD\n    section Dev\n    Task A :done, a1, 2024-01-01, 30d\n    Task B :active, b1, after a1, 20d\n";
        let p = GanttParser::new();
        let mut db = GanttDatabase::new();
        p.parse(input, &mut db).unwrap();
        assert_eq!(db.title.as_deref(), Some("Test"));
        assert_eq!(db.sections.len(), 1);
        assert_eq!(db.sections[0].tasks.len(), 2);
        assert_eq!(db.sections[0].tasks[0].status, TaskStatus::Done);
        assert_eq!(db.sections[0].tasks[1].status, TaskStatus::Active);
    }

    #[test]
    fn test_parse_date() {
        let d = Date::parse_ymd("2024-06-15").unwrap();
        assert_eq!(d.year, 2024);
        assert_eq!(d.month, 6);
        assert_eq!(d.day, 15);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30d"), Some(30));
        assert_eq!(parse_duration("2w"), Some(14));
        assert_eq!(parse_duration("1M"), Some(30));
        assert_eq!(parse_duration("7"), Some(7));
    }
}
