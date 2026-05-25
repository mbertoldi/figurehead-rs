//! Quadrant chart ASCII renderer

use anyhow::Result;
use tracing::debug;

use super::database::QuadrantChartDatabase;

#[derive(Debug, Clone)]
pub struct QuadrantChartConfig {
    pub chart_width: usize,
    pub chart_height: usize,
}

impl Default for QuadrantChartConfig {
    fn default() -> Self {
        Self {
            chart_width: 60,
            chart_height: 18,
        }
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

        let cw = self.config.chart_width;
        let ch = self.config.chart_height;
        let inner_w = cw;
        let inner_h = ch;
        let total_w = inner_w + 2;
        let total_h = inner_h + 2;

        let mut canvas = vec![vec![' '; total_w]; total_h];

        // Borders
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

        // Title in top border
        if let Some(title) = &db.title {
            let title_str = format!(" {} ", title);
            let fit = title_str.len().min(total_w.saturating_sub(2));
            for (i, ch) in title_str.chars().take(fit).enumerate() {
                canvas[0][1 + i] = ch;
            }
        }

        let chart_left = 1;
        let chart_top = 1;
        let chart_right = total_w - 2;
        let chart_bottom = total_h - 2;
        let chart_inner_w = chart_right - chart_left + 1;
        let chart_inner_h = chart_bottom - chart_top + 1;

        let mid_x = chart_left + chart_inner_w / 2;
        let mid_y = chart_top + chart_inner_h / 2;

        // Axes
        for x in chart_left..=chart_right {
            if canvas[mid_y][x] == ' ' {
                canvas[mid_y][x] = '─';
            }
        }
        for y in chart_top..=chart_bottom {
            if canvas[y][mid_x] == ' ' {
                canvas[y][mid_x] = '│';
            }
        }
        canvas[mid_y][mid_x] = '┼';

        // X-axis labels on bottom border
        for (i, ch) in db.x_axis_low.chars().enumerate() {
            if chart_left + 1 + i < chart_right {
                canvas[chart_bottom][chart_left + 1 + i] = ch;
            }
        }
        let x_high_start = chart_right.saturating_sub(db.x_axis_high.len());
        for (i, ch) in db.x_axis_high.chars().enumerate() {
            if x_high_start + i < chart_right {
                canvas[chart_bottom][x_high_start + i] = ch;
            }
        }

        // Y-axis labels on left side
        for (i, ch) in db.y_axis_high.chars().enumerate() {
            if chart_left + 1 + i < mid_x {
                canvas[chart_top + 1][chart_left + 1 + i] = ch;
            }
        }
        for (i, ch) in db.y_axis_low.chars().enumerate() {
            if chart_left + 1 + i < mid_x {
                canvas[chart_bottom.saturating_sub(1)][chart_left + 1 + i] = ch;
            }
        }

        // Quadrant labels
        self.draw_label(&mut canvas, &db.quadrant_labels[1], chart_left + 2, chart_top + 1, mid_x.saturating_sub(1), mid_y.saturating_sub(1));
        self.draw_label(&mut canvas, &db.quadrant_labels[0], mid_x + 1, chart_top + 1, chart_right, mid_y.saturating_sub(1));
        self.draw_label(&mut canvas, &db.quadrant_labels[2], chart_left + 2, mid_y + 1, mid_x.saturating_sub(1), chart_bottom);
        self.draw_label(&mut canvas, &db.quadrant_labels[3], mid_x + 1, mid_y + 1, chart_right, chart_bottom);

        // Plot points
        for point in &db.points {
            let px = chart_left + ((point.x * (chart_inner_w.saturating_sub(1)) as f64).round() as usize);
            let py = chart_bottom - ((point.y * (chart_inner_h.saturating_sub(1)) as f64).round() as usize);
            let px = px.clamp(chart_left, chart_right);
            let py = py.clamp(chart_top, chart_bottom);
            canvas[py][px] = '·';
            let label_x = (px + 2).min(chart_right.saturating_sub(point.label.len()));
            for (i, ch) in point.label.chars().enumerate() {
                if label_x + i <= chart_right {
                    canvas[py][label_x + i] = ch;
                }
            }
        }

        let mut lines = Vec::with_capacity(total_h);
        for row in &canvas {
            lines.push(row.iter().collect::<String>().trim_end().to_string());
        }
        Ok(lines.join("\n"))
    }

    fn draw_label(
        &self,
        canvas: &mut [Vec<char>],
        label: &str,
        x_min: usize,
        y_min: usize,
        x_max: usize,
        y_max: usize,
    ) {
        if label.is_empty() {
            return;
        }
        let region_w = x_max.saturating_sub(x_min);
        if region_w < 3 {
            return;
        }
        let y = y_min + (y_max.saturating_sub(y_min)) / 2;
        let fit = label.len().min(region_w.saturating_sub(1));
        let x = x_min + (region_w.saturating_sub(fit)) / 2;
        for (i, ch) in label.chars().take(fit).enumerate() {
            if y < canvas.len() && x + i < canvas[y].len() {
                canvas[y][x + i] = ch;
            }
        }
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
}
