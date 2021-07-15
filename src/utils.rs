//! Various utility functions for getting and further processing of symbols, tickers, 
//! websocket updates and klines obtained from Binance
#![allow(dead_code)]

use http_req::request;
use serde::{Deserialize};
use std::ops::Deref;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use dec::Decimal64;
use inlinable_string::{InlineString};

/// Parse a String into a `Decimal64`, chop off superfluous zeros
// todo: Make this return Result
pub fn parse_dec(s: &String) -> Decimal64 {
    if let Some(_) = s.find(".") {
        s.trim_end_matches("0").parse().expect("parse_dec: Couldn't parse!")
    } else {
        s.parse().expect("parse_dec: Couldn't parse!")
    }
}

/// Nicely format a `Decimal64`
// todo: move this into `Nice`
pub fn fmt_dec(d: Decimal64) -> String {
    if d.is_infinite() {
        String::from("-")
    } else if d.is_nan() || d.is_signaling_nan() {
        String::from("-")
    } else {
        let mut u = d.coefficient();
        let mut e = d.exponent();
        let mut di = d.digits() as i32;
        while (u/10)*10 == u && u!=0 {
            u /= 10;
            e += 1;
            di -= 1;
        }
        if e+di < 0 { // slash notation
            format!("{}\\{}", -di-e+1, u)
        } else {
            format!("{}", d)
        }
    }
}

/// String type for symbol
pub type Symbol = InlineString;

/// `Info` contains symbol, base, quote and precision
#[derive(Debug, Clone)]
pub struct Info {
    pub symbol: Symbol,
    pub base: Symbol,
    pub quote: Symbol,
    pub volume: Decimal64,
}

impl Info {
    pub fn short_symbol(self: &Self) -> &InlineString {
        if self.quote == "USDT" { &self.base }
        else                { &self.symbol }
    }
}

/// Subset of data returned by api/v3/exchangeInfo, for deserialisation only
#[derive(Debug, Clone, Deserialize)]
struct MarketInfo {
    symbols: Vec<MarketInfoSymbol>
}

/// Subset of data returned by api/v3/exchangeInfo, for deserialisation only
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketInfoSymbol {
    symbol: String,
    status: String,
    base_asset: String,
    quote_asset: String

}

/// Get all traded binance symbols (unsorted)
fn _get_infos() -> Result<HashMap<Symbol, Info>, Box<dyn std::error::Error>> {
    let mut writer = Vec::with_capacity(3000000);   // exchangeInfo size is <2MB usually
    if !request::get("https://api.binance.com/api/v3/exchangeInfo", &mut writer)?.status_code().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Req api/v3/exchangeInfo failed")));
    }
    let cow = String::from_utf8_lossy(&writer);
    let market_info: MarketInfo = serde_json::from_str(cow.deref())?;
    let mut out = HashMap::<Symbol, Info>::new();
    for sym in market_info.symbols.iter() {
        if sym.status == "TRADING" {
            let symbol = InlineString::from(sym.symbol.as_str());
            let base = InlineString::from(sym.base_asset.as_str());
            let quote = InlineString::from(sym.quote_asset.as_str());
            out.insert(symbol.clone(),  Info { symbol: symbol, base: base, quote: quote, volume: Decimal64::NAN});
        }    
    } 
    Ok(out)
}

/// Market information subset as retrieved by API GET /api/v3/ticker/24hr
#[derive(Debug)]
pub struct Market {
    pub price: Decimal64,
    pub volume: Decimal64,
    pub price_change: Decimal64,
}

/// Subset of data returned by api/v3/ticker/24hr, for deserialisation only
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ticker {
    symbol: String,
    price_change: String,
    quote_volume: String,
    last_price: String
}

/// Get all traded binance symbols
pub fn get_markets<'a>() -> Result<HashMap<Symbol, Market>, Box<dyn std::error::Error>> {
    let mut writer = Vec::with_capacity(1500000);   // 24hr size is <1MB usually
    if !request::get("https://api.binance.com/api/v3/ticker/24hr", &mut writer)?.status_code().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Req api/v3/ticker/24hr failed")));
    }
    let cow = String::from_utf8_lossy(&writer);
    let tickers: Vec<Ticker> = serde_json::from_str(cow.deref())?;
    let mut out = HashMap::<Symbol, Market>::new();
    for ticker in tickers.iter() {
        let symbol = InlineString::from(ticker.symbol.as_str());
        let price_change: Decimal64 = ticker.price_change.parse()?;
        let vol: Decimal64 = ticker.quote_volume.parse()?;
        let px: Decimal64 = ticker.last_price.parse()?;
        if vol.is_positive() {
            let mkt = Market { price: px, volume: vol, price_change: price_change };
            out.insert(symbol, mkt);
        }
    }
    Ok(out)
}

/// Get all traded binance symbols sorted by trading volume (in USDT)
pub async fn get_infos() -> Result<Vec<Info>, String> {
    let infos = _get_infos().map_err(|e| format!("Get infos failed: {:?}", e))?;
    let markets = get_markets().map_err(|e| format!("Get markets failed: {:?}", e))?;
    let mut out = Vec::<Info>::new();
    for (symbol, mut info) in infos.into_iter() {
        if let Some(market) = markets.get(&symbol) {
            // if the quote ccy is not USDT we try to convert the volume to USDT 
            if info.quote != "USDT" {
                let mut usdt_sym = info.quote.clone();
                usdt_sym.push_str("USDT").map_err(|e| format!("{:?}", e))?;
                if let Some(mkt2) = markets.get(&usdt_sym) {
                    info.volume = market.volume * mkt2.price;
                    out.push(info);
                }
            } else {
                info.volume = market.volume;
                out.push(info);
            }
        }
    }
    Ok(out)
}

#[tokio::test]
async fn test_get_infos() -> Result<(), Box<dyn std::error::Error>> {
    let infos = get_infos().await?;
    assert!(infos.len()>0);
    Ok(())
}

/// Sort [`Vec`] of [`Info`] by trading volume descending
pub fn sort_infos(mut infos: Vec<Info>) -> Vec<Info> {
    infos.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    infos
}
 
/// Generate a [`Symbol`]->[`Info`] [`HashMap`] from a `Vec<Symbol>`
pub fn infos_to_lookup(infos: &Vec<Info>) -> HashMap<Symbol, Info> {
    infos.iter().map(|item| (item.symbol.clone(), item.clone())).into_iter().collect()
}

/// Extract [`Vec`] of base strings and quote strings from [`Vec`] of [`Info`], sort by volume
pub fn sort_base_quote(infos: &Vec<Info>) -> (Vec<Symbol>, Vec<Symbol>) {
    let mut bases: HashMap<Symbol, Decimal64> = HashMap::new();
    let mut quotes: HashMap<Symbol, Decimal64> = HashMap::new();
    for info in infos.iter() {
        if info.base == "USDT" { continue; }
        let vol = bases.entry(info.base.clone()).or_insert(Decimal64::from(0));
        *vol = *vol+info.volume;
        let vol = quotes.entry(info.quote.clone()).or_insert(Decimal64::from(0));
        *vol = *vol+info.volume;
    }
    let mut bases: Vec<(Symbol, &Decimal64)> = bases.iter().map(|(k,v)| (k.clone(),v)).collect();
    bases.sort_by(|a,b| b.1.partial_cmp(a.1).unwrap());
    let bases: Vec<Symbol> = bases.iter().map(|(k,_)| InlineString::from((*k).clone())).collect();
    let mut quotes: Vec<(Symbol, &Decimal64)> = quotes.iter().map(|(k,v)| (k.clone(),v)).collect();
    quotes.sort_by(|a,b| b.1.partial_cmp(a.1).unwrap());
    let quotes: Vec<Symbol> = quotes.iter().map(|(k,_)| InlineString::from((*k).clone())).collect();
    (bases, quotes)
}

/// A single ohlcv bar 
pub struct Bar {
    pub t: u64, // open time
    pub o: f32,
    pub h: f32,
    pub l: f32,
    pub c: f32,
    pub v: f32
}

/// Kline/Candlestick chart intervals.
/// 
/// See: https://binance-docs.github.io/apidocs/spot/en/#kline-candlestick-streams
#[derive(Debug)]
pub enum Interval {
    I1m, I3m, I5m, I15m, I30m, I1h, I2h, I4h, I6h, I8h, I12h, I1d, I3d, I1w, I1M
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::I1m  => write!(f,  "1m"),
            Interval::I3m  => write!(f,  "3m"),
            Interval::I5m  => write!(f,  "5m"),
            Interval::I15m => write!(f, "15m"),
            Interval::I30m => write!(f, "30m"),
            Interval::I1h  => write!(f,  "1h"),
            Interval::I2h  => write!(f,  "2h"),
            Interval::I4h  => write!(f,  "4h"),
            Interval::I6h  => write!(f,  "6h"),
            Interval::I8h  => write!(f,  "8h"),
            Interval::I12h => write!(f, "12h"),
            Interval::I1d  => write!(f,  "1d"),
            Interval::I3d  => write!(f,  "3d"),
            Interval::I1w  => write!(f,  "1w"),
            Interval::I1M  => write!(f,  "1M"),
        }
    }
}

impl Interval {
    /// `Interval` length in seconds. Approximate value for 1M.
    pub fn seconds(self: &Self) -> u32 {
        match self {
            Interval::I1m  => 60,
            Interval::I3m  => 60*3,
            Interval::I5m  => 60*5,
            Interval::I15m => 60*15,
            Interval::I30m => 60*30,
            Interval::I1h  => 60*60,
            Interval::I2h  => 60*60*2,
            Interval::I4h  => 60*60*4,
            Interval::I6h  => 60*60*6,
            Interval::I8h  => 60*60*8,
            Interval::I12h => 60*60*12,
            Interval::I1d  => 60*60*24,
            Interval::I3d  => 60*60*24*3,
            Interval::I1w  => 60*60*24*7,
            Interval::I1M  => 60*60*24*30, // !approx
        }
    }
    pub fn str(self: &Self) -> &str {
        match self {
            Interval::I1m  => "1m",
            Interval::I3m  => "3m",
            Interval::I5m  => "5m",
            Interval::I15m => "15m",
            Interval::I30m => "30m",
            Interval::I1h  => "1h",
            Interval::I2h  => "2h",
            Interval::I4h  => "4h",
            Interval::I6h  => "6h",
            Interval::I8h  => "8h",
            Interval::I12h => "12h",
            Interval::I1d  => "1d",
            Interval::I3d  => "3d",
            Interval::I1w  => "1w",
            Interval::I1M  => "1M",
        }
    }
}

/// Binance encodes a bar as a vector of various things, here are their types
type BinanceBar = (
    i64, String, String, String, String, String,
    i64, String, i64, String, String, String
);

/// helper function for `get_klines`
fn parse_bar(bbar: &BinanceBar) -> Result<Bar, Box<dyn std::error::Error>> {
    Ok(Bar{
        t: bbar.0 as u64, 
        o: bbar.1.parse()?, 
        h: bbar.2.parse()?, 
        l: bbar.3.parse()?, 
        c: bbar.4.parse()?, 
        v: bbar.5.parse()?
    })
}

/// Kline/candlestick bars for a symbol.
///  
/// See: https://binance-docs.github.io/apidocs/spot/en/#kline-candlestick-data
pub async fn get_klines(symbol: &Symbol, interval: &Interval) -> Result<Vec<Bar>, Box<dyn std::error::Error>> {
    let uri = format!("https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit=1000", symbol, interval);
    let mut writer = Vec::with_capacity(200000);   // klines size is <100kB usually
    if !request::get(uri, &mut writer)?.status_code().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Req api/v3/ticker/24hr failed")));
    }
    let cow = String::from_utf8_lossy(&writer);
    let bars: Vec<BinanceBar> = serde_json::from_str(cow.deref())?;
    let mut out: Vec<Bar> = Vec::with_capacity(1000);
    for bbar in bars.iter() {
        let bar = parse_bar(bbar)?;
        out.push(bar);
    }
    Ok(out)
}

/// A single update item from the markets websocket stream
#[derive(Debug, Clone)]
pub struct Update {
    pub symbol: Symbol,    // Exchange symbol
    pub ts: u64,           // timestamp (millis)
    pub px: Decimal64,     // price update
    pub px_24h: Decimal64, // price 24h ago
}

/// A single update item from the markets websocket stream FOR DESER PURPOSES
#[derive(Debug, Clone, Deserialize)]
struct BinanceUpdate {
    #[serde(alias = "E")]
    ts: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "x")]
    px_24h: String,
    #[serde(alias = "c")]
    px: String
}

/// Parse a ws stream message with updates (i.e. `Vec<BinanceUpdate>`)
///
/// See: https://binance-docs.github.io/apidocs/spot/en/#all-market-tickers-stream
pub fn parse_updates<'a>(s: &String, out: &'a mut Vec::<Update>) -> Result<&'a Vec<Update>, Box<dyn std::error::Error>> {
    let updates: Vec<BinanceUpdate> = serde_json::from_str(s.as_str())?;
    for update in updates.iter() {
        let ts = update.ts;
        let symbol = InlineString::from(update.symbol.as_str());
        let px_24h:Decimal64 = parse_dec(&update.px_24h);
        let px:Decimal64 = parse_dec(&update.px);
        out.push(Update{symbol: symbol, ts: ts as u64, px: px, px_24h: px_24h});
    }
    Ok(out)
}

/// Get system timestamp in microseconds
pub fn now_timestamp() -> u64 {
    let ts = SystemTime::now();
    ts.duration_since(UNIX_EPOCH).expect("System clock is messed up!").as_millis() as u64
}
