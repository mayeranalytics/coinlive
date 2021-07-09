/// The help page
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Block, Borders, Paragraph},
    text::{Span, Spans},
};

pub fn help<'a>() -> Paragraph<'a> {
    let help: Vec<(&str, &str)> = vec!
    [ ("h",    "Display help")
    , ("l",    "Show price list")
    , ("t",    "Show price table")
    , ("g",    "Show graph at current time scale")
    , ("0..9", "Show graph at time scale 0 to 9 (1m to 1d)")
    , ("s",    "Select symbol")
    , ("Home", "Set cursor to top left symbol (select symbol page)")
    , ("%",    "Toggle percent/price display")
    , ("x",    "Toggle extended/reduced view (Table display)")
    , ("a",    "Display about page")
    , ("Esc",  "Go back to previous view")
    , ("q",    "Quit")
    , ("C-c",  "Quit")
    ];
    let char_style = Style::default().add_modifier(Modifier::ITALIC).bg(Color::White).fg(Color::Black);
    let width: usize = help.iter().map(|tup| tup.0.len()).max().unwrap_or(4);
    let text: Vec<tui::text::Spans> = help.iter().map(|(k, txt)| {
        Spans::from(vec![
            Span::styled(format!(" {:<width$} ", k, width=width),char_style),
            Span::raw(format!("  {}", *txt)),
        ])
    }).collect();
    Paragraph::new(text)
        .block(Block::default().title("Help").borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black))
}
