//! Quadrant chart ASCII renderer

use anyhow::Result;
use std::collections::HashSet;
use tracing::debug;

use crate::core::Renderer;
use super::database::QuadrantChartDatabase;

/// Minimum / maximum chart dimensions.
const MIN_WIDTH: usize = 50;
const MAX_WIDTH: usize = 90;
const MIN_HEIGHT: usize = 14;
const MAX_HEIGHT: usize = 30;
/// Extra rows per N points.
const ROWS_PER_POINTS: usize = 4;

#[derive(Debug, Clone)]
pub struct QuadrantChartConfig {
    /// Override chart width (0 = auto-calculate).
    pub width_hint: usize,
}

impl Default for QuadrantChartConfig {
    fn default() -> Self {
        Self { width_hint: 0 }
    }
}

pub struct QuadrantChartRenderer {
    config: QuadrantChartConfig,
}

impl Default for QuadrantChartRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl QuadrantChartRenderer {
    pub fn new() -> Self {
        Self {
            config: QuadrantChartConfig::default(),
        }
    }

    pub fn with_config(config: QuadrantChartConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, db: &QuadrantChartDatabase) -> Result<String> {
        debug!("Rendering quadrant chart");

        // ── Compute canvas dimensions ──────────────────────────
        let max_label_len = db.points.iter()
            .map(|p| p.label.len())
            .max()
            .unwrap_or(10)
            .max(db.quadrant_labels.iter().map(|l| l.len()).max().unwrap_or(6));

        let cw = if self.config.width_hint > 0 {
            self.config.width_hint.max(MIN_WIDTH)
        } else {
            // Width grows with longest label, capped
            (max_label_len + 12).max(MIN_WIDTH).min(MAX_WIDTH)
        };
        let ch = {
            let base = MIN_HEIGHT;
            let extra = db.points.len().saturating_sub(1) / ROWS_PER_POINTS;
            (base + extra).min(MAX_HEIGHT)
        };

        let total_w = cw + 2; // +2 for borders
        let total_h = ch + 2;

        let mut canvas = vec![vec![' '; total_w]; total_h];

        // ── Borders ────────────────────────────────────────────
        canvas[0][0] = '┌';
        canvas[0][total_w - 1] = '┐';
        for x in 1..total_w - 1 {
            canvas[0][x] = '─';
        }
        canvas[total_h - 1][0] = '└';
        canvas[total_h - 1][total_w - 1] = '┘';
        for x in 1..total_w - 1 {
            canvas[total_h - 1][x] = '─';
        }
        for y in 1..total_h - 1 {
            canvas[y][0] = '│';
            canvas[y][total_w - 1] = '│';
        }

        // ── Title ──────────────────────────────────────────────
        if let Some(title) = &db.title {
            let title_str = format!(" {} ", title);
            let fit = title_str.len().min(total_w.saturating_sub(2));
            for (i, ch) in title_str.chars().take(fit).enumerate() {
                canvas[0][1 + i] = ch;
            }
        }

        // ── Chart area coords ──────────────────────────────────
        let cl = 1;                    // chart left
        let ct = 1;                    // chart top
        let cr = total_w - 2;         // chart right
        let cb = total_h - 2;         // chart bottom
        let ciw = cr - cl + 1;        // chart inner width
        let cih = cb - ct + 1;        // chart inner height

        let mid_x = cl + ciw / 2;
        let mid_y = ct + cih / 2;

        // ── Axes ───────────────────────────────────────────────
        for x in cl..=cr {
            if canvas[mid_y][x] == ' ' {
                canvas[mid_y][x] = '─';
            }
        }
        for y in ct..=cb {
            if canvas[y][mid_x] == ' ' {
                canvas[y][mid_x] = '│';
            }
        }
        canvas[mid_y][mid_x] = '┼';

        // ── Axis labels ────────────────────────────────────────
        self.write_str(
            &mut canvas, &db.y_axis_high,
            cl + 1, ct + 1, mid_x.saturating_sub(1), ct + 1,
        );
        self.write_str(
            &mut canvas, &db.y_axis_low,
            cl + 1, cb.saturating_sub(1), mid_x.saturating_sub(1), cb.saturating_sub(1),
        );
        self.write_str(
            &mut canvas, &db.x_axis_low,
            cl + 1, cb, cr.saturating_sub(db.x_axis_high.len()), cb,
        );
        self.write_str_right(
            &mut canvas, &db.x_axis_high,
            cr.saturating_sub(db.x_axis_high.len()), cb,
        );

        // ── Quadrant labels ────────────────────────────────────
        // Q2 (top-left)
        self.draw_label_region(&mut canvas, &db.quadrant_labels[1],
            cl + 2, ct + 2, mid_x.saturating_sub(2), mid_y.saturating_sub(2));
        // Q1 (top-right)
        self.draw_label_region(&mut canvas, &db.quadrant_labels[0],
            mid_x + 2, ct + 2, cr.saturating_sub(1), mid_y.saturating_sub(2));
        // Q3 (bottom-left)
        self.draw_label_region(&mut canvas, &db.quadrant_labels[2],
            cl + 2, mid_y + 2, mid_x.saturating_sub(2), cb.saturating_sub(1));
        // Q4 (bottom-right)
        self.draw_label_region(&mut canvas, &db.quadrant_labels[3],
            mid_x + 2, mid_y + 2, cr.saturating_sub(1), cb.saturating_sub(1));

        // ── Plot points ────────────────────────────────────────
        let mut occupied: HashSet<(usize, usize)> = HashSet::new();

        // Sort points by y descending so higher points are placed first
        let mut sorted_points: Vec<_> = db.points.iter().enumerate().collect();
        sorted_points.sort_by(|a, b| b.1.y.partial_cmp(&a.1.y).unwrap_or(std::cmp::Ordering::Equal));

        for (_i, point) in &sorted_points {
            let px = cl + ((point.x * (ciw.saturating_sub(1)) as f64).round() as usize);
            let py = cb - ((point.y * (cih.saturating_sub(1)) as f64).round() as usize);
            let px = px.clamp(cl + 1, cr.saturating_sub(1));
            let py = py.clamp(ct + 1, cb.saturating_sub(1));

            // Place dot, finding a free row if needed
            let dot_y = self.find_free_row(&occupied, px, py, cb);
            canvas[dot_y][px] = '·';
            occupied.insert((px, dot_y));

            // Write label to the right of the dot, wrapping if needed
            let avail = cr.saturating_sub(px + 2);
            if avail >= 3 {
                let wrapped = wrap_text(&point.label, avail);
                for (li, line) in wrapped.iter().enumerate() {
                    let ly = (dot_y + li).min(cb);
                    self.write_str(&mut canvas, line, px + 2, ly, cr, ly);
                }
            }
        }

        // ── Convert to string ──────────────────────────────────
        let mut lines = Vec::with_capacity(total_h);
        for row in &canvas {
            lines.push(row.iter().collect::<String>().trim_end().to_string());
        }
        Ok(lines.join("\n"))
    }

    /// Write a string at (x_start, y), clipped to x_max.
    fn write_str(&self, canvas: &mut [Vec<char>], s: &str, x: usize, y: usize, x_max: usize, _row: usize) {
        if y >= canvas.len() { return; }
        for (i, ch) in s.chars().enumerate() {
            let xp = x + i;
            if xp <= x_max && xp < canvas[y].len() {
                canvas[y][xp] = ch;
            }
        }
    }

    /// Write a string right-aligned ending at x_end.
    fn write_str_right(&self, canvas: &mut [Vec<char>], s: &str, x_end: usize, y: usize) {
        if y >= canvas.len() { return; }
        let start = x_end.saturating_sub(s.len());
        for (i, ch) in s.chars().enumerate() {
            let xp = start + i;
            if xp < canvas[y].len() {
                canvas[y][xp] = ch;
            }
        }
    }

    /// Draw a label centred within a rectangular region, wrapping if needed.
    fn draw_label_region(
        &self,
        canvas: &mut [Vec<char>],
        label: &str,
        x_min: usize,
        y_min: usize,
        x_max: usize,
        y_max: usize,
    ) {
        if label.is_empty() { return; }
        let region_w = x_max.saturating_sub(x_min);
        if region_w < 3 { return; }

        let wrapped = wrap_text(label, region_w);
        let total_h = wrapped.len();
        let y_start = y_min + (y_max.saturating_sub(y_min)).saturating_sub(total_h) / 2;

        for (li, line) in wrapped.iter().enumerate() {
            let y = y_start + li;
            if y > y_max { break; }
            let x = x_min + (region_w.saturating_sub(line.len())) / 2;
            for (i, ch) in line.chars().enumerate() {
                let xp = x + i;
                if xp <= x_max && y < canvas.len() && xp < canvas[y].len() {
                    canvas[y][xp] = ch;
                }
            }
        }
    }

    /// Find a free row near (px, py) for the dot, to avoid overlap.
    fn find_free_row(&self, occupied: &HashSet<(usize, usize)>, px: usize, py: usize, max_y: usize) -> usize {
        // Check ±2 rows around py
        for dy in 0i32..=3i32 {
            for sign in [1i32, -1i32] {
                let candidate = (py as i32 + sign * dy) as usize;
                if candidate <= max_y && candidate > 0 && !occupied.contains(&(px, candidate)) {
                    return candidate;
                }
            }
        }
        py
    }
}

/// Wrap text to fit within `max_width` columns, breaking at word boundaries
/// when possible, falling back to character-level breaks.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            if word.len() <= max_width {
                current.push_str(word);
            } else {
                // Word is too long — char-break it
                for ch in word.chars() {
                    if current.len() >= max_width {
                        lines.push(current);
                        current = String::new();
                    }
                    current.push(ch);
                }
            }
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = String::new();
            if word.len() <= max_width {
                current.push_str(word);
            } else {
                for ch in word.chars() {
                    if current.len() >= max_width {
                        lines.push(current);
                        current = String::new();
                    }
                    current.push(ch);
                }
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

impl Renderer<QuadrantChartDatabase> for QuadrantChartRenderer {
    type Output = String;

    fn render(&self, database: &QuadrantChartDatabase) -> Result<Self::Output> {
        QuadrantChartRenderer::render(self, database)
    }

    fn name(&self) -> &'static str {
        "quadrantchart-ascii"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn format(&self) -> &'static str {
        "ascii"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::quadrantchart::parser::QuadrantChartParser;

    #[test]
    fn test_render_basic() {
        let input = "quadrantChart\n    title Risk vs Effort\n    x-axis Low Effort --> High Effort\n    y-axis Low Risk --> High Risk\n    quadrant-1 \"Quick Wins\"\n    quadrant-2 \"Major Projects\"\n    quadrant-3 \"Fill-ins\"\n    quadrant-4 \"Thanks\"\n    \"Item A\": [0.2, 0.8]\n    \"Item B\": [0.7, 0.3]\n";
        let parser = QuadrantChartParser::new();
        let mut db = QuadrantChartDatabase::new();
        parser.parse(input, &mut db).unwrap();
        let renderer = QuadrantChartRenderer::new();
        let output = renderer.render(&db).unwrap();
        assert!(output.contains("Risk vs Effort"));
        assert!(output.contains("Quick Wins"));
        assert!(output.contains('·'));
        assert!(output.contains('┼'));
    }

    #[test]
    fn test_wrap_text() {
        let result = wrap_text("hello world test", 8);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "world");
    }
}
