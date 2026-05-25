//! Quadrant chart ASCII renderer
//!
//! Layout:
//! ```text
//! ┌─ Title ───────────────────────────────────────────────────────┐
//! │ H  Q2 Title                  │  Q1 Title                      │
//! │ I                            │                                │
//! │ G  · Point A description     │  · Point B                     │
//! │ H  · Another point           │                                │
//! │                              │                                │
//! │ R ───────────────────────────┼────────────────────────────────│
//! │ I                            │                                │
//! │ S  Q3 Title                  │  Q4 Title                      │
//! │ K                            │                                │
//! │                              │  · Point C                     │
//! │                              │                                │
//! │       Low Effort ────────────┴────────────── High Effort ────│
//! └───────────────────────────────────────────────────────────────┘
//! ```

use anyhow::Result;
use tracing::debug;

use crate::core::Renderer;
use super::database::QuadrantChartDatabase;

const MIN_W: usize = 40;
const LABEL_COL: usize = 6; // columns reserved for vertical Y-axis label
const Q_PAD: usize = 2; // padding inside quadrants

#[derive(Debug, Clone)]
pub struct QuadrantChartConfig {
    pub width_hint: usize,
}

impl Default for QuadrantChartConfig {
    fn default() -> Self { Self { width_hint: 0 } }
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

    pub fn with_config(config: QuadrantChartConfig) -> Self { Self { config } }

    pub fn render(&self, db: &QuadrantChartDatabase) -> Result<String> {
        debug!("Rendering quadrant chart");

        // ── Assign points to quadrants ──────────────────────────
        // Q1: top-right (x>=0.5, y>=0.5), Q2: top-left (x<0.5, y>=0.5)
        // Q3: bottom-left (x<0.5, y<0.5), Q4: bottom-right (x>=0.5, y<0.5)
        let mut q_points: [Vec<&str>; 4] = [vec![], vec![], vec![], vec![]];
        for p in &db.points {
            let qi = match (p.x >= 0.5, p.y >= 0.5) {
                (true, true) => 0,   // Q1 top-right
                (false, true) => 1,  // Q2 top-left
                (false, false) => 2, // Q3 bottom-left
                (true, false) => 3,  // Q4 bottom-right
            };
            q_points[qi].push(&p.label);
        }

        // ── Calculate widths ─────────────────────────────────────
        let y_label_len = db.y_axis_high.len().max(db.y_axis_low.len());
        let use_vertical_y = y_label_len <= 12;
        let left_margin = if use_vertical_y { LABEL_COL } else { y_label_len + 2 };

        // Per-quadrant content width
        let q_label_w: Vec<usize> = db.quadrant_labels.iter().map(|l| l.len()).collect();
        let mut half_w = MIN_W / 2;
        for qi in 0..4 {
            let max_pt = q_points[qi].iter().map(|l| l.len()).max().unwrap_or(0);
            half_w = half_w.max(q_label_w[qi] + Q_PAD).max(max_pt + Q_PAD + 2);
        }
        half_w = half_w.min(45); // cap to avoid excessive width

        let total_w = left_margin + half_w * 2 + 1 + 2; // margins + half + axis + borders
        let total_w = if self.config.width_hint > 0 {
            self.config.width_hint.max(total_w)
        } else {
            total_w.max(MIN_W)
        };

        // ── Calculate heights ────────────────────────────────────
        // Each point takes 1 row, plus quadrant title row, plus padding
        let mut top_rows = 1usize; // title row
        let mut bot_rows = 1usize; // title row
        for qi in 0..4 {
            let pts = q_points[qi].len();
            let rows_needed = 1 + pts; // title row + point rows
            if qi <= 1 { top_rows = top_rows.max(rows_needed); }
            else { bot_rows = bot_rows.max(rows_needed); }
        }
        top_rows = top_rows.max(3);
        bot_rows = bot_rows.max(3);
        let total_h = top_rows + bot_rows + 1 + 3; // +1 axis + 2 borders + 1 bottom label

        let mut canvas = vec![vec![' '; total_w]; total_h];

        // ── Borders ────────────────────────────────────────────
        canvas[0][0] = '┌'; canvas[0][total_w-1] = '┐';
        for x in 1..total_w-1 { canvas[0][x] = '─'; }
        canvas[total_h-1][0] = '└'; canvas[total_h-1][total_w-1] = '┘';
        for x in 1..total_w-1 { canvas[total_h-1][x] = '─'; }
        for y in 1..total_h-1 { canvas[y][0] = '│'; canvas[y][total_w-1] = '│'; }

        // ── Title ──────────────────────────────────────────────
        if let Some(title) = &db.title {
            let t = format!(" {} ", title);
            for (i, ch) in t.chars().take(total_w.saturating_sub(2)).enumerate() {
                canvas[0][1+i] = ch;
            }
        }

        // ── Chart coords ────────────────────────────────────────
        let cl = 1 + left_margin;
        let ct = 1;
        let cr = total_w - 2;
        let cb = ct + top_rows + bot_rows; // last chart row
        let mid_x = 1 + left_margin + half_w;
        let mid_y = ct + top_rows;

        // ── Axes ───────────────────────────────────────────────
        for x in cl..=cr {
            if canvas[mid_y][x] == ' ' { canvas[mid_y][x] = '─'; }
        }
        for y in ct..=cb {
            if canvas[y][mid_x] == ' ' { canvas[y][mid_x] = '│'; }
        }
        canvas[mid_y][mid_x] = '┼';

        // ── Y-axis labels (vertical if possible) ────────────────
        if use_vertical_y {
            let high_start = ct + 1;
            write_vertical(&mut canvas, &db.y_axis_high, cl.saturating_sub(3), high_start);
            let low_start = cb.saturating_sub(db.y_axis_low.len());
            write_vertical(&mut canvas, &db.y_axis_low, cl.saturating_sub(3), low_start);
        } else {
            let hy = ct + top_rows / 2;
            write_str(&mut canvas, &db.y_axis_high, cl.saturating_sub(db.y_axis_high.len() + 1), hy);
            let ly = cb - bot_rows / 2;
            write_str(&mut canvas, &db.y_axis_low, cl.saturating_sub(db.y_axis_low.len() + 1), ly);
        }

        // ── X-axis labels (centered in each half) ───────────────
        let xl_row = total_h - 2;
        write_centered(&mut canvas, &db.x_axis_low, cl, xl_row, mid_x.saturating_sub(1));
        write_centered(&mut canvas, &db.x_axis_high, mid_x, xl_row, cr);

        // ── Quadrant titles ─────────────────────────────────────
        // Q2 (top-left) [1], Q1 (top-right) [0]
        write_str(&mut canvas, &db.quadrant_labels[1], cl + Q_PAD, ct + 1);
        write_str(&mut canvas, &db.quadrant_labels[0], mid_x + Q_PAD, ct + 1);
        // Q3 (bottom-left) [2], Q4 (bottom-right) [3]
        write_str(&mut canvas, &db.quadrant_labels[2], cl + Q_PAD, mid_y + 1);
        write_str(&mut canvas, &db.quadrant_labels[3], mid_x + Q_PAD, mid_y + 1);

        // ── Plot points as list per quadrant ────────────────────
        for qi in 0..4 {
            let (x_start, y_start) = match qi {
                0 => (mid_x + Q_PAD, ct + 2),       // Q1 top-right
                1 => (cl + Q_PAD, ct + 2),           // Q2 top-left
                2 => (cl + Q_PAD, mid_y + 2),        // Q3 bottom-left
                3 => (mid_x + Q_PAD, mid_y + 2),     // Q4 bottom-right
                _ => unreachable!(),
            };
            let y_max = match qi {
                0 | 1 => mid_y.saturating_sub(1),
                _ => cb,
            };
            for (pi, label) in q_points[qi].iter().enumerate() {
                let y = y_start + pi;
                if y > y_max { break; }
                // Dot
                let dot_x = x_start;
                if dot_x < canvas[y].len() { canvas[y][dot_x] = '·'; }
                // Label after dot
                let label_x = dot_x + 2;
                let max_w = cr.saturating_sub(label_x + 1);
                let display = clip(label, max_w);
                for (i, ch) in display.chars().enumerate() {
                    let xp = label_x + i;
                    if xp < canvas[y].len() { canvas[y][xp] = ch; }
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
}

fn write_str(canvas: &mut [Vec<char>], s: &str, x: usize, y: usize) {
    if y >= canvas.len() { return; }
    for (i, ch) in s.chars().enumerate() {
        let xp = x + i;
        if xp < canvas[y].len() { canvas[y][xp] = ch; }
    }
}

fn write_centered(canvas: &mut [Vec<char>], s: &str, x1: usize, y: usize, x2: usize) {
    let region = x2.saturating_sub(x1);
    let x = x1 + region.saturating_sub(s.len()) / 2;
    write_str(canvas, s, x, y);
}

fn write_vertical(canvas: &mut [Vec<char>], s: &str, col: usize, y_start: usize) {
    for (i, ch) in s.chars().enumerate() {
        let y = y_start + i;
        if y < canvas.len() && col < canvas[y].len() {
            canvas[y][col] = ch;
        }
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { s.chars().take(max).collect() }
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
