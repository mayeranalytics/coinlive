///! Widget `PriceList`
use crate::utils::*;
use crate::ui::MarketState;
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Widget},
    layout::Rect,
    text::{Span, Spans},
    buffer::{Buffer}
};
use std::collections::HashMap;

/// Widget PriceList
pub struct PriceTable<'a> {
    infos: &'a Vec<Info>,                       // sorted list of `Info`
    markets: &'a HashMap<Symbol, MarketState>,  // map symbol to `MarketState`
    show_percent: bool,                         // flag indicating whether % change should be shown
    extended: bool,                             // flag indicating extended view vs. reduced
    quotes: Vec<Symbol>,
    bases: Vec<Symbol>,
}

impl<'a> PriceTable<'a> {
    pub fn new(infos: &'a Vec<Info>, markets: &'a HashMap<Symbol, MarketState>, 
               show_percent: bool, extended: bool) -> PriceTable<'a> {
        let (bases ,quotes) = sort_base_quote(&infos);
        PriceTable {infos: infos, markets: markets, show_percent: show_percent, extended: extended, 
                    quotes: quotes, bases: bases }
    }
    fn render_info(self: &Self, info: &Info, width: usize) -> Spans<'a> {
        let grey = Style::default().fg(Color::Gray);
        let mkt = self.markets.get(&info.symbol);
        let symbol_span = Span::styled(format!("{:<width$} ", info.short_symbol(), width=width), 
                                        Style::default().add_modifier(Modifier::BOLD));
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

impl<'a> Widget for PriceTable<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_width = if self.show_percent {6} else {8};
        let mut x: u16 = 0;
        let height = area.height as usize;
        let mut counter: usize = 0;
        loop {
            if x+col_width+2 > area.width { break; }
            if counter >= self.bases.len() { break; }
            let bases = &self.bases[counter..(counter+height-1).min(self.bases.len())];
            counter += bases.len();
            // draw vertical header with base strings
            for (y,base) in bases.iter().enumerate() {
                let span = Span::styled(String::from(&**base), Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC));
                buf.set_spans(x, y as u16+1, &Spans::from(vec![span]), base.len() as u16);
            }
            x += col_width + 2;
            let quotes = if self.extended {
                vec!["USDT", "BTC", "EUR", "GBP", "BNB", "ETH"]     // extended view
            } else {
                vec!["USDT", "BTC", "BNB", "ETH"]                   // reduced view
            };
            // columns
            for quote in quotes.iter() {
                // header
                let span = Span::styled(*quote, Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC));
                buf.set_spans(x, 0, &Spans::from(vec![span]), quote.len() as u16);
                // prices
                for (y,base) in bases.iter().enumerate() {
                    let mut symbol = base.clone();
                    symbol.push_str(quote).unwrap(); // this should really be ok! If not, something weird is happening
                    if let Some(mkt) = self.markets.get(&symbol) {
                        if self.show_percent {
                            let percentage = mkt.percentage_string();
                            let perc_len = percentage.len() as u16;
                            if x+perc_len < area.width {
                                let percentage_span = Span::styled(percentage, mkt.style_percent());
                                let spans = Spans::from(vec![percentage_span]);
                                buf.set_spans(x, y as u16+1, &spans, perc_len);
                            }
                        } else {
                            let price = mkt.price_string();
                            let price_len = price.len() as u16;
                            if x+price_len < area.width {
                                let price_span = Span::styled(price, mkt.style());
                                let spans = Spans::from(vec![price_span]);
                                buf.set_spans(x, y as u16+1, &spans, price_len);
                            }
                        }
                    }
                }
                x += col_width+2;
                if x >= area.width { break; }
            }
            x += 3;
            if x >= area.width { break; }
        }
    }
}
