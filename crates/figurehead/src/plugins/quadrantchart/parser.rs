//! Quadrant chart parser
//!
//! Parses Mermaid quadrantChart syntax into a QuadrantChartDatabase.

use anyhow::{Context, Result, bail};
use tracing::{debug, trace, warn};

use super::database::{QuadrantChartDatabase, QuadrantPoint};

/// Parser for quadrantChart syntax.
pub struct QuadrantChartParser;

impl Default for QuadrantChartParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QuadrantChartParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse quadrantChart input into a database.
    pub fn parse(&self, input: &str, database: &mut QuadrantChartDatabase) -> Result<()> {
        let lines: Vec<&str> = input.lines().collect();

        let first = lines.first().map(|l| l.trim()).unwrap_or("");
        if !first.to_lowercase().starts_with("quadrantchart")
            && !first.to_lowercase().starts_with("quadrant chart")
        {
            bail!("Input does not start with 'quadrantChart'");
        }

        for line in &lines[1..] {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.parse_line(trimmed, database)
                .with_context(|| format!("Failed to parse line: {trimmed}"))?;
        }

        if database.x_axis_low.is_empty() || database.x_axis_high.is_empty() {
            warn!("Missing x-axis definition");
        }
        if database.y_axis_low.is_empty() || database.y_axis_high.is_empty() {
            warn!("Missing y-axis definition");
        }

        debug!(
            point_count = database.points.len(),
            "Quadrant chart parsing completed"
        );
        Ok(())
    }

    fn parse_line(&self, line: &str, db: &mut QuadrantChartDatabase) -> Result<()> {
        let lower = line.to_lowercase();

        if lower.starts_with("title ") {
            db.title = Some(line["title ".len()..].trim().to_string());
            return Ok(());
        }

        if lower.starts_with("x-axis ") {
            let rest = &line["x-axis ".len()..];
            if let Some((low, high)) = rest.split_once("-->") {
                db.x_axis_low = low.trim().to_string();
                db.x_axis_high = high.trim().to_string();
            } else if let Some((low, high)) = rest.split_once("->") {
                db.x_axis_low = low.trim().to_string();
                db.x_axis_high = high.trim().to_string();
            } else {
                bail!("Invalid x-axis format, expected 'x-axis <low> --> <high>'");
            }
            return Ok(());
        }

        if lower.starts_with("y-axis ") {
            let rest = &line["y-axis ".len()..];
            if let Some((low, high)) = rest.split_once("-->") {
                db.y_axis_low = low.trim().to_string();
                db.y_axis_high = high.trim().to_string();
            } else if let Some((low, high)) = rest.split_once("->") {
                db.y_axis_low = low.trim().to_string();
                db.y_axis_high = high.trim().to_string();
            } else {
                bail!("Invalid y-axis format, expected 'y-axis <low> --> <high>'");
            }
            return Ok(());
        }

        for n in 1..=4 {
            let prefix = format!("quadrant-{n}");
            if lower.starts_with(&prefix) {
                let rest = line[prefix.len()..].trim();
                let label = rest.trim_matches('"').trim_matches('\'').to_string();
                db.quadrant_labels[n - 1] = label;
                return Ok(());
            }
        }

        if let Some((label, coords)) = parse_point(line) {
            db.points.push(QuadrantPoint {
                label,
                x: coords.0,
                y: coords.1,
            });
            return Ok(());
        }

        trace!(line, "Skipping unrecognized line");
        Ok(())
    }
}

/// Parse a point definition: `"Label text": [0.5, 0.8]`
fn parse_point(line: &str) -> Option<(String, (f64, f64))> {
    let after_label = if let Some(rest) = line.strip_prefix('"') {
        let end = rest.find('"')?;
        let label = rest[..end].to_string();
        (label, &rest[end + 1..])
    } else if let Some(rest) = line.strip_prefix('\'') {
        let end = rest.find('\'')?;
        let label = rest[..end].to_string();
        (label, &rest[end + 1..])
    } else {
        return None;
    };

    let rest = after_label.1.trim();
    let rest = rest.strip_prefix(':')?.trim();
    let rest = rest.strip_prefix('[')?.trim();
    let rest = rest.strip_suffix(']')?.trim();

    let (x_str, y_str) = rest.split_once(',')?;
    let x: f64 = x_str.trim().parse().ok()?;
    let y: f64 = y_str.trim().parse().ok()?;

    Some((after_label.0, (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_quadrant_chart() {
        let input = "quadrantChart\n    title Test\n    x-axis Low E --> High E\n    y-axis Low R --> High R\n    quadrant-1 \"Quick Wins\"\n    quadrant-2 \"Major\"\n    quadrant-3 \"Fill-ins\"\n    quadrant-4 \"Thanks\"\n    \"Point A\": [0.2, 0.8]\n    \"Point B\": [0.7, 0.3]\n";
        let parser = QuadrantChartParser::new();
        let mut db = QuadrantChartDatabase::new();
        parser.parse(input, &mut db).unwrap();

        assert_eq!(db.title.as_deref(), Some("Test"));
        assert_eq!(db.x_axis_low, "Low E");
        assert_eq!(db.x_axis_high, "High E");
        assert_eq!(db.quadrant_labels[0], "Quick Wins");
        assert_eq!(db.points.len(), 2);
    }

    #[test]
    fn test_rejects_non_quadrant_chart() {
        let input = "graph TD\n    A --> B";
        let parser = QuadrantChartParser::new();
        let mut db = QuadrantChartDatabase::new();
        assert!(parser.parse(input, &mut db).is_err());
    }
}
