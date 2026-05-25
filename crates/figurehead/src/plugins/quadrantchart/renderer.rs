//! Quadrant chart ASCII renderer
//!
//! Layout:
//! ```text
//! ┌─ Title ──────────────────────────────────────────────────┐
//! │                │ Q1 Title            │ Q2 Title          │
//! │ H              │                     │                   │
//! │ I  · Label A   │                     │                   │
//! │ G              │  · Label B          │                   │
//! │ H              │                     │                   │
//! │   ─────────────┼─────────────────────│                   │
//! │ R              │                     │                   │
//! │ I  Q3 Title    │ Q4 Title            │                   │
//! │ S              │                     │                   │
//! │ K              │  · Label C          │                   │
//! │                │                     │                   │
//! │    Low ────────┴─────────── High ────│                   │
//! └──────────────────────────────────────────────────────────┘
//! ```

use anyhow::Result;
use std::collections::HashSet;
use tracing::debug;

use crate::core::Renderer;
use super::database::QuadrantChartDatabase;

const MIN_CHART_W: usize = 30;
const MAX_CHART_W: usize = 70;
const MIN_CHART_H: usize = 10;
const LABEL_COL: usize = 6; // columns reserved for vertical Y-axis label

#[derive(Debug, Clone)]
pub struct QuadrantChartConfig {
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
    fn default() -> Self { Self::new() }
}

impl QuadrantChartRenderer {
    pub fn new() -> Self {
        Self { config: QuadrantChartConfig::default() }
    }

    pub fn with_config(config: QuadrantChartConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, db: &QuadrantChartDatabase) -> Result<String> {
        debug!("Rendering quadrant chart");

        // ── Compute dimensions ─────────────────────────────────
        let max_label = db.points.iter()
            .map(|p| p.label.len())
            .max().unwrap_or(8)
            .max(db.quadrant_labels.iter().map(|l| l.len()).max().unwrap_or(6));

        let y_label_len = db.y_axis_high.len().max(db.y_axis_low.len());
        let use_vertical_y = y_label_len <= 12; // vertical if ≤ 12 chars

        // Left margin: space for vertical Y labels or horizontal Y labels
        let left_margin = if use_vertical_y { LABEL_COL } else { y_label_len + 2 };

        let cw = if self.config.width_hint > 0 {
            (self.config.width_hint.saturating_sub(left_margin)).max(MIN_CHART_W)
        } else {
            (max_label + 8 + left_margin).max(MIN_CHART_W).min(MAX_CHART_W)
        };
        let chart_w = cw - left_margin;

        let ch = {
            let base = MIN_CHART_H;
            let extra = db.points.len().saturating_sub(1) / 3;
            (base + extra).max(8).min(26)
        };

        // Total canvas
        let total_w = cw + 2; // +2 for borders
        let total_h = ch + 3; // +2 borders + 1 bottom row for x-axis labels

        let mut canvas = vec![vec![' '; total_w]; total_h];

        // ── Main border ────────────────────────────────────────
        canvas[0][0] = '┌';
        canvas[0][total_w - 1] = '┐';
        for x in 1..total_w - 1 { canvas[0][x] = '─'; }
        canvas[total_h - 1][0] = '└';
        canvas[total_h - 1][total_w - 1] = '┘';
        for x in 1..total_w - 1 { canvas[total_h - 1][x] = '─'; }
        for y in 1..total_h - 1 {
            canvas[y][0] = '│';
            canvas[y][total_w - 1] = '│';
        }

        // ── Title ──────────────────────────────────────────────
        if let Some(title) = &db.title {
            let t = format!(" {} ", title);
            for (i, ch) in t.chars().take(total_w.saturating_sub(2)).enumerate() {
                canvas[0][1 + i] = ch;
            }
        }

        // ── Chart area coords (inside borders, above bottom row) ──
        let cl = 1 + left_margin;  // chart left (after Y labels)
        let ct = 1;                 // chart top
        let cr = total_w - 2;       // chart right
        let cb = total_h - 3;       // chart bottom (above x-axis labels)
        let ciw = cr - cl + 1;
        let cih = cb - ct + 1;

        let mid_x = cl + ciw / 2;
        let mid_y = ct + cih / 2;

        // ── Axes ───────────────────────────────────────────────
        for x in cl..=cr {
            if canvas[mid_y][x] == ' ' { canvas[mid_y][x] = '─'; }
        }
        for y in ct..=cb {
            if canvas[y][mid_x] == ' ' { canvas[y][mid_x] = '│'; }
        }
        canvas[mid_y][mid_x] = '┼';

        // ── Y-axis labels (left of chart) ──────────────────────
        if use_vertical_y {
            // High label: top half, vertical
            self.write_vertical(&mut canvas, &db.y_axis_high,
                cl.saturating_sub(3), ct + 2, ct + 2 + db.y_axis_high.len());
            // Low label: bottom half, vertical
            let low_start = cb.saturating_sub(db.y_axis_low.len() + 1);
            self.write_vertical(&mut canvas, &db.y_axis_low,
                cl.saturating_sub(3), low_start, low_start + db.y_axis_low.len());
        } else {
            // Horizontal Y labels, centered in each half
            let high_y = ct + cih / 4;
            self.write_str(&mut canvas, &db.y_axis_high, cl.saturating_sub(db.y_axis_high.len() + 1), high_y);
            let low_y = cb - cih / 4;
            self.write_str(&mut canvas, &db.y_axis_low, cl.saturating_sub(db.y_axis_low.len() + 1), low_y);
        }

        // ── X-axis labels (below chart, on bottom border row) ──
        let label_row = total_h - 2; // second-to-last row
        let left_region_end = mid_x.saturating_sub(1);
        // Low label centered in left half
        self.write_centered(&mut canvas, &db.x_axis_low, cl, label_row, left_region_end);
        // High label centered in right half
        self.write_centered(&mut canvas, &db.x_axis_high, mid_x, label_row, cr);

        // ── Quadrant titles (top of each quadrant) ─────────────
        let qpad = 2;
        // Q2: top-left → quadrant_labels[1]
        self.write_str(&mut canvas, &db.quadrant_labels[1], cl + qpad,
            ct + 1);
        // Q1: top-right → quadrant_labels[0]
        self.write_str(&mut canvas, &db.quadrant_labels[0], mid_x + qpad,
            ct + 1);
        // Q3: bottom-left → quadrant_labels[2]
        self.write_str(&mut canvas, &db.quadrant_labels[2], cl + qpad,
            mid_y + 1);
        // Q4: bottom-right → quadrant_labels[3]
        self.write_str(&mut canvas, &db.quadrant_labels[3], mid_x + qpad,
            mid_y + 1);

        // ── Plot points ────────────────────────────────────────
        // Points with dot + label, avoiding overlaps
        let mut occupied: HashSet<(usize, usize)> = HashSet::new();

        // Sort by y desc so higher points get first pick of rows
        let mut sorted: Vec<_> = db.points.iter().enumerate().collect();
        sorted.sort_by(|a, b| b.1.y.partial_cmp(&a.1.y).unwrap_or(std::cmp::Ordering::Equal));

        for (_i, point) in &sorted {
            let px = cl + 1 + ((point.x * (ciw.saturating_sub(2)) as f64).round() as usize);
            let py = cb.saturating_sub(1) - ((point.y * (cih.saturating_sub(2)) as f64).round() as usize);
            let px = px.clamp(cl + 1, cr.saturating_sub(1));
            let py = py.clamp(ct + 3, cb.saturating_sub(1)); // skip title row

            // Find free row for the dot
            let dot_y = find_free_row(&occupied, px, py, cb);
            canvas[dot_y][px] = '·';
            occupied.insert((px, dot_y));

            // Label: "· Label text" starting at px+1
            let avail = cr.saturating_sub(px + 2).max(3);
            let wrapped = wrap_text(&point.label, avail.saturating_sub(1));
            for (li, line) in wrapped.iter().enumerate() {
                let ly = (dot_y + li).min(cb);
                for (i, ch) in line.chars().enumerate() {
                    let xp = px + 2 + i;
                    if xp <= cr && ly < canvas.len() && xp < canvas[ly].len() {
                        canvas[ly][xp] = ch;
                    }
                }
            }
        }

        // ── Stringify ──────────────────────────────────────────
        let mut lines = Vec::with_capacity(total_h);
        for row in &canvas {
            lines.push(row.iter().collect::<String>().trim_end().to_string());
        }
        Ok(lines.join("\n"))
    }

    fn write_str(&self, canvas: &mut [Vec<char>], s: &str, x: usize, y: usize) {
        if y >= canvas.len() { return; }
        for (i, ch) in s.chars().enumerate() {
            let xp = x + i;
            if xp < canvas[y].len() { canvas[y][xp] = ch; }
        }
    }

    fn write_centered(&self, canvas: &mut [Vec<char>], s: &str, x1: usize, y: usize, x2: usize) {
        let region = x2.saturating_sub(x1);
        let x = x1 + region.saturating_sub(s.len()) / 2;
        self.write_str(canvas, s, x, y);
    }

    fn write_vertical(&self, canvas: &mut [Vec<char>], s: &str, col: usize, y_start: usize, _y_end: usize) {
        for (i, ch) in s.chars().enumerate() {
            let y = y_start + i;
            if y < canvas.len() && col < canvas[y].len() {
                canvas[y][col] = ch;
            }
        }
    }
}

fn find_free_row(occupied: &HashSet<(usize, usize)>, px: usize, py: usize, max_y: usize) -> usize {
    for dy in 0i32..=5i32 {
        for sign in [1i32, -1i32] {
            let candidate = (py as i32 + sign * dy).max(1).min(max_y as i32) as usize;
            if !occupied.contains(&(px, candidate)) {
                return candidate;
            }
        }
    }
    py
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 { return vec![text.to_string()]; }
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            if word.len() <= max_width { cur.push_str(word); }
            else {
                for ch in word.chars() {
                    if cur.len() >= max_width { lines.push(cur); cur = String::new(); }
                    cur.push(ch);
                }
            }
        } else if cur.len() + 1 + word.len() <= max_width {
            cur.push(' '); cur.push_str(word);
        } else {
            lines.push(cur); cur = String::new();
            if word.len() <= max_width { cur.push_str(word); }
            else {
                for ch in word.chars() {
                    if cur.len() >= max_width { lines.push(cur); cur = String::new(); }
                    cur.push(ch);
                }
            }
        }
    }
    if !cur.is_empty() { lines.push(cur); }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

impl Renderer<QuadrantChartDatabase> for QuadrantChartRenderer {
    type Output = String;
    fn render(&self, db: &QuadrantChartDatabase) -> Result<Self::Output> {
        QuadrantChartRenderer::render(self, db)
    }
    fn name(&self) -> &'static str { "quadrantchart-ascii" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn format(&self) -> &'static str { "ascii" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::quadrantchart::parser::QuadrantChartParser;

    #[test]
    fn test_render_basic() {
        let input = "quadrantChart\n    title Risk vs Effort\n    x-axis Low Effort --> High Effort\n    y-axis Low Risk --> High Risk\n    quadrant-1 \"Quick Wins\"\n    quadrant-2 \"Major Projects\"\n    quadrant-3 \"Fill-ins\"\n    quadrant-4 \"Thanks\"\n    \"Item A\": [0.2, 0.8]\n    \"Item B\": [0.7, 0.3]\n";
        let p = QuadrantChartParser::new();
        let mut db = QuadrantChartDatabase::new();
        p.parse(input, &mut db).unwrap();
        let r = QuadrantChartRenderer::new();
        let o = r.render(&db).unwrap();
        assert!(o.contains("Risk vs Effort"));
        assert!(o.contains("Quick Wins"));
        assert!(o.contains('·'));
        assert!(o.contains('┼'));
    }
}
