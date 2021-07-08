/// The about page
use tui::{
    style::{Style, Color, Modifier},
    widgets::{Block, Borders, Paragraph, BorderType},
    layout::{Alignment},
    text::{Span, Spans},
};
use crate::version::VERSION;
  
pub fn about<'a>() -> Paragraph<'a> {
    let txt = vec![
        Spans::from(Span::styled("     ####    #####    ######  ##   ##  ####      ######  ##   ##  #######  ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("    ##  ##  ### ###     ##    ###  ##   ##         ##    ##   ##   ##   #  ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("   ##       ##   ##     ##    #### ##   ##         ##    ##   ##   ##      ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("   ##       ##   ##     ##    #######   ##         ##     ## ##    ####    ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("   ##       ##   ##     ##    ## ####   ##         ##     ## ##    ##      ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("    ##  ##  ### ###     ##    ##  ###   ##  ##     ##      ###     ##   #  ", Style::default().fg(Color::LightCyan))),
        Spans::from(Span::styled("     ####    #####    ######  ##   ##  #######   ######    ###    #######  ", Style::default().fg(Color::LightCyan))),
        Spans::from(Vec::new()),
        Spans::from(Vec::new()),
        Spans::from(Span::styled("Live cryptocurrency prices in the CLI",
                                 Style::default().fg(Color::LightCyan).add_modifier(Modifier::ITALIC))),
        Spans::from(Vec::new()),
        Spans::from(Vec::new()),
        Spans::from(Span::styled("(c) Mayer Analytics, GPL-3.0", Style::default().fg(Color::Red))),
        Spans::from(Vec::new()),
        Spans::from(Span::styled("https://github.com/mayeranalytics/coinlive", Style::default().fg(Color::Green))),
        Spans::from(Vec::new()),
        Spans::from(Span::styled(format!("Version {}", VERSION), Style::default().fg(Color::Gray))),
    ];
    Paragraph::new(txt)
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Gray).bg(Color::Black))
                .title("About")
                .border_type(BorderType::Plain),
        )
}
