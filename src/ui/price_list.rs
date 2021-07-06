///! Widget `PriceList`
use crate::utils::*;
use crate::ui::MarketState;
use std::io;
use std::io::{Read, Write};
use termion::raw::IntoRawMode;
use tui::{Terminal};
use tui::{
    style::{Style, Color, Modifier},
    backend::TermionBackend,
    widgets::{Widget, Block, Borders, Paragraph, ListItem, List, BorderType},
    layout::{Layout, Constraint, Direction, Alignment, Rect},
    text::{Span, Spans, Text},
    buffer::{Buffer, Cell}
};
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use tokio::{select, signal, sync::oneshot};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time;
use tokio_stdin_stdout::stdin;
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures_util::{future, pin_mut, StreamExt};
use url::Url;
use std::collections::HashMap;
use chrono::{Local, format::strftime};

/// Widget PriceList
pub struct PriceList<'a> {
    infos: &'a Vec<Info>,                       // sorted list of `Info`
    markets: &'a HashMap<Symbol, MarketState>,  // map symbol to `MarketState`
    show_percent: bool                          // flag indicating whether % change should be shown
}

impl<'a> PriceList<'a> {
    pub fn new(infos: &'a Vec<Info>, markets: &'a HashMap<Symbol, MarketState>, show_percent: bool) -> PriceList<'a> {
        PriceList {infos: infos, markets: markets, show_percent: show_percent }
    }
    fn render_info(self: &Self, info: &Info, width: usize) -> Spans<'a> {
        let grey = Style::default().fg(Color::Gray);
        let mkt = self.markets.get(&info.symbol);
        let mut symbol = info.short_symbol().clone();
        while symbol.len() < width { symbol.push(' ').unwrap_or(()); } // format! with {:<width$} does not work!
        let symbol_span = Span::styled(format!("{} ",symbol), 
                                        Style::default().add_modifier(Modifier::BOLD)
                                                        .add_modifier(Modifier::ITALIC));
        if self.show_percent {
            let percentage = mkt.map(|s| String::from(" ")+&s.percentage_string()).unwrap_or(String::from("-"));
            let percentage_span = Span::styled(percentage, mkt.map(|m| m.style_percent()).unwrap_or(grey));
            Spans::from(vec![symbol_span, percentage_span])
        } else {
                let px = mkt.map(|s| s.price_string()).unwrap_or(String::from("-"));
                let price_span = Span::styled(px, mkt.map(|m| m.style()).unwrap_or(grey));
                Spans::from(vec![symbol_span, price_span])
        }
    }
    fn render_infos(self: &Self, infos: &'a [Info]) -> (usize, Vec<Spans>) {
        let width: usize = infos.iter().map(|i| i.short_symbol().len()).max().unwrap_or(0).max(8);
        let spans = infos.iter().map(|info| self.render_info(info, width)).collect::<Vec<Spans>>();
        let width = spans.iter().map(|t| t.width()).max().unwrap_or(0);
        (width, spans)
    }
}

impl<'a> Widget for PriceList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut x: u16 = 0;
        let mut counter: usize = 0;
        let height = area.height as usize;
        while counter < self.infos.len() {
            let (width, spanss) = self.render_infos(&self.infos[counter..(counter+height).min(self.infos.len())]);
            if x + width as u16 >= area.width { break; }
            for (y, spans) in spanss.iter().enumerate() {
                buf.set_spans(x, y as u16, spans, width as u16);
            }
            x += width as u16 + 4;
            counter += spanss.len();
        }
    }
}
