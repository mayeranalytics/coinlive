//! The UI is made of different pages

/// The help page
pub mod help;
/// The about page
pub mod about;
/// The price list page
pub mod price_list;
/// The price table page
pub mod price_table;
/// The graph page
pub mod graph;
/// The search page
pub mod search;
/// Pretty printing of floats and Decimal
pub mod nice;

use crate::utils::*;
use std::io;
use std::cell::RefCell;
use std::rc::Rc;
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
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::collections::HashMap;
use chrono::Local;
use std::marker::Copy;
use dec::Decimal64;
use inlinable_string::{InlineString, StringExt};

/// Stores the relevant market data with some extra rendering information
pub struct MarketState {
    px: Decimal64,
    ts: u64,
    last_px: Decimal64,
    px_24h: Decimal64,
    precision: Precision    // ! do we need this?
}

impl MarketState {
    /// Create new `MarketState` with NANs.
    fn new(info: &Info) -> Self {
        MarketState { px: Decimal64::NAN, ts: 0, last_px: Decimal64::NAN, px_24h:Decimal64::NAN, precision: 8 }
    }
    /// Update `MarketState` with data from `Update`
    fn update(self: &mut Self, update: &Update) {
        self.last_px = self.px;
        self.px = update.px;
        self.px_24h = update.px_24h;
        self.ts = update.ts;
    }
    /// Make a price string that has 'precision' length
    pub fn price_string(self: &Self) -> String {
        fmt_dec(self.px)
    }
    /// Make a percentage string that has 6 width !TODO! improve
    pub fn percentage_string(self: &Self) -> String {
        let hundred: Decimal64 = "100".parse().expect("INTERNAL ERROR");
        let p = (self.last_px-self.px_24h)/self.px_24h;
        let mut s = if p.is_infinite() || p.is_nan() {
            String::from("-")
        } else {
            format!("{}", p*hundred)
        };
        if p.is_positive() && !(p.is_infinite() || p.is_nan()) { 
            s.insert_str(0,"+");
        } 
        s.truncate(6);
        format!("{:>6}", s)
    } 
    /// Generate a style for this price
    pub fn style(self: &Self) -> Style {
        if self.px > self.last_px {
            Style::default().fg(Color::Green)
        } else if self.px < self.last_px {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    }
    /// Generate a style for this percentage
    pub fn style_percent(self: &Self) -> Style {
        if self.px > self.px_24h {
            Style::default().fg(Color::Green)
        } else if self.px < self.px_24h {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    }
}

/// Messages that the `UI` can receive
#[derive(Debug)]
pub enum Msg {
    WS(u64, String),    // timestamp (millis) and websocket data
    Infos(Vec<Info>),   // Downloaded infos for each symbol
    Msg(String),        // info message to UI
    PriceList,          // On 'l' key press show PriceList
    PriceTable,         // On 't' key press show PriceTable
    Graph(Option<u32>), // On 'g' display graph with given time scale, or stored time scale if Nothing
    TogglePercent,      // On '%' key press
    ToggleExtended,     // On 'x' key press
    Search,             // On 's' show the search widget
    ArrowUp,            // On arrow up
    ArrowDown,          // On arrow down
    ArrowLeft,          // On srrow left
    ArrowRight,         // On arrow right
    Home,               // Home Home key reset cursor to top left
    Enter,              // On pressing enter
    Help,               // On 'h' key press show help
    About,              // On 'a' key press show about page
    Esc,                // On ESC go back to previous page
    Stop                // stop ui
}

/// Just tui::Terminal<...>
type Term = tui::Terminal<tui::backend::TermionBackend<termion::raw::RawTerminal<std::io::Stdout>>>;

/// All the different pages
#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum UIView {
    PriceList,  // display PriceList
    PriceTable, // display PriceTable
    Graph,      // display graph
    Search,     // display search widget
    Empty,      // display PriceTable
    Help,       // display help
    About,      // display help
}

impl Copy for UIView { }


/// Current state of the `UI`
pub struct UIState {
    message: String,
    markets: HashMap<Symbol, MarketState>,
    latency: u64,
    ui_mode: UIView,
    ui_mode_back: Option<UIView>,       // where to go back to if ESC is pressed
    show_percent: bool,                 // 
    extended: bool,                     // extended view of table page
    ts_last_update: u64,                // ts of last market update
    lookup: Option<HashMap<Symbol, Info>>,
    infos: Option<Vec<Info>>,
    klines: Option<Vec<Bar>>,
    symbol: Symbol,
    time_scale: u32,                    // time scale for graph
    cursor_ix: u16,                     // x position of symbol in search widget
    cursor_iy: u16,                     // y position of symbol in search widget
}

impl UIState {
    /// New `UIState` with empty fields, 0 latency, ui_mode `PriceList`
    fn new() -> Self {
        UIState { 
            message: String::new(), 
            markets: HashMap::new(),
            latency: 0,
            ui_mode: UIView::Empty,
            ui_mode_back: None,
            show_percent: false,
            extended: true,
            ts_last_update: 0,
            lookup: None,
            infos: None,
            klines: None,
            symbol: InlineString::from("BTCUSDT"),
            time_scale: 0,
            cursor_ix: 0,
            cursor_iy: 0,
        }
    }
    fn update(self: &mut Self, updates: &Vec<Update>) {
        if let Some(lookup) = &self.lookup {
            for u in updates {
                if u.ts > self.ts_last_update { self.ts_last_update = u.ts; }
                let info = lookup.get(&u.symbol);
                if let Some(info) = info {
                    self.markets.entry(u.symbol.clone()).or_insert(MarketState::new(&info)).update(&u);
                }
            }
        }
    }
}
/// Encapsulates the `UI`
pub struct UI {
    pub tx: Sender<Msg>,
    pub handle: tokio::task::JoinHandle<()>,
}

impl UI {
    /// Create new `UI`
    pub fn new(mut terminal: Term) -> Self {
        terminal.clear().expect("Terminal failed!");
        let (tx, mut rx) = channel(64);
        let handle = tokio::spawn( async move {
            let mut state = UIState::new();
            let mut buf: Vec<Update> = Vec::with_capacity(2000);    // buffer for parse_updates
            let mut cursor_moved: bool = false;                     // used for setting message after draw is done
            while let Some(msg) = rx.recv().await {
                match msg {
                    Msg::Infos(infos_) => {
                        state.infos = Some(infos_.iter().cloned().filter(|i| i.quote != "TUSD" && i.quote != "BUSD" && i.quote != "USDC").collect());
                        state.lookup = Some(infos_to_lookup(&infos_));
                        state.ui_mode = UIView::PriceList;
                    },
                    Msg::WS(ts_rec, msg) => {
                        if let Ok(us) = parse_updates(&msg, &mut buf) {
                            state.update(&us);
                        } else if let Ok(ts) = msg.parse::<u64>() {
                            state.latency = ts_rec-ts;
                        } else {
                            state.message = format!("{:?}", msg);
                            break;
                        }
                    },
                    Msg::Msg(msg) => {
                        state.message = msg;
                    },
                    Msg::PriceList => {
                        state.ui_mode = UIView::PriceList;
                        state.message = String::from("Show price list");
                    },
                    Msg::PriceTable => {
                        state.ui_mode = UIView::PriceTable;
                        state.message = String::from("Show price table");
                    },
                    Msg::Graph(scale) => {
                        state.time_scale = scale.unwrap_or(state.time_scale);
                        UI::graph(&mut state, &mut terminal).await;
                    },
                    Msg::Search => {
                        state.ui_mode_back = Some(state.ui_mode);
                        state.ui_mode = UIView::Search;
                        state.message = String::from("Select symbol");
                    },
                    Msg::ArrowUp => {
                        if state.ui_mode == UIView::Search {
                            if state.cursor_iy > 0 { 
                                state.cursor_iy -= 1;
                                cursor_moved = true;
                            }
                        }
                    },
                    Msg::ArrowDown => {
                        if state.ui_mode == UIView::Search {
                            state.cursor_iy += 1;   // ! height needs to be checked elsewhere!
                            cursor_moved = true;
                        }
                    },
                    Msg::ArrowLeft => {
                        if state.ui_mode == UIView::Search {
                            if state.cursor_ix > 0 { 
                                state.cursor_ix -= 1;
                                cursor_moved = true;
                            }
                        }
                    },
                    Msg::ArrowRight => {
                        if state.ui_mode == UIView::Search {
                            state.cursor_ix += 1;   // ! width needs to be checked elsewhere!
                            cursor_moved = true;
                        }
                    },
                    Msg::Home => {
                        if state.ui_mode == UIView::Search {
                            state.cursor_ix = 0;
                            state.cursor_iy = 0;
                            cursor_moved = true;
                        }
                    },
                    Msg::Enter => {
                        if state.ui_mode == UIView::Search {
                            state.message = format!("Graph {}", state.symbol);
                            state.ui_mode_back = Some(state.ui_mode);
                            state.ui_mode = UIView::Graph;
                            UI::graph(&mut state, &mut terminal).await;
                        }
                    },
                    Msg::TogglePercent => {
                        state.show_percent = !state.show_percent;
                        if state.show_percent { state.message = String::from("Show %"); }
                        else { state.message = String::from("Hide %"); }
                    },
                    Msg::ToggleExtended => {
                        state.extended = !state.extended;
                        if state.extended { state.message = String::from("Show extended"); }
                        else { state.message = String::from("Show reduced"); }
                    },
                    Msg::Help => {
                        state.ui_mode_back = Some(state.ui_mode);
                        state.ui_mode = UIView::Help;
                        state.message = String::from("Help");
                    },
                    Msg::About => {
                        state.ui_mode_back = Some(state.ui_mode);
                        state.ui_mode = UIView::About;
                        state.message = String::from("About");
                    },
                    Msg::Esc => {
                        state.ui_mode = state.ui_mode_back.unwrap_or(UIView::PriceList);
                        state.ui_mode_back = None;
                        state.message.clear();
                    },
                    Msg::Stop => { 
                        state.message = String::from("Stop");
                        UI::draw(&mut state, &mut terminal);
                        return; 
                    }
                }
                UI::draw(&mut state, &mut terminal); 
                if cursor_moved {
                    state.message = format!("SEL {}", state.symbol);
                    cursor_moved = false;
                }
            }
        });
        UI { tx: tx, handle: handle }
    }
    /// Draw Graph
    pub async fn graph(mut state: &mut UIState, mut terminal: &mut Term) {
        let interval: Interval = match state.time_scale {
            1 => Interval::I5m,
            2 => Interval::I15m,
            3 => Interval::I30m,
            4 => Interval::I1h,
            5 => Interval::I2h,
            6 => Interval::I4h,
            7 => Interval::I8h,
            8 => Interval::I12h,
            9 => Interval::I1d,
            _ => Interval::I1m,
        };
        state.message = format!("Getting {} klines for {}", interval.str(), state.symbol);
        UI::draw(&mut state, &mut terminal);
        match get_klines(&state.symbol, &interval).await {
            Ok(klines) => {
                state.ui_mode = UIView::Graph;
                state.message = format!("Show {} klines for {}", interval.str(), state.symbol);
                state.klines = Some(klines);
            },
            Err(e) => {
                state.message = format!("Failed to get klines: {:?}", e);
            }
        }
    }
    /// Draw `UI`
    fn draw(state: &mut UIState, terminal: &mut Term) {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Length(size.height-1),
                        Constraint::Length(1),
                    ].as_ref()
                )
                .split(size);
            match state.ui_mode {
                UIView::PriceList => {
                    if let Some(infos) = &state.infos {
                        let price_list = price_list::PriceList::new(&infos, &state.markets, state.show_percent);
                        f.render_widget(price_list, chunks[0]);
                    }
                },
                UIView::PriceTable => {
                    if let Some(infos) = &state.infos {
                        let price_table = price_table::PriceTable::new(&infos, &state.markets, state.show_percent, state.extended);
                        f.render_widget(price_table, chunks[0]);
                    }
                },
                UIView::Graph => {
                    if let Some(infos) = &mut state.infos {
                        if let Some(klines) = &state.klines {
                            let graph = graph::Graph::new(&infos, klines, Interval::I1m, state.symbol.clone());
                            f.render_widget(graph, chunks[0]);
                        }
                    }
                },
                UIView::Search => {
                    // The `Search` object needs to be able to modify i_symbol and cursor (ix, iy), so
                    // we use interior mutability via Rc<RefCell<...>>.
                    // - i_symbol is the index of the selected symbol
                    // - cursor is unchanged unless the display bounds are be exceeded
                    if let Some(infos) = &state.infos {
                        let ref_i_symbol = Rc::new(RefCell::new(0));
                        let ref_cursor = Rc::new(RefCell::new((state.cursor_ix,state.cursor_iy)));
                        let search = search::Search::new(infos, ref_i_symbol.clone(), ref_cursor.clone());
                        f.render_widget(search, chunks[0]);
                        // Now stick cursor (ix, iy) back into state
                        let (ix, iy) = (*ref_cursor).take();
                        state.cursor_ix = ix; state.cursor_iy = iy;
                        // Finally adjust state.symbol if necessary
                        if let Some(infos) = &state.infos {
                            let i_symbol: usize = (*ref_i_symbol).take();
                            if i_symbol < infos.len() { // check bounds just in case
                                let symbol = &infos[i_symbol].symbol;
                                state.symbol = symbol.clone();
                            }
                        }
                    }
                },
                UIView::Empty => {
                    // draw nothing
                }
                UIView::Help => {
                    f.render_widget(help::help(), chunks[0]);
                },
                UIView::About => {
                    f.render_widget(about::about(), chunks[0]);
                }
            }
            let message = Paragraph::new(UI::mk_message_bar(&state, size.width));
            f.render_widget(message, chunks[1]);
        }).expect("Failed to draw!");
    }
    /// Make the message bar at the bottom
    fn mk_message_bar(state: &UIState, width: u16) -> String {
        let width = width as usize;
        let now = Local::now();
        let now_str = format!("{}", now.format("%H:%M:%S"));
        //let lat = if state.latency == 0 {String::from("?ms")} else {format!("{}ms", state.latency)};
        let lat = if state.ts_last_update != 0 {
            format!("{}ms", now.timestamp_millis() as u64-state.ts_last_update)
        } else {
            String::from("?ms")
        };
        let l = now_str.len()+state.message.len()+lat.len()+2;
        if width > l {
            format!("{} {} {:>width$}", now_str, state.message, lat, width=width-l)
        } else {
            let mut s = format!("{} {}", now_str, state.message);
            s.truncate(width);
            s
        }
    }
}