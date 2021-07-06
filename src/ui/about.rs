/// The about page
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Block, Borders, Paragraph, BorderType},
    layout::{Layout, Constraint, Direction, Alignment, Rect},
    text::{Span, Spans},
};
use crate::version::VERSION;

pub fn about<'a>() -> Paragraph<'a> {
    let txt = format!(
        "\n\
        coinlive - Live cryptocurrency prices in the CLI
        \n\n\
        (c) Mayer Analytics, GPL-2.0
        \n\
        Source: https://github.com/mayeranalytics/coinlive
        \n\n\
        Version: {}",
        VERSION
    );
    Paragraph::new(txt)
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("About")
                .border_type(BorderType::Plain),
        )
}
