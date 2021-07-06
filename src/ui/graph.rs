///! Widget `Graph`
use crate::utils::*;
use crate::ui::nice::{f64_nice_range, Nice};
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Axis, Chart, Widget, Block, Dataset, GraphType, Paragraph},
    layout::{Rect},
    text::{Span},
    buffer::{Buffer},
    symbols
};
use chrono::{Utc, prelude::DateTime};
use std::time::{UNIX_EPOCH, Duration};
use inlinable_string::InlineString;


/// Widget Graph
/// 
/// Shows a time/closing-price graph of a symbol.
pub struct Graph<'a> {
    symbol: Symbol,
    infos: &'a Vec<Info>,   // sorted list of `Info`
    klines: &'a Vec<Bar>,
    interval: Interval,     // 1m, 3m, 5m, etc.
}

impl<'a> Graph<'a> {
    pub fn new(infos: &'a Vec<Info>, klines: &'a Vec<Bar>, interval: Interval, symbol: Symbol) -> Graph<'a> {
        Graph { symbol: symbol, infos: infos, klines: klines, interval: interval }
    }
}

impl<'a> Widget for Graph<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.klines.len() < 1 {
            Paragraph::new("No data!")
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .style(Style::default().fg(Color::White))
                    .title("Error")
            ).render(area, buf);
            return;
        }
        let mut t_min: f64 = f64::MAX;
        let mut t_max: f64 = 0.0;
        let mut p_min: f64 = f64::MAX;
        let mut p_max: f64 = 0.0;
        // we want to show high and low only. This gives a fuzzier, less crisp graph. 
        // The advantage is, obviously, that high and low become visible.
        let mut data: Vec<(f64,f64)> = Vec::with_capacity(self.klines.len()*2+1); // two values per ohlc bar plus the first open
        let delta = (self.klines[1].t - self.klines[0].t) as f64;
        data.push((self.klines[0].t as f64, self.klines[0].o as f64));
        for bar in self.klines.iter() {
            let t_o = bar.t as  f64;    // because bar.t is timestamp of open
            let t_c = t_o + delta;
            let (o,h,l,c) = (bar.o as f64, bar.h as f64, bar.l as f64, bar.c as f64);
            // push to data
            if c >= o { // first high then low
                data.push((t_o + delta/2.0, h));
                data.push((t_c, l));
            } else {    // first low then high
                data.push((t_o + delta/2.0, l));
                data.push((t_c, h));
            }
            // update the extrema
            if h > p_max { p_max = h; }
            if l < p_min { p_min = l; }
            if t_c > t_max { t_max = t_c; }
            if t_o < t_min { t_min = t_o; }

        }
        let (p_min, p_max) = f64_nice_range(p_min, p_max);
        let datasets = vec![
            Dataset::default()
                //.name(self.symbol.unwrap_or(&default_name))
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(data.as_slice())
        ];
        let t1 = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(t_min as u64));
        let t2 = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(((t_min+t_max)/2.0) as u64));
        let t3 = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(t_max as u64));
        let x_labels = vec![
            Span::styled(
                t1.format("%H:%M").to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(t2.format("%Y-%m-%d %H:%M").to_string()),
            Span::styled(
                t3.format("%Y-%m-%d %H:%M").to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let y_labels = vec![
            Span::styled(
                p_min.compact_str(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(((p_min+p_max)/2.0).compact_str()),
            Span::styled(
                p_max.compact_str(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let title: InlineString = self.symbol;
        let graph = Chart::new(datasets)
            .block(Block::default().title(String::from(&*title)))
            .x_axis(Axis::default()
                .style(Style::default().fg(Color::White))
                .bounds([t_min, t_max])
                .labels(x_labels))
            .y_axis(Axis::default()
                //.title(Span::styled("", Style::default().fg(Color::Red)))
                .style(Style::default().fg(Color::White))
                .bounds([p_min, p_max])
                .labels(y_labels));
        graph.render(area, buf);
    }
}
