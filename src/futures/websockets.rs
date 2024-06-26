use crate::errors::*;
use crate::config::*;
use crate::model::*;
use crate::futures::model;
use url::Url;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::TcpStream;

use std::time::Duration;

use native_tls::{TlsConnector, TlsStream};
use tungstenite::client;

use tungstenite::Message;
use tungstenite::protocol::WebSocket;
use tungstenite::handshake::client::Response;
#[allow(clippy::all)]
enum FuturesWebsocketAPI {
    Default,
    MultiStream,
    Custom(String),
}

pub enum FuturesMarket {
    USDM,
    COINM,
    Vanilla,
}

impl FuturesWebsocketAPI {
    fn params(self, market: FuturesMarket, subscription: &str) -> String {
        let baseurl = match market {
            FuturesMarket::USDM => "wss://fstream.binance.com",
            FuturesMarket::COINM => "wss://dstream.binance.com",
            FuturesMarket::Vanilla => "wss://vstream.binance.com",
        };

        match self {
            FuturesWebsocketAPI::Default => {
                format!("{}/ws/{}", baseurl, subscription)
            }
            FuturesWebsocketAPI::MultiStream => {
                format!("{}/stream?streams={}", baseurl, subscription)
            }
            FuturesWebsocketAPI::Custom(url) => url,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FuturesWebsocketEvent {
    AccountUpdate(AccountUpdateEvent),
    OrderTrade(model::OrderTradeEvent),
    AggrTrades(AggrTradesEvent),
    Trade(TradeEvent),
    OrderBook(OrderBook),
    DayTicker(DayTickerEvent),
    MiniTicker(MiniTickerEvent),
    MiniTickerAll(Vec<MiniTickerEvent>),
    IndexPrice(IndexPriceEvent),
    MarkPrice(MarkPriceEvent),
    MarkPriceAll(Vec<MarkPriceEvent>),
    DayTickerAll(Vec<DayTickerEvent>),
    Kline(KlineEvent),
    ContinuousKline(ContinuousKlineEvent),
    IndexKline(IndexKlineEvent),
    Liquidation(LiquidationEvent),
    DepthOrderBook(DepthOrderBookEvent),
    BookTicker(BookTickerEvent),
    UserDataStreamExpiredEvent(UserDataStreamExpiredEvent),
}

pub struct FuturesWebSockets<'a> {
    pub socket: Option<(WebSocket<TlsStream<TcpStream>>, Response)>,
    handler: Box<dyn FnMut(FuturesWebsocketEvent) -> Result<()> + 'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FuturesEvents {
    Vec(Vec<DayTickerEvent>),
    DayTickerEvent(DayTickerEvent),
    BookTickerEvent(BookTickerEvent),
    MiniTickerEvent(MiniTickerEvent),
    VecMiniTickerEvent(Vec<MiniTickerEvent>),
    AccountUpdateEvent(AccountUpdateEvent),
    OrderTradeEvent(model::OrderTradeEvent),
    AggrTradesEvent(AggrTradesEvent),
    IndexPriceEvent(IndexPriceEvent),
    MarkPriceEvent(MarkPriceEvent),
    VecMarkPriceEvent(Vec<MarkPriceEvent>),
    TradeEvent(TradeEvent),
    KlineEvent(KlineEvent),
    ContinuousKlineEvent(ContinuousKlineEvent),
    IndexKlineEvent(IndexKlineEvent),
    LiquidationEvent(LiquidationEvent),
    OrderBook(OrderBook),
    DepthOrderBookEvent(DepthOrderBookEvent),
    UserDataStreamExpiredEvent(UserDataStreamExpiredEvent),
}

impl<'a> FuturesWebSockets<'a> {
    pub fn new<Callback>(handler: Callback) -> FuturesWebSockets<'a>
    where
        Callback: FnMut(FuturesWebsocketEvent) -> Result<()> + 'a,
    {
        FuturesWebSockets {
            socket: None,
            handler: Box::new(handler),
        }
    }

    pub fn connect<T: Into<String>>(&mut self, market: FuturesMarket, subscription: T) -> Result<()> {
        self.connect_wss(FuturesWebsocketAPI::Default.params(market, &subscription.into()))
    }

    pub fn connect_with_config<T: Into<String>>(
        &mut self, market: FuturesMarket, subscription: T, config: &Config,
    ) -> Result<()> {
        self.connect_wss(
            FuturesWebsocketAPI::Custom(config.ws_endpoint.clone()).params(market, &subscription.into()),
        )
    }

    pub fn connect_multiple_streams(
        &mut self, market: FuturesMarket, endpoints: &[String],
    ) -> Result<()> {
        self.connect_wss(FuturesWebsocketAPI::MultiStream.params(market, &endpoints.join("/")))
    }

    fn connect_wss(&mut self, wss: String) -> Result<()> {
        let url = Url::parse(&wss)?;
        let host = url.host_str().unwrap();
        let port = url.port().unwrap_or(443);
        let connector = TlsConnector::new()?;
        let tcp_stream = TcpStream::connect((host, port))?;
        tcp_stream.set_read_timeout(Some(Duration::from_secs(10)))?; // Установите желаемый таймаут
        let tls_stream = connector.connect(host, tcp_stream)?;
        let (socket, response) = client(url, tls_stream)?;
        self.socket = Some((socket, response));
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None)?;
            return Ok(());
        }
        bail!("Not able to close the connection");
    }

    pub fn test_handle_msg(&mut self, msg: &str) -> Result<()> {
        self.handle_msg(msg)
    }

    fn handle_msg(&mut self, msg: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(msg)?;

        if let Some(data) = value.get("data") {
            self.handle_msg(&data.to_string())?;
            return Ok(());
        }


        if let Ok(events) = serde_json::from_value::<FuturesEvents>(value) {
            let action = match events {
                FuturesEvents::Vec(v) => FuturesWebsocketEvent::DayTickerAll(v),
                FuturesEvents::DayTickerEvent(v) => FuturesWebsocketEvent::DayTicker(v),
                FuturesEvents::BookTickerEvent(v) => FuturesWebsocketEvent::BookTicker(v),
                FuturesEvents::MiniTickerEvent(v) => FuturesWebsocketEvent::MiniTicker(v),
                FuturesEvents::VecMiniTickerEvent(v) => FuturesWebsocketEvent::MiniTickerAll(v),
                FuturesEvents::AccountUpdateEvent(v) => FuturesWebsocketEvent::AccountUpdate(v),
                FuturesEvents::OrderTradeEvent(v) => FuturesWebsocketEvent::OrderTrade(v),
                FuturesEvents::IndexPriceEvent(v) => FuturesWebsocketEvent::IndexPrice(v),
                FuturesEvents::MarkPriceEvent(v) => FuturesWebsocketEvent::MarkPrice(v),
                FuturesEvents::VecMarkPriceEvent(v) => FuturesWebsocketEvent::MarkPriceAll(v),
                FuturesEvents::TradeEvent(v) => FuturesWebsocketEvent::Trade(v),
                FuturesEvents::ContinuousKlineEvent(v) => FuturesWebsocketEvent::ContinuousKline(v),
                FuturesEvents::IndexKlineEvent(v) => FuturesWebsocketEvent::IndexKline(v),
                FuturesEvents::LiquidationEvent(v) => FuturesWebsocketEvent::Liquidation(v),
                FuturesEvents::KlineEvent(v) => FuturesWebsocketEvent::Kline(v),
                FuturesEvents::OrderBook(v) => FuturesWebsocketEvent::OrderBook(v),
                FuturesEvents::DepthOrderBookEvent(v) => FuturesWebsocketEvent::DepthOrderBook(v),
                FuturesEvents::AggrTradesEvent(v) => FuturesWebsocketEvent::AggrTrades(v),
                FuturesEvents::UserDataStreamExpiredEvent(v) => {
                    FuturesWebsocketEvent::UserDataStreamExpiredEvent(v)
                }
            };
            (self.handler)(action)?;
        }
        Ok(())
    }



    pub fn event_loop(&mut self, should_stop: &AtomicBool) -> Result<()> {
        let mut ping_counter = 0;

        while !should_stop.load(Ordering::Relaxed) {
            if let Some(ref mut socket) = self.socket {
                let message = socket.0.read_message();
                match message {
                    Ok(message) => match message {
                        Message::Text(msg) => {
                            if let Err(e) = self.handle_msg(&msg) {
                                bail!(format!("Error on handling stream message: {}", e));
                            }
                        }
                        Message::Ping(payload) => {
                            socket.0.write_message(Message::Pong(payload)).unwrap();
                        }
                        Message::Pong(_) => {
                            ping_counter = 0;
                        }
                        Message::Binary(_) => (),
                        Message::Close(e) => bail!(format!("Disconnected {:?}", e)),
                    },
                    Err(error) => {
                        // Таймаут истек; вы можете обработать эту ситуацию, например, закрыть соединение
                        // отправляем 3 пинга если нет ответа - ошибка
                        if let Err(err) = socket.0.write_message(Message::Ping(vec![])){
                             bail!(format!("Disconnected loop is dead {err:?} {error:?}"));
                        };
                        ping_counter += 1;

                        if ping_counter >= 10{
                            bail!(format!("Disconnected loop is dead {error}"));
                        }
                    }
                }
            }
        }
        bail!("running loop closed");
    }
}
