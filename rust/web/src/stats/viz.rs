//! #29 player stats: presentational visualization components.
//!
//! Pure functions of their props - no DB access, no resources, no effects.
//! Compiles for both ssr and hydrate/wasm targets.

use leptos::prelude::*;

use crate::stats::{FormResult, RatingPoint};

const SPARKLINE_BLOCKS: [char; 8] = [
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

/// Maps values onto Unicode block characters (U+2581..U+2588), scaled
/// linearly across the series' min/max. Flat or single-value series render
/// as the middle block.
pub fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if max <= min {
        return SPARKLINE_BLOCKS[3].to_string().repeat(values.len());
    }
    values
        .iter()
        .map(|v| {
            let scaled = (v - min) / (max - min);
            let idx = (scaled * (SPARKLINE_BLOCKS.len() - 1) as f64).round() as usize;
            SPARKLINE_BLOCKS[idx.min(SPARKLINE_BLOCKS.len() - 1)]
        })
        .collect()
}

#[component]
pub fn Sparkline(values: Vec<f64>) -> impl IntoView {
    let text = sparkline(&values);
    view! { <span class="sparkline">{text}</span> }
}

pub fn form_cell(place: Option<i32>) -> (String, &'static str) {
    match place {
        Some(1) => ("1".to_string(), "form-gold"),
        Some(2) => ("2".to_string(), "form-silver"),
        Some(3) => ("3".to_string(), "form-bronze"),
        Some(p) => (p.to_string(), "form-other"),
        None => ("-".to_string(), "form-none"),
    }
}

#[component]
pub fn FormStrip(results: Vec<FormResult>) -> impl IntoView {
    view! {
        <span class="form-strip" title="recent form (oldest to newest)">
            {results
                .into_iter()
                .map(|r| {
                    let (label, class) = form_cell(r.place);
                    view! { <span class=class>{label}</span> }
                })
                .collect_view()}
        </span>
    }
}

/// Maps rating index/value pairs onto SVG coordinates within the given
/// padded viewport. Flat or single-point series centre vertically; a single
/// point also centres horizontally.
pub fn chart_coords(ratings: &[i32], width: f64, height: f64, pad: f64) -> Vec<(f64, f64)> {
    if ratings.is_empty() {
        return Vec::new();
    }
    let mid_y = height / 2.0;
    if ratings.len() == 1 {
        return vec![(width / 2.0, mid_y)];
    }
    let min = ratings.iter().cloned().min().unwrap_or_default();
    let max = ratings.iter().cloned().max().unwrap_or_default();
    let x_span = width - 2.0 * pad;
    let y_span = height - 2.0 * pad;
    ratings
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let x = pad + x_span * (i as f64 / (ratings.len() - 1) as f64);
            let y = if max > min {
                pad + y_span * (1.0 - (r - min) as f64 / (max - min) as f64)
            } else {
                mid_y
            };
            (x, y)
        })
        .collect()
}

const CHART_WIDTH: f64 = 320.0;
const CHART_HEIGHT: f64 = 120.0;
const CHART_PAD: f64 = 16.0;

#[component]
pub fn RatingChart(points: Vec<RatingPoint>) -> impl IntoView {
    let ratings: Vec<i32> = points.iter().map(|p| p.rating).collect();
    let coords = chart_coords(&ratings, CHART_WIDTH, CHART_HEIGHT, CHART_PAD);
    let polyline_points = coords
        .iter()
        .map(|(x, y)| format!("{:.1},{:.1}", x, y))
        .collect::<Vec<_>>()
        .join(" ");

    let min = ratings.iter().cloned().min();
    let max = ratings.iter().cloned().max();

    let circles = coords
        .iter()
        .zip(points.iter())
        .map(|((x, y), p)| {
            let title = format!("{} - {}", p.finished_at.date(), p.rating);
            view! {
                <circle class="rating-point" cx=format!("{:.1}", x) cy=format!("{:.1}", y) r="3">
                    <title>{title}</title>
                </circle>
            }
        })
        .collect_view();

    view! {
        <svg class="rating-chart" viewBox="0 0 320 120" role="img">
            <line
                class="chart-axis"
                x1=format!("{:.1}", CHART_PAD)
                y1=format!("{:.1}", CHART_HEIGHT - CHART_PAD)
                x2=format!("{:.1}", CHART_WIDTH - CHART_PAD)
                y2=format!("{:.1}", CHART_HEIGHT - CHART_PAD)
            ></line>
            <polyline fill="none" class="rating-line" points=polyline_points></polyline>
            {circles}
            {max.map(|m| view! {
                <text class="chart-label" x="0" y=format!("{:.1}", CHART_PAD)>{m.to_string()}</text>
            })}
            {min.map(|m| view! {
                <text class="chart-label" x="0" y=format!("{:.1}", CHART_HEIGHT - CHART_PAD)>{m.to_string()}</text>
            })}
        </svg>
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBucket {
    pub label: String,
    pub count: i64,
}

/// Maps bucket counts to bar heights, proportional to the series max.
/// All-zero or empty input yields all-zero (or empty) heights.
pub fn bar_heights(counts: &[i64], max_height: f64) -> Vec<f64> {
    let max = counts.iter().cloned().max().unwrap_or(0);
    if max <= 0 {
        return vec![0.0; counts.len()];
    }
    counts
        .iter()
        .map(|&c| max_height * (c as f64 / max as f64))
        .collect()
}

const HIST_WIDTH: f64 = 320.0;
const HIST_HEIGHT: f64 = 120.0;
const HIST_PAD: f64 = 16.0;

#[component]
pub fn Histogram(buckets: Vec<HistogramBucket>) -> impl IntoView {
    let counts: Vec<i64> = buckets.iter().map(|b| b.count).collect();
    let max_bar_height = HIST_HEIGHT - 2.0 * HIST_PAD;
    let heights = bar_heights(&counts, max_bar_height);

    let n = buckets.len().max(1);
    let plot_width = HIST_WIDTH - 2.0 * HIST_PAD;
    let bar_slot = plot_width / n as f64;
    let bar_width = bar_slot * 0.6;

    let bars = buckets
        .into_iter()
        .zip(heights.iter())
        .enumerate()
        .map(|(i, (bucket, &h))| {
            let x = HIST_PAD + bar_slot * i as f64 + (bar_slot - bar_width) / 2.0;
            let y = HIST_HEIGHT - HIST_PAD - h;
            let title = format!("{}: {}", bucket.label, bucket.count);
            view! {
                <rect
                    class="histogram-bar"
                    x=format!("{:.1}", x)
                    y=format!("{:.1}", y)
                    width=format!("{:.1}", bar_width)
                    height=format!("{:.1}", h)
                >
                    <title>{title}</title>
                </rect>
                <text
                    class="chart-label"
                    x=format!("{:.1}", x + bar_width / 2.0)
                    y=format!("{:.1}", HIST_HEIGHT - HIST_PAD + 10.0)
                >{bucket.label}</text>
            }
        })
        .collect_view();

    view! {
        <svg class="histogram" viewBox="0 0 320 120">
            <line
                class="chart-axis"
                x1=format!("{:.1}", HIST_PAD)
                y1=format!("{:.1}", HIST_HEIGHT - HIST_PAD)
                x2=format!("{:.1}", HIST_WIDTH - HIST_PAD)
                y2=format!("{:.1}", HIST_HEIGHT - HIST_PAD)
            ></line>
            {bars}
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_empty_is_empty_string() {
        assert_eq!(sparkline(&[]), "");
    }

    #[test]
    fn sparkline_single_value_is_middle_block() {
        assert_eq!(sparkline(&[5.0]), "\u{2584}");
    }

    #[test]
    fn sparkline_flat_series_is_all_middle_block() {
        assert_eq!(sparkline(&[3.0, 3.0, 3.0]), "\u{2584}\u{2584}\u{2584}");
    }

    #[test]
    fn sparkline_ascending_hits_min_and_max_blocks() {
        let s = sparkline(&[0.0, 1.0, 2.0, 3.0]);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars.len(), 4);
        assert_eq!(chars[0], '\u{2581}');
        assert_eq!(chars[3], '\u{2588}');
    }

    #[test]
    fn chart_coords_two_points_span_padded_edges() {
        let coords = chart_coords(&[0, 10], 100.0, 100.0, 10.0);
        assert_eq!(coords.len(), 2);
        assert_eq!(coords[0].0, 10.0);
        assert_eq!(coords[1].0, 90.0);
    }

    #[test]
    fn chart_coords_min_max_map_to_bottom_top() {
        let coords = chart_coords(&[5, 15], 100.0, 100.0, 10.0);
        // rating 5 is min -> bottom (y = height - pad); rating 15 is max -> top (y = pad)
        assert_eq!(coords[0].1, 90.0);
        assert_eq!(coords[1].1, 10.0);
    }

    #[test]
    fn chart_coords_flat_series_is_vertically_centered() {
        let coords = chart_coords(&[7, 7, 7], 100.0, 100.0, 10.0);
        for (_, y) in &coords {
            assert_eq!(*y, 50.0);
        }
    }

    #[test]
    fn chart_coords_single_point_is_centered() {
        let coords = chart_coords(&[42], 100.0, 100.0, 10.0);
        assert_eq!(coords, vec![(50.0, 50.0)]);
    }

    #[test]
    fn chart_coords_empty_is_empty() {
        assert_eq!(chart_coords(&[], 100.0, 100.0, 10.0), Vec::new());
    }

    #[test]
    fn bar_heights_proportional_to_max() {
        let heights = bar_heights(&[1, 2, 4], 100.0);
        assert_eq!(heights, vec![25.0, 50.0, 100.0]);
    }

    #[test]
    fn bar_heights_all_zero() {
        assert_eq!(bar_heights(&[0, 0, 0], 100.0), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn bar_heights_empty() {
        assert_eq!(bar_heights(&[], 100.0), Vec::<f64>::new());
    }

    #[test]
    fn form_cell_first_is_gold() {
        assert_eq!(form_cell(Some(1)), ("1".to_string(), "form-gold"));
    }

    #[test]
    fn form_cell_second_is_silver() {
        assert_eq!(form_cell(Some(2)), ("2".to_string(), "form-silver"));
    }

    #[test]
    fn form_cell_third_is_bronze() {
        assert_eq!(form_cell(Some(3)), ("3".to_string(), "form-bronze"));
    }

    #[test]
    fn form_cell_fourth_and_beyond_is_other() {
        assert_eq!(form_cell(Some(4)), ("4".to_string(), "form-other"));
    }

    #[test]
    fn form_cell_none_is_dash() {
        assert_eq!(form_cell(None), ("-".to_string(), "form-none"));
    }
}
