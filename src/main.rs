mod utils;
use crate::utils::*;
mod ui;
mod version;
use crate::version::VERSION;
use crate::ui::*;
use std::io;
use termion::raw::IntoRawMode;
use tui::{Terminal, backend::TermionBackend};
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use tokio_tungstenite::{connect_async};
use tokio::sync::mpsc::{Sender};
use futures_util::{future, StreamExt};
use url::Url;
use clap::{Arg, App};

/// Duration of `sleep` in `listen_keys` loop
const LISTEN_KEYS_SLEEP_MILLIS: u64 = 100;

/// Binance 24h ticker stream endpoint
const URI_WS_TICKER: &str = "wss://stream.binance.com:9443/ws/!ticker@arr";

/// Listen to terminal input.
/// 
/// This is simply an endless loop that reads the terminal input in `LOOP_SPEED` intervals and sends
/// the appropriate message to `tx`.
async fn listen_keys(tx: Sender<Msg>) -> Result<(), String> {
    let mut stdin = termion::async_stdin().keys();
    loop {
        if let Some(Ok(key)) = stdin.next() {
            match key {
                Key::Char('q') => {
                    tx.send(Msg::Stop).await.expect("UI failed");
                    break;
                },
                Key::Ctrl('c') => {
                    tx.send(Msg::Stop).await.expect("UI failed");
                    break;
                },
                Key::Char('l')  => { tx.send(Msg::PriceList).await.expect("UI failed"); },
                Key::Char('t')  => { tx.send(Msg::PriceTable).await.expect("UI failed"); },
                Key::Char('%')  => { tx.send(Msg::TogglePercent).await.expect("UI failed"); },
                Key::Char('x')  => { tx.send(Msg::ToggleExtended).await.expect("UI failed"); },
                Key::Char('s')  => { tx.send(Msg::Search).await.expect("UI failed"); },
                Key::Char('h')  => { tx.send(Msg::Help).await.expect("UI failed"); },
                Key::Char('a')  => { tx.send(Msg::About).await.expect("UI failed"); },
                Key::Char('g')  => { tx.send(Msg::Graph(None)).await.expect("UI failed"); },
                Key::Char('0')  => { tx.send(Msg::Graph(Some(0))).await.expect("UI failed"); },
                Key::Char('1')  => { tx.send(Msg::Graph(Some(1))).await.expect("UI failed"); },
                Key::Char('2')  => { tx.send(Msg::Graph(Some(2))).await.expect("UI failed"); },
                Key::Char('3')  => { tx.send(Msg::Graph(Some(3))).await.expect("UI failed"); },
                Key::Char('4')  => { tx.send(Msg::Graph(Some(4))).await.expect("UI failed"); },
                Key::Char('5')  => { tx.send(Msg::Graph(Some(5))).await.expect("UI failed"); },
                Key::Char('6')  => { tx.send(Msg::Graph(Some(6))).await.expect("UI failed"); },
                Key::Char('7')  => { tx.send(Msg::Graph(Some(7))).await.expect("UI failed"); },
                Key::Char('8')  => { tx.send(Msg::Graph(Some(8))).await.expect("UI failed"); },
                Key::Char('9')  => { tx.send(Msg::Graph(Some(9))).await.expect("UI failed"); },
                Key::Up         => { tx.send(Msg::ArrowUp).await.expect("UI failed"); },
                Key::Down       => { tx.send(Msg::ArrowDown).await.expect("UI failed"); },
                Key::Left       => { tx.send(Msg::ArrowLeft).await.expect("UI failed"); },
                Key::Right      => { tx.send(Msg::ArrowRight).await.expect("UI failed"); },
                Key::Home       => { tx.send(Msg::Home).await.expect("UI failed"); },
                Key::Char('\n') => { tx.send(Msg::Enter).await.expect("UI failed"); },
                Key::Esc        => { tx.send(Msg::Esc).await.expect("UI failed"); },
                key => { 
                    tx.send(Msg::Msg(format!("Unknown command {:?}", key))).await
                      .map_err(|e| format!("UI failed: {:?}", e))?; 
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(LISTEN_KEYS_SLEEP_MILLIS)).await;
    }
    Ok(())
}

/// Websocket stream
async fn ws(uri: &str, ui_tx: Sender<Msg>) -> Result<(), String> {
    let uri: Url = Url::parse(uri).map_err(|e| format!("Bad url: {:?}", e))?;
    let (ws_stream, response) = match connect_async(uri).await {
        Ok((ws_stream, response)) => { (ws_stream, response) },
        Err(e) => { 
            ui_tx.send(Msg::Msg(format!("Error connecting: {:?}", e))).await
                 .map_err(|e| format!("UI failed: {:?}", e))?;
            return Ok(());
        }
    };
    ui_tx.send(Msg::Msg(format!("Websocket connected:\n{:?}", response))).await
         .map_err(|e| format!("UI failed: {:?}", e))?;

    let (_, mut read) = ws_stream.split();

    ui_tx.send(Msg::Msg(String::from("Starting..."))).await.expect("UI failed");
    loop {
        let next = read.next().await;
        let now = now_timestamp();
        match next {
            Some(msg) => {
                match msg {
                    Ok(msg)  => {
                        let msg = msg.to_string();
                        ui_tx.send(Msg::WS(now, msg)).await
                             .map_err(|e| format!("UI failed: {:?}", e))?;
                    }, 
                    Err(e) => {
                        ui_tx.send(Msg::Msg(format!("Error: {:?}", e))).await
                             .map_err(|e| format!("UI failed: {:?}", e))?;
                        return Err(format!("Websocket error: {:?}", e));
                    }
                }
            },
            None => {
                ui_tx.send(Msg::Msg(String::from("Stream end"))).await
                     .map_err(|e| format!("UI failed: {:?}", e))?;
                break;
            }
        }
    }
    Ok(())
}

/// Essentially calls `get_infos`, sorts the `Info` vector and sends the `Msg`s.
async fn get_symbols_async(tx: Sender<Msg>) -> Result<(), String> {
    tx.send(Msg::Msg(String::from("Getting symbols..."))).await.map_err(|e| format!("UI failed: {:?}", e))?;
    if let Ok(infos) = get_infos().await {
        let infos = sort_infos(infos);
        tx.send(Msg::Msg(format!("Got {} symbols", infos.len()))).await.map_err(|e| format!("UI failed: {:?}", e))?;
        tx.send(Msg::Infos(infos)).await.map_err(|e| format!("UI failed: {:?}", e))?;
    } else {
        tx.send(Msg::Msg(String::from("Failed to get symbols"))).await.map_err(|e| format!("UI failed: {:?}", e))?;
        tx.send(Msg::Stop).await.map_err(|e| format!("UI failed: {:?}", e))?; 
    }
    Ok(())
}

/// The main function
#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {

    let matches = App::new("coinlive")
        .about("Live cryptocurrency prices CLI")
        .version(VERSION)
        .author("Mayer Analytics. https://github.com/mayeranalytics/coinlive")
        .arg(Arg::with_name("version")
            .short("V")
            .long("version")
            .help("Print version information and exit.")
            .takes_value(false))
        .get_matches();

    if matches.is_present("version") {
        println!("{}", VERSION);
        return Ok(());
    }

    // terminal raw mode to allow reading stdin one key at a time
    let stdout = io::stdout().into_raw_mode().unwrap();
    let backend = TermionBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    let ui = UI::new(terminal);

    tokio::spawn(get_symbols_async(ui.tx.clone()));

    let listen_keys_handle = tokio::spawn(listen_keys(ui.tx.clone()));

    ui.tx.send(Msg::Msg(String::from("Starting stream... "))).await?;
    let ws_task = tokio::spawn(ws(URI_WS_TICKER, ui.tx));

    future::select(ws_task, future::select(ui.handle, listen_keys_handle)).await;
    Ok(())
}


#[tokio::test]
async fn test_get_infos() -> Result<(), Box<dyn std::error::Error>> {
    let infos = get_infos().await?;
    assert!(infos.len()>0);
    Ok(())
}