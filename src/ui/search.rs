///! Widget `Search`
use crate::utils::*;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Widget},
    layout::{Rect},
    text::{Span, Spans},
    buffer::{Buffer}
};


/// Widget Search
pub struct Search<'a> {
    symbol_width: usize,    // width of longest symbol in info
    infos: &'a Vec<Info>,
    pub ref_i_symbol: Rc<RefCell<usize>>,    // index of selected symbol in infos (interior mutablity via Rc<RefCell<_>>)
    pub ref_cursor: Rc<RefCell<(u16, u16)>>,    // cursor position ix and iy (interior mutablity via Rc<RefCell<_>>)
}

impl<'a> Search<'a> {
    pub fn new(infos: &'a Vec<Info>, ref_i_symbol: Rc<RefCell<usize>>, 
               ref_cursor: Rc<RefCell<(u16,u16)>>) -> Search<'a> {
        let width = infos.iter().map(|info| info.symbol.len()).max().unwrap_or(8);
        Search { 
            symbol_width: width,
            infos: infos,
            ref_i_symbol: ref_i_symbol,
            ref_cursor: ref_cursor,
        }
    }
}

impl<'a> Widget for Search<'a> {
    fn render(self: Self, area: Rect, buf: &mut Buffer) {
        let mut x: u16 = 0;   // x position in area
        let mut y: u16 = 0;   // y position in area
        let mut ix: u16 = 0;  // x position in table of symbols
        let mut iy: u16 = 0;  // y position in table of symbols (always same as variable `y`, actually)
        let mut cursor: RefMut<(u16, u16)> = self.ref_cursor.borrow_mut();
        let height = area.height as u16;
        let width = area.width as u16;
        let symbol_width = self.symbol_width as u16;
        for (i_symbol, info) in self.infos.iter().enumerate() {
            if y >= height { 
                x += symbol_width + 1; 
                if x >= width { break; }
                y = 0; 
                ix += 1;
                iy = 0;
            }
            let style: tui::style::Style = if ix==cursor.0 && iy==cursor.1 {
                *self.ref_i_symbol.borrow_mut() = i_symbol;
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD).add_modifier(Modifier::ITALIC)
            } else {
                Style::default()
            };
            let span = Span::styled(format!("{}", info.symbol), style);
            let spans = Spans::from(vec![span]);
            buf.set_spans(x, y as u16, &spans, symbol_width);
            y += 1;
            iy += 1;
        }
        if cursor.0 > ix {    // don't allow increasing cursor's ix beyond ix at end of for loop
            cursor.0 = ix;
        }
        if cursor.1 >= height {    // don't allow increasing cursor's iy beyond height
            cursor.1 = height - 1;
        }
    }
}
